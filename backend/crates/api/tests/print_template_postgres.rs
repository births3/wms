use chrono::{TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    print_template::{
        PgPrintTemplateRepository, PrintFieldDefinitionInput, PublishPrintFieldLibraryRequest,
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
