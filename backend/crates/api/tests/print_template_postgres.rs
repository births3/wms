use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    response::Response,
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
        PrintTemplatePreviewRequest, PrintTemplatePrintRequest, PrintTemplateScope,
        SavePrintTemplateRequest, UpdatePrintFieldDefinitionRequest,
    },
    print_template_handlers::{print_template_router, PrintTemplateAppState},
    system_dictionary::PgSystemDictionaryRepository,
    ApiDoc,
};
use wms_domain::{
    DisableSystemDictionaryItemRequest, PRINT_TEMPLATE_TYPE_ACCEPTANCE_RECORD,
    PRINT_TEMPLATE_TYPE_ASN, PRINT_TEMPLATE_TYPE_DELIVERY_NOTE, PRINT_TEMPLATE_TYPE_LOCATION_LABEL,
    PRINT_TEMPLATE_TYPE_LPN_LABEL, PRINT_TEMPLATE_TYPE_PRODUCT_LABEL,
    SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
};

mod postgres_test_support;
use postgres_test_support::ensure_audit_partition;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "print-template-test".to_string(),
        permissions: vec!["h9.print_template.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn ctx_with_permissions(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        permissions: permissions.iter().map(ToString::to_string).collect(),
        ..ctx(owner_id)
    }
}

fn field_library_request() -> GeneratePrintFieldLibraryDraftRequest {
    GeneratePrintFieldLibraryDraftRequest {
        library_code: "m2_asn".to_string(),
        library_name: "M2 ASN 字段库".to_string(),
        business_module: "M2".to_string(),
        source_schema: "ReceivingOrder".to_string(),
    }
}

async fn published_library(
    repo: &PgPrintTemplateRepository,
    pool: &PgPool,
    auth: &AuthContext,
    display_name: &str,
    now: chrono::DateTime<Utc>,
    key: &str,
) -> wms_api::print_template::PrintFieldLibraryVersion {
    ensure_audit_partition(pool, now).await;
    let openapi = serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI should serialize");
    let draft = repo
        .generate_field_library_draft(
            pool,
            auth,
            field_library_request(),
            &openapi,
            now,
            &format!("{key}-draft"),
        )
        .await
        .expect("field library draft should generate");
    let field = repo
        .list_field_version_fields(pool, draft.value.id)
        .await
        .expect("generated fields should list")
        .into_iter()
        .find(|field| field.field_path == "receipt_no")
        .expect("receipt_no should be generated");
    repo.update_field_definition(
        pool,
        auth,
        draft.value.id,
        field.id,
        UpdatePrintFieldDefinitionRequest {
            display_name: display_name.to_string(),
            group_code: "order".to_string(),
            group_name: "订单信息".to_string(),
            description: "收货单号".to_string(),
            example_value: Some(json!("ASN-202607050001")),
            printable: true,
            sensitive: false,
            masking_rule: None,
            formatting_rule: None,
            supports_barcode: true,
            supports_qrcode: false,
            is_table_detail: false,
            sort_order: 10,
        },
        now,
        &format!("{key}-field"),
    )
    .await
    .expect("draft field metadata should update");
    repo.publish_field_library_draft(
        pool,
        auth,
        draft.value.id,
        &openapi,
        now,
        &format!("{key}-publish"),
    )
    .await
    .expect("field library should publish")
    .value
}

