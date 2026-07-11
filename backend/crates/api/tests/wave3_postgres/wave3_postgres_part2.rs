#[sqlx::test(migrations = "../../migrations")]
async fn receiving_receipt_is_single_closure_and_idempotent(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
        .single()
        .expect("valid time");
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;

    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-001", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create receiving order");
    repo.release_receiving_order(&ctx, order.id, now)
        .await
        .expect("release receiving order");

    let req = ReceiveReceivingOrderRequest {
        actual_qty: 8,
        shortage_qty: 2,
        rejected_qty: 0,
        arrival_temperature_celsius: Some(4.8),
        exception_note: None,
    };
    let first = repo
        .receive_receiving_order(&ctx, order.id, req.clone(), now, "idem-receive-1")
        .await
        .expect("first receive should insert");
    let replay = repo
        .receive_receiving_order(&ctx, order.id, req, now, "idem-receive-1")
        .await
        .expect("same idempotency key should replay first result");
    assert_eq!(first.id, replay.id);

    let conflict = repo
        .receive_receiving_order(
            &ctx,
            order.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 7,
                shortage_qty: 3,
                rejected_qty: 0,
                arrival_temperature_celsius: Some(4.8),
                exception_note: None,
            },
            now,
            "idem-receive-1",
        )
        .await;
    assert!(matches!(
        conflict,
        Err(Wave3RepositoryError::IdempotencyConflict)
    ));

    sqlx::query("UPDATE receiving_orders SET status = 'released' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("simulate invalid status rollback to verify unique receipt constraint");
    let duplicate = repo
        .receive_receiving_order(
            &ctx,
            order.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 8,
                shortage_qty: 2,
                rejected_qty: 0,
                arrival_temperature_celsius: None,
                exception_note: None,
            },
            now,
            "idem-receive-2",
        )
        .await
        .expect_err("a receiving order can only have one receipt closure");
    assert!(matches!(duplicate, Wave3RepositoryError::DuplicateReceipt));

    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM receiving_order_receipts WHERE receiving_order_id = $1",
    )
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("count receipts");
    assert_eq!(receipt_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn receiving_order_reject_closes_order_and_replays_idempotently(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 15, 0)
        .single()
        .expect("valid time");
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;

    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-REJECT-001", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create receiving order");
    repo.release_receiving_order(&ctx, order.id, now)
        .await
        .expect("release receiving order");

    let req = RejectReceivingOrderRequest {
        reason: "外包装严重破损，整单拒收".to_string(),
    };
    let first = repo
        .reject_receiving_order(&ctx, order.id, req.clone(), now, "idem-reject-1")
        .await
        .expect("first reject should insert");
    let replay = repo
        .reject_receiving_order(&ctx, order.id, req, now, "idem-reject-1")
        .await
        .expect("same idempotency key should replay first reject");
    assert_eq!(first.id, replay.id);

    let closed: (i64, i64, i64, Option<String>, String, i64) = sqlx::query_as(
        r#"
        SELECT
            receipt.actual_qty,
            receipt.shortage_qty,
            receipt.rejected_qty,
            receipt.exception_note,
            orders.status,
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2 AND idempotency_key = 'idem-reject-1')
          FROM receiving_order_receipts receipt
          JOIN receiving_orders orders ON orders.id = receipt.receiving_order_id
         WHERE receipt.receiving_order_id = $1
        "#,
    )
    .bind(order.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("closed reject row");
    assert_eq!(
        closed,
        (
            0,
            0,
            10,
            Some("外包装严重破损，整单拒收".to_string()),
            "closed_rejected".to_string(),
            1,
        )
    );

    let receiving_order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-REJECT-002", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create receiving status receiving order");
    repo.release_receiving_order(&ctx, receiving_order.id, now)
        .await
        .expect("release receiving status receiving order");
    sqlx::query("UPDATE receiving_orders SET status = 'receiving' WHERE id = $1 AND owner_id = $2")
        .bind(receiving_order.id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("mark order receiving");
    let receiving_reject = repo
        .reject_receiving_order(
            &ctx,
            receiving_order.id,
            RejectReceivingOrderRequest {
                reason: "收货中发现货损，整单拒收".to_string(),
            },
            now,
            "idem-reject-receiving",
        )
        .await
        .expect("receiving status order can be rejected");
    assert_eq!(receiving_reject.rejected_qty, 10);

    let draft = repo
        .create_receiving_order(&ctx, receiving_order_req("ASN-PG-REJECT-003"), now)
        .await
        .expect("create draft receiving order");
    let invalid = repo
        .reject_receiving_order(
            &ctx,
            draft.id,
            RejectReceivingOrderRequest {
                reason: "未放行不能拒收".to_string(),
            },
            now,
            "idem-reject-draft",
        )
        .await
        .expect_err("non released order cannot be rejected");
    assert!(matches!(
        invalid,
        Wave3RepositoryError::InvalidStatus { .. }
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_same_idempotency_key_replays_first_receipt(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 30, 0)
        .single()
        .expect("valid time");
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;

    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-RACE-001", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create receiving order");
    repo.release_receiving_order(&ctx, order.id, now)
        .await
        .expect("release receiving order");

    let req = ReceiveReceivingOrderRequest {
        actual_qty: 8,
        shortage_qty: 2,
        rejected_qty: 0,
        arrival_temperature_celsius: Some(4.8),
        exception_note: None,
    };
    let (left, right) = tokio::join!(
        repo.receive_receiving_order(&ctx, order.id, req.clone(), now, "idem-receive-race"),
        repo.receive_receiving_order(&ctx, order.id, req, now, "idem-receive-race"),
    );
    let left = left.expect("left request should succeed");
    let right = right.expect("right request should replay");

    assert_eq!(left.id, right.id);
    let counts: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM receiving_order_receipts WHERE receiving_order_id = $1),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2 AND idempotency_key = 'idem-receive-race')
        "#,
    )
    .bind(order.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_commits_receiving_inventory_and_movement_in_one_transaction(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 11, 0, 0)
        .single()
        .expect("valid time");

    let order = repo
        .create_receiving_order(&ctx, receiving_order_req("ASN-PG-002"), now)
        .await
        .expect("create receiving order");
    sqlx::query("UPDATE receiving_orders SET status = 'putaway' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare putaway state");

    let req = PutawayRequest {
        batch_no: "B202606".to_string(),
        product_code: "P-001".to_string(),
        qty: 10,
        location_id: Uuid::new_v4(),
        location_code: "A-01-01".to_string(),
        quality_status: STATUS_QUALIFIED.to_string(),
    };
    let first = repo
        .putaway_receiving_order_and_inventory_with_audit(
            &ctx,
            order.id,
            req.clone(),
            now,
            "idem-putaway-1",
            Some(wms_api::audit::AuditWriteRequest::from_auth_context(
                &ctx,
                "putaway",
                "M2",
                "receiving_order",
                order.id.to_string(),
                None,
            )),
        )
        .await
        .expect("putaway should commit");
    let replay = repo
        .putaway_receiving_order_and_inventory_with_audit(
            &ctx,
            order.id,
            req,
            now,
            "idem-putaway-1",
            Some(wms_api::audit::AuditWriteRequest::from_auth_context(
                &ctx,
                "putaway",
                "M2",
                "receiving_order",
                order.id.to_string(),
                None,
            )),
        )
        .await
        .expect("same idempotency key should replay");

    assert_eq!(first.value.putaway.id, replay.value.putaway.id);
    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(first.value.inventory_batch.qty_on_hand, 10);
    assert_eq!(first.value.inventory_movement.qty_delta, 10);

    let counts: (i64, i64, i64, String, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM receiving_putaways WHERE receiving_order_id = $1),
            (SELECT COUNT(*) FROM inventory_batches WHERE owner_id = $2),
            (SELECT COUNT(*) FROM inventory_movements WHERE owner_id = $2),
            (SELECT status FROM receiving_orders WHERE id = $1),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'putaway')
        "#,
    )
    .bind(order.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (1, 1, 1, "completed".to_string(), 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_audit_failure_rolls_back_all_business_writes(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 11, 30, 0)
        .single()
        .expect("valid time");
    let order = repo
        .create_receiving_order(&ctx, receiving_order_req("ASN-PG-ROLLBACK"), now)
        .await
        .expect("create order");
    sqlx::query("UPDATE receiving_orders SET status='putaway' WHERE id=$1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare putaway");
    let mut invalid_audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "putaway",
        "M2",
        "receiving_order",
        order.id.to_string(),
        None,
    );
    invalid_audit.occurred_at = Utc
        .with_ymd_and_hms(2099, 1, 1, 0, 0, 0)
        .single()
        .expect("valid future time");
    let result = repo
        .putaway_receiving_order_and_inventory_with_audit(
            &ctx,
            order.id,
            PutawayRequest {
                batch_no: "B202606".into(),
                product_code: "P-001".into(),
                qty: 10,
                location_id: Uuid::new_v4(),
                location_code: "A01-01-01-02".into(),
                quality_status: STATUS_QUALIFIED.into(),
            },
            now,
            "idem-putaway-rollback",
            Some(invalid_audit),
        )
        .await;
    assert!(
        matches!(result, Err(Wave3RepositoryError::Audit(_))),
        "unexpected result: {result:?}"
    );
    let counts: (i64, i64, i64, i64, String) = sqlx::query_as(r#"SELECT
        (SELECT COUNT(*) FROM receiving_putaways WHERE receiving_order_id=$1),
        (SELECT COUNT(*) FROM inventory_batches WHERE owner_id=$2),
        (SELECT COUNT(*) FROM inventory_movements WHERE owner_id=$2),
        (SELECT COUNT(*) FROM idempotency_request WHERE owner_id=$2 AND idempotency_key='idem-putaway-rollback'),
        (SELECT status FROM receiving_orders WHERE id=$1)"#)
        .bind(order.id).bind(owner_id).fetch_one(&pool).await.expect("rollback counts");
    assert_eq!(counts, (0, 0, 0, 0, "putaway".into()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn billing_rule_effective_window_rejects_overlap(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
        .single()
        .expect("valid time");

    let account = repo
        .create_billing_account(
            &ctx,
            CreateBillingAccountRequest {
                account_code: "OWNER-A-BILL".to_string(),
                account_name: "Owner A Billing".to_string(),
            },
            now,
        )
        .await
        .expect("account");
    let contract = repo
        .create_billing_contract(
            &ctx,
            CreateBillingContractRequest {
                account_id: account.id,
                contract_no: "CONTRACT-PG-001".to_string(),
                valid_from: "2026-06-01".to_string(),
                valid_to: "2027-05-31".to_string(),
            },
            now,
        )
        .await
        .expect("contract");

    repo.create_billing_rule(
        &ctx,
        CreateBillingRuleRequest {
            contract_id: contract.id,
            charge_item: "storage".to_string(),
            unit: "pallet_day".to_string(),
            unit_price_cents: 100,
            billing_cycle: "monthly".to_string(),
            effective_from: "2026-06-01".to_string(),
            effective_to: "2026-06-30".to_string(),
        },
        now,
    )
    .await
    .expect("first rule");

    let overlap = repo
        .create_billing_rule(
            &ctx,
            CreateBillingRuleRequest {
                contract_id: contract.id,
                charge_item: "storage".to_string(),
                unit: "pallet_day".to_string(),
                unit_price_cents: 110,
                billing_cycle: "monthly".to_string(),
                effective_from: "2026-06-15".to_string(),
                effective_to: "2026-07-15".to_string(),
            },
            now,
        )
        .await
        .expect_err("overlapping effective windows should be rejected");
    assert!(matches!(overlap, Wave3RepositoryError::BillingRuleConflict));
}

#[sqlx::test(migrations = "../../migrations")]
async fn receiving_order_survives_repository_and_pool_restart(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let options = pool.connect_options().as_ref().clone();
    let order_id = {
        let repo = PgWave3Repository::new(pool.clone());
        repo.create_receiving_order(&ctx, receiving_order_req("ASN-PG-RESTART-001"), Utc::now())
            .await
            .expect("create before restart")
            .id
    };

    pool.close().await;
    let restarted_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .expect("reconnect after repository process restart");
    let restarted_repo = PgWave3Repository::new(restarted_pool);
    let persisted = restarted_repo
        .get_receiving_order(&ctx, order_id)
        .await
        .expect("persisted order should be readable after restart");

    assert_eq!(persisted.receipt_no, "ASN-PG-RESTART-001");
    assert_eq!(persisted.owner_id, owner_id);
}
