use chrono::Utc;
use sqlx::PgPool;
use wms_api::{
    lpn_container_repository::PgLpnContainerRepository,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};

#[path = "support/lpn_container.rs"]
mod lpn_support;
mod postgres_test_support;
use lpn_support::{
    at, batch_container_lpn, create_req, ctx, loose_putaway_req, putaway_req, seed_lpn_numbering,
    seed_putaway,
};
use postgres_test_support::ensure_audit_partition;

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_rejects_lpn_after_loose_same_sku_batch_location(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let fixture = seed_putaway(&pool).await;
    seed_lpn_numbering(&pool, at(0), fixture.owner_id).await;
    let actor = ctx(fixture.owner_id);
    let lpn_repo = PgLpnContainerRepository::new(pool.clone());
    let wave3 = PgWave3Repository::new(pool.clone());
    let created = lpn_repo
        .create(&actor, create_req(), at(9), "lpn-after-loose")
        .await
        .expect("lpn master");
    wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            loose_putaway_req(&fixture),
            Utc::now(),
            "lpn-loose-first",
            None,
        )
        .await
        .expect("loose putaway");
    let denied = wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            putaway_req(&fixture, &created.lpn_code),
            Utc::now(),
            "lpn-after-loose-second",
            None,
        )
        .await
        .expect_err("lpn must not claim loose stock");
    assert_eq!(denied, Wave3RepositoryError::LpnNotUsable);
    assert_eq!(batch_container_lpn(&pool, fixture.owner_id).await, None);
    let qty: wms_domain::Quantity = sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE owner_id = $1 AND product_code = 'LPN-P-001' AND batch_no = 'LPN-B-001'",
    )
    .bind(fixture.owner_id)
    .fetch_one(&pool)
    .await
    .expect("loose qty unchanged");
    assert_eq!(qty, 2.into());
    let status: String = sqlx::query_scalar(
        "SELECT status FROM lpn_containers WHERE owner_id = $1 AND lpn_code = $2",
    )
    .bind(fixture.owner_id)
    .bind(&created.lpn_code)
    .fetch_one(&pool)
    .await
    .expect("lpn stays idle");
    assert_eq!(status, "idle");
    let putaway_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM receiving_putaways WHERE owner_id = $1 AND receiving_order_id = $2",
    )
    .bind(fixture.owner_id)
    .bind(fixture.order_id)
    .fetch_one(&pool)
    .await
    .expect("putaway rows");
    assert_eq!(putaway_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_rejects_loose_after_lpn_same_sku_batch_location(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let fixture = seed_putaway(&pool).await;
    seed_lpn_numbering(&pool, at(0), fixture.owner_id).await;
    let actor = ctx(fixture.owner_id);
    let lpn_repo = PgLpnContainerRepository::new(pool.clone());
    let wave3 = PgWave3Repository::new(pool.clone());
    let created = lpn_repo
        .create(&actor, create_req(), at(9), "lpn-before-loose")
        .await
        .expect("lpn master");
    wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            putaway_req(&fixture, &created.lpn_code),
            Utc::now(),
            "lpn-identity-first",
            None,
        )
        .await
        .expect("lpn putaway");
    let denied = wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            loose_putaway_req(&fixture),
            Utc::now(),
            "lpn-loose-second",
            None,
        )
        .await
        .expect_err("loose must not merge onto lpn stock");
    assert_eq!(denied, Wave3RepositoryError::LpnNotUsable);
    assert_eq!(
        batch_container_lpn(&pool, fixture.owner_id)
            .await
            .as_deref(),
        Some(created.lpn_code.as_str())
    );
    let qty: wms_domain::Quantity = sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE owner_id = $1 AND product_code = 'LPN-P-001' AND batch_no = 'LPN-B-001'",
    )
    .bind(fixture.owner_id)
    .fetch_one(&pool)
    .await
    .expect("lpn qty unchanged");
    assert_eq!(qty, 2.into());
    let putaway_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM receiving_putaways WHERE owner_id = $1 AND receiving_order_id = $2",
    )
    .bind(fixture.owner_id)
    .bind(fixture.order_id)
    .fetch_one(&pool)
    .await
    .expect("putaway rows");
    assert_eq!(putaway_count, 1);
}
