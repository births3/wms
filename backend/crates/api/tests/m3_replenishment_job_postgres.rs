//! T06：领取 / 下架 / 送达确认（GWT 5/6/7/8/12/13/15/16/18/29）。

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
    auth(owner_id, Uuid::new_v4(), "m3.replenishment.manage")
}

fn execute_ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    auth(owner_id, user_id, "m3.replenishment.execute")
}

fn auth(owner_id: Uuid, user_id: Uuid, permission: &str) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "replenish-op".into(),
        permissions: vec![permission.into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

struct World {
    owner_id: Uuid,
    replenish_group_id: Uuid,
    product_id: Uuid,
    storage_id: Uuid,
    pick_id: Uuid,
    source_batch_id: Uuid,
    source_lpn_id: Option<Uuid>,
}

async fn seed_world(pool: &PgPool, source_on_hand: i64, with_lpn: bool) -> World {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '补货作业货主')",
    )
    .bind(owner_id)
    .bind(format!("RJ-{}", &owner_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("owner");
    let product_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '补货作业商品', '1', 'pending_mapping', now(), now())
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
        ) VALUES ($1, $2, $3, '作业仓', 'physical', 'active', now(), now())
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
        ) VALUES ($1, $2, $3, 'Z-RJ', '合格区', 'normal_10_30', 'qualified_green', 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("zone");
    let replenish_group_id =
        seed_open_replenish_group(pool, owner_id, warehouse_id, "replenish-all").await;
    let storage_id = seed_location(pool, owner_id, warehouse_id, zone_id, "ST-01", "storage").await;
    let pick_id = seed_location(pool, owner_id, warehouse_id, zone_id, "PP-01", "piece_pick").await;
    let source_lpn_id = if with_lpn {
        let lpn_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO lpn_containers (
                id, owner_id, lpn_code, container_type, status, location_id, created_at, updated_at
            ) VALUES ($1, $2, 'LPN-RJ-01', 'pallet', 'in_use', $3, now(), now())
            "#,
        )
        .bind(lpn_id)
        .bind(owner_id)
        .bind(storage_id)
        .execute(pool)
        .await
        .expect("lpn");
        Some(lpn_id)
    } else {
        None
    };
    let source_batch_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_id, product_code, batch_no,
            production_date, expiry_date, qty_on_hand, qty_frozen, qty_allocated,
            qty_replenish_in_transit, qty_replenish_out_transit,
            status, location_id, location_code, container_lpn, recall_flag,
            created_at, updated_at, version
        ) VALUES (
            $1, $2, $3, 'P-RJ', 'B-SRC',
            $4, $5, $6, 0, 0,
            0, 0,
            'qualified', $7, 'ST-01', $8, FALSE, now(), now(), 1
        )
        "#,
    )
    .bind(source_batch_id)
    .bind(owner_id)
    .bind(product_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("prod"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("exp"))
    .bind(Quantity::from(source_on_hand))
    .bind(storage_id)
    .bind(if with_lpn { Some("LPN-RJ-01") } else { None })
    .execute(pool)
    .await
    .expect("batch");
    World {
        owner_id,
        replenish_group_id,
        product_id,
        storage_id,
        pick_id,
        source_batch_id,
        source_lpn_id,
    }
}

async fn seed_open_replenish_group(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    code: &str,
) -> Uuid {
    let group_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO task_groups (
            id, owner_id, task_group_code, task_group_name, warehouse_id,
            zone_ids, task_type_codes, enabled
        ) VALUES ($1, $2, $3, '全仓补货班组', $4, '{}', ARRAY['replenish'], TRUE)
        "#,
    )
    .bind(group_id)
    .bind(owner_id)
    .bind(code)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("open replenish group");
    group_id
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

async fn seed_location(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    zone_id: Uuid,
    code: &str,
    location_type: &str,
) -> Uuid {
    let location_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code,
            row_no, column_no, layer_no, max_volume_cm3, max_sku_count,
            location_type, current_owner_id, status, allows_container,
            mix_product_policy, mix_batch_policy, lock_status, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5,
            1, 1, 1, 100000, 10,
            $6, $2, 'available', FALSE,
            'single_product_only', 'single_batch', 'normal', now(), now()
        )
        "#,
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(code)
    .bind(location_type)
    .execute(pool)
    .await
    .expect("location");
    location_id
}

