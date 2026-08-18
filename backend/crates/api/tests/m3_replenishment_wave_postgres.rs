//! T09：波次缺口引擎（GWT 4/27）。不改正文出库行。

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    replenishment_repository::PgReplenishmentRepository,
    replenishment_service::{CreateWaveGapTasksRequest, ReplenishmentService},
};
use wms_domain::Quantity;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::nil(),
        owner_id,
        actor_name: "wave-engine".into(),
        permissions: vec!["m3.replenishment.manage".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

struct World {
    owner_id: Uuid,
    product_id: Uuid,
    pick_id: Uuid,
    source_batch_id: Uuid,
}

async fn seed_world(pool: &PgPool, pick_on_hand: i64, source_on_hand: i64) -> World {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '缺口货主')")
        .bind(owner_id)
        .bind(format!("WG-{}", &owner_id.simple().to_string()[..8]))
        .execute(pool)
        .await
        .expect("owner");
    let product_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '缺口商品', '1', 'pending_mapping', now(), now())
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
        ) VALUES ($1, $2, $3, '缺口仓', 'physical', 'active', now(), now())
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
        ) VALUES ($1, $2, $3, 'Z-WG', '合格区', 'normal_10_30', 'qualified_green', 'active', now(), now())
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
        seed_batch(pool, owner_id, product_id, pick_id, "PP-01", pick_on_hand).await;
    }
    let source_batch_id = seed_batch(
        pool,
        owner_id,
        product_id,
        storage_id,
        "ST-01",
        source_on_hand,
    )
    .await;
    World {
        owner_id,
        product_id,
        pick_id,
        source_batch_id,
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

async fn seed_batch(
    pool: &PgPool,
    owner_id: Uuid,
    product_id: Uuid,
    location_id: Uuid,
    location_code: &str,
    on_hand: i64,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_id, product_code, batch_no,
            production_date, expiry_date, qty_on_hand, qty_frozen, qty_allocated,
            qty_replenish_in_transit, qty_replenish_out_transit,
            status, location_id, location_code, recall_flag, created_at, updated_at, version
        ) VALUES (
            $1, $2, $3, 'P-WG', 'B1', $4, $5, $6, 0, 0, 0, 0,
            'qualified', $7, $8, FALSE, now(), now(), 1
        )
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(product_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("p"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("e"))
    .bind(Quantity::from(on_hand))
    .bind(location_id)
    .bind(location_code)
    .execute(pool)
    .await
    .expect("batch");
    id
}

async fn insert_wave_gap_strategy(
    pool: &PgPool,
    owner_id: Uuid,
    product_id: Uuid,
    pick_id: Uuid,
) -> Uuid {
    let strategy_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO replenishment_strategies (
            id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
            location_type, source_type, target_type,
            min_safety_threshold, max_replenish_target, trigger_modes, enabled
        ) VALUES (
            $1, $2, 'STR-WG', '缺口策略', 'product', $3,
            'piece_pick', 'storage', 'piece_pick',
            0, 100, ARRAY['wave_gap'], TRUE
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

fn gap_req(world: &World, demand: i64) -> CreateWaveGapTasksRequest {
    CreateWaveGapTasksRequest {
        wave_id: Uuid::new_v4(),
        outbound_order_id: Uuid::new_v4(),
        outbound_line_no: 1,
        product_id: world.product_id,
        demand_qty: Quantity::from(demand),
        target_location_id: world.pick_id,
    }
}

async fn line_column_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = 'outbound_order_lines'
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("columns")
}

#[sqlx::test(migrations = "../../migrations")]
async fn wave_gap_creates_urgent_task_and_waiting_event(pool: PgPool) {
    let world = seed_world(&pool, 3, 30).await;
    let strategy_id =
        insert_wave_gap_strategy(&pool, world.owner_id, world.product_id, world.pick_id).await;
    let before_cols = line_column_count(&pool).await;
    let req = gap_req(&world, 10);
    let created = service(pool.clone())
        .create_wave_gap_tasks(&ctx(world.owner_id), req)
        .await
        .expect("gap");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].trigger_mode, "wave_gap");
    assert_eq!(created[0].priority, "urgent");
    assert_eq!(created[0].qty, Quantity::from(7));
    assert_eq!(created[0].strategy_id, Some(strategy_id));
    assert_eq!(created[0].source_batch_id, world.source_batch_id);
    assert!(created[0].created_by.starts_with("system:wave:"));
    let waiting: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM event_bus_event
         WHERE owner_id = $1 AND event_type = 'replenishment.waiting'
        "#,
    )
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("waiting");
    assert_eq!(waiting, 1);
    assert_eq!(line_column_count(&pool).await, before_cols);
}

