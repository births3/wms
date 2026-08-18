//! T05：任务生成（FEFO + 双字段 + 编号 + 手工发起）。GWT 2 / L6。

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
    replenishment_service::ReplenishmentService,
};
use wms_domain::Quantity;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "replenish-supervisor".into(),
        permissions: vec!["m3.replenishment.manage".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

struct World {
    owner_id: Uuid,
    product_id: Uuid,
    storage_id: Uuid,
    pick_id: Uuid,
    source_batch_id: Uuid,
}

async fn seed_world(pool: &PgPool, pick_on_hand: i64, source_on_hand: i64) -> World {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '补货生成货主')",
    )
    .bind(owner_id)
    .bind(format!("RG-{}", &owner_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed owner");

    let product_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '补货生成商品', '1', 'pending_mapping', now(), now())
        "#,
    )
    .bind(product_id)
    .bind(owner_id)
    .bind(format!("P-{}", &product_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed product");

    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '补货仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, status, created_at, updated_at
        ) VALUES ($1, $2, $3, 'Z-RG', '合格区', 'normal_10_30', 'qualified_green', 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("seed zone");

    let storage_id = seed_location(pool, owner_id, warehouse_id, zone_id, "ST-01", "storage").await;
    let pick_id = seed_location(pool, owner_id, warehouse_id, zone_id, "PP-01", "piece_pick").await;

    if pick_on_hand > 0 {
        seed_batch(
            pool,
            owner_id,
            product_id,
            pick_id,
            "PP-01",
            "B-PICK",
            pick_on_hand,
            NaiveDate::from_ymd_opt(2028, 6, 1).expect("expiry"),
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
        NaiveDate::from_ymd_opt(2028, 1, 1).expect("expiry"),
    )
    .await;

    World {
        owner_id,
        product_id,
        storage_id,
        pick_id,
        source_batch_id,
    }
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
    .expect("seed location");
    location_id
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
            $1, $2, $3, 'P-RG', $4,
            $5, $6, $7, 0, 0,
            0, 0,
            'qualified', $8, $9, FALSE, now(), now(), 1
        )
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(product_id)
    .bind(batch_no)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("production"))
    .bind(expiry)
    .bind(Quantity::from(on_hand))
    .bind(location_id)
    .bind(location_code)
    .execute(pool)
    .await
    .expect("seed batch");
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
            $1, $2, 'STR-GWT2', '零拣补货', 'product', $3,
            'piece_pick', 'storage', 'piece_pick',
            5, 20, ARRAY['min_max','wave_gap'], TRUE
        )
        "#,
    )
    .bind(strategy_id)
    .bind(owner_id)
    .bind(product_id)
    .execute(pool)
    .await
    .expect("seed strategy");
    sqlx::query(
        r#"
        UPDATE warehouse_locations
           SET replenish_strategy_id = $2
         WHERE id = $1 AND owner_id = $3
        "#,
    )
    .bind(pick_id)
    .bind(strategy_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("bind strategy");
    strategy_id
}

fn app(pool: PgPool, owner_id: Uuid) -> axum::Router {
    replenishment_router(ReplenishmentAppState::with_postgres(pool)).layer(Extension(ctx(owner_id)))
}

async fn post_task(
    app: axum::Router,
    body: serde_json::Value,
    idem: &str,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/replenishment/tasks")
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
    .expect("qty snapshot")
}

async fn target_in_transit(pool: &PgPool, location_id: Uuid, product_id: Uuid) -> Quantity {
    sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(qty_replenish_in_transit), 0)
          FROM inventory_batches
         WHERE location_id = $1 AND product_id = $2
        "#,
    )
    .bind(location_id)
    .bind(product_id)
    .fetch_one(pool)
    .await
    .expect("target in transit")
}

