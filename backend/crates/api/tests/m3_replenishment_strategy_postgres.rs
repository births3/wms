//! T04：策略 / 库位组 / 挂接 / 预览 HTTP 契约（GWT 1/22/23/30）。

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Extension,
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    replenishment_handlers::{replenishment_router, ReplenishmentAppState},
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "replenish-test".into(),
        permissions: vec!["m3.replenishment.manage".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_product(pool: &PgPool, owner_id: Uuid, code: &str) -> Uuid {
    let product_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '补货商品', '1', 'pending_mapping', now(), now())
        "#,
    )
    .bind(product_id)
    .bind(owner_id)
    .bind(code)
    .execute(pool)
    .await
    .expect("seed product");
    product_id
}

async fn seed_owner(pool: &PgPool) -> Uuid {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '补货策略货主')",
    )
    .bind(owner_id)
    .bind(format!("RP-{}", &owner_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed owner");
    owner_id
}

async fn seed_pick_location(pool: &PgPool, owner_id: Uuid, code: &str) -> Uuid {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
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
        ) VALUES ($1, $2, $3, 'Z-RP', '合格区', 'normal_10_30', 'qualified_green', 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("seed zone");
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
            'piece_pick', $2, 'available', FALSE,
            'single_product_only', 'single_batch', 'normal', now(), now()
        )
        "#,
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(code)
    .execute(pool)
    .await
    .expect("seed location");
    location_id
}

fn app(pool: PgPool, owner_id: Uuid) -> axum::Router {
    replenishment_router(ReplenishmentAppState::with_postgres(pool)).layer(Extension(ctx(owner_id)))
}

async fn post_strategy(
    app: axum::Router,
    body: serde_json::Value,
    idem: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/v1/replenishment/strategies")
        .header("content-type", "application/json");
    if let Some(key) = idem {
        builder = builder.header("idempotency-key", key);
    }
    let response = app
        .oneshot(builder.body(Body::from(body.to_string())).expect("request"))
        .await
        .expect("oneshot");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({}));
    (status, json)
}

fn valid_body(code: &str, product_id: Uuid) -> serde_json::Value {
    serde_json::json!({
        "strategy_code": code,
        "strategy_name": "零拣补货",
        "scope_type": "product",
        "scope_ref": product_id,
        "source_type": "storage",
        "target_type": "piece_pick",
        "min_safety_threshold": "5",
        "max_replenish_target": "20",
        "trigger_modes": ["min_max", "wave_gap"],
        "enabled": true
    })
}

#[sqlx::test(migrations = "../../migrations")]
async fn post_illegal_route_returns_strategy_invalid(pool: PgPool) {
    let owner_id = seed_owner(&pool).await;
    let (status, body) = post_strategy(
        app(pool, owner_id),
        serde_json::json!({
            "strategy_code": "BAD-ROUTE",
            "strategy_name": "非法动线",
            "scope_type": "product",
            "scope_ref": Uuid::new_v4(),
            "source_type": "piece_pick",
            "target_type": "storage",
            "min_safety_threshold": "1",
            "max_replenish_target": "10",
            "trigger_modes": ["min_max"]
        }),
        Some("rp-gwt-1"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M3_REPLENISH_STRATEGY_INVALID");
}

#[sqlx::test(migrations = "../../migrations")]
async fn post_strategy_without_idempotency_key_is_rejected(pool: PgPool) {
    let owner_id = seed_owner(&pool).await;
    let (status, body) = post_strategy(
        app(pool, owner_id),
        valid_body("NO-KEY", Uuid::new_v4()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "M3_REPLENISH_IDEMPOTENCY_REQUIRED");
}

#[sqlx::test(migrations = "../../migrations")]
async fn bind_location_already_owned_by_other_strategy_conflicts(pool: PgPool) {
    let owner_id = seed_owner(&pool).await;
    let location_id = seed_pick_location(&pool, owner_id, "PP-01").await;
    let product_a = seed_product(&pool, owner_id, "P-A").await;
    let product_b = seed_product(&pool, owner_id, "P-B").await;
    let router = app(pool.clone(), owner_id);
    let (created_a, body_a) = post_strategy(
        router.clone(),
        valid_body("STR-A", product_a),
        Some("rp-str-a"),
    )
    .await;
    assert_eq!(created_a, StatusCode::OK);
    let id_a = body_a["id"].as_str().expect("strategy a id");
    let (created_b, body_b) = post_strategy(
        router.clone(),
        valid_body("STR-B", product_b),
        Some("rp-str-b"),
    )
    .await;
    assert_eq!(created_b, StatusCode::OK);
    let id_b = body_b["id"].as_str().expect("strategy b id");

    let bind_a = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/replenishment/strategies/{id_a}/locations"))
                .header("content-type", "application/json")
                .header("idempotency-key", "rp-bind-a")
                .body(Body::from(
                    serde_json::json!({ "location_ids": [location_id] }).to_string(),
                ))
                .expect("bind a"),
        )
        .await
        .expect("oneshot bind a");
    assert_eq!(bind_a.status(), StatusCode::OK);

    let bind_b = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/replenishment/strategies/{id_b}/locations"))
                .header("content-type", "application/json")
                .header("idempotency-key", "rp-bind-b")
                .body(Body::from(
                    serde_json::json!({ "location_ids": [location_id] }).to_string(),
                ))
                .expect("bind b"),
        )
        .await
        .expect("oneshot bind b");
    assert_eq!(bind_b.status(), StatusCode::CONFLICT);
    let bytes = to_bytes(bind_b.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(json["code"], "M3_REPLENISH_LOCATION_BOUND");
}

#[sqlx::test(migrations = "../../migrations")]
async fn category_scope_ref_must_be_special_drug_dictionary_item(pool: PgPool) {
    let owner_id = seed_owner(&pool).await;
    let (status, body) = post_strategy(
        app(pool, owner_id),
        serde_json::json!({
            "strategy_code": "BAD-CAT",
            "strategy_name": "非法品类",
            "scope_type": "category",
            "scope_ref": Uuid::new_v4(),
            "source_type": "storage",
            "target_type": "piece_pick",
            "min_safety_threshold": "1",
            "max_replenish_target": "10",
            "trigger_modes": ["min_max"]
        }),
        Some("rp-gwt-30"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "M3_REPLENISH_SCOPE_NOT_FOUND");
}
