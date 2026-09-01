use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    lpn_container_repository::{LpnContainerRepositoryError, PgLpnContainerRepository},
    wave3_repository::PgWave3Repository,
};
use wms_domain::{
    BatchCreateLpnContainerRequest, UpdateLpnContainerRequest, LPN_BATCH_CREATE_MAX_COUNT,
    LPN_CONTAINER_STATUS_DISABLED, LPN_CONTAINER_STATUS_IDLE, LPN_CONTAINER_TYPE_PALLET,
};

#[path = "support/lpn_container.rs"]
mod lpn_support;
mod postgres_test_support;
use lpn_support::{
    at, create_req, ctx, insert_owner, lpn_status, putaway_count, putaway_req, seed_lpn_numbering,
    seed_putaway,
};
use postgres_test_support::ensure_audit_partition;

#[sqlx::test(migrations = "../../migrations")]
async fn get_update_and_soft_delete_idle_container(pool: PgPool) {
    // POST /api/v1/master-data/lpn-containers
    // PATCH /api/v1/master-data/lpn-containers/{id}
    // DELETE /api/v1/master-data/lpn-containers/{id}
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = ctx(owner_id);
    let created = repo
        .create(&actor, create_req(), at(9), "lpn-crud-create")
        .await
        .expect("create");

    let fetched = repo.get(&actor, created.id).await.expect("get created");
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.status, LPN_CONTAINER_STATUS_IDLE);

    let updated = repo
        .update(
            &actor,
            created.id,
            UpdateLpnContainerRequest {
                status: None,
                location_id: None,
                capacity_cm3: Some(9_001),
            },
            at(10),
            "lpn-crud-update",
        )
        .await
        .expect("update capacity");
    assert_eq!(updated.capacity_cm3, Some(9_001));

    let deleted = repo
        .delete(&actor, created.id, at(11), "lpn-crud-delete")
        .await
        .expect("soft delete");
    assert_eq!(deleted.status, LPN_CONTAINER_STATUS_DISABLED);
    assert_eq!(
        lpn_status(&pool, owner_id, &created.lpn_code).await,
        LPN_CONTAINER_STATUS_DISABLED
    );

    let listed = repo
        .list(&actor, None, None, None, None)
        .await
        .expect("default list hides disabled");
    assert!(listed.iter().all(|row| row.id != created.id));

    let shown = repo
        .list(
            &actor,
            None,
            None,
            Some(LPN_CONTAINER_STATUS_DISABLED),
            None,
        )
        .await
        .expect("filter disabled");
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].id, created.id);

    let still_there = repo.get(&actor, created.id).await.expect("get disabled");
    assert_eq!(still_there.status, LPN_CONTAINER_STATUS_DISABLED);

    let replay = repo
        .delete(&actor, created.id, at(12), "lpn-crud-delete")
        .await
        .expect("delete replay");
    assert_eq!(replay.id, created.id);
    assert_eq!(replay.status, LPN_CONTAINER_STATUS_DISABLED);

    postgres_test_support::audit_event(&pool, owner_id, 3).await;
    postgres_test_support::idempotency_request(&pool, owner_id, "lpn-crud-create").await;
    postgres_test_support::idempotency_request(&pool, owner_id, "lpn-crud-update").await;
    postgres_test_support::idempotency_request(&pool, owner_id, "lpn-crud-delete").await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_rejects_in_use_and_keeps_row(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let fixture = seed_putaway(&pool).await;
    seed_lpn_numbering(&pool, at(0), fixture.owner_id).await;
    let actor = ctx(fixture.owner_id);
    let lpn_repo = PgLpnContainerRepository::new(pool.clone());
    let wave3 = PgWave3Repository::new(pool.clone());
    let created = lpn_repo
        .create(&actor, create_req(), at(9), "lpn-del-in-use-c")
        .await
        .expect("create");
    wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            putaway_req(&fixture, &created.lpn_code),
            Utc::now(),
            "lpn-del-in-use-putaway",
            None,
        )
        .await
        .expect("bind");

    let denied = lpn_repo
        .delete(&actor, created.id, at(11), "lpn-del-in-use")
        .await
        .expect_err("in-use must not delete");
    assert_eq!(denied, LpnContainerRepositoryError::NotDeletable);
    assert_eq!(
        lpn_status(&pool, fixture.owner_id, &created.lpn_code).await,
        "in_use"
    );
    assert_eq!(
        putaway_count(&pool, fixture.owner_id, fixture.order_id).await,
        1
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn patch_cannot_set_disabled_and_disabled_row_is_not_updated(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    let repo = PgLpnContainerRepository::new(pool);
    let actor = ctx(owner_id);
    let created = repo
        .create(&actor, create_req(), at(9), "lpn-patch-disabled-c")
        .await
        .expect("create");
    assert!(matches!(
        repo.update(
            &actor,
            created.id,
            UpdateLpnContainerRequest {
                status: Some(LPN_CONTAINER_STATUS_DISABLED.to_string()),
                location_id: None,
                capacity_cm3: None,
            },
            at(10),
            "lpn-patch-disabled",
        )
        .await,
        Err(LpnContainerRepositoryError::StatusInvalid)
    ));

    repo.delete(&actor, created.id, at(11), "lpn-patch-after-del")
        .await
        .expect("soft delete");
    assert!(matches!(
        repo.update(
            &actor,
            created.id,
            UpdateLpnContainerRequest {
                status: None,
                location_id: None,
                capacity_cm3: Some(1),
            },
            at(12),
            "lpn-patch-gone",
        )
        .await,
        Err(LpnContainerRepositoryError::NotFound)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_other_owner_is_not_found(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let other = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    insert_owner(&pool, other).await;
    let repo = PgLpnContainerRepository::new(pool);
    let created = repo
        .create(&ctx(owner_id), create_req(), at(9), "lpn-del-owner-c")
        .await
        .expect("create");
    assert!(matches!(
        repo.delete(&ctx(other), created.id, at(10), "lpn-del-owner")
            .await,
        Err(LpnContainerRepositoryError::NotFound)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn batch_create_allocates_unique_codes_and_replays(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = ctx(owner_id);
    let first = repo
        .batch_create(
            &actor,
            BatchCreateLpnContainerRequest {
                container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
                capacity_cm3: Some(8_000),
                count: 3,
            },
            at(9),
            "lpn-batch-create",
        )
        .await
        .expect("batch create");
    assert_eq!(first.data.len(), 3);
    let codes: Vec<&str> = first.data.iter().map(|row| row.lpn_code.as_str()).collect();
    assert_eq!(
        codes
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    assert!(first
        .data
        .iter()
        .all(|row| row.status == LPN_CONTAINER_STATUS_IDLE && row.capacity_cm3 == Some(8_000)));

    let replay = repo
        .batch_create(
            &actor,
            BatchCreateLpnContainerRequest {
                container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
                capacity_cm3: Some(8_000),
                count: 3,
            },
            at(10),
            "lpn-batch-create",
        )
        .await
        .expect("replay");
    assert_eq!(
        replay.data.iter().map(|row| row.id).collect::<Vec<_>>(),
        first.data.iter().map(|row| row.id).collect::<Vec<_>>()
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lpn_containers WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("row count");
    assert_eq!(count, 3);
}

#[sqlx::test(migrations = "../../migrations")]
async fn batch_create_rejects_invalid_count(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    let repo = PgLpnContainerRepository::new(pool);
    let actor = ctx(owner_id);
    assert!(matches!(
        repo.batch_create(
            &actor,
            BatchCreateLpnContainerRequest {
                container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
                capacity_cm3: None,
                count: 0,
            },
            at(9),
            "lpn-batch-zero",
        )
        .await,
        Err(LpnContainerRepositoryError::BatchCountInvalid)
    ));
    assert!(matches!(
        repo.batch_create(
            &actor,
            BatchCreateLpnContainerRequest {
                container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
                capacity_cm3: None,
                count: LPN_BATCH_CREATE_MAX_COUNT + 1,
            },
            at(10),
            "lpn-batch-over",
        )
        .await,
        Err(LpnContainerRepositoryError::BatchCountInvalid)
    ));
}
