use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    lpn_container_repository::{LpnContainerRepositoryError, PgLpnContainerRepository},
};
use wms_domain::{UpsertLpnContainerTypePolicyRequest, LPN_CONTAINER_TYPE_PALLET};

mod postgres_test_support;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "lpn-policy-test".to_string(),
        permissions: vec!["m1.master_data.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn lpn_type_policy_put_replays_idempotency_and_writes_audit(pool: PgPool) {
    // PUT /api/v1/master-data/lpn-container-type-policies
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'LPN 策略测试货主')",
    )
    .bind(owner_id)
    .bind(format!("LP-{}", &owner_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed owner");
    postgres_test_support::ensure_audit_partition(&pool, Utc::now()).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = ctx(owner_id);
    let request = UpsertLpnContainerTypePolicyRequest {
        container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
        allow_mix_batch: true,
        allow_mix_sku: false,
    };

    let first = repo
        .upsert_type_policy_idempotent(&actor, request.clone(), "lpn-policy-put-1")
        .await
        .expect("first policy upsert");
    let replay = repo
        .upsert_type_policy_idempotent(&actor, request, "lpn-policy-put-1")
        .await
        .expect("same request should replay");

    assert_eq!(replay.owner_id, first.owner_id);
    assert_eq!(replay.container_type, first.container_type);
    assert_eq!(replay.allow_mix_batch, first.allow_mix_batch);
    assert_eq!(replay.allow_mix_sku, first.allow_mix_sku);

    let conflict = repo
        .upsert_type_policy_idempotent(
            &actor,
            UpsertLpnContainerTypePolicyRequest {
                container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
                allow_mix_batch: false,
                allow_mix_sku: true,
            },
            "lpn-policy-put-1",
        )
        .await
        .expect_err("same idempotency key with different payload must conflict");
    assert_eq!(conflict, LpnContainerRepositoryError::IdempotencyConflict);

    let policy_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM lpn_container_type_policies WHERE owner_id = $1 AND container_type = $2",
    )
    .bind(owner_id)
    .bind(LPN_CONTAINER_TYPE_PALLET)
    .fetch_one(&pool)
    .await
    .expect("policy count");
    assert_eq!(policy_count, 1);

    let stored_policy: (bool, bool) = sqlx::query_as(
        "SELECT allow_mix_batch, allow_mix_sku FROM lpn_container_type_policies WHERE owner_id = $1 AND container_type = $2",
    )
    .bind(owner_id)
    .bind(LPN_CONTAINER_TYPE_PALLET)
    .fetch_one(&pool)
    .await
    .expect("stored policy");
    assert_eq!(
        stored_policy,
        (true, false),
        "conflicting replay must not mutate the stored policy"
    );

    let action_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'upsert_lpn_type_policy'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("policy audit");
    assert_eq!(
        action_count, 1,
        "replay or conflict must not duplicate audit"
    );

    let audit_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_event WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("policy audit total");
    assert_eq!(audit_total, 1);

    postgres_test_support::idempotency_request(&pool, owner_id, "lpn-policy-put-1").await;
}