#[sqlx::test(migrations = "../../migrations")]
async fn generate_min_max_creates_pending_qty_and_dual_transit(pool: PgPool) {
    let world = seed_world(&pool, 2, 30).await;
    let strategy_id = insert_strategy(&pool, world.owner_id, world.product_id, world.pick_id).await;
    let service = ReplenishmentService::new(
        wms_api::replenishment_repository::PgReplenishmentRepository::new(pool.clone()),
    );
    let created = service
        .generate_task(
            &ctx(world.owner_id),
            strategy_id,
            world.pick_id,
            world.product_id,
        )
        .await
        .expect("generate");
    assert_eq!(created.len(), 1);
    let task = &created[0];

    assert_eq!(task.status, "pending");
    assert_eq!(task.trigger_mode, "min_max");
    assert_eq!(task.qty, Quantity::from(18));
    assert_eq!(task.source_batch_id, world.source_batch_id);
    assert!(
        task.task_no.starts_with("RT-"),
        "M-CG numbered: {}",
        task.task_no
    );

    let (src_on, _src_in, src_out) = qty_snapshot(&pool, world.source_batch_id).await;
    assert_eq!(src_on, Quantity::from(30));
    assert_eq!(src_out, Quantity::from(18));
    assert_eq!(
        target_in_transit(&pool, world.pick_id, world.product_id).await,
        Quantity::from(18)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn post_manual_task_uses_same_generate_and_reserves(pool: PgPool) {
    let world = seed_world(&pool, 0, 30).await;
    let (status, body) = post_task(
        app(pool.clone(), world.owner_id),
        serde_json::json!({
            "source_location_id": world.storage_id,
            "source_batch_id": world.source_batch_id,
            "target_location_id": world.pick_id,
            "qty": "18"
        }),
        "rp-manual-1",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "pending");
    assert_eq!(body["trigger_mode"], "manual");
    let qty: Quantity = serde_json::from_value(body["qty"].clone()).expect("qty");
    assert_eq!(qty, Quantity::from(18));
    assert_eq!(body["source_batch_id"], world.source_batch_id.to_string());

    let (src_on, _, src_out) = qty_snapshot(&pool, world.source_batch_id).await;
    assert_eq!(src_on, Quantity::from(30));
    assert_eq!(src_out, Quantity::from(18));
    assert_eq!(
        target_in_transit(&pool, world.pick_id, world.product_id).await,
        Quantity::from(18)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn second_task_does_not_generate_when_source_exhausted(pool: PgPool) {
    let world = seed_world(&pool, 0, 18).await;
    let first = post_task(
        app(pool.clone(), world.owner_id),
        serde_json::json!({
            "source_location_id": world.storage_id,
            "source_batch_id": world.source_batch_id,
            "target_location_id": world.pick_id,
            "qty": "18"
        }),
        "rp-l6-1",
    )
    .await;
    assert_eq!(first.0, StatusCode::OK);

    let second = post_task(
        app(pool.clone(), world.owner_id),
        serde_json::json!({
            "source_location_id": world.storage_id,
            "source_batch_id": world.source_batch_id,
            "target_location_id": world.pick_id,
            "qty": "18"
        }),
        "rp-l6-2",
    )
    .await;
    assert_eq!(second.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(second.1["code"], "M3_REPLENISH_SOURCE_UNAVAILABLE");

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM replenishment_tasks WHERE owner_id = $1")
            .bind(world.owner_id)
            .fetch_one(&pool)
            .await
            .expect("count tasks");
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn generate_selects_earliest_expiry_source_batch(pool: PgPool) {
    let world = seed_world(&pool, 2, 10).await;
    let later = seed_batch(
        &pool,
        world.owner_id,
        world.product_id,
        world.storage_id,
        "ST-01",
        "B-LATE",
        30,
        NaiveDate::from_ymd_opt(2029, 1, 1).expect("later expiry"),
    )
    .await;
    let strategy_id = insert_strategy(&pool, world.owner_id, world.product_id, world.pick_id).await;
    let service = ReplenishmentService::new(
        wms_api::replenishment_repository::PgReplenishmentRepository::new(pool.clone()),
    );
    let created = service
        .generate_task(
            &ctx(world.owner_id),
            strategy_id,
            world.pick_id,
            world.product_id,
        )
        .await
        .expect("generate");
    assert_eq!(created.len(), 2);
    assert_eq!(created[0].source_batch_id, world.source_batch_id);
    assert_eq!(created[0].qty, Quantity::from(10));
    assert_eq!(created[1].source_batch_id, later);
    assert_eq!(created[1].qty, Quantity::from(8));
}

#[sqlx::test(migrations = "../../migrations")]
async fn generate_sets_source_lpn_when_qty_meets_full_lpn_ratio(pool: PgPool) {
    let world = seed_world(&pool, 0, 10).await;
    let lpn_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO lpn_containers (
            id, owner_id, lpn_code, container_type, status, location_id, created_at, updated_at
        ) VALUES ($1, $2, 'LPN-GEN-01', 'pallet', 'in_use', $3, now(), now())
        "#,
    )
    .bind(lpn_id)
    .bind(world.owner_id)
    .bind(world.storage_id)
    .execute(&pool)
    .await
    .expect("lpn");
    sqlx::query(
        "UPDATE inventory_batches SET container_lpn = 'LPN-GEN-01' WHERE id = $1 AND owner_id = $2",
    )
    .bind(world.source_batch_id)
    .bind(world.owner_id)
    .execute(&pool)
    .await
    .expect("bind lpn");
    let strategy_id = insert_strategy(&pool, world.owner_id, world.product_id, world.pick_id).await;
    let service = ReplenishmentService::new(
        wms_api::replenishment_repository::PgReplenishmentRepository::new(pool.clone()),
    );
    let created = service
        .generate_task(
            &ctx(world.owner_id),
            strategy_id,
            world.pick_id,
            world.product_id,
        )
        .await
        .expect("generate");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].qty, Quantity::from(10));
    assert_eq!(created[0].source_lpn_id, Some(lpn_id));
}

#[sqlx::test(migrations = "../../migrations")]
async fn generate_writes_audit_event(pool: PgPool) {
    let world = seed_world(&pool, 2, 30).await;
    let strategy_id = insert_strategy(&pool, world.owner_id, world.product_id, world.pick_id).await;
    let service = ReplenishmentService::new(
        wms_api::replenishment_repository::PgReplenishmentRepository::new(pool.clone()),
    );
    let created = service
        .generate_task(
            &ctx(world.owner_id),
            strategy_id,
            world.pick_id,
            world.product_id,
        )
        .await
        .expect("generate");
    assert_eq!(created.len(), 1);
    let audits: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM audit_event
         WHERE owner_id = $1 AND action = 'create_replenishment_task'
        "#,
    )
    .bind(world.owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit");
    assert_eq!(audits, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn post_task_without_numbering_rule_returns_unavailable(pool: PgPool) {
    sqlx::query("UPDATE document_number_rules SET enabled = FALSE WHERE document_type = $1")
        .bind("replenishment_task")
        .execute(&pool)
        .await
        .expect("disable numbering");
    let world = seed_world(&pool, 0, 30).await;
    let (status, body) = post_task(
        app(pool, world.owner_id),
        serde_json::json!({
            "source_location_id": world.storage_id,
            "source_batch_id": world.source_batch_id,
            "target_location_id": world.pick_id,
            "qty": "18"
        }),
        "rp-no-rule",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M3_REPLENISH_NUMBERING_UNAVAILABLE");
}
