#[sqlx::test(migrations = "../../migrations")]
async fn outbound_order_generates_number_when_request_omits_wms_number(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 8, 0, 0)
        .single()
        .expect("valid time");
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'OUTOWNER', '出库编号测试货主')",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("owner should be seeded");
    sqlx::query(
        r#"
        INSERT INTO document_number_rules (
            id, owner_id, document_type, rule_code, rule_name, template,
            reset_policy, sequence_width, enabled, created_at, updated_at
        ) VALUES ($1, $2, 'sales_outbound', 'outbound-order-test', '出库订单测试规则',
                  '{OWNER}-OUT-{YYYY}{MM}{DD}-{SEQ}', 'daily', 4, TRUE, $3, $3)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("outbound numbering rule should be seeded");
    let (customer_id, delivery_address_id) =
        seed_customer_delivery_address(&pool, owner_id).await;

    let order = PgWave4Repository::new(pool.clone())
        .create_outbound_order(
            &ctx,
            CreateOutboundOrderRequest {
                document_type: "sales_outbound".to_string(),
                wms_order_no: String::new(),
                erp_order_no: Some("ERP-OUT-001".to_string()),
                customer_id,
                delivery_address_id,
                warehouse_id: Uuid::new_v4(),
                required_ship_at: Some(now),
                lines: vec![CreateOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: "P-OUT-NUMBER".to_string(),
                    batch_no: "B-OUT-NUMBER".to_string(),
                    planned_qty: 1,
                }],
            },
            now,
            "outbound-numbering-1",
            None,
        )
        .await
        .expect("outbound order should generate a number")
        .value;

    assert_eq!(order.wms_order_no, "OUTOWNER-OUT-20260605-0001");
    let allocation_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM document_number_allocations WHERE owner_id = $1 AND source_document_id = $2",
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("number allocation should be stored");
    assert_eq!(allocation_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_order_rejects_non_outbound_document_type(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'OUTOWNER2', '出库单据类型测试货主')",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("owner should be seeded");

    let result = PgWave4Repository::new(pool).create_outbound_order(
        &ctx(owner_id),
        CreateOutboundOrderRequest {
            document_type: "purchase_inbound".to_string(),
            wms_order_no: "OUT-INVALID-TYPE".to_string(),
            erp_order_no: None,
            customer_id: Uuid::new_v4(),
            delivery_address_id: Uuid::new_v4(),
            warehouse_id: Uuid::new_v4(),
            required_ship_at: None,
            lines: vec![CreateOutboundOrderLineRequest {
                line_no: 1,
                product_code: "P-OUT-INVALID".to_string(),
                batch_no: "B-OUT-INVALID".to_string(),
                planned_qty: 1,
            }],
        },
        Utc::now(),
        "outbound-invalid-type-1",
        None,
    )
    .await
    .expect_err("inbound document type must be rejected");

    assert_eq!(result, Wave4RepositoryError::InvalidDocumentType);
}
