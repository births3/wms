//! M4 订单与波次动作接口（重新校验 / 作废申请 / 波次下发）的 Postgres 集成测试。

use chrono::{NaiveDate, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    inventory::STATUS_QUALIFIED,
    wave4_repository::{PgWave4Repository, Wave4RepositoryError},
};
use wms_domain::{CreateOutboundOrderLineRequest, CreateOutboundOrderRequest, OutboundOrder};

#[path = "support/h9.rs"]
mod h9_support;
use h9_support::seed_outbound_route_binding;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m4-outbound-actions-test".to_string(),
        permissions: vec!["m4.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

/// 参照 wave4_postgres.rs 的完整仓储链路种子：货主 → 仓库 → 库区 → 库位 → 库存批次。
async fn seed_outbound_inventory(
    pool: &PgPool,
    owner_id: Uuid,
    product_code: &str,
    batch_no: &str,
    location_code: &str,
    qty: i64,
    now: chrono::DateTime<Utc>,
) -> Uuid {
    let batch_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'M4 动作测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("M4A-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed owner");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'M4 动作测试仓', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("M4A-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, $4, 'M4 动作测试区', 'normal', 'qualified_green', 'active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(format!("M4A-ZONE-{}", &zone_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed zone");
    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status) VALUES ($1, $2, $3, $4, $5, 1, 1, 1, 100000, 0, 100, 'storage', 'available')",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(location_code)
    .execute(pool)
    .await
    .expect("seed location");
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, $9, $10, FALSE, $11, $11)
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(product_code)
    .bind(batch_no)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid date"))
    .bind(qty)
    .bind(STATUS_QUALIFIED)
    .bind(location_id)
    .bind(location_code)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed inventory batch");
    batch_id
}

async fn create_confirmed_order(
    pool: &PgPool,
    repo: &PgWave4Repository,
    ctx: &AuthContext,
    wms_order_no: &str,
    product_code: &str,
    batch_no: &str,
    planned_qty: i64,
    now: chrono::DateTime<Utc>,
    idempotency_key: &str,
) -> OutboundOrder {
    let customer_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let delivery_address_id =
        seed_outbound_route_binding(pool, ctx.owner_id, warehouse_id, customer_id, now).await;
    repo.create_outbound_order(
        ctx,
        CreateOutboundOrderRequest {
            document_type: "sales_outbound".to_string(),
            wms_order_no: wms_order_no.to_string(),
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
                product_code: product_code.to_string(),
                batch_no: batch_no.to_string(),
                planned_qty,
            }],
        },
        now,
        idempotency_key,
        None,
    )
    .await
    .expect("outbound order should be created")
    .value
}

