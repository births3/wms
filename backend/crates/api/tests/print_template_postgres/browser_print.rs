#[sqlx::test(migrations = "../../migrations")]
async fn browser_print_masks_sensitive_data_and_counts_retries(pool: PgPool) {
    let repo = PgPrintTemplateRepository::new();
    let owner_id = Uuid::new_v4();
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 16, 0, 0)
        .single()
        .expect("valid time");
    ensure_audit_partition(&pool, now).await;
    let openapi = serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI should serialize");
    let draft = repo
        .generate_field_library_draft(
            &pool,
            &auth,
            field_library_request(),
            &openapi,
            now,
            "h9-browser-print-library-draft",
        )
        .await
        .expect("field library draft should generate");
    let fields = repo
        .list_field_version_fields(&pool, draft.value.id)
        .await
        .expect("generated fields should list");
    for (path, display_name, table_detail) in [
        ("receipt_no", "ASN 号", false),
        ("lines[].product_code", "商品编码", true),
    ] {
        let field = fields
            .iter()
            .find(|field| field.field_path == path)
            .expect("required field should be generated");
        repo.update_field_definition(
            &pool,
            &auth,
            draft.value.id,
            field.id,
            UpdatePrintFieldDefinitionRequest {
                display_name: display_name.to_string(),
                group_code: if table_detail { "detail" } else { "order" }.to_string(),
                group_name: if table_detail {
                    "明细信息"
                } else {
                    "订单信息"
                }
                .to_string(),
                description: display_name.to_string(),
                example_value: None,
                printable: true,
                sensitive: true,
                masking_rule: Some("keep_last_4".to_string()),
                formatting_rule: None,
                supports_barcode: false,
                supports_qrcode: false,
                is_table_detail: table_detail,
                sort_order: if table_detail { 20 } else { 10 },
            },
            now,
            &format!("h9-browser-print-field-{path}"),
        )
        .await
        .expect("sensitive metadata should update");
    }
    let library = repo
        .publish_field_library_draft(
            &pool,
            &auth,
            draft.value.id,
            &openapi,
            now,
            "h9-browser-print-library-publish",
        )
        .await
        .expect("field library should publish")
        .value;
    let mut request = template_request(library.id);
    request.field_bindings.push(PrintTemplateBinding {
        field_path: "lines[].product_code".to_string(),
        required: true,
    });
    let saved = repo
        .save_template(&pool, &auth, request, now, "h9-browser-print-template-save")
        .await
        .expect("template should save");
    let published = repo
        .publish_template_draft(
            &pool,
            &auth,
            saved.value.template_id,
            saved.value.id,
            now,
            "h9-browser-print-template-publish",
        )
        .await
        .expect("template should publish")
        .value;
    let business_data = json!({
        "receipt_no": "ASN-SECRET-1234",
        "lines": [{ "product_code": "PRODUCT-SECRET-5678" }]
    });

    let preview = repo
        .preview_template(
            &pool,
            &auth,
            PrintTemplatePreviewRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
                business_document_id: "ASN-ID-001".to_string(),
                data: business_data.clone(),
            },
        )
        .await
        .expect("nested required fields should validate");
    assert_eq!(preview.data["receipt_no"], "***********1234");
    assert_eq!(
        preview.data["lines"][0]["product_code"],
        "***************5678"
    );
    let missing_nested = repo
        .preview_template(
            &pool,
            &auth,
            PrintTemplatePreviewRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
                business_document_id: "ASN-ID-001".to_string(),
                data: json!({ "receipt_no": "ASN-SECRET-1234", "lines": [{}] }),
            },
        )
        .await
        .expect_err("missing nested required field should be rejected");
    assert_eq!(
        missing_nested,
        wms_api::print_template::PrintTemplateError::TemplateFieldMissing(vec![
            "lines[].product_code".to_string()
        ])
    );

    let missing_reason = repo
        .record_print(
            &pool,
            &auth,
            PrintTemplatePrintRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
                business_module: "M2".to_string(),
                business_document_type: "asn".to_string(),
                business_document_id: "ASN-ID-001".to_string(),
                data: business_data.clone(),
                status: "failed".to_string(),
                failure_reason: None,
            },
            now,
            "h9-browser-print-missing-reason",
        )
        .await
        .expect_err("failed print requires a reason");
    assert!(matches!(
        missing_reason,
        wms_api::print_template::PrintTemplateError::InvalidRequest(_)
    ));

    let mut retry_counts = Vec::new();
    for (index, (status, reason)) in [
        ("failed", Some("浏览器打印未完成".to_string())),
        ("cancelled", None),
        ("printed", None),
    ]
    .into_iter()
    .enumerate()
    {
        let result = repo
            .record_print(
                &pool,
                &auth,
                PrintTemplatePrintRequest {
                    template_code: Some("m2_asn_default".to_string()),
                    template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
                    business_module: "M2".to_string(),
                    business_document_type: "asn".to_string(),
                    business_document_id: "ASN-ID-001".to_string(),
                    data: business_data.clone(),
                    status: status.to_string(),
                    failure_reason: reason.clone(),
                },
                now + chrono::Duration::seconds(index as i64),
                &format!("h9-browser-print-attempt-{index}"),
            )
            .await
            .expect("print result should be recorded");
        assert_eq!(result.value.template_version_id, published.id);
        assert_eq!(result.value.business_document_id, "ASN-ID-001");
        assert_eq!(result.value.operator_id, auth.user_id);
        assert_eq!(result.value.status, status);
        assert_eq!(result.value.failure_reason, reason);
        retry_counts.push(result.value.retry_count);
    }
    assert_eq!(retry_counts, vec![0, 1, 2]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn print_http_requires_print_permission_and_idempotency_key(pool: PgPool) {
    let read_only = ctx_with_permissions(Uuid::new_v4(), &["h9.print_template.read"]);
    let body = json!({
        "template_type_code": PRINT_TEMPLATE_TYPE_ASN,
        "business_module": "M2",
        "business_document_type": "asn",
        "business_document_id": "ASN-FORBIDDEN",
        "data": {},
        "status": "printed"
    });
    let forbidden = print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
        .layer(Extension(read_only))
        .oneshot(json_request(
            "POST",
            "/api/v1/print-templates/print",
            Some("h9-print-forbidden"),
            body.clone(),
        ))
        .await
        .expect("print route should respond");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let missing_key = print_template_router(PrintTemplateAppState::with_postgres(pool))
        .layer(Extension(ctx_with_permissions(
            Uuid::new_v4(),
            &["h9.print_template.print"],
        )))
        .oneshot(json_request(
            "POST",
            "/api/v1/print-templates/print",
            None,
            body,
        ))
        .await
        .expect("print route should respond");
    assert_eq!(missing_key.status(), StatusCode::BAD_REQUEST);
}