fn app(pool: PgPool, auth: AuthContext) -> axum::Router {
    replenishment_router(ReplenishmentAppState::with_postgres(pool)).layer(Extension(auth))
}

async fn json_post(
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
                .expect("request"),
        )
        .await
        .expect("oneshot");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({}));
    (status, json)
}

async fn create_task(pool: &PgPool, world: &World, qty: i64, idem: &str) -> serde_json::Value {
    let mut body = serde_json::json!({
        "source_location_id": world.storage_id,
        "source_batch_id": world.source_batch_id,
        "target_location_id": world.pick_id,
        "qty": qty.to_string()
    });
    if let Some(lpn_id) = world.source_lpn_id {
        body["source_lpn_id"] = serde_json::json!(lpn_id);
    }
    let (status, json) = json_post(
        app(pool.clone(), manage_ctx(world.owner_id)),
        "/api/v1/replenishment/tasks",
        body,
        idem,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create task {json}");
    json
}

fn task_uri(id: &str, action: &str) -> String {
    format!("/api/v1/replenishment/tasks/{id}/{action}")
}

#[sqlx::test(migrations = "../../migrations")]
async fn pick_wrong_location_is_source_mismatch(pool: PgPool) {
    let world = seed_world(&pool, 10, false).await;
    let task = create_task(&pool, &world, 10, "job-18-create").await;
    let id = task["id"].as_str().expect("id");
    let op = seed_operator(&pool, &world).await;
    let claimed = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "job-18-claim",
    )
    .await;
    assert_eq!(claimed.0, StatusCode::OK);
    let (status, body) = json_post(
        app(pool, execute_ctx(world.owner_id, op)),
        &task_uri(id, "pick"),
        serde_json::json!({
            "version": claimed.1["version"],
            "scanned_location_code": "ST-99",
            "qty": "10"
        }),
        "job-18-pick",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M3_REPLENISH_SOURCE_MISMATCH");
}

#[sqlx::test(migrations = "../../migrations")]
async fn second_claim_same_version_conflicts(pool: PgPool) {
    let world = seed_world(&pool, 10, false).await;
    let task = create_task(&pool, &world, 10, "job-5-create").await;
    let id = task["id"].as_str().expect("id");
    let first_op = seed_operator(&pool, &world).await;
    let second_op = seed_operator(&pool, &world).await;
    let first = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, first_op)),
        &task_uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "job-5-a",
    )
    .await;
    let second = json_post(
        app(pool, execute_ctx(world.owner_id, second_op)),
        &task_uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "job-5-b",
    )
    .await;
    assert_eq!(first.0, StatusCode::OK);
    assert_eq!(first.1["status"], "in_progress");
    assert_eq!(second.0, StatusCode::CONFLICT);
    assert_eq!(second.1["code"], "M3_REPLENISH_CLAIM_CONFLICT");
}

#[sqlx::test(migrations = "../../migrations")]
async fn pick_over_remaining_qty_is_exceeded(pool: PgPool) {
    let world = seed_world(&pool, 10, false).await;
    let task = create_task(&pool, &world, 10, "job-6-create").await;
    let id = task["id"].as_str().expect("id");
    let op = seed_operator(&pool, &world).await;
    let claimed = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "job-6-claim",
    )
    .await;
    let first_pick = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "pick"),
        serde_json::json!({
            "version": claimed.1["version"],
            "scanned_location_code": "ST-01",
            "qty": "8"
        }),
        "job-6-pick-1",
    )
    .await;
    assert_eq!(first_pick.0, StatusCode::OK);
    let (status, body) = json_post(
        app(pool, execute_ctx(world.owner_id, op)),
        &task_uri(id, "pick"),
        serde_json::json!({
            "version": first_pick.1["version"],
            "scanned_location_code": "ST-01",
            "qty": "3"
        }),
        "job-6-pick-2",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M3_REPLENISH_QTY_EXCEEDED");
}

