#[sqlx::test(migrations = "../../migrations")]
async fn outbound_ship_rolls_back_all_side_effects(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let shipping_service = Wave4ShippingService::new(Arc::new(repo.clone()));
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 10, 0, 0)
        .single()
        .expect("valid time");
    let order = create_read_order(
        &pool,
        &repo,
        &ctx,
        "WMS-R-ROLLBACK-001",
        "ERP-R-ROLLBACK-001",
        now,
    )
    .await;
    let product_code = "P-WMS-R-ROLLBACK-001";
    let batch_no = "B-WMS-R-ROLLBACK-001";
    seed_outbound_inventory(&pool, owner_id, product_code, batch_no, 6, now).await;
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, special_drug_category, status) VALUES ($1, $2, $3, '发运回滚商品', '1 unit', 'normal_10_30', 'none', 'active')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(product_code)
    .execute(&pool)
    .await
    .expect("seed rollback product");
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '回滚司机', 'test-hash', 'active')",
    )
    .bind(ctx.user_id)
    .bind(format!("m4-rollback-driver-{}", &ctx.user_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed rollback driver");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(ctx.user_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("bind rollback driver");
    sqlx::query(
        "UPDATE outbound_orders SET status = 'reviewed', short_pick = FALSE, updated_at = $3 WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(order.id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("mark rollback order reviewed");
    sqlx::query(
        "UPDATE outbound_order_lines SET picked_qty = planned_qty, reviewed_qty = planned_qty WHERE owner_id = $1 AND outbound_order_id = $2",
    )
    .bind(owner_id)
    .bind(order.id)
    .execute(&pool)
    .await
    .expect("mark rollback lines reviewed");
    sqlx::query(
        r#"
        INSERT INTO outbound_shipments (
            id, owner_id, outbound_order_id, delivery_provider_type,
            vehicle_no, plate_no, driver_user_id, driver_name,
            cold_chain, cold_chain_packages, package_count,
            handover_by, shipped_at, created_at
        )
        VALUES ($1, $2, $3, 'own_fleet', 'ROLLBACK-VEHICLE', '沪A00001', $4, '回滚司机',
                FALSE, '[]'::jsonb, 1, $4, $5, $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(order.id)
    .bind(ctx.user_id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed duplicate shipment to force late failure");

    let error = shipping_service
        .ship_outbound_order(
            &ctx,
            order.id,
            ShipOutboundOrderRequest {
                delivery_provider_type: "own_fleet".to_string(),
                vehicle_no: Some("ROLLBACK-VEHICLE-2".to_string()),
                plate_no: "沪A00002".to_string(),
                driver_user_id: Some(ctx.user_id),
                courier_name: None,
                courier_phone: None,
                signature_attachment_id: None,
                loading_temperature_celsius: None,
                cold_chain_packages: Vec::new(),
                package_count: 1,
            },
            now,
            "outbound-ship-rollback-1",
        )
        .await
        .expect_err("duplicate shipment should fail inside transaction");
    assert!(matches!(error, Wave4RepositoryError::DuplicateCode));

    let state: (String, wms_domain::Quantity, wms_domain::Quantity, i64, i64, i64) =
        sqlx::query_as(
        r#"
        SELECT
            (SELECT status FROM outbound_orders WHERE owner_id = $1 AND id = $2),
            (SELECT shipped_qty FROM outbound_order_lines WHERE owner_id = $1 AND outbound_order_id = $2 AND line_no = 1),
            (SELECT qty_on_hand FROM inventory_batches WHERE owner_id = $1 AND product_code = $3 AND batch_no = $4),
            (SELECT COUNT(*) FROM shipment_confirm_erp_feedback_outbox WHERE owner_id = $1 AND outbound_order_id = $2),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'outbound-ship-rollback-1'),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'ship_outbound_order' AND resource_id = $2::text)
        "#,
    )
    .bind(owner_id)
    .bind(order.id)
    .bind(product_code)
    .bind(batch_no)
    .fetch_one(&pool)
    .await
    .expect("rollback state should be queryable");
    assert_eq!(
        state,
        (
            "reviewed".to_string(),
            wms_domain::Quantity::ZERO,
            wms_domain::Quantity::from(6),
            0,
            0,
            0
        )
    );
}
