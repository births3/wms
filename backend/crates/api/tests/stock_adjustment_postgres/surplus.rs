fn create_surplus_request(warehouse_id: Uuid, batch_id: Uuid) -> CreateStockSurplusOrderRequest {
    CreateStockSurplusOrderRequest {
        warehouse_id,
        batch_id,
        quantity: 3,
        reason: StockSurplusReason::InventorySurplus,
        source: StockAdjustmentSource::Manual,
        external_ref: None,
        requires_quality_approval: false,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn manual_surplus_is_numbered_executed_atomically_audited_and_idempotent(pool: PgPool) {
    let (owner_id, warehouse_id, _, batch_id, first_operator_id) =
        seed_loss_fixture(&pool, "none").await;
    let repository = PgStockAdjustmentRepository::new(pool.clone());
    let ctx = ctx(owner_id, first_operator_id);
    let now = Utc::now();
    let request = create_surplus_request(warehouse_id, batch_id);

    let created = repository
        .create_surplus_order(&ctx, request.clone(), now, "sa-surplus-create-1")
        .await
        .expect("manual surplus should be created");
    let replay = repository
        .create_surplus_order(&ctx, request.clone(), now, "sa-surplus-create-1")
        .await
        .expect("same surplus create request should replay");
    assert_eq!(created.value.id, replay.value.id);
    assert!(replay.replayed);
    sqlx::query(
        "UPDATE idempotency_request SET method = 'PATCH', path = '/wrong-path' WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_id)
    .bind("sa-surplus-create-1")
    .execute(&pool)
    .await
    .expect("idempotency metadata should be mutable for the regression check");
    let metadata_conflict = repository
        .create_surplus_order(&ctx, request, now, "sa-surplus-create-1")
        .await
        .expect_err("method and path changes must invalidate a replay");
    assert_eq!(metadata_conflict, StockAdjustmentError::IdempotencyConflict);
    assert!(created.value.order_no.starts_with("BY"));
    assert_eq!(created.value.status.as_str(), "pending_execution");

    repository
        .start_surplus_order(&ctx, created.value.id, now, "sa-surplus-start-1")
        .await
        .expect("pending surplus should start");
    let completed = repository
        .execute_surplus_order(&ctx, created.value.id, None, now, "sa-surplus-execute-1")
        .await
        .expect("single-person surplus should complete");
    let execute_replay = repository
        .execute_surplus_order(&ctx, created.value.id, None, now, "sa-surplus-execute-1")
        .await
        .expect("same surplus execution should replay");
    assert_eq!(completed.value.id, execute_replay.value.id);
    assert!(execute_replay.replayed);
    assert_eq!(completed.value.status.as_str(), "completed");

    let (
        qty_on_hand,
        used_volume,
        movement_count,
        movement_delta,
        execution_process,
        outbox_count,
        execute_audit_count,
        execute_idempotency_count,
    ): (i64, i64, i64, i64, String, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
          (SELECT qty_on_hand FROM inventory_batches WHERE id = $1),
          (SELECT location.used_volume_cm3 FROM warehouse_locations location JOIN inventory_batches batch ON batch.location_id = location.id WHERE batch.id = $1),
          (SELECT COUNT(*) FROM inventory_movements WHERE source_document_id = $2 AND movement_type = 'stock_surplus'),
          (SELECT qty_delta FROM inventory_movements WHERE source_document_id = $2 AND movement_type = 'stock_surplus'),
          (SELECT process_code FROM stock_adjustment_execution_records WHERE order_id = $2),
          (SELECT COUNT(*) FROM stock_adjustment_erp_feedback_outbox WHERE order_id = $2 AND event_type = 'stock_surplus_completed' AND payload->>'warehouse_id' = $4::TEXT),
          (SELECT COUNT(*) FROM audit_event WHERE owner_id = $3 AND action = 'execute_stock_surplus_order' AND resource_id = $2::TEXT),
          (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $3 AND idempotency_key = 'sa-surplus-execute-1')
        "#,
    )
    .bind(batch_id)
    .bind(created.value.id)
    .bind(owner_id)
    .bind(warehouse_id)
    .fetch_one(&pool)
    .await
    .expect("surplus evidence should load");
    assert_eq!(qty_on_hand, 13);
    assert_eq!(used_volume, 1300);
    assert_eq!(movement_count, 1);
    assert_eq!(movement_delta, 3);
    assert_eq!(execution_process, "报溢");
    assert_eq!(outbox_count, 1);
    assert_eq!(execute_audit_count, 1);
    assert_eq!(execute_idempotency_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn surplus_putaway_rule_mismatch_rolls_back_inventory_and_location(pool: PgPool) {
    let (owner_id, warehouse_id, _, batch_id, first_operator_id) =
        seed_loss_fixture(&pool, "none").await;
    let repository = PgStockAdjustmentRepository::new(pool.clone());
    let ctx = ctx(owner_id, first_operator_id);
    let now = Utc::now();
    let created = repository
        .create_surplus_order(
            &ctx,
            create_surplus_request(warehouse_id, batch_id),
            now,
            "sa-surplus-mismatch-create",
        )
        .await
        .expect("surplus should be created before location changes");
    repository
        .start_surplus_order(&ctx, created.value.id, now, "sa-surplus-mismatch-start")
        .await
        .expect("surplus should start");
    sqlx::query(
        "UPDATE warehouse_zones SET temperature_zone = 'cold' WHERE owner_id = $1 AND warehouse_id = $2",
    )
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await
    .expect("temperature mismatch should seed");

    let error = repository
        .execute_surplus_order(
            &ctx,
            created.value.id,
            None,
            now,
            "sa-surplus-mismatch-execute",
        )
        .await
        .expect_err("temperature mismatch must block surplus putaway");
    assert_eq!(error, StockAdjustmentError::InvalidPutawayTarget);

    let (quantity, used_volume, status, movement_count): (i64, i64, String, i64) =
        sqlx::query_as(
            r#"
            SELECT
              (SELECT qty_on_hand FROM inventory_batches WHERE id = $1),
              (SELECT location.used_volume_cm3 FROM warehouse_locations location JOIN inventory_batches batch ON batch.location_id = location.id WHERE batch.id = $1),
              (SELECT status FROM stock_adjustment_orders WHERE id = $2),
              (SELECT COUNT(*) FROM inventory_movements WHERE source_document_id = $2)
            "#,
        )
        .bind(batch_id)
        .bind(created.value.id)
        .fetch_one(&pool)
        .await
        .expect("rollback evidence should load");
    assert_eq!(quantity, 10);
    assert_eq!(used_volume, 1000);
    assert_eq!(status, "in_progress");
    assert_eq!(movement_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn surplus_quality_approval_and_dual_person_policy_are_enforced(pool: PgPool) {
    let (owner_id, warehouse_id, _, batch_id, first_operator_id) =
        seed_loss_fixture(&pool, "narcotic").await;
    let second_operator_id = Uuid::new_v4();
    seed_operator(&pool, owner_id, second_operator_id, "custodian").await;
    let repository = PgStockAdjustmentRepository::new(pool.clone());
    let ctx = ctx(owner_id, first_operator_id);
    let now = Utc::now();
    let mut request = create_surplus_request(warehouse_id, batch_id);
    request.requires_quality_approval = true;
    let created = repository
        .create_surplus_order(&ctx, request, now, "sa-surplus-approval-create")
        .await
        .expect("surplus requiring approval should be created");
    assert_eq!(created.value.status.as_str(), "pending_approval");

    let approved = repository
        .record_surplus_quality_approval(
            &ctx,
            created.value.id,
            "QL-SURPLUS-001",
            true,
            now,
            "sa-surplus-quality-approval",
        )
        .await
        .expect("quality approval should release surplus");
    let approval_replay = repository
        .record_surplus_quality_approval(
            &ctx,
            created.value.id,
            "QL-SURPLUS-001",
            true,
            now,
            "sa-surplus-quality-approval",
        )
        .await
        .expect("same quality approval should replay");
    assert!(approval_replay.replayed);
    assert_eq!(approved.value.status.as_str(), "pending_execution");
    let approval_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event
         WHERE owner_id = $1
           AND action = 'record_stock_surplus_quality_approval'
           AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(created.value.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("surplus quality approval audit should query");
    assert_eq!(approval_audit_count, 1);
    repository
        .start_surplus_order(
            &ctx,
            created.value.id,
            now,
            "sa-surplus-approval-start",
        )
        .await
        .expect("approved surplus should start");

    let missing_second = repository
        .execute_surplus_order(
            &ctx,
            created.value.id,
            None,
            now,
            "sa-surplus-missing-second",
        )
        .await
        .expect_err("special drug surplus must require second operator");
    assert_eq!(missing_second, StockAdjustmentError::MissingSecondOperator);

    let h4_approval_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO h4_approval_records (
            id, owner_id, scenario, business_ref, dedupe_key, approver_user,
            process_id, callback_path, summary, status, approved_by, approved_at,
            created_at, updated_at
        )
        VALUES ($1, $2, 'mvr.dual_person', $3, 'sa-surplus-h4-1', 'warehouse-manager',
                'PROC-SA-SURPLUS-001', '/api/v1/stock-adjustments/surplus-orders/callback',
                '报溢双人策略审批', 'approved', 'warehouse-manager', $4, $4, $4)
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
        .execute_surplus_order(
            &ctx,
            created.value.id,
            Some(second_operator_id),
            now,
            "sa-surplus-approved-execute",
        )
        .await
        .expect("approved dual-person surplus should complete");
    assert_eq!(completed.value.second_operator_id, Some(second_operator_id));
    assert_eq!(completed.value.approval_record_id, Some(h4_approval_id));
    assert!(completed.value.source_rule_id.is_some());

    let (source, approval_id, process, node): (String, String, String, String) = sqlx::query_as(
        r#"
        SELECT movement.approval_source, movement.approval_id,
               execution.process_code, execution.node_code
          FROM inventory_movements movement
          JOIN stock_adjustment_execution_records execution
            ON execution.order_id = movement.source_document_id
         WHERE movement.source_document_id = $1
        "#,
    )
    .bind(created.value.id)
    .fetch_one(&pool)
    .await
    .expect("surplus approval evidence should load");
    assert_eq!(source, "质量联系单");
    assert_eq!(approval_id, "QL-SURPLUS-001");
    assert_eq!(process, "报溢");
    assert_eq!(node, "报溢执行");
}

#[sqlx::test(migrations = "../../migrations")]
async fn stock_surplus_api_requires_write_permission_and_idempotency_key(pool: PgPool) {
    let (owner_id, warehouse_id, _, batch_id, first_operator_id) =
        seed_loss_fixture(&pool, "none").await;
    let app = stock_adjustment_router(StockAdjustmentAppState::with_postgres(pool));
    let request_body = serde_json::to_vec(&create_surplus_request(warehouse_id, batch_id))
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
                .uri("/api/v1/stock-adjustments/surplus-orders")
                .header("content-type", "application/json")
                .extension(read_only_ctx.clone())
                .body(Body::from(request_body.clone()))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let missing_key = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/stock-adjustments/surplus-orders")
                .header("content-type", "application/json")
                .extension(ctx(owner_id, first_operator_id))
                .body(Body::from(request_body))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(missing_key.status(), StatusCode::BAD_REQUEST);

    let order_id = Uuid::new_v4();
    let forbidden_approval = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/stock-adjustments/surplus-orders/{order_id}/quality-approval"
                ))
                .header("content-type", "application/json")
                .header("idempotency-key", "sa-surplus-forbidden-approval")
                .extension(read_only_ctx.clone())
                .body(Body::from(
                    r#"{"quality_liaison_id":"QL-FORBIDDEN","approved":true}"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(forbidden_approval.status(), StatusCode::FORBIDDEN);

    let forbidden_execute = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/stock-adjustments/surplus-orders/{order_id}/start"
                ))
                .header("idempotency-key", "sa-surplus-forbidden-start")
                .extension(read_only_ctx)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(forbidden_execute.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn repeated_erp_surplus_reference_replays_and_cross_owner_read_is_forbidden(pool: PgPool) {
    let (owner_id, warehouse_id, _, batch_id, first_operator_id) =
        seed_loss_fixture(&pool, "none").await;
    let repository = PgStockAdjustmentRepository::new(pool.clone());
    let auth = ctx(owner_id, first_operator_id);
    let now = Utc::now();
    let mut request = create_surplus_request(warehouse_id, batch_id);
    request.source = StockAdjustmentSource::Erp;
    request.external_ref = Some("ERP-SURPLUS-001".to_string());

    let (first, replay) = tokio::join!(
        repository.create_surplus_order(
            &auth,
            request.clone(),
            now,
            "sa-erp-surplus-create-1"
        ),
        repository.create_surplus_order(
            &auth,
            request.clone(),
            now,
            "sa-erp-surplus-create-2"
        ),
    );
    let first = first.expect("first ERP surplus should succeed");
    let replay = replay.expect("same concurrent ERP surplus should replay");
    assert_eq!(first.value.id, replay.value.id);
    assert!(first.replayed || replay.replayed);

    request.quantity = 4;
    let conflict = repository
        .create_surplus_order(&auth, request, now, "sa-erp-surplus-create-3")
        .await
        .expect_err("same ERP reference with changed payload must conflict");
    assert_eq!(conflict, StockAdjustmentError::IdempotencyConflict);

    let (other_owner_id, _, _, _, other_user_id) = seed_loss_fixture(&pool, "none").await;
    let cross_owner = repository
        .get_surplus_order(&ctx(other_owner_id, other_user_id), first.value.id)
        .await
        .expect_err("another owner must not read surplus order");
    assert_eq!(cross_owner, StockAdjustmentError::CrossOwner);
}