#[sqlx::test(migrations = "../../migrations")]
async fn confirm_converts_on_hand_and_finishes_task(pool: PgPool) {
    let world = seed_world(&pool, 10, false).await;
    let task = create_task(&pool, &world, 10, "job-7-create").await;
    let id = task["id"].as_str().expect("id");
    let op = seed_operator(&pool, &world).await;
    let claimed = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "job-7-claim",
    )
    .await;
    let picked = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "pick"),
        serde_json::json!({
            "version": claimed.1["version"],
            "scanned_location_code": "ST-01",
            "qty": "10"
        }),
        "job-7-pick",
    )
    .await;
    let (status, body) = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "confirm"),
        serde_json::json!({
            "version": picked.1["version"],
            "scanned_location_code": "PP-01",
            "qty": "10"
        }),
        "job-7-confirm",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "confirm body={body}");
    assert_eq!(body["status"], "done");
    let (src_on, _, src_out): (Quantity, Quantity, Quantity) = sqlx::query_as(
        "SELECT qty_on_hand, qty_replenish_in_transit, qty_replenish_out_transit FROM inventory_batches WHERE id = $1",
    )
    .bind(world.source_batch_id)
    .fetch_one(&pool)
    .await
    .expect("src");
    assert_eq!(src_on, Quantity::ZERO);
    assert_eq!(src_out, Quantity::ZERO);
    let tgt_on: Quantity = sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE location_id = $1 AND product_id = $2",
    )
    .bind(world.pick_id)
    .bind(world.product_id)
    .fetch_one(&pool)
    .await
    .expect("tgt");
    assert_eq!(tgt_on, Quantity::from(10));
    let events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM event_bus_event WHERE owner_id = $1 AND event_type = $2",
    )
    .bind(world.owner_id)
    .bind("replenishment.done")
    .fetch_one(&pool)
    .await
    .expect("events");
    assert_eq!(events, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn partial_confirm_keeps_in_progress(pool: PgPool) {
    let world = seed_world(&pool, 10, false).await;
    let task = create_task(&pool, &world, 10, "job-8-create").await;
    let id = task["id"].as_str().expect("id");
    let op = seed_operator(&pool, &world).await;
    let claimed = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "job-8-claim",
    )
    .await;
    let picked = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "pick"),
        serde_json::json!({
            "version": claimed.1["version"],
            "scanned_location_code": "ST-01",
            "qty": "4"
        }),
        "job-8-pick",
    )
    .await;
    let (status, body) = json_post(
        app(pool, execute_ctx(world.owner_id, op)),
        &task_uri(id, "confirm"),
        serde_json::json!({
            "version": picked.1["version"],
            "scanned_location_code": "PP-01",
            "qty": "4"
        }),
        "job-8-confirm",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "in_progress");
    let done: Quantity = serde_json::from_value(body["done_qty"].clone()).expect("done");
    assert_eq!(done, Quantity::from(4));
}

#[sqlx::test(migrations = "../../migrations")]
async fn confirm_blocked_when_target_zone_not_qualified(pool: PgPool) {
    let world = seed_world(&pool, 10, false).await;
    let task = create_task(&pool, &world, 10, "job-12-create").await;
    let id = task["id"].as_str().expect("id");
    let op = seed_operator(&pool, &world).await;
    let claimed = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "job-12-claim",
    )
    .await;
    let picked = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "pick"),
        serde_json::json!({
            "version": claimed.1["version"],
            "scanned_location_code": "ST-01",
            "qty": "10"
        }),
        "job-12-pick",
    )
    .await;
    sqlx::query(
        r#"
        UPDATE warehouse_zones
           SET quality_color = 'quarantine_yellow'
         WHERE owner_id = $1
        "#,
    )
    .bind(world.owner_id)
    .execute(&pool)
    .await
    .expect("lock zone");
    let (status, body) = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "confirm"),
        serde_json::json!({
            "version": picked.1["version"],
            "scanned_location_code": "PP-01",
            "qty": "10"
        }),
        "job-12-confirm",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M3_REPLENISH_PUTAWAY_BLOCKED");
    let src_on: Quantity =
        sqlx::query_scalar("SELECT qty_on_hand FROM inventory_batches WHERE id = $1")
            .bind(world.source_batch_id)
            .fetch_one(&pool)
            .await
            .expect("on hand");
    assert_eq!(src_on, Quantity::from(10));
}

