use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    stock_adjustment::{PgStockAdjustmentRepository, StockAdjustmentError},
    stock_adjustment_handlers::{stock_adjustment_router, StockAdjustmentAppState},
};
use wms_domain::{
    CreateStockLossOrderRequest, CreateStockSurplusOrderRequest, Quantity, StockAdjustmentSource,
    StockLossReason, StockSurplusReason,
};

fn ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "stock-adjustment-test".to_string(),
        permissions: vec![
            "msa.stock-adjustment.read".to_string(),
            "msa.stock-adjustment.write".to_string(),
            "msa.stock-adjustment.execute".to_string(),
            "msa.stock-adjustment.quality-approve".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_loss_fixture(pool: &PgPool, category: &str) -> (Uuid, Uuid, Uuid, Uuid, Uuid) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let first_operator_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '报损测试货主')",
    )
    .bind(owner_id)
    .bind(format!("SA-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("owner should seed");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, '报损测试仓', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("SA-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("warehouse should seed");
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, special_drug_category, volume_cm3, status) VALUES ($1, $2, $3, '报损测试商品', '1 unit', 'normal_10_30', $4, 100, 'active')",
    )
    .bind(product_id)
    .bind(owner_id)
    .bind(format!("SA-P-{}", &product_id.to_string()[..8]))
    .bind(category)
    .execute(pool)
    .await
    .expect("product should seed");
    let product_code: String =
        sqlx::query_scalar("SELECT product_code FROM products WHERE id = $1")
            .bind(product_id)
            .fetch_one(pool)
            .await
            .expect("product code should load");
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1,$2,$3,$4,'报损测试区','normal_10_30','unqualified_red','active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(format!("SA-Z-{}", &zone_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("zone should seed");
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no,
            layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status
        ) VALUES ($1,$2,$3,$4,$5,1,1,1,100000,1000,10,'storage','occupied')
        "#,
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(format!("SA-L-{}", &location_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("location should seed");
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_frozen, status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, 'SA-BATCH-001', DATE '2026-01-01', DATE '2028-01-01',
                10, 0, 'unqualified', $5, 'UNQUALIFIED-01', FALSE, $6, $6)
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(product_id)
    .bind(product_code)
    .bind(location_id)
    .bind(Utc::now())
    .execute(pool)
    .await
    .expect("batch should seed");
    seed_operator(pool, owner_id, first_operator_id, "custodian").await;
    (
        owner_id,
        warehouse_id,
        product_id,
        batch_id,
        first_operator_id,
    )
}

