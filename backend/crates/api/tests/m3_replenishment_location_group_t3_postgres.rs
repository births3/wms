use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext, replenishment_repository::PgReplenishmentRepository,
    replenishment_service::ReplenishmentService,
};
use wms_domain::UpsertReplenishmentLocationGroupRequest;

mod postgres_test_support;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "replenishment-group-test".to_string(),
        permissions: vec!["m3.replenishment.manage".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn location_group_create_update_and_disable_replays(pool: PgPool) {
    // POST /api/v1/replenishment/location-groups
    // PUT /api/v1/replenishment/location-groups/{id}
    // POST /api/v1/replenishment/location-groups/{id}/disable
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '补货库位组测试货主')",
    )
    .bind(owner_id)
    .bind(format!("RPG-{}", &owner_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed owner");
    postgres_test_support::ensure_audit_partition(&pool, Utc::now()).await;

    let service = ReplenishmentService::new(PgReplenishmentRepository::new(pool.clone()));
    let actor = ctx(owner_id);
    let create_request = UpsertReplenishmentLocationGroupRequest {
        group_code: "GRP-01".to_string(),
        group_name: "零拣补货组".to_string(),
        enabled: true,
        location_ids: vec![],
    };

    let created = service
        .upsert_location_group(&actor, create_request.clone(), "rp-group-create")
        .await
        .expect("create group");
    let create_replay = service
        .upsert_location_group(&actor, create_request, "rp-group-create")
        .await
        .expect("create replay");
    assert_eq!(create_replay.id, created.id);

    let update_request = UpsertReplenishmentLocationGroupRequest {
        group_code: "GRP-01".to_string(),
        group_name: "零拣补货组-更新".to_string(),
        enabled: true,
        location_ids: vec![],
    };
    let updated = service
        .update_location_group(
            &actor,
            created.id,
            update_request.clone(),
            "rp-group-update",
        )
        .await
        .expect("update group");
    let update_replay = service
        .update_location_group(&actor, created.id, update_request, "rp-group-update")
        .await
        .expect("update replay");
    assert_eq!(update_replay.id, updated.id);
    assert_eq!(update_replay.group_name, updated.group_name);

    let disabled = service
        .disable_location_group(&actor, created.id, "rp-group-disable")
        .await
        .expect("disable group");
    let disable_replay = service
        .disable_location_group(&actor, created.id, "rp-group-disable")
        .await
        .expect("disable replay");
    assert!(!disabled.enabled);
    assert_eq!(disable_replay.id, disabled.id);
    assert!(!disable_replay.enabled);

    postgres_test_support::audit_event(&pool, owner_id, 3).await;
    postgres_test_support::idempotency_request(&pool, owner_id, "rp-group-create").await;
    postgres_test_support::idempotency_request(&pool, owner_id, "rp-group-update").await;
    postgres_test_support::idempotency_request(&pool, owner_id, "rp-group-disable").await;
}
