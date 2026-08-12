#[sqlx::test(migrations = "../../migrations")]
async fn sales_return_batch_rejection_keeps_unrejected_quantities_inspectable(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let mut sales_return = request(RECEIVING_DOCUMENT_TYPE_SALES_RETURN, Some("B-001"));
    sales_return.lines.push(ReceivingOrderLine {
        line_no: 2,
        product_id: None,
        product_code: "P-M2-001".to_string(),
        expected_qty: 6.into(),
        batch_no: Some("B-002".to_string()),
        production_date: None,
        expiry_date: None,
    });
    seed_asn_references(&pool, owner_id, &mut sales_return).await;
    let order = repository
        .create_receiving_order(&ctx, sales_return, chrono::Utc::now())
        .await
        .expect("create multi-batch sales return");
    sqlx::query("UPDATE receiving_orders SET status = 'released' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare released state");
    let receipt = repository
        .receive_receiving_order_with_audit(
            &ctx,
            order.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 8.into(),
                shortage_qty: 6.into(),
                rejected_qty: 2.into(),
                arrival_temperature_celsius: None,
                exception_note: Some("B-002 外包装破损".to_string()),
                details: Some(ReceivingReceiptDetails {
                    delivery_qty: 10.into(),
                    second_receiver_id: None,
                    sales_return_batches: vec![
                        wms_domain::SalesReturnReceivingBatch {
                            batch_no: "B-001".to_string(),
                            quantity: 4.into(),
                            rejected_qty: 0.into(),
                            reject_reason: None,
                        },
                        wms_domain::SalesReturnReceivingBatch {
                            batch_no: "B-002".to_string(),
                            quantity: 6.into(),
                            rejected_qty: 2.into(),
                            reject_reason: Some("外包装破损".to_string()),
                        },
                    ],
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
            chrono::Utc::now(),
            "receive-sales-return-batches",
            None,
        )
        .await
        .expect("receive multi-batch sales return")
        .value;
    let details = receipt.details.expect("receipt details");
    assert_eq!(details.sales_return_batches[1].rejected_qty, 2.into());

    for (batch_no, qty, key) in [
        ("B-001", 4_i64, "inspect-b-001"),
        ("B-002", 4_i64, "inspect-b-002"),
    ] {
        repository
            .inspect_receiving_order_with_audit(
                &ctx,
                order.id,
                InspectReceivingOrderRequest {
                    batch_no: batch_no.to_string(),
                    accepted_qty: qty.into(),
                    rejected_qty: 0.into(),
                    production_date: "2026-01-01".to_string(),
                    expiry_date: "2028-01-01".to_string(),
                    quality_status: STATUS_QUALIFIED.to_string(),
                    trace_codes: vec![format!("TRACE-{batch_no}")],
                    appearance_check: Some("完好".to_string()),
                    package_check: Some("完好".to_string()),
                    instruction_check: Some("有".to_string()),
                    label_check: Some("清晰".to_string()),
                    sampling_qty: Some(1.into()),
                    approval_no: None,
                },
                chrono::NaiveDate::from_ymd_opt(2026, 7, 12).expect("valid date"),
                chrono::Utc::now(),
                key,
                None,
            )
            .await
            .expect("inspect each sales-return batch");
    }

    let batches: Vec<(i32, String, String)> = sqlx::query_as(
        "SELECT line_no, batch_no, expiry_date::TEXT FROM receiving_order_lines WHERE receiving_order_id = $1 ORDER BY line_no",
    )
    .bind(order.id)
    .fetch_all(&pool)
    .await
    .expect("read inspected lines");
    assert_eq!(
        batches,
        vec![
            (1, "B-001".to_string(), "2028-01-01".to_string()),
            (2, "B-002".to_string(), "2028-01-01".to_string())
        ]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_is_partial_until_all_accepted_quantity_is_committed(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let (warehouse_id, location_id) = seed_putaway_location(&pool, owner_id).await;
    let mut purchase = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut purchase).await;
    purchase.warehouse_id = warehouse_id;
    let order = repository
        .create_receiving_order(&ctx, purchase, chrono::Utc::now())
        .await
        .expect("create purchase inbound");
    sqlx::query("UPDATE receiving_orders SET status = 'released' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare released state");
    repository
        .receive_receiving_order_with_audit(
            &ctx,
            order.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 10.into(),
                shortage_qty: 0.into(),
                rejected_qty: 0.into(),
                arrival_temperature_celsius: None,
                exception_note: None,
                details: Some(ReceivingReceiptDetails {
                    delivery_qty: 10.into(),
                    second_receiver_id: None,
                    sales_return_batches: vec![],
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
            chrono::Utc::now(),
            "receive-putaway",
            None,
        )
        .await
        .expect("receive purchase inbound");
    repository
        .inspect_receiving_order_with_audit(
            &ctx,
            order.id,
            InspectReceivingOrderRequest {
                batch_no: "B-PUTAWAY-001".to_string(),
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
            chrono::NaiveDate::from_ymd_opt(2026, 7, 12).expect("valid date"),
            chrono::Utc::now(),
            "inspect-putaway",
            None,
        )
        .await
        .expect("inspect putaway batch");
    sqlx::query("UPDATE receiving_orders SET status = 'putaway' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("restore putaway state after inspection");

    for (qty, key) in [(6_i64, "putaway-part-1"), (4_i64, "putaway-part-2")] {
        repository
            .putaway_receiving_order_and_inventory_with_audit(
                &ctx,
                order.id,
                PutawayRequest {
                    batch_no: "B-PUTAWAY-001".to_string(),
                    product_code: "P-M2-001".to_string(),
                    qty: qty.into(),
                    location_id,
                    location_code: sqlx::query_scalar(
                        "SELECT location_code FROM warehouse_locations WHERE id = $1",
                    )
                    .bind(location_id)
                    .fetch_one(&pool)
                    .await
                    .expect("read location code"),
                    quality_status: STATUS_QUALIFIED.to_string(),
                    lpn_code: None,
                },
                chrono::Utc::now(),
                key,
                None,
            )
            .await
            .expect("putaway partial quantity");
    }

    let status: String = sqlx::query_scalar("SELECT status FROM receiving_orders WHERE id = $1")
        .bind(order.id)
        .fetch_one(&pool)
        .await
        .expect("read final status");
    assert_eq!(status, "completed");
    let putaway_qty: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(qty), 0)::BIGINT FROM receiving_putaways WHERE receiving_order_id = $1",
    )
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("read putaway total");
    assert_eq!(putaway_qty, 10);
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_is_idempotent_and_audited_in_the_postgres_path(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    seed_numbering_rule(&pool, owner_id).await;
    let mut req = request_with_receipt_no(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None, Some(""));
    seed_asn_references(&pool, owner_id, &mut req).await;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create",
        "M2",
        "receiving_order",
        "pending",
        None,
    );
    let first = repository
        .create_receiving_order_with_audit(
            &ctx,
            req.clone(),
            chrono::Utc::now(),
            "create-idempotency",
            audit.clone(),
        )
        .await
        .expect("create ASN");
    let replay = repository
        .create_receiving_order_with_audit(
            &ctx,
            req,
            chrono::Utc::now(),
            "create-idempotency",
            audit,
        )
        .await
        .expect("replay create ASN");
    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(first.value.id, replay.value.id);
    assert!(first.value.receipt_no.starts_with("ASN-M2OWNER-"));
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM receiving_orders WHERE owner_id = $1), (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'create')",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("read create idempotency evidence");
    assert_eq!(counts, (1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn h8_sales_return_create_replays_without_duplicate_order_or_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let mut req = request(
        RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
        Some("B-H8-RETURN-001"),
    );
    seed_asn_references(&pool, owner_id, &mut req).await;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create",
        "M2",
        "receiving_order",
        "pending",
        None,
    );

    let first = repository
        .create_receiving_order_with_audit(
            &ctx,
            req.clone(),
            chrono::Utc::now(),
            "h8-sales-return-1",
            audit.clone(),
        )
        .await
        .expect("sales return should be created");
    let replayed = repository
        .create_receiving_order_with_audit(
            &ctx,
            req,
            chrono::Utc::now(),
            "h8-sales-return-1",
            audit,
        )
        .await
        .expect("sales return should replay");

    assert!(!first.replayed);
    assert!(replayed.replayed);
    assert_eq!(replayed.value.id, first.value.id);
    let evidence: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM receiving_orders WHERE owner_id = $1 AND document_type = 'sales_return'), (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'create'), (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'h8-sales-return-1')",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("sales return evidence should query");
    assert_eq!(evidence, (1, 1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn dashboard_groups_real_postgres_receiving_statuses(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let mut first_request = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut first_request).await;
    let first = repository
        .create_receiving_order(&ctx, first_request, chrono::Utc::now())
        .await
        .expect("create first dashboard order");
    let mut second_request = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut second_request).await;
    let second = repository
        .create_receiving_order(&ctx, second_request, chrono::Utc::now())
        .await
        .expect("create second dashboard order");
    sqlx::query("UPDATE receiving_orders SET status = 'closed_rejected' WHERE id = $1")
        .bind(second.id)
        .execute(&pool)
        .await
        .expect("mark abnormal order");

    let rows = repository
        .list_receiving_dashboard(&ctx, &ReceivingDashboardQuery::default())
        .await
        .expect("read dashboard");
    assert!(rows
        .iter()
        .any(|row| row.status == "draft" && row.order_count == 1));
    let abnormal = rows
        .iter()
        .find(|row| row.status == "closed_rejected")
        .expect("abnormal dashboard row");
    assert!(abnormal.abnormal);
    assert_eq!(abnormal.expected_qty, 10.into());
    assert_ne!(first.id, second.id);
}

include!("../m2_deferred_closeout/print_data.rs");
include!("../m2_deferred_closeout/closeout_actions.rs");
