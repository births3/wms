async fn publish_template_for_scope(
    repo: &PgPrintTemplateRepository,
    pool: &PgPool,
    auth: &AuthContext,
    field_library_version_id: Uuid,
    template_code: &str,
    scope: PrintTemplateScope,
    is_default: bool,
    now: chrono::DateTime<Utc>,
) -> wms_api::print_template::PrintTemplateVersion {
    let mut request = template_request(field_library_version_id);
    request.template_code = template_code.to_string();
    request.template_name = format!("{template_code} 模板");
    request.scope = scope;
    request.is_default = is_default;
    let draft = repo
        .save_template(
            pool,
            auth,
            request,
            now,
            &format!("{template_code}-save"),
        )
        .await
        .expect("template should save");
    repo.publish_template_draft(
        pool,
        auth,
        draft.value.template_id,
        draft.value.id,
        now + chrono::Duration::seconds(1),
        &format!("{template_code}-publish"),
    )
    .await
    .expect("template should publish")
    .value
}

#[sqlx::test(migrations = "../../migrations")]
async fn template_resolution_uses_explicit_owner_global_order_without_cross_owner_fallback(
    pool: PgPool,
) {
    let repo = PgPrintTemplateRepository::new();
    let global_admin = ctx(Uuid::new_v4());
    let owner = ctx(Uuid::new_v4());
    let other_owner = ctx(Uuid::new_v4());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 18, 0, 0)
        .single()
        .expect("valid time");
    let library = published_library(
        &repo,
        &pool,
        &global_admin,
        "ASN 号",
        now,
        "h9-business-resolution-library",
    )
    .await;
    let global = publish_template_for_scope(
        &repo,
        &pool,
        &global_admin,
        library.id,
        "asn_global_default",
        PrintTemplateScope::Global,
        true,
        now + chrono::Duration::minutes(1),
    )
    .await;

    let global_fallback = repo
        .resolve_template(
            &pool,
            &owner,
            wms_api::print_template::ResolvePrintTemplateRequest {
                template_code: None,
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
            },
        )
        .await
        .expect("owner without override should use global default");
    assert_eq!(global_fallback.version.id, global.id);

    let owner_default = publish_template_for_scope(
        &repo,
        &pool,
        &owner,
        library.id,
        "asn_owner_default",
        PrintTemplateScope::Owner,
        true,
        now + chrono::Duration::minutes(2),
    )
    .await;
    let resolved_owner = repo
        .resolve_template(
            &pool,
            &owner,
            wms_api::print_template::ResolvePrintTemplateRequest {
                template_code: None,
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
            },
        )
        .await
        .expect("owner default should win");
    assert_eq!(resolved_owner.version.id, owner_default.id);

    let explicit_global = repo
        .resolve_template(
            &pool,
            &owner,
            wms_api::print_template::ResolvePrintTemplateRequest {
                template_code: Some("asn_global_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
            },
        )
        .await
        .expect("explicit global template should resolve");
    assert_eq!(explicit_global.version.id, global.id);

    publish_template_for_scope(
        &repo,
        &pool,
        &other_owner,
        library.id,
        "asn_other_non_default",
        PrintTemplateScope::Owner,
        false,
        now + chrono::Duration::minutes(3),
    )
    .await;
    let other_fallback = repo
        .resolve_template(
            &pool,
            &other_owner,
            wms_api::print_template::ResolvePrintTemplateRequest {
                template_code: None,
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
            },
        )
        .await
        .expect("non-default owner template must not shadow the global default");
    assert_eq!(other_fallback.version.id, global.id);

    let cross_owner = repo
        .resolve_template(
            &pool,
            &other_owner,
            wms_api::print_template::ResolvePrintTemplateRequest {
                template_code: Some("asn_owner_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
            },
        )
        .await
        .expect_err("another owner's template must not resolve");
    assert_eq!(
        cross_owner,
        wms_api::print_template::PrintTemplateError::TemplateNotFound
    );

    repo.set_template_enabled(
        &pool,
        &owner,
        owner_default.template_id,
        false,
        now + chrono::Duration::minutes(4),
        "asn-owner-disable",
    )
    .await
    .expect("owner template should disable");
    let disabled = repo
        .resolve_template(
            &pool,
            &owner,
            wms_api::print_template::ResolvePrintTemplateRequest {
                template_code: Some("asn_owner_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
            },
        )
        .await
        .expect_err("explicit disabled template must not fall back");
    assert_eq!(
        disabled,
        wms_api::print_template::PrintTemplateError::TemplateDisabled
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn resolve_http_reports_unpublished_library_and_runtime_field_mismatch(pool: PgPool) {
    let repo = PgPrintTemplateRepository::new();
    let owner_id = Uuid::new_v4();
    let write = ctx(owner_id);
    let read = ctx_with_permissions(owner_id, &["h9.print_template.read"]);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 19, 0, 0)
        .single()
        .expect("valid time");
    ensure_audit_partition(&pool, now).await;
    let openapi = serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI should serialize");
    let draft_library = repo
        .generate_field_library_draft(
            &pool,
            &write,
            field_library_request(),
            &openapi,
            now,
            "h9-unpublished-resolve-library",
        )
        .await
        .expect("field library draft should generate")
        .value;
    let template_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO print_templates (
            id, owner_id, template_code, template_name, template_type_code,
            scope, enabled, is_default, created_by, updated_by
        )
        VALUES ($1, $2, 'asn_unpublished', '未发布字段库模板', $3,
                'owner', TRUE, TRUE, $4, $4)
        "#,
    )
    .bind(template_id)
    .bind(owner_id)
    .bind(PRINT_TEMPLATE_TYPE_ASN)
    .bind(write.user_id)
    .execute(&pool)
    .await
    .expect("invalid runtime template fixture should insert");
    sqlx::query(
        r#"
        INSERT INTO print_template_versions (
            id, template_id, field_library_version_id, template_name,
            template_type_code, scope, is_default, version_no, status,
            hiprint_json, field_bindings, paper, designer_version, request_hash,
            created_by, published_at, published_by
        )
        VALUES (
            $1, $2, $3, '未发布字段库模板', $4, 'owner', TRUE, 1, 'published',
            '{"panels":[]}'::jsonb, '[]'::jsonb, '{}'::jsonb,
            'hiprint@0.4.0', 'invalid-unpublished-library', $5, $6, $5
        )
        "#,
    )
    .bind(version_id)
    .bind(template_id)
    .bind(draft_library.id)
    .bind(PRINT_TEMPLATE_TYPE_ASN)
    .bind(write.user_id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("invalid runtime version fixture should insert");

    let unpublished = print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
        .layer(Extension(read.clone()))
        .oneshot(json_request(
            "POST",
            "/api/v1/print-templates/resolve",
            None,
            json!({
                "template_code": "asn_unpublished",
                "template_type_code": PRINT_TEMPLATE_TYPE_ASN,
            }),
        ))
        .await
        .expect("resolve route should respond");
    assert_eq!(unpublished.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(unpublished).await["code"],
        "H9_FIELD_LIBRARY_NOT_PUBLISHED"
    );

    sqlx::query(
        r#"
        UPDATE print_field_library_versions
           SET status = 'published', published_at = $1, published_by = $2
         WHERE id = $3
        "#,
    )
    .bind(now)
    .bind(write.user_id)
    .bind(draft_library.id)
    .execute(&pool)
    .await
    .expect("test library should publish");
    sqlx::query(
        r#"
        UPDATE system_dictionary_items
           SET params = jsonb_set(params, '{field_library_code}', '"other_library"')
         WHERE dict_code = $1
           AND item_code = $2
           AND owner_id IS NULL
        "#,
    )
    .bind(SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE)
    .bind(PRINT_TEMPLATE_TYPE_ASN)
    .execute(&pool)
    .await
    .expect("dictionary field library binding should change");

    let mismatch = print_template_router(PrintTemplateAppState::with_postgres(pool))
        .layer(Extension(read))
        .oneshot(json_request(
            "POST",
            "/api/v1/print-templates/resolve",
            None,
            json!({
                "template_code": "asn_unpublished",
                "template_type_code": PRINT_TEMPLATE_TYPE_ASN,
            }),
        ))
        .await
        .expect("resolve route should respond");
    assert_eq!(mismatch.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response_json(mismatch).await["code"],
        "H9_TEMPLATE_FIELD_MISMATCH"
    );
}

async fn seed_business_template(
    pool: &PgPool,
    actor_id: Uuid,
    template_type_code: &str,
    library_code: &str,
    business_module: &str,
    source_schema: &str,
    field_path: &str,
) {
    let library_id = Uuid::new_v4();
    let library_version_id = Uuid::new_v4();
    let template_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO print_field_libraries (
            id, library_code, library_name, business_module, source_schema
        )
        VALUES ($1, $2, $2, $3, $4)
        "#,
    )
    .bind(library_id)
    .bind(library_code)
    .bind(business_module)
    .bind(source_schema)
    .execute(pool)
    .await
    .expect("business field library should insert");
    sqlx::query(
        r#"
        INSERT INTO print_field_library_versions (
            id, library_id, version_no, status, source_schema,
            business_module, request_hash, created_by
        )
        VALUES ($1, $2, 1, 'draft', $3, $4, $5, $6)
        "#,
    )
    .bind(library_version_id)
    .bind(library_id)
    .bind(source_schema)
    .bind(business_module)
    .bind(format!("{library_code}-v1"))
    .bind(actor_id)
    .execute(pool)
    .await
    .expect("business field library draft should insert");
    sqlx::query(
        r#"
        INSERT INTO print_field_definitions (
            id, library_version_id, field_path, field_type, source_schema,
            display_name, group_code, group_name
        )
        VALUES ($1, $2, $3, 'string', $4, $3, 'business', '业务信息')
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(library_version_id)
    .bind(field_path)
    .bind(source_schema)
    .execute(pool)
    .await
    .expect("business print field should insert");
    sqlx::query(
        r#"
        UPDATE print_field_library_versions
           SET status = 'published', published_at = now(), published_by = $1
         WHERE id = $2
        "#,
    )
    .bind(actor_id)
    .bind(library_version_id)
    .execute(pool)
    .await
    .expect("business field library should publish");
    sqlx::query(
        r#"
        INSERT INTO print_templates (
            id, owner_id, template_code, template_name, template_type_code,
            scope, enabled, is_default, created_by, updated_by
        )
        VALUES (
            $1, $2, $3, $3, $4, 'global', TRUE, TRUE, $5, $5
        )
        "#,
    )
    .bind(template_id)
    .bind(Uuid::nil())
    .bind(format!("{template_type_code}_default"))
    .bind(template_type_code)
    .bind(actor_id)
    .execute(pool)
    .await
    .expect("business template should insert");
    sqlx::query(
        r#"
        INSERT INTO print_template_versions (
            id, template_id, field_library_version_id, template_name,
            template_type_code, scope, is_default, version_no, status,
            hiprint_json, field_bindings, paper, designer_version, request_hash,
            created_by, published_at, published_by
        )
        VALUES (
            $1, $2, $3, $4, $5, 'global', TRUE, 1, 'published',
            jsonb_build_object(
                'panels', jsonb_build_array(
                    jsonb_build_object(
                        'printElements', jsonb_build_array(
                            jsonb_build_object('options', jsonb_build_object('field', $6))
                        )
                    )
                )
            ),
            jsonb_build_array(jsonb_build_object('field_path', $6, 'required', TRUE)),
            '{}'::jsonb, 'hiprint@0.4.0', $7, $8, now(), $8
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(template_id)
    .bind(library_version_id)
    .bind(format!("{template_type_code}_default"))
    .bind(template_type_code)
    .bind(field_path)
    .bind(format!("{template_type_code}-template-v1"))
    .bind(actor_id)
    .execute(pool)
    .await
    .expect("business template version should insert");
}

#[sqlx::test(migrations = "../../migrations")]
async fn six_business_template_types_resolve_preview_and_record_through_one_contract(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let auth = ctx_with_permissions(owner_id, &["h9.print_template.print"]);
    let cases = [
        (
            PRINT_TEMPLATE_TYPE_ASN,
            "m2_asn",
            "M2",
            "ReceivingOrderPrintData",
            "order.receipt_no",
            json!({ "order": { "receipt_no": "ASN-H9-005" } }),
        ),
        (
            PRINT_TEMPLATE_TYPE_ACCEPTANCE_RECORD,
            "m2_acceptance_record",
            "M2",
            "ReceivingOrderPrintData",
            "order.receipt_no",
            json!({ "order": { "receipt_no": "ASN-H9-005" } }),
        ),
        (
            PRINT_TEMPLATE_TYPE_DELIVERY_NOTE,
            "m4_delivery_note",
            "M4",
            "OutboundOrder",
            "wms_order_no",
            json!({ "wms_order_no": "SO-H9-005" }),
        ),
        (
            PRINT_TEMPLATE_TYPE_LOCATION_LABEL,
            "m1_location_label",
            "M1",
            "Location",
            "location_code",
            json!({ "location_code": "A01-01-01" }),
        ),
        (
            PRINT_TEMPLATE_TYPE_LPN_LABEL,
            "m3_lpn_label",
            "M3",
            "InventoryBatch",
            "container_lpn",
            json!({ "container_lpn": "LPN-H9-005" }),
        ),
        (
            PRINT_TEMPLATE_TYPE_PRODUCT_LABEL,
            "m1_product_label",
            "M1",
            "Product",
            "product_code",
            json!({ "product_code": "P-H9-005" }),
        ),
    ];
    for (template_type, library_code, module, schema, field_path, data) in cases {
        seed_business_template(
            &pool,
            auth.user_id,
            template_type,
            library_code,
            module,
            schema,
            field_path,
        )
        .await;
        let template_code = format!("{template_type}_default");
        let resolve = print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
            .layer(Extension(auth.clone()))
            .oneshot(json_request(
                "POST",
                "/api/v1/print-templates/resolve",
                None,
                json!({
                    "template_code": null,
                    "template_type_code": template_type,
                }),
            ))
            .await
            .expect("resolve route should respond");
        assert_eq!(resolve.status(), StatusCode::OK, "{template_type}");
        assert_eq!(
            response_json(resolve).await["template"]["template_code"],
            template_code
        );

        let preview = print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
            .layer(Extension(auth.clone()))
            .oneshot(json_request(
                "POST",
                "/api/v1/print-templates/preview",
                None,
                json!({
                    "template_code": null,
                    "template_type_code": template_type,
                    "business_document_id": format!("{template_type}-business-id"),
                    "data": data,
                }),
            ))
            .await
            .expect("preview route should respond");
        assert_eq!(preview.status(), StatusCode::OK, "{template_type}");

        let printed = print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
            .layer(Extension(auth.clone()))
            .oneshot(json_request(
                "POST",
                "/api/v1/print-templates/print",
                Some(&format!("{template_type}-print")),
                json!({
                    "template_code": template_code,
                    "template_type_code": template_type,
                    "business_module": module,
                    "business_document_type": template_type,
                    "business_document_id": format!("{template_type}-business-id"),
                    "data": data,
                    "status": "printed",
                    "failure_reason": null,
                }),
            ))
            .await
            .expect("print route should respond");
        assert_eq!(printed.status(), StatusCode::OK, "{template_type}");
    }

    let records: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM print_records WHERE owner_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("business print records should count");
    assert_eq!(records, 6);
}
