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
    DOCUMENT_TYPE_PURCHASE_INBOUND, SYSTEM_DICTIONARY_DOCUMENT_TYPE,
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
