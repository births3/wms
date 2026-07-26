use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Extension,
};
use chrono::{TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use utoipa::OpenApi;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    print_template::{
        GeneratePrintFieldLibraryDraftRequest, PgPrintTemplateRepository, PrintTemplateBinding,
        PrintTemplateError, PrintTemplateScope, SavePrintTemplateRequest,
        UpdatePrintFieldDefinitionRequest,
    },
    print_template_handlers::{print_template_router, PrintTemplateAppState},
    ApiDoc,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "print-field-library-test".to_string(),
        permissions: vec![
            "h9.print_template.read".to_string(),
            "h9.print_template.write".to_string(),
            "h9.print_template.publish".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn current_openapi() -> serde_json::Value {
    serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI should serialize")
}

fn draft_request() -> GeneratePrintFieldLibraryDraftRequest {
    GeneratePrintFieldLibraryDraftRequest {
        library_code: "m2_receiving_order".to_string(),
        library_name: "M2 收货单字段库".to_string(),
        business_module: "M2".to_string(),
        source_schema: "CreateReceivingOrderRequest".to_string(),
    }
}

fn template_request(field_library_version_id: Uuid) -> SavePrintTemplateRequest {
    SavePrintTemplateRequest {
        template_id: None,
        template_code: "m2_receiving_order_default".to_string(),
        template_name: "M2 收货单默认模板".to_string(),
        template_type_code: "asn".to_string(),
        scope: PrintTemplateScope::Global,
        is_default: true,
        remark: None,
        field_library_version_id,
        hiprint_json: json!({ "panels": [] }),
        field_bindings: vec![PrintTemplateBinding {
            field_path: "receipt_no".to_string(),
            required: true,
        }],
        paper: json!({ "paperType": "A4" }),
        designer_version: "hiprint@0.4.0".to_string(),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn openapi_all_of_references_are_included_in_the_draft(pool: PgPool) {
    let repo = PgPrintTemplateRepository::new();
    let auth = ctx(Uuid::new_v4());
    let openapi = json!({
        "components": {
            "schemas": {
                "BaseDocument": {
                    "type": "object",
                    "properties": {
                        "document_no": {
                            "type": "string",
                            "description": "单据号"
                        }
                    }
                },
                "ExtendedDocument": {
                    "allOf": [
                        { "$ref": "#/components/schemas/BaseDocument" },
                        {
                            "type": "object",
                            "properties": {
                                "document_no": {
                                    "type": "string",
                                    "description": "覆盖后的单据号"
                                },
                                "remark": {
                                    "type": "string",
                                    "description": "备注"
                                }
                            }
                        }
                    ]
                }
            }
        }
    });
    let draft = repo
        .generate_field_library_draft(
            &pool,
            &auth,
            GeneratePrintFieldLibraryDraftRequest {
                library_code: "all_of_document".to_string(),
                library_name: "组合单据字段库".to_string(),
                business_module: "H9".to_string(),
                source_schema: "ExtendedDocument".to_string(),
            },
            &openapi,
            Utc::now(),
            "h9-field-library-all-of",
        )
        .await
        .expect("allOf schema should generate a draft");
    let paths: Vec<String> = repo
        .list_field_version_fields(&pool, draft.value.id)
        .await
        .expect("allOf fields should query")
        .into_iter()
        .map(|field| field.field_path)
        .collect();
    assert_eq!(paths, vec!["document_no", "remark"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn openapi_draft_metadata_publish_versioning_and_audit_are_closed(pool: PgPool) {
    let repo = PgPrintTemplateRepository::new();
    let auth = ctx(Uuid::new_v4());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 13, 0, 0)
        .single()
        .expect("valid time");
    let openapi = current_openapi();

    let draft = repo
        .generate_field_library_draft(
            &pool,
            &auth,
            draft_request(),
            &openapi,
            now,
            "h9-field-library-draft-v1",
        )
        .await
        .expect("OpenAPI schema should generate a field library draft");
    assert_eq!(draft.value.status, "draft");
    assert_eq!(draft.value.version_no, 1);
    assert_eq!(draft.value.business_module, "M2");
    assert_eq!(draft.value.source_schema, "CreateReceivingOrderRequest");
    let draft_replay = repo
        .generate_field_library_draft(
            &pool,
            &auth,
            draft_request(),
            &openapi,
            now,
            "h9-field-library-draft-v1",
        )
        .await
        .expect("same draft request should replay");
    assert!(draft_replay.replayed);
    assert_eq!(draft_replay.value.id, draft.value.id);

    let fields = repo
        .list_field_version_fields(&pool, draft.value.id)
        .await
        .expect("generated fields should be queryable");
    let receipt_no = fields
        .iter()
        .find(|field| field.field_path == "receipt_no")
        .expect("receipt_no should be generated");
    let product_code = fields
        .iter()
        .find(|field| field.field_path == "lines[].product_code")
        .expect("nested line field should be generated");
    assert_eq!(receipt_no.field_type, "string");
    assert_eq!(receipt_no.source_schema, "CreateReceivingOrderRequest");
    assert_eq!(receipt_no.display_name, "收货单号");
    assert_eq!(receipt_no.group_name, "基本信息");
    assert_eq!(product_code.source_schema, "ReceivingOrderLine");
    assert_eq!(product_code.display_name, "商品编码");
    assert_eq!(product_code.group_name, "明细信息");
    assert!(product_code.is_table_detail);

    let updated = repo
        .update_field_definition(
            &pool,
            &auth,
            draft.value.id,
            receipt_no.id,
            UpdatePrintFieldDefinitionRequest {
                display_name: "收货单号".to_string(),
                group_code: "order".to_string(),
                group_name: "单据信息".to_string(),
                description: "仓库收货作业的业务单号".to_string(),
                example_value: Some(json!("ASN-202607260001")),
                printable: true,
                sensitive: true,
                masking_rule: Some("keep_last_4".to_string()),
                formatting_rule: Some("uppercase".to_string()),
                supports_barcode: true,
                supports_qrcode: true,
                is_table_detail: false,
                sort_order: 10,
            },
            now + chrono::Duration::minutes(1),
            "h9-field-library-field-update",
        )
        .await
        .expect("draft metadata should update");
    assert_eq!(updated.value.display_name, "收货单号");
    assert_eq!(updated.value.example_value, Some(json!("ASN-202607260001")));
    assert!(updated.value.sensitive);
    assert!(updated.value.supports_barcode);
    assert!(updated.value.supports_qrcode);
    let update_replay = repo
        .update_field_definition(
            &pool,
            &auth,
            draft.value.id,
            receipt_no.id,
            UpdatePrintFieldDefinitionRequest {
                display_name: "收货单号".to_string(),
                group_code: "order".to_string(),
                group_name: "单据信息".to_string(),
                description: "仓库收货作业的业务单号".to_string(),
                example_value: Some(json!("ASN-202607260001")),
                printable: true,
                sensitive: true,
                masking_rule: Some("keep_last_4".to_string()),
                formatting_rule: Some("uppercase".to_string()),
                supports_barcode: true,
                supports_qrcode: true,
                is_table_detail: false,
                sort_order: 10,
            },
            now + chrono::Duration::minutes(1),
            "h9-field-library-field-update",
        )
        .await
        .expect("same field metadata request should replay");
    assert!(update_replay.replayed);

    let invalid_format_error = repo
        .update_field_definition(
            &pool,
            &auth,
            draft.value.id,
            receipt_no.id,
            UpdatePrintFieldDefinitionRequest {
                display_name: "收货单号".to_string(),
                group_code: "order".to_string(),
                group_name: "单据信息".to_string(),
                description: "仓库收货作业的业务单号".to_string(),
                example_value: Some(json!("ASN-202607260001")),
                printable: true,
                sensitive: true,
                masking_rule: Some("keep_last_4".to_string()),
                formatting_rule: Some("uppercase()".to_string()),
                supports_barcode: true,
                supports_qrcode: true,
                is_table_detail: false,
                sort_order: 10,
            },
            now + chrono::Duration::minutes(2),
            "h9-field-library-invalid-format",
        )
        .await
        .expect_err("malformed formatting rule must be rejected");
    assert_eq!(
        invalid_format_error,
        PrintTemplateError::FieldFormatInvalid("uppercase()".to_string())
    );

    let draft_template_error = repo
        .save_template(
            &pool,
            &auth,
            template_request(draft.value.id),
            now + chrono::Duration::minutes(2),
            "h9-draft-library-template",
        )
        .await
        .expect_err("draft field library must not bind to a template");
    assert_eq!(
        draft_template_error,
        PrintTemplateError::FieldLibraryNotPublished
    );

    let published = repo
        .publish_field_library_draft(
            &pool,
            &auth,
            draft.value.id,
            &openapi,
            now + chrono::Duration::minutes(3),
            "h9-field-library-publish-v1",
        )
        .await
        .expect("valid current OpenAPI paths should publish");
    assert_eq!(published.value.status, "published");
    assert_eq!(published.value.published_by, Some(auth.user_id));
    let publish_replay = repo
        .publish_field_library_draft(
            &pool,
            &auth,
            draft.value.id,
            &openapi,
            now + chrono::Duration::minutes(3),
            "h9-field-library-publish-v1",
        )
        .await
        .expect("same publish request should replay");
    assert!(publish_replay.replayed);

    let immutable_error = repo
        .update_field_definition(
            &pool,
            &auth,
            draft.value.id,
            receipt_no.id,
            UpdatePrintFieldDefinitionRequest {
                display_name: "非法改写".to_string(),
                group_code: "order".to_string(),
                group_name: "单据信息".to_string(),
                description: String::new(),
                example_value: None,
                printable: true,
                sensitive: false,
                masking_rule: None,
                formatting_rule: None,
                supports_barcode: false,
                supports_qrcode: false,
                is_table_detail: false,
                sort_order: 10,
            },
            now + chrono::Duration::minutes(4),
            "h9-field-library-published-update",
        )
        .await
        .expect_err("published field library must be immutable");
    assert_eq!(
        immutable_error,
        PrintTemplateError::PublishedFieldLibraryImmutable
    );

    let mut second_request = draft_request();
    second_request.business_module = "M4".to_string();
    let second_draft = repo
        .generate_field_library_draft(
            &pool,
            &auth,
            second_request,
            &openapi,
            now + chrono::Duration::minutes(5),
            "h9-field-library-draft-v2",
        )
        .await
        .expect("field changes should create a new draft version");
    assert_eq!(second_draft.value.version_no, 2);
    assert_eq!(second_draft.value.status, "draft");
    assert_eq!(second_draft.value.business_module, "M4");
    let first_version_business_module: String = sqlx::query_scalar(
        "SELECT business_module FROM print_field_library_versions WHERE id = $1",
    )
    .bind(draft.value.id)
    .fetch_one(&pool)
    .await
    .expect("published version business module snapshot should query");
    assert_eq!(first_version_business_module, "M2");

    sqlx::query(
        r#"
        INSERT INTO print_field_definitions (
            id, library_version_id, field_path, field_type, source_schema,
            display_name, group_code, group_name, sort_order
        )
        VALUES ($1, $2, 'removed_from_openapi', 'string', $3, '失效字段', 'other', '其他', 999)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(second_draft.value.id)
    .bind(&second_draft.value.source_schema)
    .execute(&pool)
    .await
    .expect("test-only invalid draft field should insert");

    let invalid_path_error = repo
        .publish_field_library_draft(
            &pool,
            &auth,
            second_draft.value.id,
            &openapi,
            now + chrono::Duration::minutes(6),
            "h9-field-library-publish-invalid",
        )
        .await
        .expect_err("removed OpenAPI field path must block publish");
    assert_eq!(
        invalid_path_error,
        PrintTemplateError::FieldPathInvalid(vec!["removed_from_openapi".to_string()])
    );

    let status: String =
        sqlx::query_scalar("SELECT status FROM print_field_library_versions WHERE id = $1")
            .bind(second_draft.value.id)
            .fetch_one(&pool)
            .await
            .expect("draft status should query");
    assert_eq!(status, "draft");

    let audit_actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_event WHERE resource_id = $1 ORDER BY occurred_at",
    )
    .bind(draft.value.id.to_string())
    .fetch_all(&pool)
    .await
    .expect("field library audit should query");
    assert_eq!(
        audit_actions,
        vec![
            "generate_print_field_library_draft",
            "update_print_field_definition",
            "publish_print_field_library",
        ]
    );
    let audit_diffs: Vec<(String, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT action, diff FROM audit_event WHERE resource_id = $1 ORDER BY occurred_at",
    )
    .bind(draft.value.id.to_string())
    .fetch_all(&pool)
    .await
    .expect("field library audit diffs should query");
    let update_diff = audit_diffs
        .iter()
        .find(|(action, _)| action == "update_print_field_definition")
        .and_then(|(_, diff)| diff.as_ref())
        .expect("field metadata audit should include a diff");
    assert!(update_diff["changed_keys"]
        .as_array()
        .is_some_and(|keys| keys.iter().any(|key| key == "description")));
    let publish_diff = audit_diffs
        .iter()
        .find(|(action, _)| action == "publish_print_field_library")
        .and_then(|(_, diff)| diff.as_ref())
        .expect("field library publish audit should include a diff");
    assert!(publish_diff["changed_keys"]
        .as_array()
        .is_some_and(|keys| keys.iter().any(|key| key == "status")));
}

#[sqlx::test(migrations = "../../migrations")]
async fn field_library_http_enforces_permission_and_returns_invalid_path_contract(pool: PgPool) {
    let auth = ctx(Uuid::new_v4());
    let app = print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
        .layer(Extension(auth.clone()));
    let draft_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/print-templates/field-libraries/drafts",
            Some("h9-http-draft"),
            serde_json::to_value(draft_request()).expect("draft request should serialize"),
        ))
        .await
        .expect("draft route should respond");
    assert_eq!(draft_response.status(), StatusCode::OK);
    let draft: serde_json::Value = serde_json::from_slice(
        &to_bytes(draft_response.into_body(), usize::MAX)
            .await
            .expect("draft body should read"),
    )
    .expect("draft response should be json");
    let version_id = Uuid::parse_str(
        draft["id"]
            .as_str()
            .expect("draft response should include version id"),
    )
    .expect("version id should be UUID");

    let field_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM print_field_definitions WHERE library_version_id = $1 AND field_path = 'receipt_no'",
    )
    .bind(version_id)
    .fetch_one(&pool)
    .await
    .expect("generated receipt_no field should exist");
    let invalid_format_response = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            &format!("/api/v1/print-templates/field-libraries/{version_id}/fields/{field_id}"),
            Some("h9-http-invalid-format"),
            json!({
                "display_name": "收货单号",
                "group_code": "order",
                "group_name": "单据信息",
                "description": "",
                "example_value": null,
                "printable": true,
                "sensitive": false,
                "masking_rule": null,
                "formatting_rule": "uppercase()",
                "supports_barcode": false,
                "supports_qrcode": false,
                "is_table_detail": false,
                "sort_order": 10
            }),
        ))
        .await
        .expect("invalid format route should respond");
    assert_eq!(
        invalid_format_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let invalid_format_error: serde_json::Value = serde_json::from_slice(
        &to_bytes(invalid_format_response.into_body(), usize::MAX)
            .await
            .expect("invalid format body should read"),
    )
    .expect("invalid format response should be json");
    assert_eq!(invalid_format_error["code"], "H9_FIELD_FORMAT_INVALID");
    assert_eq!(invalid_format_error["details"]["rule"], "uppercase()");

    sqlx::query(
        r#"
        INSERT INTO print_field_definitions (
            id, library_version_id, field_path, field_type, source_schema,
            display_name, group_code, group_name, sort_order
        )
        VALUES ($1, $2, 'removed_from_openapi', 'string', 'CreateReceivingOrderRequest',
                '失效字段', 'other', '其他', 999)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(version_id)
    .execute(&pool)
    .await
    .expect("invalid test field should insert into draft");

    let publish_response = app
        .oneshot(json_request(
            "POST",
            &format!("/api/v1/print-templates/field-libraries/{version_id}/publish"),
            Some("h9-http-publish-invalid"),
            json!({}),
        ))
        .await
        .expect("publish route should respond");
    assert_eq!(publish_response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error: serde_json::Value = serde_json::from_slice(
        &to_bytes(publish_response.into_body(), usize::MAX)
            .await
            .expect("error body should read"),
    )
    .expect("error response should be json");
    assert_eq!(error["code"], "H9_FIELD_PATH_INVALID");
    assert_eq!(error["details"]["fields"][0], "removed_from_openapi");

    let publish_only = print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
        .layer(Extension(AuthContext {
            permissions: vec!["h9.print_template.publish".to_string()],
            ..auth.clone()
        }))
        .oneshot(json_request(
            "POST",
            "/api/v1/print-templates/field-libraries/drafts",
            Some("h9-http-publish-only"),
            serde_json::to_value(draft_request()).expect("draft request should serialize"),
        ))
        .await
        .expect("publish-only route should respond");
    assert_eq!(publish_only.status(), StatusCode::FORBIDDEN);

    let denied = print_template_router(PrintTemplateAppState::with_postgres(pool))
        .layer(Extension(AuthContext {
            permissions: vec!["h9.print_template.read".to_string()],
            ..auth
        }))
        .oneshot(json_request(
            "POST",
            "/api/v1/print-templates/field-libraries/drafts",
            Some("h9-http-denied"),
            serde_json::to_value(draft_request()).expect("draft request should serialize"),
        ))
        .await
        .expect("denied route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
}

fn json_request(
    method: &str,
    uri: &str,
    idempotency_key: Option<&str>,
    body: serde_json::Value,
) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(key) = idempotency_key {
        request = request.header("Idempotency-Key", key);
    }
    request
        .body(Body::from(body.to_string()))
        .expect("request should build")
}
