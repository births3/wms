//! T03：库存补货三命令账务面（不经任务表）。

use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::inventory::{
    confirm_replenish_in_tx, release_replenish_in_tx, reserve_replenish_in_tx,
    InventoryReplenishError,
};
use wms_domain::Quantity;

struct SeededBatch {
    id: Uuid,
    owner_id: Uuid,
    product_id: Uuid,
}

async fn seed_source_batch(pool: &PgPool, on_hand: i64) -> SeededBatch {
    let owner_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_id, product_code, batch_no,
            production_date, expiry_date, qty_on_hand, qty_frozen, qty_allocated,
            qty_replenish_in_transit, qty_replenish_out_transit,
            status, location_id, location_code, recall_flag, created_at, updated_at, version
        )
        VALUES (
            $1, $2, $3, 'P-RP', 'B-RP',
            $4, $5, $6, 0, 0,
            0, 0,
            'qualified', $7, 'SRC-01', FALSE, $8, $8, 1
        )
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(product_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("production date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("expiry date"))
    .bind(Quantity::from(on_hand))
    .bind(location_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed source batch");
    SeededBatch {
        id,
        owner_id,
        product_id,
    }
}

async fn qty_snapshot(pool: &PgPool, batch_id: Uuid) -> (Quantity, Quantity, Quantity) {
    sqlx::query_as(
        r#"
        SELECT qty_on_hand, qty_replenish_in_transit, qty_replenish_out_transit
          FROM inventory_batches
         WHERE id = $1
        "#,
    )
    .bind(batch_id)
    .fetch_one(pool)
    .await
    .expect("fetch qty snapshot")
}

#[sqlx::test(migrations = "../../migrations")]
async fn reserve_increments_transit_without_on_hand_or_movements(pool: PgPool) {
    let source = seed_source_batch(&pool, 30).await;
    let target_location = Uuid::new_v4();
    let now = Utc::now();
    let mut tx = pool.begin().await.expect("begin");
    let target_id = reserve_replenish_in_tx(
        &mut tx,
        source.owner_id,
        source.id,
        target_location,
        Quantity::from(18),
        now,
    )
    .await
    .expect("reserve should succeed");
    tx.commit().await.expect("commit reserve");

    let (src_on, src_in, src_out) = qty_snapshot(&pool, source.id).await;
    assert_eq!(src_on, Quantity::from(30));
    assert_eq!(src_in, Quantity::ZERO);
    assert_eq!(src_out, Quantity::from(18));

    let (tgt_on, tgt_in, tgt_out) = qty_snapshot(&pool, target_id).await;
    assert_eq!(tgt_on, Quantity::ZERO);
    assert_eq!(tgt_in, Quantity::from(18));
    assert_eq!(tgt_out, Quantity::ZERO);

    let movements: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM inventory_movements WHERE owner_id = $1")
            .bind(source.owner_id)
            .fetch_one(&pool)
            .await
            .expect("count movements");
    assert_eq!(movements, 0, "reserve must not write inventory_movements");
    assert_ne!(target_id, source.id);
    let _ = source.product_id;
}

#[sqlx::test(migrations = "../../migrations")]
async fn confirm_converts_transit_to_on_hand_and_writes_two_replenish_movements(pool: PgPool) {
    let source = seed_source_batch(&pool, 30).await;
    let target_location = Uuid::new_v4();
    let now = Utc::now();
    let task_id = Uuid::new_v4();
    let mut tx = pool.begin().await.expect("begin");
    let target_id = reserve_replenish_in_tx(
        &mut tx,
        source.owner_id,
        source.id,
        target_location,
        Quantity::from(18),
        now,
    )
    .await
    .expect("reserve");
    confirm_replenish_in_tx(
        &mut tx,
        source.owner_id,
        source.id,
        target_id,
        Quantity::from(18),
        task_id,
        "SYSTEM",
        task_id.to_string().as_str(),
        now,
    )
    .await
    .expect("confirm");
    tx.commit().await.expect("commit confirm");

    let (src_on, _src_in, src_out) = qty_snapshot(&pool, source.id).await;
    assert_eq!(src_on, Quantity::from(12));
    assert_eq!(src_out, Quantity::ZERO);

    let (tgt_on, tgt_in, _tgt_out) = qty_snapshot(&pool, target_id).await;
    assert_eq!(tgt_on, Quantity::from(18));
    assert_eq!(tgt_in, Quantity::ZERO);

    let rows: Vec<(Uuid, String, Quantity)> = sqlx::query_as(
        r#"
        SELECT batch_id, movement_type, qty_delta
          FROM inventory_movements
         WHERE owner_id = $1
         ORDER BY qty_delta
        "#,
    )
    .bind(source.owner_id)
    .fetch_all(&pool)
    .await
    .expect("fetch movements");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, source.id);
    assert_eq!(rows[0].1, "replenish");
    assert_eq!(rows[0].2, Quantity::from(-18));
    assert_eq!(rows[1].0, target_id);
    assert_eq!(rows[1].1, "replenish");
    assert_eq!(rows[1].2, Quantity::from(18));
}

#[sqlx::test(migrations = "../../migrations")]
async fn release_reverses_transit_without_touching_on_hand(pool: PgPool) {
    let source = seed_source_batch(&pool, 30).await;
    let target_location = Uuid::new_v4();
    let now = Utc::now();
    let mut tx = pool.begin().await.expect("begin");
    let target_id = reserve_replenish_in_tx(
        &mut tx,
        source.owner_id,
        source.id,
        target_location,
        Quantity::from(18),
        now,
    )
    .await
    .expect("reserve");
    release_replenish_in_tx(
        &mut tx,
        source.owner_id,
        source.id,
        target_id,
        Quantity::from(18),
        now,
    )
    .await
    .expect("release");
    tx.commit().await.expect("commit release");

    let (src_on, _src_in, src_out) = qty_snapshot(&pool, source.id).await;
    assert_eq!(src_on, Quantity::from(30));
    assert_eq!(src_out, Quantity::ZERO);

    let (tgt_on, tgt_in, _tgt_out) = qty_snapshot(&pool, target_id).await;
    assert_eq!(tgt_on, Quantity::ZERO);
    assert_eq!(tgt_in, Quantity::ZERO);

    let movements: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM inventory_movements WHERE owner_id = $1")
            .bind(source.owner_id)
            .fetch_one(&pool)
            .await
            .expect("count movements");
    assert_eq!(movements, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn reserve_rejects_when_source_available_qty_insufficient(pool: PgPool) {
    let source = seed_source_batch(&pool, 5).await;
    let mut tx = pool.begin().await.expect("begin");
    let err = reserve_replenish_in_tx(
        &mut tx,
        source.owner_id,
        source.id,
        Uuid::new_v4(),
        Quantity::from(18),
        Utc::now(),
    )
    .await
    .expect_err("over-reserve must fail");
    assert_eq!(err, InventoryReplenishError::Insufficient);
}
