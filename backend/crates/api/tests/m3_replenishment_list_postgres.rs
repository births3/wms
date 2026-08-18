//! T13：execute 列表排序与库区过滤（GWT 20/21，规范 §8 / §10.6）。

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

fn execute_ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "replenish-pda".into(),
        permissions: vec!["m3.replenishment.execute".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[allow(dead_code)]
struct World {
    owner_id: Uuid,
    operator_id: Uuid,
    warehouse_id: Uuid,
    zone_in: Uuid,
    zone_out: Uuid,
    storage_id: Uuid,
    pick_in_seq10: Uuid,
    pick_in_seq30: Uuid,
    pick_in_null: Uuid,
    pick_out_normal: Uuid,
    pick_out_urgent: Uuid,
    source_batch_id: Uuid,
    product_id: Uuid,
}

async fn seed_world(pool: &PgPool) -> World {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '补货列表货主')",
    )
    .bind(owner_id)
    .bind(format!("RL-{}", &owner_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("owner");
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
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("binding");

    let warehouse_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '列表仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("warehouse");
    let zone_in = Uuid::new_v4();
    let zone_out = Uuid::new_v4();
    for (id, code) in [(zone_in, "Z-IN"), (zone_out, "Z-OUT")] {
        sqlx::query(
            r#"
            INSERT INTO warehouse_zones (
                id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
                quality_color, status, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $4, 'normal_10_30', 'qualified_green', 'active', now(), now())
            "#,
        )
        .bind(id)
        .bind(owner_id)
        .bind(warehouse_id)
        .bind(code)
        .execute(pool)
        .await
        .expect("zone");
    }

    let storage_id = seed_location(
        pool,
        owner_id,
        warehouse_id,
        zone_in,
        "ST-01",
        "storage",
        Some(1),
    )
    .await;
    let pick_in_seq10 = seed_location(
        pool,
        owner_id,
        warehouse_id,
        zone_in,
        "PP-10",
        "piece_pick",
        Some(10),
    )
    .await;
    let pick_in_seq30 = seed_location(
        pool,
        owner_id,
        warehouse_id,
        zone_in,
        "PP-30",
        "piece_pick",
        Some(30),
    )
    .await;
    let pick_in_null = seed_location(
        pool,
        owner_id,
        warehouse_id,
        zone_in,
        "PP-NN",
        "piece_pick",
        None,
    )
    .await;
    let pick_out_normal = seed_location(
        pool,
        owner_id,
        warehouse_id,
        zone_out,
        "PP-OUT-N",
        "piece_pick",
        Some(5),
    )
    .await;
    let pick_out_urgent = seed_location(
        pool,
        owner_id,
        warehouse_id,
        zone_out,
        "PP-OUT-U",
        "piece_pick",
        Some(2),
    )
    .await;

    let product_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '列表商品', '1', 'pending_mapping', now(), now())
        "#,
    )
    .bind(product_id)
    .bind(owner_id)
    .bind(format!("P-{}", &product_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("product");
    let source_batch_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_id, product_code, batch_no,
            production_date, expiry_date,
            qty_on_hand, qty_frozen, qty_allocated,
            qty_replenish_in_transit, qty_replenish_out_transit,
            status, location_id, location_code, recall_flag, created_at, updated_at, version
        ) VALUES (
            $1, $2, $3, 'P-RL', 'B-SRC',
            $5, $6,
            100, 0, 0, 0, 0,
            'qualified', $4, 'ST-01', FALSE, now(), now(), 1
        )
        "#,
    )
    .bind(source_batch_id)
    .bind(owner_id)
    .bind(product_id)
    .bind(storage_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("prod"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("exp"))
    .execute(pool)
    .await
    .expect("batch");

    let group_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO task_groups (
            id, owner_id, task_group_code, task_group_name, warehouse_id,
            zone_ids, task_type_codes, enabled
        ) VALUES ($1, $2, 'replenish-in', '区内补货班组', $3, $4, ARRAY['replenish'], TRUE)
        "#,
    )
    .bind(group_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(&[zone_in][..])
    .execute(pool)
    .await
    .expect("task group");
    sqlx::query(
        r#"
        INSERT INTO task_group_memberships (task_group_id, owner_id, user_id)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(group_id)
    .bind(owner_id)
    .bind(operator_id)
    .execute(pool)
    .await
    .expect("membership");

    World {
        owner_id,
        operator_id,
        warehouse_id,
        zone_in,
        zone_out,
        storage_id,
        pick_in_seq10,
        pick_in_seq30,
        pick_in_null,
        pick_out_normal,
        pick_out_urgent,
        source_batch_id,
        product_id,
    }
}

#[allow(clippy::too_many_arguments)]
async fn seed_location(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    zone_id: Uuid,
    code: &str,
    location_type: &str,
    pick_sequence: Option<i32>,
) -> Uuid {
    let location_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code,
            row_no, column_no, layer_no, max_volume_cm3, max_sku_count,
            location_type, current_owner_id, status, allows_container,
            mix_product_policy, mix_batch_policy, lock_status,
            pick_sequence_no, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5,
            1, 1, 1, 100000, 10,
            $6, $2, 'available', FALSE,
            'single_product_only', 'single_batch', 'normal',
            $7, now(), now()
        )
        "#,
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(code)
    .bind(location_type)
    .bind(pick_sequence)
    .execute(pool)
    .await
    .expect("location");
    location_id
}

