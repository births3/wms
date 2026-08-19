//! T10：超时扫描（GWT 14/28；urgent 10 分钟告警）。

use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    replenishment_repository::PgReplenishmentRepository,
    replenishment_service::{CreateWaveGapTasksRequest, ReplenishmentService},
    replenishment_timeout_job,
};
use wms_domain::Quantity;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::nil(),
        owner_id,
        actor_name: "timeout-scan".into(),
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

async fn seed_world(pool: &PgPool) -> World {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '超时货主')")
        .bind(owner_id)
        .bind(format!("TO-{}", &owner_id.simple().to_string()[..8]))
        .execute(pool)
        .await
        .expect("owner");
    let product_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '超时商品', '1', 'pending_mapping', now(), now())
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
        ) VALUES ($1, $2, $3, '超时仓', 'physical', 'active', now(), now())
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
        ) VALUES ($1, $2, $3, 'Z-TO', '合格区', 'normal_10_30', 'qualified_green', 'active', now(), now())
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
    let source_batch_id = seed_batch(pool, owner_id, product_id, storage_id, "ST-01", 30).await;
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
            $1, $2, $3, 'P-TO', 'B1', $4, $5, $6, 0, 0, 0, 0,
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

async fn create_urgent(pool: &PgPool, world: &World) -> Uuid {
    let tasks = ReplenishmentService::new(PgReplenishmentRepository::new(pool.clone()))
        .create_wave_gap_tasks(
            &ctx(world.owner_id),
            CreateWaveGapTasksRequest {
                wave_id: Uuid::new_v4(),
                outbound_order_id: Uuid::new_v4(),
                outbound_line_no: 1,
                product_id: world.product_id,
                demand_qty: Quantity::from(8),
                target_location_id: world.pick_id,
            },
        )
        .await
        .expect("gap");
    assert_eq!(tasks.len(), 1);
    tasks[0].id
}

fn service(pool: PgPool) -> ReplenishmentService {
    ReplenishmentService::new(PgReplenishmentRepository::new(pool))
}

async fn event_count(pool: &PgPool, owner_id: Uuid, event_type: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM event_bus_event WHERE owner_id = $1 AND event_type = $2",
    )
    .bind(owner_id)
    .bind(event_type)
    .fetch_one(pool)
    .await
    .expect("events")
}

async fn transit_out(pool: &PgPool, batch_id: Uuid) -> Quantity {
    sqlx::query_scalar("SELECT qty_replenish_out_transit FROM inventory_batches WHERE id = $1")
        .bind(batch_id)
        .fetch_one(pool)
        .await
        .expect("out")
}

#[sqlx::test(migrations = "../../migrations")]
async fn urgent_pending_20_minutes_is_cancelled_and_released(pool: PgPool) {
    let world = seed_world(&pool).await;
    let task_id = create_urgent(&pool, &world).await;
    assert!(transit_out(&pool, world.source_batch_id).await > Quantity::ZERO);
    sqlx::query("UPDATE replenishment_tasks SET created_at = $2 WHERE id = $1 AND owner_id = $3")
        .bind(task_id)
        .bind(Utc::now() - Duration::minutes(21))
        .bind(world.owner_id)
        .execute(&pool)
        .await
        .expect("backdate");
    service(pool.clone())
        .run_timeout_scan(Utc::now())
        .await
        .expect("scan");
    let status: String = sqlx::query_scalar(
        "SELECT status FROM replenishment_tasks WHERE id = $1 AND owner_id = $2",
    )
    .bind(task_id)
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("status");
    assert_eq!(status, "cancelled");
    assert_eq!(
        transit_out(&pool, world.source_batch_id).await,
        Quantity::ZERO
    );
    assert_eq!(
        event_count(&pool, world.owner_id, "replenishment.cancelled").await,
        1
    );
    assert_eq!(
        event_count(
            &pool,
            world.owner_id,
            "business.replenishment_urgent_timeout"
        )
        .await,
        1
    );
    let audits: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM audit_event
         WHERE owner_id = $1 AND action = 'timeout_cancel_replenishment_task'
        "#,
    )
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit");
    assert_eq!(audits, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn urgent_pending_10_minutes_alerts_once_and_stays_pending(pool: PgPool) {
    let world = seed_world(&pool).await;
    let task_id = create_urgent(&pool, &world).await;
    sqlx::query("UPDATE replenishment_tasks SET created_at = $2 WHERE id = $1 AND owner_id = $3")
        .bind(task_id)
        .bind(Utc::now() - Duration::minutes(11))
        .bind(world.owner_id)
        .execute(&pool)
        .await
        .expect("backdate");
    let svc = service(pool.clone());
    svc.run_timeout_scan(Utc::now()).await.expect("scan 1");
    svc.run_timeout_scan(Utc::now()).await.expect("scan 2");
    let status: String = sqlx::query_scalar(
        "SELECT status FROM replenishment_tasks WHERE id = $1 AND owner_id = $2",
    )
    .bind(task_id)
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("status");
    assert_eq!(status, "pending");
    assert_eq!(
        event_count(
            &pool,
            world.owner_id,
            "business.replenishment_urgent_unclaimed"
        )
        .await,
        1
    );
    let minutes: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT (payload ->> 'unclaimed_minutes')::BIGINT
          FROM event_bus_event
         WHERE owner_id = $1 AND event_type = 'business.replenishment_urgent_unclaimed'
         LIMIT 1
        "#,
    )
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("payload");
    assert!(minutes.unwrap_or(0) >= 10);
}

#[sqlx::test(migrations = "../../migrations")]
async fn in_progress_one_hour_alerts_without_cancel(pool: PgPool) {
    let world = seed_world(&pool).await;
    let task_id = create_urgent(&pool, &world).await;
    let operator = Uuid::new_v4();
    sqlx::query(
        r#"
        UPDATE replenishment_tasks
           SET status = 'in_progress',
               operator_id = $2,
               last_progress_at = $3
         WHERE id = $1 AND owner_id = $4
        "#,
    )
    .bind(task_id)
    .bind(operator)
    .bind(Utc::now() - Duration::minutes(61))
    .bind(world.owner_id)
    .execute(&pool)
    .await
    .expect("age progress");
    service(pool.clone())
        .run_timeout_scan(Utc::now())
        .await
        .expect("scan");
    let status: String = sqlx::query_scalar(
        "SELECT status FROM replenishment_tasks WHERE id = $1 AND owner_id = $2",
    )
    .bind(task_id)
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("status");
    assert_eq!(status, "in_progress");
    assert!(transit_out(&pool, world.source_batch_id).await > Quantity::ZERO);
    assert_eq!(
        event_count(&pool, world.owner_id, "business.replenishment_no_progress").await,
        1
    );
    let stale: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT (payload ->> 'stale_minutes')::BIGINT
          FROM event_bus_event
         WHERE owner_id = $1 AND event_type = 'business.replenishment_no_progress'
         LIMIT 1
        "#,
    )
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("stale payload");
    assert!(stale.unwrap_or(0) >= 60);
}

#[test]
fn timeout_job_is_registered_on_mte_skeleton() {
    assert_eq!(replenishment_timeout_job::JOB_NAME, "replenishment_timeout");
}
