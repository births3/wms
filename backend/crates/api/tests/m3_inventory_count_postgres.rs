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
        warehouse_scope: None,
    }
}

async fn seed_inventory(pool: &PgPool, owner_id: Uuid, qty: i64) -> (Uuid, Uuid, Uuid, Uuid) {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'M3 盘点测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("M3-COUNT-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed count owner");
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
        "INSERT INTO outbound_orders (id, owner_id, wms_order_no, customer_id, delivery_address_id, delivery_address_snapshot, warehouse_id, status) VALUES ($1, $2, 'M3-COUNT-OUT', $3, gen_random_uuid(), '{}'::jsonb, $4, 'confirmed')",
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
    // 盲盘在实盘提交前不得回显账面数量
    assert_eq!(created.value.lines[0].book_qty, 0);

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
                // |差异| 2 / 账面 8 = 25% > 10%，需高级审批源
                approval_source: "盘点-高级".to_string(),
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

    let snapshot: (String, serde_json::Value, String) = sqlx::query_as(
        "SELECT snapshot_no, payload, status FROM inventory_snapshot_erp_feedback_outbox WHERE owner_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("approved count should publish inventory snapshot");
    assert_eq!(snapshot.0, format!("{}:{warehouse_id}", created.value.id));
    assert_eq!(snapshot.1["warehouse_id"], warehouse_id.to_string());
    assert_eq!(snapshot.1["count_id"], created.value.id.to_string());
    assert_eq!(snapshot.1["lines"][0]["qty_on_hand"], 8);
    assert_eq!(snapshot.1["lines"][0]["qty_available"], 6);
    assert_eq!(snapshot.2, "pending");

    let replay = repository
        .approve_inventory_count_with_audit(
            &manager,
            created.value.id,
            ApproveInventoryCountRequest {
                approval_source: "盘点-高级".to_string(),
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
    let (outbox_count, audit_count, idempotency_count): (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
          (SELECT COUNT(*) FROM inventory_snapshot_erp_feedback_outbox
            WHERE owner_id = $1),
          (SELECT COUNT(*) FROM audit_event
            WHERE owner_id = $1 AND action = 'approve_inventory_count'
              AND resource_id = $2),
          (SELECT COUNT(*) FROM idempotency_request
            WHERE owner_id = $1 AND idempotency_key = $3)
        "#,
    )
    .bind(owner_id)
    .bind(created.value.id.to_string())
    .bind("m3-count-approve-1")
    .fetch_one(&pool)
    .await
    .expect("inventory snapshot replay evidence should query");
    assert_eq!((outbox_count, audit_count, idempotency_count), (1, 1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_snapshot_failure_rolls_back_count_approval(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id, _location_id, batch_id) = seed_inventory(&pool, owner_id, 10).await;
    let repository = PgWave3Repository::new(pool.clone());
    let keeper = ctx(owner_id, &["m3.inventory_count.write"]);
    let manager = ctx(owner_id, &["m3.inventory_count.approve"]);
    let created = repository
        .create_inventory_count_with_audit(
            &keeper,
            CreateInventoryCountRequest {
                count_type: "cycle".to_string(),
                warehouse_id: Some(warehouse_id),
                zone_id: Some(zone_id),
                product_code: None,
            },
            Utc::now(),
            "m3-count-rollback-create",
            None,
        )
        .await
        .expect("rollback count should create");
    repository
        .submit_inventory_count_line_with_audit(
            &keeper,
            created.value.id,
            created.value.lines[0].id,
            SubmitInventoryCountLineRequest { physical_qty: 6 },
            Utc::now(),
            "m3-count-rollback-submit",
            None,
        )
        .await
        .expect("rollback count line should submit");
    sqlx::query(
        "ALTER TABLE inventory_snapshot_erp_feedback_outbox ADD CONSTRAINT reject_inventory_snapshot_test CHECK (FALSE)",
    )
    .execute(&pool)
    .await
    .expect("snapshot failure constraint should install");

    let error = repository
        .approve_inventory_count_with_audit(
            &manager,
            created.value.id,
            ApproveInventoryCountRequest {
                approval_source: "盘点-高级".to_string(),
                approval_id: "M3-COUNT-ROLLBACK-APPROVAL".to_string(),
            },
            Utc::now(),
            "m3-count-rollback-approve",
            None,
        )
        .await
        .expect_err("outbox failure must reject count approval");
    assert!(matches!(error, Wave3RepositoryError::Database(_)));
    let state: (String, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT count_sheet.status, batch.qty_on_hand,
          (SELECT COUNT(*) FROM inventory_movements movement
            WHERE movement.owner_id = $1 AND movement.source_document_id = $2),
          (SELECT COUNT(*) FROM inventory_snapshot_erp_feedback_outbox outbox
            WHERE outbox.owner_id = $1),
          (SELECT COUNT(*) FROM audit_event audit
            WHERE audit.owner_id = $1 AND audit.action = 'approve_inventory_count'
              AND audit.resource_id = $2::text),
          (SELECT COUNT(*) FROM idempotency_request request
            WHERE request.owner_id = $1
              AND request.idempotency_key = $4)
          FROM inventory_counts count_sheet
          JOIN inventory_batches batch ON batch.owner_id = count_sheet.owner_id
           AND batch.id = $3
         WHERE count_sheet.owner_id = $1 AND count_sheet.id = $2
        "#,
    )
    .bind(owner_id)
    .bind(created.value.id)
    .bind(batch_id)
    .bind("m3-count-rollback-approve")
    .fetch_one(&pool)
    .await
    .expect("rolled back count state should query");
    assert_eq!(state, ("pending_approval".to_string(), 10, 0, 0, 0, 0));
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