fn template_request(field_library_version_id: Uuid) -> SavePrintTemplateRequest {
    SavePrintTemplateRequest {
        template_id: None,
        template_code: "m2_asn_default".to_string(),
        template_name: "M2 ASN 默认模板".to_string(),
        template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
        scope: PrintTemplateScope::Owner,
        is_default: true,
        remark: Some("H9 hiprint 测试模板".to_string()),
        field_library_version_id,
        hiprint_json: json!({
            "panels": [
                {
                    "index": 0,
                    "paperType": "A4",
                    "printElements": [
                        {
                            "options": {
                                "field": "receipt_no",
                                "title": "ASN 号",
                                "left": 20,
                                "top": 20,
                                "width": 120,
                                "height": 20
                            },
                            "printElementType": {"type": "text"}
                        }
                    ]
                }
            ]
        }),
        field_bindings: vec![PrintTemplateBinding {
            field_path: "receipt_no".to_string(),
            required: true,
        }],
        paper: json!({ "paperType": "A4", "width": 210, "height": 297 }),
        designer_version: "hiprint@0.4.0".to_string(),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn disabled_print_template_type_rejects_new_template(pool: PgPool) {
    let print_templates = PgPrintTemplateRepository::new();
    let dictionaries = PgSystemDictionaryRepository::new(pool.clone());
    let owner_id = Uuid::new_v4();
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 7, 8, 0, 0)
        .single()
        .expect("valid time");
    let field_library = published_library(
        &print_templates,
        &pool,
        &auth,
        "ASN 号",
        now,
        "h9-disabled-type-field-library",
    )
    .await;
    dictionaries
        .disable_item(
            &auth,
            SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
            PRINT_TEMPLATE_TYPE_ASN,
            DisableSystemDictionaryItemRequest {
                owner_id: None,
                disabled_reason: Some("test disabled".to_string()),
            },
            now + chrono::Duration::minutes(1),
            "h9-disable-template-type",
        )
        .await
        .expect("template type should disable");

    let mut request = template_request(field_library.id);
    request.template_type_code = PRINT_TEMPLATE_TYPE_ASN.to_string();
    let error = print_templates
        .save_template(
            &pool,
            &auth,
            request,
            now + chrono::Duration::minutes(2),
            "h9-disabled-type-template-save",
        )
        .await
        .expect_err("disabled template type must reject new template");

    assert_eq!(
        error,
        wms_api::print_template::PrintTemplateError::TemplateDisabled
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn published_field_library_versions_are_immutable_idempotent_and_audited(pool: PgPool) {
    let repo = PgPrintTemplateRepository::new();
    let owner_id = Uuid::new_v4();
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 5, 9, 0, 0)
        .single()
        .expect("valid time");

    let first = published_library(&repo, &pool, &auth, "ASN 号", now, "h9-field-publish-1").await;
    assert_eq!(first.version_no, 1);

    let openapi = serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI should serialize");
    let replay = repo
        .publish_field_library_draft(
            &pool,
            &auth,
            first.id,
            &openapi,
            now,
            "h9-field-publish-1-publish",
        )
        .await
        .expect("same idempotency key should replay first publish");
    assert_eq!(replay.value.id, first.id);
    assert!(replay.replayed);

    let second = published_library(
        &repo,
        &pool,
        &auth,
        "ASN 编号",
        now + chrono::Duration::minutes(1),
        "h9-field-publish-2",
    )
    .await;
    assert_eq!(second.version_no, 2);

    let first_fields = repo
        .list_field_version_fields(&pool, first.id)
        .await
        .expect("first version fields should be queryable");
    let second_fields = repo
        .list_field_version_fields(&pool, second.id)
        .await
        .expect("second version fields should be queryable");
    assert_eq!(
        first_fields
            .iter()
            .find(|field| field.field_path == "receipt_no")
            .expect("first receipt_no")
            .display_name,
        "ASN 号"
    );
    assert_eq!(
        second_fields
            .iter()
            .find(|field| field.field_path == "receipt_no")
            .expect("second receipt_no")
            .display_name,
        "ASN 编号"
    );

    let libraries = repo
        .list_field_libraries(&pool)
        .await
        .expect("latest field libraries should be queryable");
    assert_eq!(libraries.len(), 1);
    assert_eq!(libraries[0].library_code, "m2_asn");
    assert_eq!(libraries[0].version_no, 2);
    assert_eq!(libraries[0].field_count, second_fields.len() as i64);

    let audit_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM audit_event
         WHERE module = 'H9'
           AND resource_type = 'print_field_library'
           AND owner_id = $1
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should query");
    assert_eq!(audit_count, 6);
}

#[sqlx::test(migrations = "../../migrations")]
async fn print_template_versions_are_listed_by_template_and_owner(pool: PgPool) {
    let repo = PgPrintTemplateRepository::new();
    let owner_id = Uuid::new_v4();
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 7, 10, 0, 0)
        .single()
        .expect("valid time");
    let field_library = published_library(
        &repo,
        &pool,
        &auth,
        "ASN 号",
        now,
        "h9-field-publish-version-list",
    )
    .await;

    let first = repo
        .save_template(
            &pool,
            &auth,
            template_request(field_library.id),
            now + chrono::Duration::minutes(1),
            "h9-template-version-list-1",
        )
        .await
        .expect("first template version should save");
    let mut second_request = template_request(field_library.id);
    second_request.template_id = Some(first.value.template_id);
    second_request.template_name = "M2 ASN 调整模板".to_string();
    let second = repo
        .save_template(
            &pool,
            &auth,
            second_request,
            now + chrono::Duration::minutes(2),
            "h9-template-version-list-2",
        )
        .await
        .expect("second template version should save");

    let versions = repo
        .list_template_versions(&pool, &auth, first.value.template_id)
        .await
        .expect("template versions should list");
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].id, second.value.id);
    assert_eq!(versions[0].version_no, 2);
    assert_eq!(versions[1].id, first.value.id);

    let other_owner_versions = repo
        .list_template_versions(&pool, &ctx(Uuid::new_v4()), first.value.template_id)
        .await
        .expect("cross owner version list should not leak data");
    assert!(other_owner_versions.is_empty());
}

