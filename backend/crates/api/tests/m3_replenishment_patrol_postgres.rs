//! T08：Min-Max 巡检引擎（GWT 2/3/11/19/26）。

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    replenishment_min_max_job, replenishment_repository::PgReplenishmentRepository,
    replenishment_service::ReplenishmentService,
};
use wms_domain::Quantity;

struct World {
    owner_id: Uuid,
    product_id: Uuid,
    storage_id: Uuid,
    pick_id: Uuid,
    source_batch_id: Uuid,
    strategy_id: Uuid,
}

async fn seed_world(pool: &PgPool, pick_on_hand: i64, source_on_hand: i64) -> World {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '巡检货主')")
        .bind(owner_id)
        .bind(format!("PT-{}", &owner_id.simple().to_string()[..8]))
        .execute(pool)
        .await
        .expect("owner");
    let product_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '巡检商品', '1', 'pending_mapping', now(), now())
        "#,
    )
    .bind(product_id)
    .bind(owner_id)
    .bind(format!("P-{}", &product_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("product");
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '巡检仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("wh");
    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, status, created_at, updated_at
        ) VALUES ($1, $2, $3, 'Z-PT', '合格区', 'normal_10_30', 'qualified_green', 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("zone");
    let storage_id = seed_loc(pool, owner_id, warehouse_id, zone_id, "ST-01", "storage").await;
    let pick_id = seed_loc(pool, owner_id, warehouse_id, zone_id, "PP-01", "piece_pick").await;
    if pick_on_hand > 0 {
        seed_batch(
            pool,
            owner_id,
            product_id,
            pick_id,
            "PP-01",
            "B-PICK",
            pick_on_hand,
            NaiveDate::from_ymd_opt(2028, 6, 1).expect("e"),
            None,
        )
        .await;
    }
    let source_batch_id = seed_batch(
        pool,
        owner_id,
        product_id,
        storage_id,
        "ST-01",
        "B-SRC",
        source_on_hand,
        NaiveDate::from_ymd_opt(2028, 1, 1).expect("e"),
        None,
    )
    .await;
    let strategy_id = insert_strategy(pool, owner_id, product_id, pick_id).await;
    World {
        owner_id,
        product_id,
        storage_id,
        pick_id,
        source_batch_id,
        strategy_id,
    }
}

async fn seed_loc(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    zone_id: Uuid,
    code: &str,
    location_type: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code,
            row_no, column_no, layer_no, max_volume_cm3, max_sku_count,
            location_type, current_owner_id, status, allows_container,
            mix_product_policy, mix_batch_policy, lock_status, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, 1, 1, 1, 100000, 10,
            $6, $2, 'available', FALSE, 'single_product_only', 'single_batch', 'normal', now(), now()
        )
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(code)
    .bind(location_type)
    .execute(pool)
    .await
    .expect("loc");
    id
}

#[allow(clippy::too_many_arguments)]
async fn seed_batch(
    pool: &PgPool,
    owner_id: Uuid,
    product_id: Uuid,
    location_id: Uuid,
    location_code: &str,
    batch_no: &str,
    on_hand: i64,
    expiry: NaiveDate,
    container_lpn: Option<&str>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_id, product_code, batch_no,
            production_date, expiry_date, qty_on_hand, qty_frozen, qty_allocated,
            qty_replenish_in_transit, qty_replenish_out_transit,
            status, location_id, location_code, container_lpn, recall_flag,
            created_at, updated_at, version
        ) VALUES (
            $1, $2, $3, 'P-PT', $4, $5, $6, $7, 0, 0, 0, 0,
            'qualified', $8, $9, $10, FALSE, now(), now(), 1
        )
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(product_id)
    .bind(batch_no)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("p"))
    .bind(expiry)
    .bind(Quantity::from(on_hand))
    .bind(location_id)
    .bind(location_code)
    .bind(container_lpn)
    .execute(pool)
    .await
    .expect("batch");
    id
}

