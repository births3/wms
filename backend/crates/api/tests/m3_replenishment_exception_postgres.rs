//! T07：取消 / 改派 / 退回 / 来源冻结挂起（GWT 9/10/17/24/25/31）。

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Extension,
};
use chrono::NaiveDate;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    replenishment_handlers::{replenishment_router, ReplenishmentAppState},
};
use wms_domain::Quantity;

fn manage_ctx(owner_id: Uuid) -> AuthContext {
    ctx(owner_id, Uuid::new_v4(), "m3.replenishment.manage")
}

fn execute_ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    ctx(owner_id, user_id, "m3.replenishment.execute")
}

fn ctx(owner_id: Uuid, user_id: Uuid, permission: &str) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "replenish-ex".into(),
        permissions: vec![permission.into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

struct World {
    owner_id: Uuid,
    replenish_group_id: Uuid,
    storage_id: Uuid,
    pick_id: Uuid,
    source_batch_id: Uuid,
}

async fn seed_world(pool: &PgPool, on_hand: i64) -> World {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '补货异常货主')",
    )
    .bind(owner_id)
    .bind(format!("RX-{}", &owner_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("owner");
    let product_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '补货异常商品', '1', 'pending_mapping', now(), now())
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
        ) VALUES ($1, $2, $3, '异常仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("warehouse");
    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, status, created_at, updated_at
        ) VALUES ($1, $2, $3, 'Z-RX', '合格区', 'normal_10_30', 'qualified_green', 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("zone");
    let replenish_group_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO task_groups (
            id, owner_id, task_group_code, task_group_name, warehouse_id,
            zone_ids, task_type_codes, enabled
        ) VALUES ($1, $2, 'replenish-all', '全仓补货班组', $3, '{}', ARRAY['replenish'], TRUE)
        "#,
    )
    .bind(replenish_group_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("open replenish group");
    let storage_id = seed_loc(pool, owner_id, warehouse_id, zone_id, "ST-01", "storage").await;
    let pick_id = seed_loc(pool, owner_id, warehouse_id, zone_id, "PP-01", "piece_pick").await;
    let source_batch_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_id, product_code, batch_no,
            production_date, expiry_date, qty_on_hand, qty_frozen, qty_allocated,
            qty_replenish_in_transit, qty_replenish_out_transit,
            status, location_id, location_code, recall_flag, created_at, updated_at, version
        ) VALUES (
            $1, $2, $3, 'P-RX', 'B-SRC', $4, $5, $6, 0, 0, 0, 0,
            'qualified', $7, 'ST-01', FALSE, now(), now(), 1
        )
        "#,
    )
    .bind(source_batch_id)
    .bind(owner_id)
    .bind(product_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("p"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("e"))
    .bind(Quantity::from(on_hand))
    .bind(storage_id)
    .execute(pool)
    .await
    .expect("batch");
    World {
        owner_id,
        replenish_group_id,
        storage_id,
        pick_id,
        source_batch_id,
    }
}

async fn seed_operator(pool: &PgPool, world: &World) -> Uuid {
    let operator_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '作业员', 'test-hash', 'active')",
    )
    .bind(operator_id)
    .bind(format!("op-{}", &operator_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("user");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(operator_id)
    .bind(world.owner_id)
    .execute(pool)
    .await
    .expect("binding");
    sqlx::query(
        "INSERT INTO task_group_memberships (task_group_id, owner_id, user_id) VALUES ($1, $2, $3)",
    )
    .bind(world.replenish_group_id)
    .bind(world.owner_id)
    .bind(operator_id)
    .execute(pool)
    .await
    .expect("membership");
    operator_id
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

fn app(pool: PgPool, auth: AuthContext) -> axum::Router {
    replenishment_router(ReplenishmentAppState::with_postgres(pool)).layer(Extension(auth))
}

async fn post(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
    idem: &str,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header("idempotency-key", idem)
                .body(Body::from(body.to_string()))
                .expect("req"),
        )
        .await
        .expect("oneshot");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({})),
    )
}