async fn force_order_status(pool: &PgPool, owner_id: Uuid, order_id: Uuid, status: &str) {
    sqlx::query("UPDATE outbound_orders SET status = $3 WHERE owner_id = $1 AND id = $2")
        .bind(owner_id)
        .bind(order_id)
        .bind(status)
        .execute(pool)
        .await
        .expect("force order status");
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_revalidate_passes_exception_order_and_replays(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 9, 0, 0)
        .single()
        .expect("valid time");
    seed_outbound_inventory(&pool, owner_id, "P-RV-001", "B-RV-001", "RV-A-01", 10, now).await;
    let order = create_confirmed_order(
        &pool,
        &repo,
        &ctx,
        "WMS-RV-001",
        "P-RV-001",
        "B-RV-001",
        6,
        now,
        "rv-order-1",
    )
    .await;
    force_order_status(&pool, owner_id, order.id, "validation_exception").await;

    let revalidated = repo
        .revalidate_outbound_order(&ctx, order.id, now, "rv-action-1", None)
        .await
        .expect("revalidate should succeed");
    assert!(!revalidated.replayed);
    assert_eq!(revalidated.value.status, "confirmed");

    let replay = repo
        .revalidate_outbound_order(&ctx, order.id, now, "rv-action-1", None)
        .await
        .expect("same key should replay");
    assert!(replay.replayed);
    assert_eq!(replay.value.id, order.id);

    let evidence: (String, i64, i64) = sqlx::query_as(
        r#"SELECT
             (SELECT status FROM outbound_orders WHERE owner_id = $1 AND id = $2),
             (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'revalidate_outbound_order' AND resource_id = $2::text),
             (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'rv-action-1')"#,
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("revalidate evidence");
    assert_eq!(evidence, ("confirmed".to_string(), 1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_revalidate_marks_exception_when_inventory_missing_or_insufficient(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 9, 30, 0)
        .single()
        .expect("valid time");
    seed_outbound_inventory(&pool, owner_id, "P-RV-002", "B-RV-002", "RV-A-02", 5, now).await;

    // 批号不存在。
    let missing_batch = create_confirmed_order(
        &pool,
        &repo,
        &ctx,
        "WMS-RV-002",
        "P-RV-002",
        "B-RV-MISSING",
        3,
        now,
        "rv-order-2",
    )
    .await;
    force_order_status(&pool, owner_id, missing_batch.id, "pending_validation").await;
    let result = repo
        .revalidate_outbound_order(&ctx, missing_batch.id, now, "rv-action-2", None)
        .await
        .expect("revalidate should persist exception status");
    assert_eq!(result.value.status, "validation_exception");

    // 批号存在但可用库存不足。
    let insufficient = create_confirmed_order(
        &pool,
        &repo,
        &ctx,
        "WMS-RV-003",
        "P-RV-002",
        "B-RV-002",
        99,
        now,
        "rv-order-3",
    )
    .await;
    force_order_status(&pool, owner_id, insufficient.id, "validation_exception").await;
    let result = repo
        .revalidate_outbound_order(&ctx, insufficient.id, now, "rv-action-3", None)
        .await
        .expect("revalidate should persist exception status");
    assert_eq!(result.value.status, "validation_exception");
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_revalidate_rejects_illegal_precondition_status(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 10, 0, 0)
        .single()
        .expect("valid time");
    seed_outbound_inventory(&pool, owner_id, "P-RV-004", "B-RV-004", "RV-A-04", 5, now).await;
    let order = create_confirmed_order(
        &pool,
        &repo,
        &ctx,
        "WMS-RV-004",
        "P-RV-004",
        "B-RV-004",
        2,
        now,
        "rv-order-4",
    )
    .await;
    force_order_status(&pool, owner_id, order.id, "shipped").await;

    let rejected = repo
        .revalidate_outbound_order(&ctx, order.id, now, "rv-action-4", None)
        .await
        .expect_err("shipped order must not be revalidated");
    assert!(matches!(
        rejected,
        Wave4RepositoryError::InvalidStatus { ref actual, .. } if actual == "shipped"
    ));

    let missing = repo
        .revalidate_outbound_order(&ctx, Uuid::new_v4(), now, "rv-action-5", None)
        .await
        .expect_err("unknown order should not revalidate");
    assert!(matches!(missing, Wave4RepositoryError::NotFound));
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_void_request_marks_confirmed_order_and_rejects_in_wave(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 11, 0, 0)
        .single()
        .expect("valid time");
    seed_outbound_inventory(&pool, owner_id, "P-VD-001", "B-VD-001", "VD-A-01", 8, now).await;
    let order = create_confirmed_order(
        &pool,
        &repo,
        &ctx,
        "WMS-VD-001",
        "P-VD-001",
        "B-VD-001",
        4,
        now,
        "vd-order-1",
    )
    .await;

    let voided = repo
        .request_void_outbound_order(&ctx, order.id, now, "vd-action-1", None)
        .await
        .expect("void request should succeed");
    assert!(!voided.replayed);
    assert_eq!(voided.value.status, "void_requested");

    let replay = repo
        .request_void_outbound_order(&ctx, order.id, now, "vd-action-1", None)
        .await
        .expect("same key should replay");
    assert!(replay.replayed);

    let evidence: (String, i64) = sqlx::query_as(
        r#"SELECT
             (SELECT status FROM outbound_orders WHERE owner_id = $1 AND id = $2),
             (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'request_void_outbound_order' AND resource_id = $2::text)"#,
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("void evidence");
    assert_eq!(evidence, ("void_requested".to_string(), 1));

    // 已进入波次的订单不允许作废申请。
    let in_wave = create_confirmed_order(
        &pool,
        &repo,
        &ctx,
        "WMS-VD-002",
        "P-VD-001",
        "B-VD-001",
        2,
        now,
        "vd-order-2",
    )
    .await;
    force_order_status(&pool, owner_id, in_wave.id, "in_wave").await;
    let rejected = repo
        .request_void_outbound_order(&ctx, in_wave.id, now, "vd-action-2", None)
        .await
        .expect_err("in_wave order must not request void");
    assert!(matches!(
        rejected,
        Wave4RepositoryError::InvalidStatus { ref actual, .. } if actual == "in_wave"
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_release_wave_locks_inventory_creates_pick_tasks_and_rejects_released(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 14, 0, 0)
        .single()
        .expect("valid time");
    seed_outbound_inventory(&pool, owner_id, "P-RL-001", "B-RL-001", "RL-A-01", 10, now).await;
    let order = create_confirmed_order(
        &pool,
        &repo,
        &ctx,
        "WMS-RL-001",
        "P-RL-001",
        "B-RL-001",
        6,
        now,
        "rl-order-1",
    )
    .await;

    // 草稿波次 + 波次订单关联由计划环节产生，这里直接落库模拟。
    let wave_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO outbound_waves (id, owner_id, wave_no, status, created_at, updated_at) VALUES ($1, $2, 'WAVE-RL-001', 'draft', $3, $3)",
    )
    .bind(wave_id)
    .bind(owner_id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed draft wave");
    sqlx::query(
        "INSERT INTO outbound_wave_orders (id, owner_id, wave_id, outbound_order_id, created_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(wave_id)
    .bind(order.id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed wave order link");

    let released = repo
        .release_outbound_wave(&ctx, wave_id, now, "rl-action-1", None)
        .await
        .expect("draft wave should release");
    assert!(!released.replayed);
    assert_eq!(released.value.status, "released");
    assert_eq!(released.value.order_ids, vec![order.id]);

    let evidence: (String, String, i64, i64, i64, i64) = sqlx::query_as(
        r#"SELECT
             (SELECT status FROM outbound_waves WHERE owner_id = $1 AND id = $2),
             (SELECT status FROM outbound_orders WHERE owner_id = $1 AND id = $3),
             (SELECT qty_locked FROM inventory_batches WHERE owner_id = $1 AND product_code = 'P-RL-001' AND batch_no = 'B-RL-001'),
             (SELECT COUNT(*) FROM outbound_pick_tasks WHERE owner_id = $1 AND wave_id = $2),
             (SELECT COUNT(*) FROM inventory_allocations WHERE owner_id = $1 AND outbound_order_id = $3 AND status = 'locked'),
             (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'release_outbound_wave' AND resource_id = $2::text)"#,
    )
    .bind(owner_id)
    .bind(wave_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("release evidence");
    assert_eq!(
        evidence,
        ("released".to_string(), "in_wave".to_string(), 6, 1, 1, 1)
    );

    let replay = repo
        .release_outbound_wave(&ctx, wave_id, now, "rl-action-1", None)
        .await
        .expect("same key should replay");
    assert!(replay.replayed);
    assert_eq!(replay.value.id, wave_id);

    // 已下发波次使用新幂等键再次下发必须被拒绝。
    let rejected = repo
        .release_outbound_wave(&ctx, wave_id, now, "rl-action-2", None)
        .await
        .expect_err("released wave must not release twice");
    assert!(matches!(
        rejected,
        Wave4RepositoryError::InvalidStatus { ref actual, .. } if actual == "released"
    ));

    let missing = repo
        .release_outbound_wave(&ctx, Uuid::new_v4(), now, "rl-action-3", None)
        .await
        .expect_err("unknown wave should not release");
    assert!(matches!(missing, Wave4RepositoryError::NotFound));
}

#[sqlx::test(migrations = "../../migrations")]
async fn m4_release_wave_rejects_orders_not_confirmed(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 15, 0, 0)
        .single()
        .expect("valid time");
    seed_outbound_inventory(&pool, owner_id, "P-RL-002", "B-RL-002", "RL-A-02", 10, now).await;
    let order = create_confirmed_order(
        &pool,
        &repo,
        &ctx,
        "WMS-RL-002",
        "P-RL-002",
        "B-RL-002",
        3,
        now,
        "rl-order-2",
    )
    .await;
    force_order_status(&pool, owner_id, order.id, "void_requested").await;

    let wave_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO outbound_waves (id, owner_id, wave_no, status, created_at, updated_at) VALUES ($1, $2, 'WAVE-RL-002', 'draft', $3, $3)",
    )
    .bind(wave_id)
    .bind(owner_id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed draft wave");
    sqlx::query(
        "INSERT INTO outbound_wave_orders (id, owner_id, wave_id, outbound_order_id, created_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(wave_id)
    .bind(order.id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed wave order link");

    let rejected = repo
        .release_outbound_wave(&ctx, wave_id, now, "rl-action-4", None)
        .await
        .expect_err("non-confirmed order must block wave release");
    assert!(matches!(
        rejected,
        Wave4RepositoryError::InvalidStatus { ref actual, .. } if actual == "void_requested"
    ));

    // 事务回滚：波次仍为草稿，未产生锁定与拣选任务。
    let evidence: (String, i64, i64) = sqlx::query_as(
        r#"SELECT
             (SELECT status FROM outbound_waves WHERE owner_id = $1 AND id = $2),
             (SELECT COUNT(*) FROM outbound_pick_tasks WHERE owner_id = $1 AND wave_id = $2),
             (SELECT COALESCE(SUM(qty_locked), 0)::BIGINT FROM inventory_batches WHERE owner_id = $1)"#,
    )
    .bind(owner_id)
    .bind(wave_id)
    .fetch_one(&pool)
    .await
    .expect("rollback evidence");
    assert_eq!(evidence, ("draft".to_string(), 0, 0));
}
