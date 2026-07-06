use chrono::{TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    system_dictionary::{PgSystemDictionaryRepository, SystemDictionaryError},
};
use wms_domain::{
    DisableSystemDictionaryItemRequest, UpsertSystemDictionaryItemRequest,
    DOCUMENT_TYPE_PURCHASE_INBOUND, DOCUMENT_TYPE_SALES_OUTBOUND, SYSTEM_DICTIONARY_DOCUMENT_TYPE,
    PRINT_TEMPLATE_TYPE_ACCEPTANCE_RECORD, PRINT_TEMPLATE_TYPE_ASN, PRINT_TEMPLATE_TYPE_DELIVERY_NOTE,
    PRINT_TEMPLATE_TYPE_LOCATION_LABEL, PRINT_TEMPLATE_TYPE_LPN_LABEL, PRINT_TEMPLATE_TYPE_PRODUCT_LABEL,
    SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "system-dictionary-test".to_string(),
        permissions: vec!["m1.system_dictionary.write".to_string()],
        jti: Uuid::new_v4().to_string(),
    }
}

fn valid_document_type_params() -> serde_json::Value {
    json!({
        "direction": "inbound",
        "workflow_template": "purchase_inbound",
        "batch_policy": "standard_batch"
    })
}

#[sqlx::test(migrations = "../../migrations")]
async fn document_type_presets_are_queryable(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool);
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 6, 28, 9, 0, 0)
        .single()
        .expect("valid time");

    let items = repo
        .list_effective_items(&ctx(owner_id), SYSTEM_DICTIONARY_DOCUMENT_TYPE, now)
        .await
        .expect("document_type presets should be queryable");
    let codes: Vec<_> = items.iter().map(|item| item.item_code.as_str()).collect();

    assert_eq!(
        codes,
        vec![
            "purchase_inbound",
            "purchase_return_outbound",
            "sales_outbound",
            "sales_return"
        ]
    );
    assert!(items.iter().all(|item| item.source == "global"));
    assert!(items
        .iter()
        .all(|item| item.params.get("direction").is_some()));
    assert!(items
        .iter()
        .all(|item| item.params.get("workflow_template").is_some()));
    assert!(items
        .iter()
        .all(|item| item.params.get("batch_policy").is_some()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn print_template_type_presets_are_queryable_and_require_field_library(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool);
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 6, 9, 0, 0)
        .single()
        .expect("valid time");

    let items = repo
        .list_effective_items(&ctx(owner_id), SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE, now)
        .await
        .expect("print_template_type presets should be queryable");
    let codes: Vec<_> = items.iter().map(|item| item.item_code.as_str()).collect();

    assert_eq!(
        codes,
        vec![
            PRINT_TEMPLATE_TYPE_ACCEPTANCE_RECORD,
            PRINT_TEMPLATE_TYPE_ASN,
            PRINT_TEMPLATE_TYPE_DELIVERY_NOTE,
            PRINT_TEMPLATE_TYPE_LOCATION_LABEL,
            PRINT_TEMPLATE_TYPE_LPN_LABEL,
            PRINT_TEMPLATE_TYPE_PRODUCT_LABEL,
        ]
    );
    assert!(items.iter().all(|item| item.source == "global"));
    assert!(items
        .iter()
        .all(|item| item.params.get("field_library_code").is_some()));

    let error = repo
        .upsert_item(
            &ctx(owner_id),
            SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
            PRINT_TEMPLATE_TYPE_ASN,
            UpsertSystemDictionaryItemRequest {
                owner_id: Some(owner_id),
                item_name: "坏模板类型".to_string(),
                enabled: true,
                params: json!({
                    "business_module": "M2",
                    "business_direction": "inbound",
                    "paper_type": "a4",
                    "default_scope": "global"
                }),
                effective_from: None,
                effective_to: None,
            },
            now,
            "system-dictionary-print-template-invalid",
        )
        .await
        .expect_err("enabled print template type without field library must be rejected");
    assert!(matches!(error, SystemDictionaryError::ParamInvalid { .. }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn owner_dictionary_item_overrides_global_and_disable_hides_it(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool);
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 6, 28, 9, 0, 0)
        .single()
        .expect("valid time");

    let created = repo
        .upsert_item(
            &ctx,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            UpsertSystemDictionaryItemRequest {
                owner_id: Some(owner_id),
                item_name: "货主采购入库".to_string(),
                enabled: true,
                params: valid_document_type_params(),
                effective_from: None,
                effective_to: None,
            },
            now,
            "system-dictionary-owner-override",
        )
        .await
        .expect("owner override should save")
        .value;
    assert_eq!(created.source, "owner");

    let items = repo
        .list_effective_items(&ctx, SYSTEM_DICTIONARY_DOCUMENT_TYPE, now)
        .await
        .expect("merged items should be queryable");
    let purchase = items
        .iter()
        .find(|item| item.item_code == DOCUMENT_TYPE_PURCHASE_INBOUND)
        .expect("purchase inbound should exist");
    assert_eq!(purchase.item_name, "货主采购入库");
    assert_eq!(purchase.owner_id, Some(owner_id));

    repo.disable_item(
        &ctx,
        SYSTEM_DICTIONARY_DOCUMENT_TYPE,
        DOCUMENT_TYPE_PURCHASE_INBOUND,
        DisableSystemDictionaryItemRequest {
            owner_id: Some(owner_id),
            disabled_reason: Some("owner closed".to_string()),
        },
        now,
        "system-dictionary-owner-disable",
    )
    .await
    .expect("owner override should disable");

    let after_disable = repo
        .list_effective_items(&ctx, SYSTEM_DICTIONARY_DOCUMENT_TYPE, now)
        .await
        .expect("merged items after disable should be queryable");
    assert!(
        after_disable
            .iter()
            .all(|item| item.item_code != DOCUMENT_TYPE_PURCHASE_INBOUND),
        "disabled owner override must fail closed instead of falling back to global"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn document_type_rejects_invalid_params(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool);
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 6, 28, 9, 0, 0)
        .single()
        .expect("valid time");

    let error = repo
        .upsert_item(
            &ctx(owner_id),
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            UpsertSystemDictionaryItemRequest {
                owner_id: Some(owner_id),
                item_name: "坏参数".to_string(),
                enabled: true,
                params: json!({
                    "direction": "sideways",
                    "workflow_template": "purchase_inbound"
                }),
                effective_from: None,
                effective_to: None,
            },
            now,
            "system-dictionary-invalid-params",
        )
        .await
        .expect_err("invalid document_type params must be rejected");

    assert!(matches!(error, SystemDictionaryError::ParamInvalid { .. }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn document_type_impact_preview_counts_owner_scoped_m2_and_m4_references(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool.clone());
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let ctx_a = ctx(owner_a);
    let effective_at = Utc
        .with_ymd_and_hms(2026, 6, 28, 12, 0, 0)
        .single()
        .expect("valid time");

    for (owner_id, receipt_no, document_type, created_at) in [
        (
            owner_a,
            "ASN-IMPACT-001",
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            effective_at - chrono::Duration::hours(2),
        ),
        (
            owner_a,
            "ASN-IMPACT-002",
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            effective_at - chrono::Duration::hours(1),
        ),
        (
            owner_a,
            "ASN-IMPACT-FUTURE",
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            effective_at + chrono::Duration::hours(1),
        ),
        (
            owner_b,
            "ASN-IMPACT-OTHER",
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            effective_at - chrono::Duration::hours(1),
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO receiving_orders (
                id, owner_id, receipt_no, document_type, warehouse_id, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, 'draft', $6, $6)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(receipt_no)
        .bind(document_type)
        .bind(Uuid::new_v4())
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("seed receiving order");
    }

    for (owner_id, wms_order_no, created_at) in [
        (
            owner_a,
            "OUT-IMPACT-001",
            effective_at - chrono::Duration::hours(2),
        ),
        (
            owner_a,
            "OUT-IMPACT-002",
            effective_at - chrono::Duration::hours(1),
        ),
        (
            owner_a,
            "OUT-IMPACT-FUTURE",
            effective_at + chrono::Duration::hours(1),
        ),
        (
            owner_b,
            "OUT-IMPACT-OTHER",
            effective_at - chrono::Duration::hours(1),
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO outbound_orders (
                id, owner_id, wms_order_no, customer_id, warehouse_id, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, 'confirmed', $6, $6)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(wms_order_no)
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("seed outbound order");
    }

    let inbound_preview = repo
        .preview_item_impact(
            &ctx_a,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            owner_a,
            effective_at,
        )
        .await
        .expect("inbound document type impact preview");

    assert_eq!(inbound_preview.total_references, 2);
    assert_eq!(inbound_preview.references.len(), 1);
    assert_eq!(inbound_preview.references[0].module_code, "M2");
    assert_eq!(
        inbound_preview.references[0].business_object,
        "receiving_orders"
    );
    assert_eq!(inbound_preview.references[0].reference_count, 2);

    let outbound_preview = repo
        .preview_item_impact(
            &ctx_a,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_SALES_OUTBOUND,
            owner_a,
            effective_at,
        )
        .await
        .expect("outbound document type impact preview");

    assert_eq!(outbound_preview.total_references, 2);
    assert_eq!(outbound_preview.references.len(), 1);
    assert_eq!(outbound_preview.references[0].module_code, "M4");
    assert_eq!(
        outbound_preview.references[0].business_object,
        "outbound_orders"
    );
    assert_eq!(outbound_preview.references[0].reference_count, 2);

    let error = repo
        .preview_item_impact(
            &ctx_a,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            owner_b,
            effective_at,
        )
        .await
        .expect_err("cross-owner preview must be rejected");
    assert_eq!(error, SystemDictionaryError::CrossOwnerAccess);
}