#[sqlx::test(migrations = "../../migrations")]
async fn confirm_idempotent_replay_does_not_double(pool: PgPool) {
    let world = seed_world(&pool, 10, false).await;
    let task = create_task(&pool, &world, 10, "job-13-create").await;
    let id = task["id"].as_str().expect("id");
    let op = seed_operator(&pool, &world).await;
    let claimed = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "job-13-claim",
    )
    .await;
    let picked = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "pick"),
        serde_json::json!({
            "version": claimed.1["version"],
            "scanned_location_code": "ST-01",
            "qty": "10"
        }),
        "job-13-pick",
    )
    .await;
    let first = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "confirm"),
        serde_json::json!({
            "version": picked.1["version"],
            "scanned_location_code": "PP-01",
            "qty": "10"
        }),
        "job-13-confirm",
    )
    .await;
    let replay = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "confirm"),
        serde_json::json!({
            "version": picked.1["version"],
            "scanned_location_code": "PP-01",
            "qty": "10"
        }),
        "job-13-confirm",
    )
    .await;
    assert_eq!(first.0, StatusCode::OK);
    assert_eq!(replay.0, StatusCode::OK);
    let done: Quantity = serde_json::from_value(replay.1["done_qty"].clone()).expect("done");
    assert_eq!(done, Quantity::from(10));
    let tgt_on: Quantity = sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE location_id = $1 AND product_id = $2",
    )
    .bind(world.pick_id)
    .bind(world.product_id)
    .fetch_one(&pool)
    .await
    .expect("tgt");
    assert_eq!(tgt_on, Quantity::from(10));
}

#[sqlx::test(migrations = "../../migrations")]
async fn claim_without_execute_is_forbidden(pool: PgPool) {
    let world = seed_world(&pool, 10, false).await;
    let task = create_task(&pool, &world, 10, "job-15-create").await;
    let id = task["id"].as_str().expect("id");
    let (status, body) = json_post(
        app(pool, manage_ctx(world.owner_id)),
        &task_uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "job-15-claim",
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(
        body["code"] == "M3_REPLENISH_PERMISSION_DENIED"
            || body["code"] == "AUTH_FORBIDDEN"
            || body["code"] == "AUTH-005",
        "code={}",
        body["code"]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn confirm_full_lpn_releases_container_to_idle(pool: PgPool) {
    let world = seed_world(&pool, 10, true).await;
    let task = create_task(&pool, &world, 10, "job-16-create").await;
    let id = task["id"].as_str().expect("id");
    let op = seed_operator(&pool, &world).await;
    let claimed = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "job-16-claim",
    )
    .await;
    let picked = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "pick"),
        serde_json::json!({
            "version": claimed.1["version"],
            "scanned_location_code": "ST-01",
            "scanned_lpn_code": "LPN-RJ-01",
            "qty": "10"
        }),
        "job-16-pick",
    )
    .await;
    let (status, _) = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "confirm"),
        serde_json::json!({
            "version": picked.1["version"],
            "scanned_location_code": "PP-01",
            "qty": "10"
        }),
        "job-16-confirm",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let lpn_status: String = sqlx::query_scalar("SELECT status FROM lpn_containers WHERE id = $1")
        .bind(world.source_lpn_id.expect("lpn"))
        .fetch_one(&pool)
        .await
        .expect("lpn status");
    assert_eq!(lpn_status, "idle");
}

#[sqlx::test(migrations = "../../migrations")]
async fn confirm_without_pick_is_state_invalid(pool: PgPool) {
    let world = seed_world(&pool, 10, false).await;
    let task = create_task(&pool, &world, 10, "job-29-create").await;
    let id = task["id"].as_str().expect("id");
    let op = seed_operator(&pool, &world).await;
    let claimed = json_post(
        app(pool.clone(), execute_ctx(world.owner_id, op)),
        &task_uri(id, "claim"),
        serde_json::json!({ "version": task["version"] }),
        "job-29-claim",
    )
    .await;
    let (status, body) = json_post(
        app(pool, execute_ctx(world.owner_id, op)),
        &task_uri(id, "confirm"),
        serde_json::json!({
            "version": claimed.1["version"],
            "scanned_location_code": "PP-01",
            "qty": "10"
        }),
        "job-29-confirm",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M3_REPLENISH_STATE_INVALID");
}
