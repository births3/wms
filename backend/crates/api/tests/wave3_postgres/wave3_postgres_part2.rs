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
        details: None,
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
                details: None,
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
                details: None,
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
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-REJECT-003", Some(supplier_id), warehouse_id),
            now,
        )
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
        details: None,
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
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;
    sqlx::query(
        "INSERT INTO system_dictionary_items (id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at) VALUES ($1, 'quality_color', 'qualified_owner_green', '货主合格绿', TRUE, $2, '{\"inventory_quality_status\": \"qualified\"}', 'owner', $3, $3)",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("owner quality-color override should persist");
    let (location_id, location_code) = seed_location(&pool, owner_id, warehouse_id).await;
    sqlx::query("UPDATE warehouse_zones SET quality_color = 'qualified_owner_green' WHERE owner_id = $1 AND warehouse_id = $2")
        .bind(owner_id)
        .bind(warehouse_id)
        .execute(&pool)
        .await
        .expect("owner quality-color override should apply to the zone");

    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-002", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create receiving order");
    sqlx::query("UPDATE receiving_orders SET status = 'released' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare released state");
    repo.receive_receiving_order_with_audit(
        &ctx,
        order.id,
        ReceiveReceivingOrderRequest {
            actual_qty: 10,
            shortage_qty: 0,
            rejected_qty: 0,
            arrival_temperature_celsius: None,
            exception_note: None,
            details: None,
        },
        now,
        "receive-putaway-commit",
        None,
    )
    .await
    .expect("receive before putaway inspection");
    repo.inspect_receiving_order_with_audit(
        &ctx,
        order.id,
        wms_domain::InspectReceivingOrderRequest {
            batch_no: "B202606".to_string(),
            accepted_qty: 10,
            rejected_qty: 0,
            production_date: "2026-01-01".to_string(),
            expiry_date: "2028-01-01".to_string(),
            quality_status: STATUS_QUALIFIED.to_string(),
            trace_codes: vec![],
        },
        now.date_naive(),
        now,
        "idem-putaway-inspect",
        None,
    )
    .await
    .expect("inspect before putaway");
    sqlx::query("UPDATE receiving_orders SET status = 'putaway' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare putaway state");

    let req = PutawayRequest {
        batch_no: "B202606".to_string(),
        product_code: "P-001".to_string(),
        qty: 10,
        location_id,
        location_code,
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
async fn inspection_accepts_quarantined_status_from_quality_color_dictionary(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 11, 15, 0)
        .single()
        .expect("valid time");
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;
    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-QUALITY-DICT", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create receiving order");
    sqlx::query("UPDATE receiving_orders SET status = 'released' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare released state");
    repo.receive_receiving_order_with_audit(
        &ctx,
        order.id,
        ReceiveReceivingOrderRequest {
            actual_qty: 10,
            shortage_qty: 0,
            rejected_qty: 0,
            arrival_temperature_celsius: None,
            exception_note: None,
            details: None,
        },
        now,
        "receive-quality-dict",
        None,
    )
    .await
    .expect("receive before quality inspection");

    repo.inspect_receiving_order_with_audit(
        &ctx,
        order.id,
        wms_domain::InspectReceivingOrderRequest {
            batch_no: "B-QUARANTINED".to_string(),
            accepted_qty: 0,
            rejected_qty: 10,
            production_date: "2026-01-01".to_string(),
            expiry_date: "2028-01-01".to_string(),
            quality_status: STATUS_QUARANTINED.to_string(),
            trace_codes: vec![],
        },
        now.date_naive(),
        now,
        "idem-quality-dict-quarantined",
        None,
    )
    .await
    .expect("quarantined status should resolve through quality-color dictionary");
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
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;
    let (location_id, location_code) = seed_location(&pool, owner_id, warehouse_id).await;
    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-ROLLBACK", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create order");
    sqlx::query("UPDATE receiving_orders SET status='released' WHERE id=$1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare released state");
    repo.receive_receiving_order_with_audit(
        &ctx,
        order.id,
        ReceiveReceivingOrderRequest {
            actual_qty: 10,
            shortage_qty: 0,
            rejected_qty: 0,
            arrival_temperature_celsius: None,
            exception_note: None,
            details: None,
        },
        now,
        "receive-putaway-rollback",
        None,
    )
    .await
    .expect("receive before rollback inspection");
    repo.inspect_receiving_order_with_audit(
        &ctx,
        order.id,
        wms_domain::InspectReceivingOrderRequest {
            batch_no: "B202606".into(),
            accepted_qty: 10,
            rejected_qty: 0,
            production_date: "2026-01-01".into(),
            expiry_date: "2028-01-01".into(),
            quality_status: STATUS_QUALIFIED.into(),
            trace_codes: vec![],
        },
        now.date_naive(),
        now,
        "idem-putaway-rollback-inspect",
        None,
    )
    .await
    .expect("inspect before rollback test");
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
                location_id,
                location_code,
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
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;
    let options = pool.connect_options().as_ref().clone();
    let order_id = {
        let repo = PgWave3Repository::new(pool.clone());
        repo.create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-RESTART-001", Some(supplier_id), warehouse_id),
            Utc::now(),
        )
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

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_batch_query_combines_filters_and_keeps_owner_scope(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    for (row_owner_id, batch_no, quality_status) in [
        (owner_id, "B-001", "qualified"),
        (owner_id, "B-002", "quarantined"),
        (other_owner_id, "B-001", "qualified"),
    ] {
        sqlx::query(
            "INSERT INTO inventory_batches (id, owner_id, product_code, batch_no, production_date, expiry_date, qty_on_hand, qty_locked, quality_status, location_id, location_code) VALUES ($1, $2, 'P-QUERY', $3, '2026-01-01', '2028-01-01', 10, 0, $4, $5, 'A-01-01')",
        )
        .bind(Uuid::new_v4())
        .bind(row_owner_id)
        .bind(batch_no)
        .bind(quality_status)
        .bind(location_id)
        .execute(&pool)
        .await
        .expect("seed inventory batch");
    }

    let ctx = ctx(owner_id);
    let rows = PgWave3Repository::new(pool)
        .list_inventory_batches_with_query(
            &ctx,
            InventoryBatchQuery {
                product_code: Some("P-QUERY".to_string()),
                batch_no: Some("B-001".to_string()),
                location_code: Some("A-01".to_string()),
                quality_status: Some("qualified".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("query inventory batches");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].owner_id, owner_id);
    assert_eq!(rows[0].batch_no, "B-001");
}
