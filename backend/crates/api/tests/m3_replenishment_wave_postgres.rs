//! T09：波次缺口引擎（GWT 4/27）。不改正文出库行。

use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext, wave4_handlers::Wave4ReplenishService, wave4_repository::PgWave4Repository,
};
use wms_domain::{
    CreateOutboundOrderLineRequest, CreateOutboundOrderRequest, CreateOutboundWaveRequest, Quantity,
};

#[path = "support/h9.rs"]
mod h9_support;
#[path = "support/replenishment_wave.rs"]
mod replenishment_wave;

use replenishment_wave::{
    ctx, gap_req, insert_wave_gap_strategy, seed_loc, seed_world, service, World,
};

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
async fn wave_gap_returns_empty_and_patrol_fail_when_no_source(pool: PgPool) {
    let world = seed_world(&pool, 0, 0).await;
    insert_wave_gap_strategy(&pool, world.owner_id, world.product_id, world.pick_id).await;
    let created = service(pool.clone())
        .create_wave_gap_tasks(&ctx(world.owner_id), gap_req(&world, 8))
        .await
        .expect("gap");
    assert!(created.is_empty());
    let fails: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM event_bus_event
         WHERE owner_id = $1 AND event_type = 'replenishment.patrol_fail'
        "#,
    )
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("fail");
    assert_eq!(fails, 1);
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
    assert_eq!(created[0].target_location_id, world.pick_id);
    assert_eq!(created[0].wave_id, Some(wave_id));
}