#[sqlx::test(migrations = "../../migrations")]
async fn wave_gap_without_strategy_uses_default_storage_route(pool: PgPool) {
    let world = seed_world(&pool, 0, 30).await;
    let created = service(pool.clone())
        .create_wave_gap_tasks(&ctx(world.owner_id), gap_req(&world, 5))
        .await
        .expect("gap");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].trigger_mode, "wave_gap");
    assert_eq!(created[0].priority, "urgent");
    assert_eq!(created[0].qty, Quantity::from(5));
    assert!(created[0].strategy_id.is_none());
    let source_type: String = sqlx::query_scalar(
        "SELECT location_type FROM warehouse_locations WHERE id = $1 AND owner_id = $2",
    )
    .bind(created[0].source_location_id)
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("source type");
    assert_eq!(source_type, "storage");
}

#[sqlx::test(migrations = "../../migrations")]
async fn wave_gap_returns_empty_when_available_covers_demand(pool: PgPool) {
    let world = seed_world(&pool, 10, 30).await;
    insert_wave_gap_strategy(&pool, world.owner_id, world.product_id, world.pick_id).await;
    let created = service(pool.clone())
        .create_wave_gap_tasks(&ctx(world.owner_id), gap_req(&world, 3))
        .await
        .expect("gap");
    assert!(created.is_empty());
    let waiting: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM event_bus_event
         WHERE owner_id = $1 AND event_type = 'replenishment.waiting'
        "#,
    )
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("waiting");
    assert_eq!(waiting, 0);
    let tasks: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM replenishment_tasks WHERE owner_id = $1")
            .bind(world.owner_id)
            .fetch_one(&pool)
            .await
            .expect("tasks");
    assert_eq!(tasks, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn fill_wave_pick_gaps_creates_urgent_from_wave_lines(pool: PgPool) {
    let world = seed_world(&pool, 0, 30).await;
    insert_wave_gap_strategy(&pool, world.owner_id, world.product_id, world.pick_id).await;
    let warehouse_id: Uuid = sqlx::query_scalar(
        "SELECT warehouse_id FROM warehouse_locations WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.pick_id)
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("warehouse");
    let wave_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO outbound_waves (id, owner_id, wave_no, status, created_at, updated_at)
        VALUES ($1, $2, $3, 'released', now(), now())
        "#,
    )
    .bind(wave_id)
    .bind(world.owner_id)
    .bind(format!("WV-{}", &wave_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .expect("wave");
    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, document_type, wms_order_no, warehouse_id,
            customer_id, delivery_address_id, delivery_address_snapshot,
            status, short_pick, created_at, updated_at
        ) VALUES (
            $1, $2, 'sales_outbound', $3, $4,
            $5, $5, '{}'::jsonb,
            'confirmed', FALSE, now(), now()
        )
        "#,
    )
    .bind(order_id)
    .bind(world.owner_id)
    .bind(format!("SO-{}", &order_id.simple().to_string()[..8]))
    .bind(warehouse_id)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("order");
    sqlx::query(
        r#"
        INSERT INTO outbound_order_lines (
            id, outbound_order_id, owner_id, line_no, product_code, batch_no, planned_qty
        ) VALUES ($1, $2, $3, 1, $4, 'B-SRC', 8)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(order_id)
    .bind(world.owner_id)
    .bind(format!("P-{}", &world.product_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .expect("line");
    sqlx::query(
        r#"
        INSERT INTO outbound_wave_orders (id, owner_id, wave_id, outbound_order_id, created_at)
        VALUES ($1, $2, $3, $4, now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(world.owner_id)
    .bind(wave_id)
    .bind(order_id)
    .execute(&pool)
    .await
    .expect("wave order");
    let created = service(pool.clone())
        .fill_wave_pick_gaps(world.owner_id, wave_id)
        .await
        .expect("fill");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].trigger_mode, "wave_gap");
    assert_eq!(created[0].priority, "urgent");
    assert_eq!(created[0].qty, Quantity::from(8));
}