async fn insert_strategy(pool: &PgPool, owner_id: Uuid, product_id: Uuid, pick_id: Uuid) -> Uuid {
    let strategy_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO replenishment_strategies (
            id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
            location_type, source_type, target_type,
            min_safety_threshold, max_replenish_target, trigger_modes, enabled
        ) VALUES (
            $1, $2, 'STR-PT', '巡检策略', 'product', $3,
            'piece_pick', 'storage', 'piece_pick',
            5, 20, ARRAY['min_max'], TRUE
        )
        "#,
    )
    .bind(strategy_id)
    .bind(owner_id)
    .bind(product_id)
    .execute(pool)
    .await
    .expect("strategy");
    sqlx::query(
        "UPDATE warehouse_locations SET replenish_strategy_id = $2 WHERE id = $1 AND owner_id = $3",
    )
    .bind(pick_id)
    .bind(strategy_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("bind");
    strategy_id
}

fn service(pool: PgPool) -> ReplenishmentService {
    ReplenishmentService::new(PgReplenishmentRepository::new(pool))
}

async fn task_count(pool: &PgPool, owner_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM replenishment_tasks WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(pool)
        .await
        .expect("count")
}

#[sqlx::test(migrations = "../../migrations")]
async fn patrol_generates_then_skips_when_in_transit_covers(pool: PgPool) {
    let world = seed_world(&pool, 2, 30).await;
    let created = service(pool.clone())
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("patrol 1");
    let mine: Vec<_> = created
        .into_iter()
        .filter(|task| task.owner_id == world.owner_id)
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].qty, Quantity::from(18));
    assert_eq!(mine[0].status, "pending");
    assert_eq!(mine[0].trigger_mode, "min_max");
    assert_eq!(mine[0].source_batch_id, world.source_batch_id);
    let second = service(pool.clone())
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("patrol 2");
    assert!(second
        .iter()
        .all(|task| task.owner_id != world.owner_id || task.id == mine[0].id));
    assert_eq!(task_count(&pool, world.owner_id).await, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn patrol_skips_quarantine_source_and_picks_next_fefo(pool: PgPool) {
    let world = seed_world(&pool, 2, 0).await;
    let lpn_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO lpn_containers (
            id, owner_id, lpn_code, container_type, status, location_id,
            current_lock_category, created_at, updated_at
        ) VALUES ($1, $2, 'LPN-Q', 'pallet', 'in_use', $3, 'quarantine', now(), now())
        "#,
    )
    .bind(lpn_id)
    .bind(world.owner_id)
    .bind(world.storage_id)
    .execute(&pool)
    .await
    .expect("lpn");
    seed_batch(
        &pool,
        world.owner_id,
        world.product_id,
        world.storage_id,
        "ST-01",
        "B-LOCK",
        30,
        NaiveDate::from_ymd_opt(2027, 1, 1).expect("earlier"),
        Some("LPN-Q"),
    )
    .await;
    let next = seed_batch(
        &pool,
        world.owner_id,
        world.product_id,
        world.storage_id,
        "ST-01",
        "B-OK",
        30,
        NaiveDate::from_ymd_opt(2028, 6, 1).expect("later"),
        None,
    )
    .await;
    let created = service(pool.clone())
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("patrol");
    let mine: Vec<_> = created
        .into_iter()
        .filter(|task| task.owner_id == world.owner_id)
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].source_batch_id, next);
}

#[sqlx::test(migrations = "../../migrations")]
async fn patrol_writes_fail_event_when_no_qualified_source(pool: PgPool) {
    let world = seed_world(&pool, 2, 0).await;
    let created = service(pool.clone())
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("patrol");
    assert!(created.iter().all(|task| task.owner_id != world.owner_id));
    let fails: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM event_bus_event
         WHERE owner_id = $1 AND event_type = 'replenishment.patrol_fail'
        "#,
    )
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("fail events");
    assert_eq!(fails, 1);
    let _ = world.strategy_id;
}