async fn insert_wave_order(pool: &PgPool, world: &World, wave_id: Uuid, planned_qty: i64) {
    let warehouse_id: Uuid = sqlx::query_scalar(
        "SELECT warehouse_id FROM warehouse_locations WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.pick_id)
    .bind(world.owner_id)
    .fetch_one(pool)
    .await
    .expect("warehouse");
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
    .execute(pool)
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
    .execute(pool)
    .await
    .expect("order");
    sqlx::query(
        r#"
        INSERT INTO outbound_order_lines (
            id, outbound_order_id, owner_id, line_no, product_code, batch_no, planned_qty
        ) VALUES ($1, $2, $3, 1, $4, 'B-SRC', $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(order_id)
    .bind(world.owner_id)
    .bind(format!("P-{}", &world.product_id.simple().to_string()[..8]))
    .bind(Quantity::from(planned_qty))
    .execute(pool)
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
    .execute(pool)
    .await
    .expect("wave order");
}

#[sqlx::test(migrations = "../../migrations")]
async fn fill_wave_pick_gaps_uses_bound_pick_not_first_warehouse_pick(pool: PgPool) {
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
    let zone_id: Uuid = sqlx::query_scalar(
        "SELECT zone_id FROM warehouse_locations WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.pick_id)
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("zone");
    let decoy = seed_loc(
        &pool,
        world.owner_id,
        warehouse_id,
        zone_id,
        "PP-00",
        "piece_pick",
    )
    .await;
    let wave_id = Uuid::new_v4();
    insert_wave_order(&pool, &world, wave_id, 8).await;
    let created = service(pool)
        .fill_wave_pick_gaps(world.owner_id, wave_id)
        .await
        .expect("fill");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].target_location_id, world.pick_id);
    assert_ne!(created[0].target_location_id, decoy);
}

#[sqlx::test(migrations = "../../migrations")]
async fn fill_wave_pick_gaps_skips_putaway_blocked_without_failing(pool: PgPool) {
    let world = seed_world(&pool, 0, 30).await;
    insert_wave_gap_strategy(&pool, world.owner_id, world.product_id, world.pick_id).await;
    sqlx::query(
        "UPDATE warehouse_locations SET lock_status = 'lock_in' WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.pick_id)
    .bind(world.owner_id)
    .execute(&pool)
    .await
    .expect("lock");
    let wave_id = Uuid::new_v4();
    insert_wave_order(&pool, &world, wave_id, 8).await;
    let created = service(pool.clone())
        .fill_wave_pick_gaps(world.owner_id, wave_id)
        .await
        .expect("fill must not fail");
    assert!(created.is_empty());
    let reason: Option<String> = sqlx::query_scalar(
        r#"
        SELECT payload ->> 'reason_code'
          FROM event_bus_event
         WHERE owner_id = $1 AND event_type = 'replenishment.patrol_fail'
         LIMIT 1
        "#,
    )
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("patrol fail");
    assert_eq!(reason.as_deref(), Some("putaway_blocked"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn fill_wave_pick_gaps_ignores_this_wave_frozen_qty(pool: PgPool) {
    let world = seed_world(&pool, 3, 30).await;
    insert_wave_gap_strategy(&pool, world.owner_id, world.product_id, world.pick_id).await;
    let wave_id = Uuid::new_v4();
    insert_wave_order(&pool, &world, wave_id, 10).await;
    let order_id: Uuid = sqlx::query_scalar(
        "SELECT outbound_order_id FROM outbound_wave_orders WHERE wave_id = $1 AND owner_id = $2",
    )
    .bind(wave_id)
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("order");
    let pick_batch_id: Uuid = sqlx::query_scalar(
        r#"
        SELECT id FROM inventory_batches
         WHERE owner_id = $1 AND location_id = $2 AND product_id = $3
        "#,
    )
    .bind(world.owner_id)
    .bind(world.pick_id)
    .bind(world.product_id)
    .fetch_one(&pool)
    .await
    .expect("pick batch");
    sqlx::query("UPDATE inventory_batches SET qty_frozen = 3 WHERE id = $1")
        .bind(pick_batch_id)
        .execute(&pool)
        .await
        .expect("freeze pick face");
    sqlx::query(
        r#"
        INSERT INTO inventory_allocations (
            id, owner_id, outbound_order_id, line_no, batch_id, allocated_qty, status, created_at, updated_at
        ) VALUES ($1, $2, $3, 1, $4, 3, 'locked', now(), now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(world.owner_id)
    .bind(order_id)
    .bind(pick_batch_id)
    .execute(&pool)
    .await
    .expect("allocation");
    let created = service(pool)
        .fill_wave_pick_gaps(world.owner_id, wave_id)
        .await
        .expect("fill");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].qty, Quantity::from(7));
    assert_eq!(created[0].target_location_id, world.pick_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn fill_wave_pick_gaps_ignores_storage_allocation_location(pool: PgPool) {
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
    let zone_id: Uuid = sqlx::query_scalar(
        "SELECT zone_id FROM warehouse_locations WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.pick_id)
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("zone");
    let storage_pick = seed_loc(
        &pool,
        world.owner_id,
        warehouse_id,
        zone_id,
        "ST-09",
        "storage",
    )
    .await;
    let wave_id = Uuid::new_v4();
    insert_wave_order(&pool, &world, wave_id, 8).await;
    let order_id: Uuid = sqlx::query_scalar(
        "SELECT outbound_order_id FROM outbound_wave_orders WHERE wave_id = $1 AND owner_id = $2",
    )
    .bind(wave_id)
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("order");
    sqlx::query(
        r#"
        INSERT INTO outbound_pick_tasks (
            id, owner_id, wave_id, outbound_order_id, line_no, batch_id,
            product_code, batch_no, location_id, location_code, planned_qty, route_sequence
        ) VALUES (
            $1, $2, $3, $4, 1, $5, $6, 'B-SRC', $7, 'ST-09', 8, 1
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(world.owner_id)
    .bind(wave_id)
    .bind(order_id)
    .bind(world.source_batch_id)
    .bind(format!("P-{}", &world.product_id.simple().to_string()[..8]))
    .bind(storage_pick)
    .execute(&pool)
    .await
    .expect("pick task");
    let created = service(pool)
        .fill_wave_pick_gaps(world.owner_id, wave_id)
        .await
        .expect("fill");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].target_location_id, world.pick_id);
    assert_ne!(created[0].target_location_id, storage_pick);
}

#[sqlx::test(migrations = "../../migrations")]
async fn fill_wave_pick_gaps_uses_default_route_when_pick_unbound(pool: PgPool) {
    let world = seed_world(&pool, 0, 30).await;
    let wave_id = Uuid::new_v4();
    insert_wave_order(&pool, &world, wave_id, 8).await;
    let created = service(pool)
        .fill_wave_pick_gaps(world.owner_id, wave_id)
        .await
        .expect("fill");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].trigger_mode, "wave_gap");
    assert_eq!(created[0].target_location_id, world.pick_id);
    assert!(created[0].strategy_id.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn wave4_replenish_after_allocate_uses_pick_face_gap(pool: PgPool) {
    let world = seed_world(&pool, 3, 30).await;
    insert_wave_gap_strategy(&pool, world.owner_id, world.product_id, world.pick_id).await;
    let warehouse_id: Uuid = sqlx::query_scalar(
        "SELECT warehouse_id FROM warehouse_locations WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.pick_id)
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("warehouse");
    let product_code = format!("P-{}", &world.product_id.simple().to_string()[..8]);
    sqlx::query(
        r#"
        UPDATE inventory_batches
           SET product_code = $3, batch_no = 'B-SRC'
         WHERE owner_id = $1 AND product_id = $2
        "#,
    )
    .bind(world.owner_id)
    .bind(world.product_id)
    .bind(&product_code)
    .execute(&pool)
    .await
    .expect("align batch codes");
    let now = Utc
        .with_ymd_and_hms(2026, 8, 19, 9, 0, 0)
        .single()
        .expect("now");
    let customer_id = Uuid::new_v4();
    let delivery_address_id = h9_support::seed_outbound_route_binding(
        &pool,
        world.owner_id,
        warehouse_id,
        customer_id,
        now,
    )
    .await;
    let waves = Arc::new(PgWave4Repository::new(pool.clone()));
    let replenishment = Arc::new(service(pool.clone()));
    let orchestrator = Wave4ReplenishService::new(pool.clone(), waves.clone(), replenishment);
    let ctx = AuthContext {
        user_id: Uuid::new_v4(),
        owner_id: world.owner_id,
        actor_name: "wave-fill-it".into(),
        permissions: vec!["m4.write".into(), "m3.replenishment.manage".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    };
    let order = waves
        .create_outbound_order(
            &ctx,
            CreateOutboundOrderRequest {
                document_type: "sales_outbound".to_string(),
                wms_order_no: "WMS-GAP-001".to_string(),
                erp_order_no: None,
                invoice_no: None,
                transport_mode_code: None,
                department_code: None,
                sales_group_code: None,
                order_group_no: None,
                business_type_code: None,
                customer_id,
                warehouse_id,
                delivery_address_id,
                required_ship_at: None,
                lines: vec![CreateOutboundOrderLineRequest {
                    line_no: 1,
                    product_code,
                    batch_no: "B-SRC".to_string(),
                    planned_qty: 10.into(),
                }],
            },
            now,
            "wave-gap-order-1",
            None,
        )
        .await
        .expect("order")
        .value;
    let wave = orchestrator
        .create_outbound_wave(
            &ctx,
            CreateOutboundWaveRequest {
                wave_no: "WAVE-GAP-001".to_string(),
                order_ids: vec![order.id],
            },
            now,
            "wave-gap-create-1",
            None,
        )
        .await
        .expect("wave")
        .value;
    let tasks: Vec<(Uuid, Quantity)> = sqlx::query_as(
        r#"
        SELECT target_location_id, qty
          FROM replenishment_tasks
         WHERE owner_id = $1 AND wave_id = $2
        "#,
    )
    .bind(world.owner_id)
    .bind(wave.id)
    .fetch_all(&pool)
    .await
    .expect("tasks");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].0, world.pick_id);
    assert_eq!(tasks[0].1, Quantity::from(7));
}