async fn create_task(pool: &PgPool, world: &World, qty: i64, idem: &str) -> serde_json::Value {
    let (status, json) = post(
        app(pool.clone(), manage_ctx(world.owner_id)),
        "/api/v1/replenishment/tasks",
        serde_json::json!({
            "source_location_id": world.storage_id,
            "source_batch_id": world.source_batch_id,
            "target_location_id": world.pick_id,
            "qty": qty.to_string()
        }),
        idem,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create {json}");
    json
}

fn uri(id: &str, action: &str) -> String {
    format!("/api/v1/replenishment/tasks/{id}/{action}")
}

async fn claim_pick(
    pool: &PgPool,
    world: &World,
    task: &serde_json::Value,
    pick_qty: i64,
    tag: &str,
) -> (Uuid, serde_json::Value) {
    let id = task["id"].as_str().expect("id");
    let op = seed_operator(pool, world).await;
    let claimed = post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        &format!("{tag}-claim"),
    )
    .await;
    assert_eq!(claimed.0, StatusCode::OK, "claim {}", claimed.1);
    let picked = post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &uri(id, "pick"),
        serde_json::json!({
            "version": claimed.1["version"],
            "scanned_location_code": "ST-01",
            "qty": pick_qty.to_string()
        }),
        &format!("{tag}-pick"),
    )
    .await;
    assert_eq!(picked.0, StatusCode::OK, "pick {}", picked.1);
    (op, picked.1)
}

async fn transit(pool: &PgPool, batch_id: Uuid) -> (Quantity, Quantity) {
    sqlx::query_as(
        "SELECT qty_on_hand, qty_replenish_out_transit FROM inventory_batches WHERE id = $1",
    )
    .bind(batch_id)
    .fetch_one(pool)
    .await
    .expect("transit")
}