async fn seed_operator(pool: &PgPool, owner_id: Uuid, user_id: Uuid, role_code: &str) {
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '报损操作人', 'test-hash', 'active')",
    )
    .bind(user_id)
    .bind(format!("sa-user-{}", &user_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("operator should seed");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("operator binding should seed");
    let role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND lower(role_code) = $2",
    )
    .bind(owner_id)
    .bind(role_code)
    .fetch_one(pool)
    .await
    .expect("seeded owner role should exist");
    sqlx::query("INSERT INTO auth_user_roles (user_id, owner_id, role_id) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(owner_id)
        .bind(role_id)
        .execute(pool)
        .await
        .expect("operator role should seed");
}

fn create_request(
    warehouse_id: Uuid,
    batch_id: Uuid,
    reason: StockLossReason,
) -> CreateStockLossOrderRequest {
    CreateStockLossOrderRequest {
        warehouse_id,
        batch_id,
        quantity: Quantity::from(4),
        reason,
        recall_id: None,
        source: StockAdjustmentSource::Manual,
        external_ref: None,
        requires_quality_approval: false,
    }
}

include!("stock_adjustment_postgres/surplus.rs");

#[sqlx::test(migrations = "../../migrations")]
async fn manual_loss_is_numbered_executed_atomically_audited_and_idempotent(pool: PgPool) {
    let (owner_id, warehouse_id, _, batch_id, first_operator_id) =
        seed_loss_fixture(&pool, "none").await;
    let repository = PgStockAdjustmentRepository::new(pool.clone());
    let ctx = ctx(owner_id, first_operator_id);
    let now = Utc::now();
    let request = create_request(warehouse_id, batch_id, StockLossReason::Damaged);

    let created = repository
        .create_loss_order(&ctx, request.clone(), now, "sa-create-1")
        .await
        .expect("manual loss should be created");
    let replay = repository
        .create_loss_order(&ctx, request, now, "sa-create-1")
        .await
        .expect("same create request should replay");
    assert_eq!(created.value.id, replay.value.id);
    assert!(replay.replayed);
    assert!(created.value.order_no.starts_with("BS"));
    assert_eq!(created.value.status.as_str(), "pending_execution");

    let started = repository
        .start_loss_order(&ctx, created.value.id, now, "sa-start-1")
        .await
        .expect("pending order should start");
    assert_eq!(started.value.status.as_str(), "in_progress");

    let completed = repository
        .execute_loss_order(&ctx, created.value.id, None, now, "sa-execute-1")
        .await
        .expect("single-person loss should complete");
    let execute_replay = repository
        .execute_loss_order(&ctx, created.value.id, None, now, "sa-execute-1")
        .await
        .expect("same execute request should replay");
    assert_eq!(completed.value.id, execute_replay.value.id);
    assert!(execute_replay.replayed);
    assert_eq!(completed.value.status.as_str(), "completed");
    assert_eq!(completed.value.first_operator_id, Some(first_operator_id));
    assert_eq!(
        completed
            .value
            .policy
            .expect("policy should persist")
            .as_str(),
        "single"
    );

    let (
        qty_on_hand,
        movement_count,
        movement_approval_source,
        movement_approval_id,
        execution_count,
        execute_audit_count,
        outbox_count,
        execute_idempotency_count,
    ): (Quantity, i64, String, String, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
          (SELECT qty_on_hand FROM inventory_batches WHERE id = $1),
          (SELECT COUNT(*) FROM inventory_movements WHERE source_document_id = $2 AND movement_type = 'stock_loss'),
          (SELECT approval_source FROM inventory_movements WHERE source_document_id = $2 AND movement_type = 'stock_loss'),
          (SELECT approval_id FROM inventory_movements WHERE source_document_id = $2 AND movement_type = 'stock_loss'),
          (SELECT COUNT(*) FROM stock_adjustment_execution_records WHERE order_id = $2),
          (SELECT COUNT(*) FROM audit_event WHERE owner_id = $3 AND action = 'execute_stock_loss_order' AND resource_id = $2::TEXT),
          (SELECT COUNT(*) FROM stock_adjustment_erp_feedback_outbox WHERE order_id = $2 AND event_type = 'stock_loss_completed' AND status = 'pending' AND payload->>'warehouse_id' = $4::TEXT),
          (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $3 AND idempotency_key = 'sa-execute-1')
        "#,
    )
    .bind(batch_id)
    .bind(created.value.id)
    .bind(owner_id)
    .bind(warehouse_id)
    .fetch_one(&pool)
    .await
    .expect("execution evidence should load");
    assert_eq!(qty_on_hand, Quantity::from(6));
    assert_eq!(movement_count, 1);
    assert_eq!(movement_approval_source, "报损报溢单");
    assert_eq!(movement_approval_id, created.value.id.to_string());
    assert_eq!(execution_count, 1);
    assert_eq!(execute_audit_count, 1);
    assert_eq!(outbox_count, 1);
    assert_eq!(execute_idempotency_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn destruction_requires_quality_approval_second_operator_and_h4_approval(pool: PgPool) {
    let (owner_id, warehouse_id, _, batch_id, first_operator_id) =
        seed_loss_fixture(&pool, "narcotic").await;
    let second_operator_id = Uuid::new_v4();
    seed_operator(&pool, owner_id, second_operator_id, "custodian").await;
    let repository = PgStockAdjustmentRepository::new(pool.clone());
    let ctx = ctx(owner_id, first_operator_id);
    let now = Utc::now();
    sqlx::query("UPDATE inventory_batches SET recall_flag = TRUE WHERE id = $1")
        .bind(batch_id)
        .execute(&pool)
        .await
        .expect("recalled batch should seed");
    let mut destruction_request =
        create_request(warehouse_id, batch_id, StockLossReason::RecallDestruction);
    destruction_request.recall_id = Some("QL-RECALL-001".to_string());

    let created = repository
        .create_loss_order(&ctx, destruction_request, now, "sa-destroy-create-1")
        .await
        .expect("destruction should be created for approval");
    assert_eq!(created.value.status.as_str(), "pending_approval");
    assert!(created.value.requires_quality_approval);

    let approved = repository
        .record_quality_approval(
            &ctx,
            created.value.id,
            "QL-DESTRUCTION-001",
            true,
            now,
            "sa-destroy-quality-approval-1",
        )
        .await
        .expect("quality approval should release destruction");
    assert_eq!(approved.value.status.as_str(), "pending_execution");
    repository
        .start_loss_order(&ctx, created.value.id, now, "sa-destroy-start-1")
        .await
        .expect("approved destruction should start");

    let missing_second = repository
        .execute_loss_order(
            &ctx,
            created.value.id,
            None,
            now,
            "sa-destroy-execute-missing",
        )
        .await
        .expect_err("narcotic destruction must require second operator");
    assert_eq!(missing_second, StockAdjustmentError::MissingSecondOperator);

    let missing_approval = repository
        .execute_loss_order(
            &ctx,
            created.value.id,
            Some(second_operator_id),
            now,
            "sa-destroy-execute-no-h4",
        )
        .await
        .expect_err("strict policy must require approved H4 record");
    assert_eq!(
        missing_approval,
        StockAdjustmentError::DualPersonApprovalRequired
    );

    let h4_approval_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO h4_approval_records (
            id, owner_id, scenario, business_ref, dedupe_key, approver_user,
            process_id, callback_path, summary, status, approved_by, approved_at,
            created_at, updated_at
        )
        VALUES ($1, $2, 'mvr.dual_person', $3, 'sa-destroy-h4-1', 'warehouse-manager',
                'PROC-SA-001', '/api/v1/stock-adjustments/loss-orders/callback',
                '销毁双人策略审批', 'approved', 'warehouse-manager', $4, $4, $4)
        "#,
    )
    .bind(h4_approval_id)
    .bind(owner_id)
    .bind(created.value.id.to_string())
    .bind(now)
    .execute(&pool)
    .await
    .expect("H4 approval should seed");

    let completed = repository
        .execute_loss_order(
            &ctx,
            created.value.id,
            Some(second_operator_id),
            now,
            "sa-destroy-execute-1",
        )
        .await
        .expect("approved dual-person destruction should complete");
    assert_eq!(completed.value.status.as_str(), "completed");
    assert_eq!(completed.value.second_operator_id, Some(second_operator_id));
    assert_eq!(completed.value.approval_record_id, Some(h4_approval_id));
    assert!(completed.value.source_rule_id.is_some());
    let recall_flag: bool =
        sqlx::query_scalar("SELECT recall_flag FROM inventory_batches WHERE id = $1")
            .bind(batch_id)
            .fetch_one(&pool)
            .await
            .expect("destroyed recall state should load");
    assert!(!recall_flag);
    let movement_approval: (String, String) = sqlx::query_as(
        "SELECT approval_source, approval_id FROM inventory_movements WHERE source_document_id = $1",
    )
    .bind(created.value.id)
    .fetch_one(&pool)
    .await
    .expect("destruction approval source should persist");
    assert_eq!(
        movement_approval,
        ("质量联系单".to_string(), "QL-DESTRUCTION-001".to_string())
    );

    let process_node: (String, String) = sqlx::query_as(
        "SELECT process_code, node_code FROM stock_adjustment_execution_records WHERE order_id = $1",
    )
    .bind(created.value.id)
    .fetch_one(&pool)
    .await
    .expect("execution process evidence should exist");
    assert_eq!(process_node, ("销毁".to_string(), "销毁执行".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn insufficient_inventory_rolls_back_loss_execution(pool: PgPool) {
    let (owner_id, warehouse_id, _, batch_id, first_operator_id) =
        seed_loss_fixture(&pool, "none").await;
    let repository = PgStockAdjustmentRepository::new(pool.clone());
    let ctx = ctx(owner_id, first_operator_id);
    let now = Utc::now();
    let mut excessive = create_request(warehouse_id, batch_id, StockLossReason::InventoryLoss);
    excessive.quantity = Quantity::from(11);
    let create_error = repository
        .create_loss_order(&ctx, excessive, now, "sa-over-create-invalid")
        .await
        .expect_err("loss above current available stock must be rejected at create");
    assert_eq!(create_error, StockAdjustmentError::QuantityExceeded);

    let mut request = create_request(warehouse_id, batch_id, StockLossReason::InventoryLoss);
    request.quantity = Quantity::from(9);
    let created = repository
        .create_loss_order(&ctx, request, now, "sa-over-create-1")
        .await
        .expect("loss within current available stock should be created");
    sqlx::query("UPDATE inventory_batches SET qty_on_hand = 5 WHERE id = $1")
        .bind(batch_id)
        .execute(&pool)
        .await
        .expect("concurrent inventory use should be simulated");
    repository
        .start_loss_order(&ctx, created.value.id, now, "sa-over-start-1")
        .await
        .expect("loss should start");

    let error = repository
        .execute_loss_order(&ctx, created.value.id, None, now, "sa-over-execute-1")
        .await
        .expect_err("execution above available stock must fail");
    assert_eq!(error, StockAdjustmentError::QuantityExceeded);

    let (quantity, status, movement_count): (Quantity, String, i64) = sqlx::query_as(
        r#"
        SELECT
          (SELECT qty_on_hand FROM inventory_batches WHERE id = $1),
          (SELECT status FROM stock_adjustment_orders WHERE id = $2),
          (SELECT COUNT(*) FROM inventory_movements WHERE source_document_id = $2)
        "#,
    )
    .bind(batch_id)
    .bind(created.value.id)
    .fetch_one(&pool)
    .await
    .expect("rollback state should load");
    assert_eq!(quantity, Quantity::from(5));
    assert_eq!(status, "in_progress");
    assert_eq!(movement_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn repeated_erp_reference_does_not_create_duplicate_loss_order(pool: PgPool) {
    let (owner_id, warehouse_id, _, batch_id, first_operator_id) =
        seed_loss_fixture(&pool, "none").await;
    let repository = PgStockAdjustmentRepository::new(pool.clone());
    let ctx = ctx(owner_id, first_operator_id);
    let now = Utc::now();
    let mut request = create_request(warehouse_id, batch_id, StockLossReason::Damaged);
    request.source = StockAdjustmentSource::Erp;
    request.external_ref = Some("ERP-LOSS-001".to_string());

    let (first, replay) = tokio::join!(
        repository.create_loss_order(&ctx, request.clone(), now, "sa-erp-create-1"),
        repository.create_loss_order(&ctx, request.clone(), now, "sa-erp-create-2"),
    );
    let first = first.expect("first concurrent ERP push should succeed");
    let replay = replay.expect("same concurrent ERP reference and payload should replay");
    assert_eq!(first.value.id, replay.value.id);
    assert!(first.replayed || replay.replayed);

    request.quantity = Quantity::from(5);
    let conflict = repository
        .create_loss_order(&ctx, request, now, "sa-erp-create-3")
        .await
        .expect_err("same ERP reference with changed payload must conflict");
    assert_eq!(conflict, StockAdjustmentError::IdempotencyConflict);
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM stock_adjustment_orders WHERE owner_id = $1 AND external_ref = 'ERP-LOSS-001'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("ERP order count should load");
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn stock_loss_api_requires_write_permission_and_idempotency_key(pool: PgPool) {
    let (owner_id, warehouse_id, _, batch_id, first_operator_id) =
        seed_loss_fixture(&pool, "none").await;
    let created = PgStockAdjustmentRepository::new(pool.clone())
        .create_loss_order(
            &ctx(owner_id, first_operator_id),
            create_request(warehouse_id, batch_id, StockLossReason::Damaged),
            Utc::now(),
            "sa-api-dynamic-route-create",
        )
        .await
        .expect("loss order should seed dynamic route test");
    let app = stock_adjustment_router(StockAdjustmentAppState::with_postgres(pool));
    let request_body = serde_json::to_vec(&create_request(
        warehouse_id,
        batch_id,
        StockLossReason::Damaged,
    ))
    .expect("request should serialize");
    let read_only_ctx = AuthContext {
        user_id: first_operator_id,
        owner_id,
        actor_name: "read-only".to_string(),
        permissions: vec!["msa.stock-adjustment.read".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    };
    let forbidden = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/stock-adjustments/loss-orders")
                .header("content-type", "application/json")
                .extension(read_only_ctx.clone())
                .body(Body::from(request_body.clone()))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/stock-adjustments/loss-orders/{}",
                    created.value.id
                ))
                .extension(read_only_ctx)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(detail.status(), StatusCode::OK);

    let missing_key = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/stock-adjustments/loss-orders")
                .header("content-type", "application/json")
                .extension(ctx(owner_id, first_operator_id))
                .body(Body::from(request_body))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(missing_key.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn cross_owner_cannot_read_stock_loss_order(pool: PgPool) {
    let (owner_id, warehouse_id, _, batch_id, first_operator_id) =
        seed_loss_fixture(&pool, "none").await;
    let repository = PgStockAdjustmentRepository::new(pool.clone());
    let created = repository
        .create_loss_order(
            &ctx(owner_id, first_operator_id),
            create_request(warehouse_id, batch_id, StockLossReason::Damaged),
            Utc::now(),
            "sa-cross-owner-create-1",
        )
        .await
        .expect("owner A loss should be created");
    let (other_owner_id, _, _, _, other_user_id) = seed_loss_fixture(&pool, "none").await;

    let error = repository
        .get_loss_order(&ctx(other_owner_id, other_user_id), created.value.id)
        .await
        .expect_err("owner B must not read owner A loss");
    assert_eq!(error, StockAdjustmentError::CrossOwner);
}