async fn insert_task(
    pool: &PgPool,
    world: &World,
    task_no: &str,
    priority: &str,
    target: Uuid,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO replenishment_tasks (
            id, owner_id, task_no, trigger_mode, priority,
            source_location_id, source_batch_id, target_location_id,
            product_id, batch_no, qty, status, created_by
        ) VALUES (
            $1, $2, $3, 'manual', $4,
            $5, $6, $7,
            $8, 'B-SRC', 1, 'pending', 'test'
        )
        "#,
    )
    .bind(id)
    .bind(world.owner_id)
    .bind(task_no)
    .bind(priority)
    .bind(world.storage_id)
    .bind(world.source_batch_id)
    .bind(target)
    .bind(world.product_id)
    .execute(pool)
    .await
    .expect("task");
    id
}

fn app(pool: PgPool, auth: AuthContext) -> axum::Router {
    replenishment_router(ReplenishmentAppState::with_postgres(pool)).layer(Extension(auth))
}

async fn get_tasks(pool: PgPool, auth: AuthContext) -> (StatusCode, serde_json::Value) {
    let response = app(pool, auth)
        .oneshot(
            Request::builder()
                .uri("/api/v1/replenishment/tasks")
                .body(Body::empty())
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

async fn claim(
    pool: PgPool,
    auth: AuthContext,
    task_id: Uuid,
    idem: &str,
) -> (StatusCode, serde_json::Value) {
    let response = app(pool, auth)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/replenishment/tasks/{task_id}/claim"))
                .header("content-type", "application/json")
                .header("idempotency-key", idem)
                .body(Body::from(r#"{"version":1}"#))
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

#[sqlx::test(migrations = "../../migrations")]
async fn claim_normal_outside_zone_is_denied(pool: PgPool) {
    let world = seed_world(&pool).await;
    let task_id = insert_task(&pool, &world, "RP-N-OUT", "normal", world.pick_out_normal).await;
    let (status, body) = claim(
        pool,
        execute_ctx(world.owner_id, world.operator_id),
        task_id,
        "list-gwt-20",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M3_REPLENISH_ZONE_DENIED");
}

#[sqlx::test(migrations = "../../migrations")]
async fn claim_urgent_outside_zone_succeeds(pool: PgPool) {
    let world = seed_world(&pool).await;
    let task_id = insert_task(&pool, &world, "RP-U-OUT", "urgent", world.pick_out_urgent).await;
    let (status, body) = claim(
        pool,
        execute_ctx(world.owner_id, world.operator_id),
        task_id,
        "list-gwt-21",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "in_progress");
    assert_eq!(body["operator_id"], world.operator_id.to_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn execute_list_hides_normal_outside_zone_and_keeps_urgent(pool: PgPool) {
    let world = seed_world(&pool).await;
    let visible_normal = insert_task(&pool, &world, "RP-N-IN", "normal", world.pick_in_seq10).await;
    let hidden_normal =
        insert_task(&pool, &world, "RP-N-OUT", "normal", world.pick_out_normal).await;
    let visible_urgent =
        insert_task(&pool, &world, "RP-U-OUT", "urgent", world.pick_out_urgent).await;
    let (status, body) = get_tasks(pool, execute_ctx(world.owner_id, world.operator_id)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let ids: Vec<&str> = body["data"]
        .as_array()
        .expect("data")
        .iter()
        .map(|row| row["id"].as_str().expect("id"))
        .collect();
    assert!(ids.contains(&visible_normal.to_string().as_str()));
    assert!(ids.contains(&visible_urgent.to_string().as_str()));
    assert!(!ids.contains(&hidden_normal.to_string().as_str()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn execute_list_orders_urgent_then_pick_sequence_then_task_no(pool: PgPool) {
    let world = seed_world(&pool).await;
    insert_task(&pool, &world, "RP-U-B", "urgent", world.pick_in_seq30).await;
    insert_task(&pool, &world, "RP-U-C", "urgent", world.pick_in_seq10).await;
    insert_task(&pool, &world, "RP-U-A", "urgent", world.pick_in_seq10).await;
    insert_task(&pool, &world, "RP-N-Z", "normal", world.pick_in_seq30).await;
    insert_task(&pool, &world, "RP-N-Y", "normal", world.pick_in_null).await;
    insert_task(&pool, &world, "RP-N-OUT", "normal", world.pick_out_normal).await;
    let (status, body) = get_tasks(pool, execute_ctx(world.owner_id, world.operator_id)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let nos: Vec<&str> = body["data"]
        .as_array()
        .expect("data")
        .iter()
        .map(|row| row["task_no"].as_str().expect("task_no"))
        .collect();
    assert_eq!(nos, vec!["RP-U-A", "RP-U-C", "RP-U-B", "RP-N-Z", "RP-N-Y"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn execute_list_includes_own_task_outside_zone(pool: PgPool) {
    let world = seed_world(&pool).await;
    let own_id = insert_task(&pool, &world, "RP-OWN", "normal", world.pick_out_normal).await;
    sqlx::query(
        "UPDATE replenishment_tasks SET status = 'in_progress', operator_id = $2 WHERE id = $1",
    )
    .bind(own_id)
    .bind(world.operator_id)
    .execute(&pool)
    .await
    .expect("own");
    let (status, body) = get_tasks(pool, execute_ctx(world.owner_id, world.operator_id)).await;
    assert_eq!(status, StatusCode::OK);
    let nos: Vec<&str> = body["data"]
        .as_array()
        .expect("data")
        .iter()
        .map(|row| row["task_no"].as_str().expect("task_no"))
        .collect();
    assert!(nos.contains(&"RP-OWN"));
}
