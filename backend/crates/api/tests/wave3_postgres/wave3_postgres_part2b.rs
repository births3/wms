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
            actual_qty: 10.into(),
            shortage_qty: 0.into(),
            rejected_qty: 0.into(),
            arrival_temperature_celsius: None,
            exception_note: None,
            details: Some(ReceivingReceiptDetails {
                    temperature_control_method: Some("普通".to_string()),
                    vehicle_no: Some("沪A00000".to_string()),
                    origin: Some("发运地".to_string()),
                    departure_at: Some(chrono::Utc::now()),
                    arrival_at: Some(chrono::Utc::now()),
                    storage_at: Some(chrono::Utc::now()),
                    transport_mode: Some("公路".to_string()),
                    carrier: Some("承运商".to_string()),
                    contact_name: Some("送货人".to_string()),
                    contact_phone: Some("13800000000".to_string()),
                    contact_id_no: Some("310101199001011234".to_string()),
                    seal_checked: Some("已核对".to_string()),
                    filing_checked: Some("已核对".to_string()),
                }),
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
            accepted_qty: 0.into(),
            rejected_qty: 10.into(),
            production_date: "2026-01-01".to_string(),
            expiry_date: "2028-01-01".to_string(),
            quality_status: STATUS_QUARANTINED.to_string(),
            trace_codes: vec![],

                appearance_check: Some("完好".to_string()),
                package_check: Some("完好".to_string()),
                instruction_check: Some("有".to_string()),
                label_check: Some("清晰".to_string()),
                sampling_qty: Some(1.into()),
                approval_no: None,
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
            actual_qty: 10.into(),
            shortage_qty: 0.into(),
            rejected_qty: 0.into(),
            arrival_temperature_celsius: None,
            exception_note: None,
            details: Some(ReceivingReceiptDetails {
                    temperature_control_method: Some("普通".to_string()),
                    vehicle_no: Some("沪A00000".to_string()),
                    origin: Some("发运地".to_string()),
                    departure_at: Some(chrono::Utc::now()),
                    arrival_at: Some(chrono::Utc::now()),
                    storage_at: Some(chrono::Utc::now()),
                    transport_mode: Some("公路".to_string()),
                    carrier: Some("承运商".to_string()),
                    contact_name: Some("送货人".to_string()),
                    contact_phone: Some("13800000000".to_string()),
                    contact_id_no: Some("310101199001011234".to_string()),
                    seal_checked: Some("已核对".to_string()),
                    filing_checked: Some("已核对".to_string()),
                }),
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
            accepted_qty: 10.into(),
            rejected_qty: 0.into(),
            production_date: "2026-01-01".into(),
            expiry_date: "2028-01-01".into(),
            quality_status: STATUS_QUALIFIED.into(),
            trace_codes: vec![],

                appearance_check: Some("完好".to_string()),
                package_check: Some("完好".to_string()),
                instruction_check: Some("有".to_string()),
                label_check: Some("清晰".to_string()),
                sampling_qty: Some(1.into()),
                approval_no: None,
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
                qty: 10.into(),
                location_id,
                location_code,
                quality_status: STATUS_QUALIFIED.into(),
                        lpn_code: None,
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
            unit_price_cents: 100.into(),
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
                unit_price_cents: 110.into(),
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
