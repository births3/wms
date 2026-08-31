//! 对抗目录夹具：跨货主写拒绝，且被访问货主零变更。

use chrono::{TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use wms_api::system_dictionary::{
    IdempotentMutation, PgSystemDictionaryRepository, SystemDictionaryError,
};
use wms_domain::{
    SystemDictionaryItem, UpsertSystemDictionaryItemRequest, SYSTEM_DICTIONARY_DOCUMENT_TYPE,
};

#[path = "support/adversarial.rs"]
mod adversarial_support;
mod postgres_test_support;

use adversarial_support::{ctx_with_permissions, seed_owner_pair};
use postgres_test_support::ensure_audit_partition;

fn write_ctx(owner_id: uuid::Uuid) -> wms_api::auth::AuthContext {
    ctx_with_permissions(
        owner_id,
        "adversarial-catalog",
        &["m1.system_dictionary.write"],
    )
}

fn owner_override_request(
    owner_id: uuid::Uuid,
    item_name: &str,
) -> UpsertSystemDictionaryItemRequest {
    UpsertSystemDictionaryItemRequest {
        owner_id: Some(owner_id),
        item_name: item_name.to_string(),
        enabled: true,
        sort_order: 10,
        params: json!({
            "direction": "inbound",
            "workflow_template": "purchase_inbound",
            "batch_policy": "standard_batch"
        }),
        effective_from: None,
        effective_to: None,
    }
}

async fn upsert(
    repo: &PgSystemDictionaryRepository,
    ctx: &wms_api::auth::AuthContext,
    item_name: &str,
    key: &str,
    now: chrono::DateTime<Utc>,
) -> Result<IdempotentMutation<SystemDictionaryItem>, SystemDictionaryError> {
    repo.upsert_item(
        ctx,
        SYSTEM_DICTIONARY_DOCUMENT_TYPE,
        "purchase_inbound",
        owner_override_request(ctx.owner_id, item_name),
        now,
        key,
    )
    .await
}

#[sqlx::test(migrations = "../../migrations")]
async fn rejects_cross_owner_dictionary_write_without_leaking_override(pool: PgPool) {
    let pair = seed_owner_pair(&pool).await;
    let repo = PgSystemDictionaryRepository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 8, 21, 9, 0, 0)
        .single()
        .expect("fixed adversarial catalog time");
    ensure_audit_partition(&pool, now).await;

    let owner_a = write_ctx(pair.owner_a);
    let owner_b = write_ctx(pair.owner_b);
    upsert(
        &repo,
        &owner_a,
        "货主A采购入库覆盖",
        "adv-catalog-a1-owner-a",
        now,
    )
    .await
    .expect("owner A dictionary override should save");

    let cross_owner = repo
        .upsert_item(
            &owner_b,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            "purchase_inbound",
            owner_override_request(pair.owner_a, "货主B冒充A"),
            now,
            "adv-catalog-a1-cross-owner",
        )
        .await
        .expect_err("cross-owner dictionary write must be rejected");
    assert_eq!(cross_owner, SystemDictionaryError::CrossOwnerAccess);

    let owner_a_items = repo
        .list_effective_items(&owner_a, SYSTEM_DICTIONARY_DOCUMENT_TYPE, now)
        .await
        .expect("owner A dictionary list");
    let owner_b_items = repo
        .list_effective_items(&owner_b, SYSTEM_DICTIONARY_DOCUMENT_TYPE, now)
        .await
        .expect("owner B dictionary list");
    let owner_a_name = owner_a_items
        .iter()
        .find(|item| item.item_code == "purchase_inbound")
        .map(|item| item.item_name.as_str());
    let owner_b_name = owner_b_items
        .iter()
        .find(|item| item.item_code == "purchase_inbound")
        .map(|item| item.item_name.as_str());
    assert_eq!(owner_a_name, Some("货主A采购入库覆盖"));
    assert_ne!(owner_b_name, Some("货主A采购入库覆盖"));
    assert_ne!(owner_b_name, Some("货主B冒充A"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn dictionary_override_replays_same_key_and_rejects_payload_conflict(pool: PgPool) {
    let pair = seed_owner_pair(&pool).await;
    let repo = PgSystemDictionaryRepository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 8, 21, 10, 0, 0)
        .single()
        .expect("fixed adversarial catalog time");
    ensure_audit_partition(&pool, now).await;
    let ctx = write_ctx(pair.owner_a);

    let created = upsert(&repo, &ctx, "货主A幂等覆盖", "adv-catalog-a4-same-key", now)
        .await
        .expect("first dictionary write should save");
    let replay = upsert(&repo, &ctx, "货主A幂等覆盖", "adv-catalog-a4-same-key", now)
        .await
        .expect("same key same payload should replay");
    assert_eq!(created.value.id, replay.value.id);
    assert!(replay.replayed);

    let conflict = repo
        .upsert_item(
            &ctx,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            "purchase_inbound",
            owner_override_request(pair.owner_a, "货主A冲突载荷"),
            now,
            "adv-catalog-a4-same-key",
        )
        .await
        .expect_err("same key different payload must conflict");
    assert_eq!(conflict, SystemDictionaryError::IdempotencyConflict);
}
