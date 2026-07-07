use chrono::{TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    print_template::{
        PgPrintTemplateRepository, PrintFieldDefinitionInput, PrintTemplateBinding,
        PrintTemplatePreviewRequest, PrintTemplatePrintRequest, PrintTemplateScope,
        PublishPrintFieldLibraryRequest, SavePrintTemplateRequest,
    },
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "print-template-test".to_string(),
        permissions: vec!["h9.print_template.write".to_string()],
        jti: Uuid::new_v4().to_string(),
    }
}

fn field(display_name: &str) -> PrintFieldDefinitionInput {
    PrintFieldDefinitionInput {
        field_path: "asn.code".to_string(),
        field_type: "string".to_string(),
        source_schema: "ReceivingOrder".to_string(),
        display_name: display_name.to_string(),
        group_code: "order".to_string(),
        group_name: "订单信息".to_string(),
        metadata: json!({
            "sample_value": "ASN-202607050001",
            "printable": true,
            "sensitive": false,
            "support_barcode": true
        }),
        sort_order: 10,
    }
}

fn publish_request(display_name: &str) -> PublishPrintFieldLibraryRequest {
    PublishPrintFieldLibraryRequest {
        library_code: "m2_acceptance_record".to_string(),
        library_name: "M2 验收记录字段库".to_string(),
        source_schema: "ReceivingOrder".to_string(),
        fields: vec![field(display_name)],
    }
}

fn template_request(field_library_version_id: Uuid) -> SavePrintTemplateRequest {
    SavePrintTemplateRequest {
        template_code: "m2_asn_default".to_string(),
        template_name: "M2 ASN 默认模板".to_string(),
        template_type_code: "m2_asn".to_string(),
        scope: PrintTemplateScope::Global,
        enabled: true,
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
                                "field": "asn.code",
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
            field_path: "asn.code".to_string(),
            required: true,
        }],
        paper: json!({ "paperType": "A4", "width": 210, "height": 297 }),
        designer_version: "hiprint@0.4.0".to_string(),
        publish: true,
    }
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

    let first = repo
        .publish_field_library(
            &pool,
            &auth,
            publish_request("ASN 号"),
            now,
            "h9-field-publish-1",
        )
        .await
        .expect("first publish should create version");
    assert_eq!(first.value.version_no, 1);
    assert!(!first.replayed);

    let replay = repo
        .publish_field_library(
            &pool,
            &auth,
            publish_request("ASN 号"),
            now,
            "h9-field-publish-1",
        )
        .await
        .expect("same idempotency key should replay first publish");
    assert_eq!(replay.value.id, first.value.id);
    assert!(replay.replayed);

    let second = repo
        .publish_field_library(
            &pool,
            &auth,
            publish_request("ASN 编号"),
            now + chrono::Duration::minutes(1),
            "h9-field-publish-2",
        )
        .await
        .expect("changed field metadata should create a new version");
    assert_eq!(second.value.version_no, 2);

    let first_fields = repo
        .list_field_version_fields(&pool, first.value.id)
        .await
        .expect("first version fields should be queryable");
    let second_fields = repo
        .list_field_version_fields(&pool, second.value.id)
        .await
        .expect("second version fields should be queryable");
    assert_eq!(first_fields[0].display_name, "ASN 号");
    assert_eq!(second_fields[0].display_name, "ASN 编号");

    let libraries = repo
        .list_field_libraries(&pool)
        .await
        .expect("latest field libraries should be queryable");
    assert_eq!(libraries.len(), 1);
    assert_eq!(libraries[0].library_code, "m2_acceptance_record");
    assert_eq!(libraries[0].version_no, 2);
    assert_eq!(libraries[0].field_count, 1);

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
    assert_eq!(audit_count, 2);
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
    let field_library = repo
        .publish_field_library(
            &pool,
            &auth,
            publish_request("ASN 号"),
            now,
            "h9-field-publish-version-list",
        )
        .await
        .expect("field library should publish");

    let first = repo
        .save_template(
            &pool,
            &auth,
            template_request(field_library.value.id),
            now + chrono::Duration::minutes(1),
            "h9-template-version-list-1",
        )
        .await
        .expect("first template version should save");
    let mut second_request = template_request(field_library.value.id);
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
    let field_library = repo
        .publish_field_library(
            &pool,
            &auth,
            publish_request("ASN 号"),
            now,
            "h9-field-publish-template",
        )
        .await
        .expect("field library should publish");

    let saved = repo
        .save_template(
            &pool,
            &auth,
            template_request(field_library.value.id),
            now + chrono::Duration::minutes(1),
            "h9-template-save-1",
        )
        .await
        .expect("hiprint template should save");
    assert_eq!(saved.value.version_no, 1);
    assert_eq!(saved.value.status, "published");
    assert_eq!(saved.value.designer_version, "hiprint@0.4.0");
    assert!(!saved.replayed);

    let replay = repo
        .save_template(
            &pool,
            &auth,
            template_request(field_library.value.id),
            now + chrono::Duration::minutes(1),
            "h9-template-save-1",
        )
        .await
        .expect("same idempotency key should replay saved template");
    assert_eq!(replay.value.id, saved.value.id);
    assert!(replay.replayed);

    let preview = repo
        .preview_template(
            &pool,
            &auth,
            PrintTemplatePreviewRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: "m2_asn".to_string(),
                business_document_id: "ASN-202607070001".to_string(),
                data: json!({ "asn": { "code": "ASN-202607070001" } }),
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
                template_type_code: "m2_asn".to_string(),
                business_document_id: "ASN-202607070002".to_string(),
                data: json!({}),
            },
        )
        .await
        .expect_err("required field missing should be rejected");
    assert_eq!(
        missing,
        wms_api::print_template::PrintTemplateError::TemplateFieldMissing(vec![
            "asn.code".to_string()
        ])
    );

    let printed = repo
        .record_print(
            &pool,
            &auth,
            PrintTemplatePrintRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: "m2_asn".to_string(),
                business_module: "M2".to_string(),
                business_document_type: "m2_asn".to_string(),
                business_document_id: "ASN-202607070001".to_string(),
                data: json!({ "asn": { "code": "ASN-202607070001" } }),
                status: "printed".to_string(),
                failure_reason: None,
            },
            now + chrono::Duration::minutes(2),
            "h9-print-1",
        )
        .await
        .expect("browser print should record print event");
    assert_eq!(printed.value.template_version_id, saved.value.id);
    assert_eq!(printed.value.status, "printed");

    let other_owner = ctx(Uuid::new_v4());
    let cross_owner = repo
        .preview_template(
            &pool,
            &other_owner,
            PrintTemplatePreviewRequest {
                template_code: Some("m2_asn_default".to_string()),
                template_type_code: "m2_asn".to_string(),
                business_document_id: "ASN-202607070003".to_string(),
                data: json!({ "asn": { "code": "ASN-202607070003" } }),
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
    assert_eq!(audit_count, 2);
}
