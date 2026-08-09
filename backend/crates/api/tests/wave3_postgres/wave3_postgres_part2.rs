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
        actual_qty: 8.into(),
        shortage_qty: 2.into(),
        rejected_qty: 0.into(),
        arrival_temperature_celsius: Some(4.8),
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
                actual_qty: 7.into(),
                shortage_qty: 3.into(),
                rejected_qty: 0.into(),
                arrival_temperature_celsius: Some(4.8),
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
                actual_qty: 8.into(),
                shortage_qty: 2.into(),
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
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "reject",
        "M2",
        "receiving_order",
        order.id.to_string(),
        None,
    );
    let first = repo
        .reject_receiving_order_with_audit(
            &ctx,
            order.id,
            req.clone(),
            now,
            "idem-reject-1",
            Some(audit.clone()),
        )
        .await
        .expect("first reject should insert");
    let replay = repo
        .reject_receiving_order_with_audit(
            &ctx,
            order.id,
            req,
            now,
            "idem-reject-1",
            Some(audit),
        )
        .await
        .expect("same idempotency key should replay first reject");
    assert_eq!(first.value.id, replay.value.id);
    assert!(replay.replayed);

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
    let audit_diff: serde_json::Value = sqlx::query_scalar(
        "SELECT diff FROM audit_event WHERE owner_id = $1 AND action = 'reject' AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(order.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("reject audit diff");
    assert_eq!(
        audit_diff["after"]["reason"],
        "外包装严重破损，整单拒收"
    );
    assert_eq!(audit_diff["after"]["status"], "closed_rejected");
    assert_eq!(audit_diff["after"]["rejected_qty"], 10);

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
    assert_eq!(receiving_reject.rejected_qty, 10.into());

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
        actual_qty: 8.into(),
        shortage_qty: 2.into(),
        rejected_qty: 0.into(),
        arrival_temperature_celsius: Some(4.8),
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
            accepted_qty: 10.into(),
            rejected_qty: 0.into(),
            production_date: "2026-01-01".to_string(),
            expiry_date: "2028-01-01".to_string(),
            quality_status: STATUS_QUALIFIED.to_string(),
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
        qty: 10.into(),
        location_id,
        location_code,
        quality_status: STATUS_QUALIFIED.to_string(),
                lpn_code: None,
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
    assert_eq!(first.value.inventory_batch.qty_on_hand, 10.into());
    assert_eq!(first.value.inventory_movement.qty_delta, 10.into());

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

include!("wave3_postgres_part2b.rs");
