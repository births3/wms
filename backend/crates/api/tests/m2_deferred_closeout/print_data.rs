#[sqlx::test(migrations = "../../migrations")]
async fn print_data_reads_receipt_inspection_and_dual_signature(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let first_signer_id = ctx.user_id;
    let second_signer_id = Uuid::new_v4();
    auth_support::seed_receiving_verifiers(&pool, owner_id, &[first_signer_id, second_signer_id])
        .await;
    let repository = PgWave3Repository::new(pool.clone());
    let mut create_request = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut create_request).await;
    let order = repository
        .create_receiving_order(&ctx, create_request, chrono::Utc::now())
        .await
        .expect("create print-data order");
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
                shortage_qty: 1.into(),
                rejected_qty: 1.into(),
                arrival_temperature_celsius: Some(4.5),
                exception_note: Some("外包装轻微破损".to_string()),
                details: Some(ReceivingReceiptDetails {
                    temperature_control_method: Some("冷藏".to_string()),
                    vehicle_no: Some("沪A-12345".to_string()),
                    origin: Some("上海配送中心".to_string()),
                    departure_at: Some("2026-06-04T08:00:00Z".parse().expect("departure")),
                    arrival_at: Some("2026-06-04T10:00:00Z".parse().expect("arrival")),
                    storage_at: Some("2026-06-04T10:15:00Z".parse().expect("storage")),
                    transport_mode: Some("冷藏车".to_string()),
                    carrier: Some("华东冷链承运商".to_string()),
                    contact_name: Some("张三".to_string()),
                    contact_phone: Some("13800000000".to_string()),
                    contact_id_no: Some("310101199001010000".to_string()),
                    seal_checked: Some("已核对".to_string()),
                    filing_checked: Some("已核对".to_string()),
                }),
            },
            chrono::Utc::now(),
            "print-data-receive",
            None,
        )
        .await
        .expect("receive print-data order")
        .value;
    assert_eq!(receipt.arrival_temperature_celsius, Some(4.5));
    assert_eq!(receipt.exception_note.as_deref(), Some("外包装轻微破损"));

    repository
        .inspect_receiving_order_with_audit(
            &ctx,
            order.id,
            InspectReceivingOrderRequest {
                batch_no: "B-PRINT-001".to_string(),
                accepted_qty: 7.into(),
                rejected_qty: 1.into(),
                production_date: "2026-01-01".to_string(),
                expiry_date: "2028-01-01".to_string(),
                quality_status: STATUS_QUALIFIED.to_string(),
                trace_codes: vec!["TRACE-PRINT-001".to_string()],

                appearance_check: Some("完好".to_string()),
                package_check: Some("完好".to_string()),
                instruction_check: Some("有".to_string()),
                label_check: Some("清晰".to_string()),
                sampling_qty: Some(1.into()),
                approval_no: None,
            },
            chrono::NaiveDate::from_ymd_opt(2026, 7, 12).expect("valid date"),
            chrono::Utc::now(),
            "print-data-inspect",
            None,
        )
        .await
        .expect("inspect print-data order");
    repository
        .sign_receiving_order_with_audit(
            &ctx,
            order.id,
            SignInspectionRequest {
                first_signer_id,
                second_signer_id: None,
                dual_required: true,
            },
            chrono::Utc::now(),
            "print-data-sign-first",
            None,
        )
        .await
        .expect("first sign print-data order");
    let mut second_ctx = ctx.clone();
    second_ctx.user_id = second_signer_id;
    repository
        .sign_receiving_order_with_audit(
            &second_ctx,
            order.id,
            SignInspectionRequest {
                first_signer_id,
                second_signer_id: Some(second_signer_id),
                dual_required: true,
            },
            chrono::Utc::now(),
            "print-data-sign-second",
            None,
        )
        .await
        .expect("second sign print-data order");

    let print_data = repository
        .get_receiving_order_print_data(&ctx, order.id)
        .await
        .expect("read print data");
    assert_eq!(print_data.order.id, order.id);
    assert_eq!(print_data.receipts.len(), 1);
    let details = print_data.receipts[0]
        .details
        .as_ref()
        .expect("receipt details");
    assert_eq!(details.temperature_control_method.as_deref(), Some("冷藏"));
    assert_eq!(details.vehicle_no.as_deref(), Some("沪A-12345"));
    assert_eq!(details.carrier.as_deref(), Some("华东冷链承运商"));
    assert_eq!(
        details.departure_at.expect("departure").to_rfc3339(),
        "2026-06-04T08:00:00+00:00"
    );
    assert_eq!(
        details.arrival_at.expect("arrival").to_rfc3339(),
        "2026-06-04T10:00:00+00:00"
    );
    assert_eq!(print_data.receipts[0].actual_qty, 8.into());
    assert_eq!(print_data.inspections[0].batch_no, "B-PRINT-001");
    assert_eq!(print_data.signatures[0].first_signer_id, first_signer_id);
    assert_eq!(
        print_data.signatures[0].second_signer_id,
        Some(second_signer_id)
    );
}