#[sqlx::test(migrations = "../../migrations")]
async fn patrol_does_not_generate_when_pack_floor_is_zero(pool: PgPool) {
    let world = seed_world(&pool, 15, 30).await;
    sqlx::query(
        r#"
        UPDATE replenishment_strategies
           SET min_safety_threshold = 16, max_replenish_target = 20
         WHERE id = $1
        "#,
    )
    .bind(world.strategy_id)
    .execute(&pool)
    .await
    .expect("need 5");
    sqlx::query(
        r#"
        INSERT INTO product_packaging_levels (
            id, owner_id, product_id, unit_code, unit_name, ratio_to_base,
            is_base, is_default, sort_order, created_at, updated_at
        ) VALUES (
            $1, $2, $3, 'CS', '箱', 12, FALSE, TRUE, 1, now(), now()
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(world.owner_id)
    .bind(world.product_id)
    .execute(&pool)
    .await
    .expect("pack");
    let created = service(pool.clone())
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("patrol");
    assert!(created.iter().all(|task| task.owner_id != world.owner_id));
    assert_eq!(task_count(&pool, world.owner_id).await, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn patrol_generates_for_empty_product_scope_location(pool: PgPool) {
    let world = seed_world(&pool, 0, 30).await;
    let created = service(pool.clone())
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("patrol");
    let mine: Vec<_> = created
        .into_iter()
        .filter(|task| task.owner_id == world.owner_id)
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].qty, Quantity::from(20));
    let (on_hand, in_t): (Quantity, Quantity) = sqlx::query_as(
        r#"
        SELECT qty_on_hand, qty_replenish_in_transit
          FROM inventory_batches
         WHERE location_id = $1 AND product_id = $2
        "#,
    )
    .bind(world.pick_id)
    .bind(world.product_id)
    .fetch_one(&pool)
    .await
    .expect("target row");
    assert_eq!(on_hand, Quantity::ZERO);
    assert_eq!(in_t, Quantity::from(20));
}

#[sqlx::test(migrations = "../../migrations")]
async fn patrol_category_scope_skips_unrelated_product(pool: PgPool) {
    let world = seed_world(&pool, 2, 30).await;
    let other = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, status,
            special_drug_category, created_at, updated_at
        ) VALUES ($1, $2, $3, '无关商品', '1', 'pending_mapping', 'none', now(), now())
        "#,
    )
    .bind(other)
    .bind(world.owner_id)
    .bind(format!("P-O-{}", &other.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .expect("other product");
    seed_batch(
        &pool,
        world.owner_id,
        other,
        world.pick_id,
        "PP-01",
        "B-OTHER",
        2,
        NaiveDate::from_ymd_opt(2028, 1, 1).expect("d"),
        None,
    )
    .await;
    sqlx::query(
        "UPDATE products SET special_drug_category = 'narcotic' WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.product_id)
    .bind(world.owner_id)
    .execute(&pool)
    .await
    .expect("mark narcotic");
    sqlx::query(
        r#"
        UPDATE replenishment_strategies
           SET scope_type = 'category',
               scope_ref = '10000000-0000-0000-0000-000000000022'
         WHERE id = $1 AND owner_id = $2
        "#,
    )
    .bind(world.strategy_id)
    .bind(world.owner_id)
    .execute(&pool)
    .await
    .expect("category scope");
    let created = service(pool.clone())
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("patrol");
    let mine: Vec<_> = created
        .into_iter()
        .filter(|task| task.owner_id == world.owner_id)
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].product_id, world.product_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn patrol_skips_target_lock_in_and_source_lock_out(pool: PgPool) {
    let world = seed_world(&pool, 2, 30).await;
    sqlx::query(
        "UPDATE warehouse_locations SET lock_status = 'lock_in' WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.pick_id)
    .bind(world.owner_id)
    .execute(&pool)
    .await
    .expect("lock target");
    let blocked = service(pool.clone())
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("target lock");
    assert!(blocked.iter().all(|task| task.owner_id != world.owner_id));
    sqlx::query(
        "UPDATE warehouse_locations SET lock_status = 'normal' WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.pick_id)
    .bind(world.owner_id)
    .execute(&pool)
    .await
    .expect("unlock target");
    sqlx::query(
        "UPDATE warehouse_locations SET lock_status = 'lock_out' WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.storage_id)
    .bind(world.owner_id)
    .execute(&pool)
    .await
    .expect("lock source");
    let source_blocked = service(pool)
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("source lock");
    assert!(source_blocked
        .iter()
        .all(|task| task.owner_id != world.owner_id));
}

#[sqlx::test(migrations = "../../migrations")]
async fn patrol_fails_when_target_temperature_mismatches(pool: PgPool) {
    let world = seed_world(&pool, 2, 30).await;
    sqlx::query(
        "UPDATE products SET storage_condition = 'cold_2_8' WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.product_id)
    .bind(world.owner_id)
    .execute(&pool)
    .await
    .expect("cold product");
    let created = service(pool.clone())
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("patrol");
    assert!(created.iter().all(|task| task.owner_id != world.owner_id));
    let fails: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM event_bus_event
         WHERE owner_id = $1 AND event_type = 'replenishment.patrol_fail'
        "#,
    )
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("fail events");
    assert_eq!(fails, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn patrol_fefo_continues_to_next_batch_when_first_is_short(pool: PgPool) {
    let world = seed_world(&pool, 0, 6).await;
    seed_batch(
        &pool,
        world.owner_id,
        world.product_id,
        world.storage_id,
        "ST-01",
        "B-SRC-2",
        30,
        NaiveDate::from_ymd_opt(2028, 6, 1).expect("later"),
        None,
    )
    .await;
    let created = service(pool.clone())
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("patrol");
    let mut mine: Vec<_> = created
        .into_iter()
        .filter(|task| task.owner_id == world.owner_id)
        .collect();
    mine.sort_by_key(|task| task.qty);
    assert_eq!(mine.len(), 2);
    assert_eq!(mine[0].source_batch_id, world.source_batch_id);
    assert_eq!(mine[0].qty, Quantity::from(6));
    assert_eq!(mine[1].qty, Quantity::from(14));
    assert_ne!(mine[1].source_batch_id, world.source_batch_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn patrol_skips_target_under_inventory_count(pool: PgPool) {
    let world = seed_world(&pool, 2, 30).await;
    let count_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_counts (
            id, owner_id, count_type, status, started_at, created_by, created_at, updated_at
        ) VALUES ($1, $2, 'cycle', 'in_progress', now(), $3, now(), now())
        "#,
    )
    .bind(count_id)
    .bind(world.owner_id)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("count");
    let pick_batch: Uuid = sqlx::query_scalar(
        "SELECT id FROM inventory_batches WHERE owner_id = $1 AND location_id = $2 LIMIT 1",
    )
    .bind(world.owner_id)
    .bind(world.pick_id)
    .fetch_one(&pool)
    .await
    .expect("pick batch");
    sqlx::query(
        r#"
        INSERT INTO inventory_count_lines (
            id, count_id, owner_id, inventory_batch_id, location_id, location_code,
            product_code, batch_no, book_qty, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, 'PP-01', 'P-PT', 'B-PICK', 2, now(), now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(count_id)
    .bind(world.owner_id)
    .bind(pick_batch)
    .bind(world.pick_id)
    .execute(&pool)
    .await
    .expect("count line");
    let created = service(pool)
        .run_min_max_patrol(chrono::Utc::now())
        .await
        .expect("patrol");
    assert!(created.iter().all(|task| task.owner_id != world.owner_id));
}

#[test]
fn min_max_job_is_registered_on_mte_skeleton() {
    assert_eq!(replenishment_min_max_job::JOB_NAME, "replenishment_min_max");
}