include!("print_template_postgres/version_lifecycle.rs");
include!("print_template_postgres/browser_print.rs");
include!("print_template_postgres/business_integration.rs");

#[sqlx::test(migrations = "../../migrations")]
async fn template_http_separates_write_publish_and_enabled_permissions(pool: PgPool) {
    let repo = PgPrintTemplateRepository::new();
    let owner_id = Uuid::new_v4();
    let write = ctx_with_permissions(owner_id, &["h9.print_template.write"]);
    let publish = ctx_with_permissions(owner_id, &["h9.print_template.publish"]);
    let read = ctx_with_permissions(owner_id, &["h9.print_template.read"]);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 12, 0, 0)
        .single()
        .expect("valid time");
    let field_library = published_library(
        &repo,
        &pool,
        &write,
        "ASN 号",
        now,
        "h9-template-http-field-library",
    )
    .await;

    let request = template_request(field_library.id);
    let save_response = print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
        .layer(Extension(write.clone()))
        .oneshot(json_request(
            "POST",
            "/api/v1/print-templates/templates",
            Some("h9-template-http-save"),
            serde_json::to_value(&request).expect("template request should serialize"),
        ))
        .await
        .expect("save route should respond");
    assert_eq!(save_response.status(), StatusCode::OK);
    let draft = response_json(save_response).await;
    let template_id = draft["template_id"]
        .as_str()
        .expect("template id should be present");
    let version_id = draft["id"].as_str().expect("version id should be present");
    assert_eq!(draft["status"], "draft");

    let mut invalid_json = request.clone();
    invalid_json.template_code = "h9_invalid_json".to_string();
    invalid_json.hiprint_json = json!({ "panels": "invalid" });
    let invalid_json_response =
        print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
            .layer(Extension(write.clone()))
            .oneshot(json_request(
                "POST",
                "/api/v1/print-templates/templates",
                Some("h9-template-http-invalid-json"),
                serde_json::to_value(invalid_json).expect("invalid request should serialize"),
            ))
            .await
            .expect("invalid JSON route should respond");
    assert_eq!(
        invalid_json_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        response_json(invalid_json_response).await["code"],
        "H9_TEMPLATE_JSON_INVALID"
    );

    let mut invalid_binding = request.clone();
    invalid_binding.template_code = "h9_invalid_binding".to_string();
    invalid_binding.field_bindings[0].field_path = "missing.field".to_string();
    let invalid_binding_response =
        print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
            .layer(Extension(write.clone()))
            .oneshot(json_request(
                "POST",
                "/api/v1/print-templates/templates",
                Some("h9-template-http-invalid-binding"),
                serde_json::to_value(invalid_binding)
                    .expect("invalid binding request should serialize"),
            ))
            .await
            .expect("invalid binding route should respond");
    assert_eq!(
        invalid_binding_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        response_json(invalid_binding_response).await["code"],
        "H9_TEMPLATE_FIELD_MISMATCH"
    );

    let duplicate_response =
        print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
            .layer(Extension(write.clone()))
            .oneshot(json_request(
                "POST",
                "/api/v1/print-templates/templates",
                Some("h9-template-http-duplicate"),
                serde_json::to_value(&request).expect("duplicate request should serialize"),
            ))
            .await
            .expect("duplicate route should respond");
    assert_eq!(duplicate_response.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(duplicate_response).await["code"],
        "H9_TEMPLATE_DUPLICATE"
    );

    let publish_path =
        format!("/api/v1/print-templates/templates/{template_id}/versions/{version_id}/publish");
    let publish_denied = print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
        .layer(Extension(write.clone()))
        .oneshot(json_request(
            "POST",
            &publish_path,
            Some("h9-template-http-publish-denied"),
            json!({}),
        ))
        .await
        .expect("write-only publish route should respond");
    assert_eq!(publish_denied.status(), StatusCode::FORBIDDEN);

    let publish_response =
        print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
            .layer(Extension(publish.clone()))
            .oneshot(json_request(
                "POST",
                &publish_path,
                Some("h9-template-http-publish"),
                json!({}),
            ))
            .await
            .expect("publish route should respond");
    assert_eq!(publish_response.status(), StatusCode::OK);

    let enabled_path = format!("/api/v1/print-templates/templates/{template_id}/enabled");
    let enabled_denied = print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
        .layer(Extension(publish))
        .oneshot(json_request(
            "PATCH",
            &enabled_path,
            Some("h9-template-http-disable-denied"),
            json!({ "enabled": false }),
        ))
        .await
        .expect("publish-only enabled route should respond");
    assert_eq!(enabled_denied.status(), StatusCode::FORBIDDEN);

    let enabled_response =
        print_template_router(PrintTemplateAppState::with_postgres(pool.clone()))
            .layer(Extension(write))
            .oneshot(json_request(
                "PATCH",
                &enabled_path,
                Some("h9-template-http-disable"),
                json!({ "enabled": false }),
            ))
            .await
            .expect("write enabled route should respond");
    assert_eq!(enabled_response.status(), StatusCode::OK);

    let versions_response = print_template_router(PrintTemplateAppState::with_postgres(pool))
        .layer(Extension(read))
        .oneshot(json_request(
            "GET",
            &format!("/api/v1/print-templates/templates/{template_id}/versions"),
            None,
            json!({}),
        ))
        .await
        .expect("read versions route should respond");
    assert_eq!(versions_response.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn hiprint_template_publish_preview_and_print_are_versioned_idempotent_and_owner_scoped(
    pool: PgPool,
) {
    let repo = PgPrintTemplateRepository::new();
    let owner_id = Uuid::new_v4();
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 7, 9, 0, 0)
        .single()
        .expect("valid time");
    let field_library = published_library(
        &repo,
        &pool,
        &auth,
        "ASN 号",
        now,
        "h9-field-publish-template",
    )
    .await;

    let saved_draft = repo
        .save_template(
            &pool,
            &auth,
            template_request(field_library.id),
            now + chrono::Duration::minutes(1),
            "h9-template-save-1",
        )
        .await
        .expect("hiprint template should save");
    assert_eq!(saved_draft.value.version_no, 1);
    assert_eq!(saved_draft.value.status, "draft");
    assert_eq!(saved_draft.value.designer_version, "hiprint@0.4.0");
    assert!(!saved_draft.replayed);

    let replay = repo
        .save_template(
            &pool,
            &auth,
            template_request(field_library.id),
            now + chrono::Duration::minutes(1),
            "h9-template-save-1",
        )
        .await
        .expect("same idempotency key should replay saved template");
    assert_eq!(replay.value.id, saved_draft.value.id);
    assert!(replay.replayed);
    let saved = repo
        .publish_template_draft(
            &pool,
            &auth,
            saved_draft.value.template_id,
            saved_draft.value.id,
            now + chrono::Duration::minutes(2),
            "h9-template-publish-1",
        )
        .await
        .expect("hiprint template draft should publish");

    let preview = repo
        .preview_template(
            &pool,
            &auth,
            PrintTemplatePreviewRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
                business_document_id: "ASN-202607070001".to_string(),
                data: json!({ "receipt_no": "ASN-202607070001" }),
            },
        )
        .await
        .expect("preview should resolve template and return hiprint json");
    assert_eq!(preview.template_version_id, saved.value.id);
    assert_eq!(preview.hiprint_json["panels"][0]["paperType"], "A4");

    let missing = repo
        .preview_template(
            &pool,
            &auth,
            PrintTemplatePreviewRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
                business_document_id: "ASN-202607070002".to_string(),
                data: json!({}),
            },
        )
        .await
        .expect_err("required field missing should be rejected");
    assert_eq!(
        missing,
        wms_api::print_template::PrintTemplateError::TemplateFieldMissing(vec![
            "receipt_no".to_string()
        ])
    );

    let printed = repo
        .record_print(
            &pool,
            &auth,
            PrintTemplatePrintRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
                business_module: "M2".to_string(),
                business_document_type: "m2_asn".to_string(),
                business_document_id: "ASN-202607070001".to_string(),
                data: json!({ "receipt_no": "ASN-202607070001" }),
                status: "printed".to_string(),
                failure_reason: None,
            },
            now + chrono::Duration::minutes(3),
            "h9-print-1",
        )
        .await
        .expect("browser print should record print event");
    assert_eq!(printed.value.template_version_id, saved.value.id);
    assert_eq!(printed.value.status, "printed");
    assert!(
        sqlx::query("DELETE FROM print_template_versions WHERE id = $1")
            .bind(saved.value.id)
            .execute(&pool)
            .await
            .is_err(),
        "a template version referenced by a print record must not be deleted"
    );

    let other_owner = ctx(Uuid::new_v4());
    let cross_owner = repo
        .preview_template(
            &pool,
            &other_owner,
            PrintTemplatePreviewRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: PRINT_TEMPLATE_TYPE_ASN.to_string(),
                business_document_id: "ASN-202607070003".to_string(),
                data: json!({ "receipt_no": "ASN-202607070003" }),
            },
        )
        .await
        .expect_err("template must not cross owner fallback");
    assert_eq!(
        cross_owner,
        wms_api::print_template::PrintTemplateError::TemplateNotFound
    );

    let audit_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM audit_event
         WHERE module = 'H9'
           AND resource_type IN ('print_template', 'print_record')
           AND owner_id = $1
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should query");
    assert_eq!(audit_count, 3);
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

async fn response_json(response: Response) -> serde_json::Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should be json")
}
