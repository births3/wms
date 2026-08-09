#[sqlx::test(migrations = "../../migrations")]
async fn outbound_order_creation_freezes_the_effective_address_route(pool: PgPool) {
    let (owner_id, warehouse_id, customer_id, address_id) = seed_scope(&pool).await;
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 8, 0, 0)
        .single()
        .expect("valid order time");
    PrintOrchestrationService::with_postgres(pool.clone())
        .publish_route_binding(
            &auth,
            PublishRouteBindingRequest {
                warehouse_id,
                customer_id,
                delivery_address_id: address_id,
                route_code: "LINE-FROZEN".to_string(),
                effective_from: now,
                effective_to: None,
            },
            now,
            "h9-route-freeze-binding",
        )
        .await
        .expect("route binding should publish");

    let order = PgWave4Repository::new(pool.clone())
        .create_outbound_order(
            &auth,
            CreateOutboundOrderRequest {
                document_type: "sales_outbound".to_string(),
                wms_order_no: "SO-H9-ROUTE-FREEZE".to_string(),
                erp_order_no: Some("ERP-H9-ROUTE-FREEZE".to_string()),
                invoice_no: None,
                transport_mode_code: None,
                department_code: None,
                sales_group_code: None,
                order_group_no: None,
                business_type_code: None,
                customer_id,
                warehouse_id,
                delivery_address_id: address_id,
                required_ship_at: None,
                lines: vec![CreateOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: "P-H9-ROUTE".to_string(),
                    batch_no: "B-H9-ROUTE".to_string(),
                    planned_qty: 1.into(),
                }],
            },
            now,
            "h9-route-freeze-order",
            None,
        )
        .await
        .expect("outbound order should freeze route")
        .value;

    let frozen: (Uuid, String) = sqlx::query_as(
        "SELECT delivery_address_id, route_code FROM h9_outbound_route_snapshots WHERE owner_id = $1 AND outbound_order_id = $2",
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("route snapshot should load");
    assert_eq!(frozen, (address_id, "LINE-FROZEN".to_string()));
}