#[sqlx::test(migrations = "../../migrations")]
async fn cancel_blocked_when_picked_qty_positive(pool: PgPool) {
    let world = seed_world(&pool, 10).await;
    let task = create_task(&pool, &world, 10, "ex-9-create").await;
    let picked = claim_pick(&pool, &world, &task, 4, "ex-9").await.1;
    let (status, body) = post(
        app(pool, manage_ctx(world.owner_id)),
        &uri(task["id"].as_str().expect("id"), "cancel"),
        serde_json::json!({ "version": picked["version"], "reason": "不再需要" }),
        "ex-9-cancel",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M3_REPLENISH_CANCEL_BLOCKED");
}

#[sqlx::test(migrations = "../../migrations")]
async fn cancel_pending_releases_transit(pool: PgPool) {
    let world = seed_world(&pool, 10).await;
    let task = create_task(&pool, &world, 10, "ex-10-create").await;
    let (status, body) = post(
        app(pool.clone(), manage_ctx(world.owner_id)),
        &uri(task["id"].as_str().expect("id"), "cancel"),
        serde_json::json!({ "version": task["version"], "reason": "计划取消" }),
        "ex-10-cancel",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "cancelled");
    let (on_hand, out_t) = transit(&pool, world.source_batch_id).await;
    assert_eq!(on_hand, Quantity::from(10));
    assert_eq!(out_t, Quantity::ZERO);
    let events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM event_bus_event WHERE owner_id = $1 AND event_type = $2",
    )
    .bind(world.owner_id)
    .bind("replenishment.cancelled")
    .fetch_one(&pool)
    .await
    .expect("events");
    assert_eq!(events, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn confirm_when_source_frozen_suspends_task(pool: PgPool) {
    let world = seed_world(&pool, 10).await;
    let task = create_task(&pool, &world, 10, "ex-17-create").await;
    let id = task["id"].as_str().expect("id");
    let op = seed_operator(&pool, &world).await;
    let claimed = post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "ex-17-claim",
    )
    .await;
    assert_eq!(claimed.0, StatusCode::OK);
    sqlx::query("UPDATE inventory_batches SET qty_frozen = 10 WHERE id = $1")
        .bind(world.source_batch_id)
        .execute(&pool)
        .await
        .expect("freeze");
    let (status, body) = post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &uri(id, "confirm"),
        serde_json::json!({
            "version": claimed.1["version"],
            "scanned_location_code": "PP-01",
            "qty": "10"
        }),
        "ex-17-confirm",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M3_REPLENISH_SOURCE_UNAVAILABLE");
    let task_status: String =
        sqlx::query_scalar("SELECT status FROM replenishment_tasks WHERE id = $1")
            .bind(Uuid::parse_str(id).expect("uuid"))
            .fetch_one(&pool)
            .await
            .expect("status");
    assert_eq!(task_status, "suspended");
    let (on_hand, _) = transit(&pool, world.source_batch_id).await;
    assert_eq!(on_hand, Quantity::from(10));
    let frozen_alerts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM event_bus_event WHERE owner_id = $1 AND event_type = $2",
    )
    .bind(world.owner_id)
    .bind("replenishment_source_frozen")
    .fetch_one(&pool)
    .await
    .expect("h4");
    assert_eq!(frozen_alerts, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn reassign_returns_to_pool_keeping_picked_qty(pool: PgPool) {
    let world = seed_world(&pool, 10).await;
    let task = create_task(&pool, &world, 10, "ex-24-create").await;
    let picked = claim_pick(&pool, &world, &task, 2, "ex-24").await.1;
    let (before_on, before_out) = transit(&pool, world.source_batch_id).await;
    let (status, body) = post(
        app(pool.clone(), manage_ctx(world.owner_id)),
        &uri(task["id"].as_str().expect("id"), "reassign"),
        serde_json::json!({ "version": picked["version"] }),
        "ex-24-reassign",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "pending");
    assert!(body["operator_id"].is_null());
    let picked_qty: Quantity = serde_json::from_value(body["picked_qty"].clone()).expect("picked");
    assert_eq!(picked_qty, Quantity::from(2));
    let (after_on, after_out) = transit(&pool, world.source_batch_id).await;
    assert_eq!(after_on, before_on);
    assert_eq!(after_out, before_out);
}

#[sqlx::test(migrations = "../../migrations")]
async fn return_blocked_when_picked_qty_positive(pool: PgPool) {
    let world = seed_world(&pool, 10).await;
    let task = create_task(&pool, &world, 10, "ex-25-create").await;
    let (op, picked) = claim_pick(&pool, &world, &task, 2, "ex-25").await;
    let (status, body) = post(
        app(pool, execute_ctx(world.owner_id, op)),
        &uri(task["id"].as_str().expect("id"), "return"),
        serde_json::json!({
            "version": picked["version"],
            "return_reason": "other"
        }),
        "ex-25-return",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M3_REPLENISH_RETURN_BLOCKED");
}

#[sqlx::test(migrations = "../../migrations")]
async fn suspended_confirm_delivers_picked_qty_only(pool: PgPool) {
    let world = seed_world(&pool, 10).await;
    let task = create_task(&pool, &world, 10, "ex-31-create").await;
    let id = task["id"].as_str().expect("id");
    let (op, picked) = claim_pick(&pool, &world, &task, 4, "ex-31").await;
    sqlx::query("UPDATE inventory_batches SET qty_frozen = 5 WHERE id = $1")
        .bind(world.source_batch_id)
        .execute(&pool)
        .await
        .expect("freeze");
    let blocked = post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &uri(id, "pick"),
        serde_json::json!({
            "version": picked["version"],
            "scanned_location_code": "ST-01",
            "qty": "1"
        }),
        "ex-31-pick2",
    )
    .await;
    assert_eq!(blocked.0, StatusCode::UNPROCESSABLE_ENTITY);
    let version: i64 = sqlx::query_scalar("SELECT version FROM replenishment_tasks WHERE id = $1")
        .bind(Uuid::parse_str(id).expect("uuid"))
        .fetch_one(&pool)
        .await
        .expect("ver");
    let confirmed = post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &uri(id, "confirm"),
        serde_json::json!({
            "version": version,
            "scanned_location_code": "PP-01",
            "qty": "4"
        }),
        "ex-31-confirm",
    )
    .await;
    assert_eq!(confirmed.0, StatusCode::OK, "confirm {}", confirmed.1);
    assert_eq!(confirmed.1["status"], "suspended");
    let done: Quantity = serde_json::from_value(confirmed.1["done_qty"].clone()).expect("done");
    let left: Quantity = serde_json::from_value(confirmed.1["picked_qty"].clone()).expect("picked");
    assert_eq!(done, Quantity::from(4));
    assert_eq!(left, Quantity::ZERO);
    let pick_again = post(
        app(pool, execute_ctx(world.owner_id, op)),
        &uri(id, "pick"),
        serde_json::json!({
            "version": confirmed.1["version"],
            "scanned_location_code": "ST-01",
            "qty": "1"
        }),
        "ex-31-pick3",
    )
    .await;
    assert_eq!(pick_again.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(pick_again.1["code"], "M3_REPLENISH_STATE_INVALID");
}
