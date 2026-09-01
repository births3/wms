use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{auth::AuthContext, lpn_container_repository::PgLpnContainerRepository};
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

    let policy_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM lpn_container_type_policies WHERE owner_id = $1 AND container_type = $2",
    )
    .bind(owner_id)
    .bind(LPN_CONTAINER_TYPE_PALLET)
    .fetch_one(&pool)
    .await
    .expect("policy count");
    assert_eq!(policy_count, 1);

    let action_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'upsert_lpn_type_policy'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("policy audit");
    assert_eq!(
        action_count, 1,
        "idempotent replay must not duplicate audit"
    );

    postgres_test_support::audit_event(&pool, owner_id, 1).await;
    postgres_test_support::idempotency_request(&pool, owner_id, "lpn-policy-put-1").await;
}
