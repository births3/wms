use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{NaiveDate, Utc};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    wave3_handlers::{wave3_router, Wave3AppState},
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{
    ApproveInventoryCountRequest, CreateInventoryCountRequest, SubmitInventoryCountLineRequest,
};

fn ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m3-inventory-count-test".to_string(),
        permissions: permissions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        jti: Uuid::new_v4().to_string(),
    }
}

async fn seed_inventory(pool: &PgPool, owner_id: Uuid, qty: i64) -> (Uuid, Uuid, Uuid, Uuid) {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, 'M3-COUNT-WH', 'M3 盘点仓', 'main', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed count warehouse");
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, 'M3-COUNT-ZONE', 'M3 盘点区', 'normal', 'qualified_green', 'active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("seed count zone");
    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status) VALUES ($1, $2, $3, $4, 'M3-COUNT-LOC', 1, 1, 1, 100000, 0, 3, 'storage', 'available')",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(pool)
    .await
    .expect("seed count location");
    sqlx::query(
        r#"INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code,
            recall_flag, created_at, updated_at
        ) VALUES ($1, $2, 'M3-COUNT-P', 'M3-COUNT-B', $3, $4, $5, 2, 'qualified', $6, 'M3-COUNT-LOC', FALSE, $7, $7)"#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("production date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("expiry date"))
    .bind(qty)
    .bind(location_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed count batch");

    (warehouse_id, zone_id, location_id, batch_id)
}

async fn seed_outbound_order(pool: &PgPool, owner_id: Uuid) -> Uuid {
    let order_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO outbound_orders (id, owner_id, wms_order_no, customer_id, warehouse_id, status) VALUES ($1, $2, 'M3-COUNT-OUT', $3, $4, 'confirmed')",
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .execute(pool)
    .await
    .expect("seed count outbound order");
    order_id
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_count_blind_submission_approves_atomic_adjustment_and_replays(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id, _location_id, batch_id) = seed_inventory(&pool, owner_id, 10).await;
    let repository = PgWave3Repository::new(pool.clone());
    let keeper = ctx(owner_id, &["m3.inventory_count.write"]);
    let manager = ctx(owner_id, &["m3.inventory_count.approve"]);

    let created = repository
        .create_inventory_count_with_audit(
            &keeper,
            CreateInventoryCountRequest {
                count_type: "blind".to_string(),
                warehouse_id: Some(warehouse_id),
                zone_id: Some(zone_id),
                product_code: None,
            },
            Utc::now(),
            "m3-count-create-1",
            None,
        )
        .await
        .expect("count should be created");
    assert_eq!(created.value.lines.len(), 1);
    assert_eq!(created.value.lines[0].inventory_batch_id, batch_id);
    assert_eq!(created.value.lines[0].book_qty, 8);

    let submitted = repository
        .submit_inventory_count_line_with_audit(
            &keeper,
            created.value.id,
            created.value.lines[0].id,
            SubmitInventoryCountLineRequest { physical_qty: 6 },
            Utc::now(),
            "m3-count-submit-1",
            None,
        )
        .await
        .expect("blind count should be submitted");
    assert_eq!(submitted.value.variance_qty, Some(-2));
    assert_eq!(submitted.value.variance_type.as_deref(), Some("loss"));

    let qty_before_approval: i64 = sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(batch_id)
    .fetch_one(&pool)
    .await
    .expect("read quantity before approval");
    assert_eq!(qty_before_approval, 10);

    let approved = repository
        .approve_inventory_count_with_audit(
            &manager,
            created.value.id,
            ApproveInventoryCountRequest {
                approval_source: "盘点".to_string(),
                approval_id: "M3-COUNT-APPROVAL-1".to_string(),
            },
            Utc::now(),
            "m3-count-approve-1",
            None,
        )
        .await
        .expect("count should be approved");
    assert_eq!(approved.value.status, "approved");

    let qty_after_approval: i64 = sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(batch_id)
    .fetch_one(&pool)
    .await
    .expect("read quantity after approval");
    assert_eq!(qty_after_approval, 8);

    let movement: (i64, String) = sqlx::query_as(
        "SELECT qty_delta, source_document_type FROM inventory_movements WHERE owner_id = $1 AND batch_id = $2 AND movement_type = 'inventory_count_adjustment'",
    )
    .bind(owner_id)
    .bind(batch_id)
    .fetch_one(&pool)
    .await
    .expect("count adjustment movement");
    assert_eq!(movement, (-2, "inventory_count".to_string()));

    let replay = repository
        .approve_inventory_count_with_audit(
            &manager,
            created.value.id,
            ApproveInventoryCountRequest {
                approval_source: "盘点".to_string(),
                approval_id: "M3-COUNT-APPROVAL-1".to_string(),
            },
            Utc::now(),
            "m3-count-approve-1",
            None,
        )
        .await
        .expect("approval should replay");
    assert!(replay.replayed);
    let movement_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventory_movements WHERE owner_id = $1 AND batch_id = $2 AND movement_type = 'inventory_count_adjustment'",
    )
    .bind(owner_id)
    .bind(batch_id)
    .fetch_one(&pool)
    .await
    .expect("count adjustment count");
    assert_eq!(movement_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_count_is_owner_scoped_and_blocks_new_allocations(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let (warehouse_id, _zone_id, _location_id, _batch_id) =
        seed_inventory(&pool, owner_id, 5).await;
    let repository = PgWave3Repository::new(pool.clone());
    let keeper = ctx(owner_id, &["m3.inventory_count.write"]);
    let created = repository
        .create_inventory_count_with_audit(
            &keeper,
            CreateInventoryCountRequest {
                count_type: "cycle".to_string(),
                warehouse_id: Some(warehouse_id),
                zone_id: None,
                product_code: None,
            },
            Utc::now(),
            "m3-count-create-owner-scope",
            None,
        )
        .await
        .expect("count should be created");

    let foreign_read = repository
        .get_inventory_count(&ctx(other_owner_id, &["m3.read"]), created.value.id)
        .await;
    assert!(matches!(foreign_read, Err(Wave3RepositoryError::NotFound)));

    let order_id = seed_outbound_order(&pool, owner_id).await;
    let allocation_result = sqlx::query(
        "INSERT INTO inventory_allocations (id, owner_id, outbound_order_id, line_no, batch_id, allocated_qty, status) VALUES ($1, $2, $3, 1, $4, 1, 'locked')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(order_id)
    .bind(created.value.lines[0].inventory_batch_id)
    .execute(&pool)
    .await;
    assert!(
        allocation_result.is_err(),
        "active count must block allocation"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_count_http_write_requires_count_permission(pool: PgPool) {
    let app = wave3_router(Wave3AppState::with_postgres(pool));
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/v1/inventory/counts")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "count_type": "blind"
            }))
            .expect("serialize count request"),
        ))
        .expect("build count request");
    request
        .extensions_mut()
        .insert(ctx(Uuid::new_v4(), &["m3.read"]));

    let response = app.oneshot(request).await.expect("count request response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
