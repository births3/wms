#[sqlx::test(migrations = "../../migrations")]
async fn location_with_stock_cannot_be_disabled(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let writer = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let warehouse: Warehouse = json_response(
        app.clone(),
        request_json(
            "POST",
            "/api/v1/master-data/warehouses",
            &writer,
            json!({"warehouse_code": "WH-STOCK-01", "warehouse_name": "库存保护仓"}),
        ),
    )
    .await;
    let zone: WarehouseZone = json_response(
        app.clone(),
        request_json(
            "POST",
            "/api/v1/master-data/warehouse-zones",
            &writer,
            json!({
                "warehouse_id": warehouse.id,
                "zone_code": "A01",
                "zone_name": "合格区",
                "temperature_zone": "normal",
                "quality_color": "qualified_green"
            }),
        ),
    )
    .await;
    let location: Location = json_response(
        app.clone(),
        request_json_with_key(
            "POST",
            "/api/v1/master-data/locations",
            &writer,
            json!({
                "warehouse_id": warehouse.id,
                "zone_id": zone.id,
                "location_code": "A01-01-01-01",
                "row_no": 1,
                "column_no": 1,
                "layer_no": 1,
                "max_volume_cm3": 1000,
                "max_sku_count": 1,
                "location_type": "storage",
                "bound_owner_id": null
            }),
            "m1-stock-location-create",
        ),
    )
    .await;
    sqlx::query(
        "INSERT INTO inventory_batches (id, owner_id, product_code, batch_no, production_date, expiry_date, qty_on_hand, qty_locked, quality_status, location_id, location_code, created_at, updated_at) VALUES ($1, $2, 'P-M1-STOCK', 'B-M1-STOCK', DATE '2026-01-01', DATE '2028-01-01', 1, 0, 'qualified', $3, 'A01-01-01-01', now(), now())",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(location.id)
    .execute(&pool)
    .await
    .expect("inventory should be seeded");

    let response = app
        .oneshot(request_json_with_key(
            "PATCH",
            &format!("/api/v1/master-data/locations/{}", location.id),
            &writer,
            json!({"status": "disabled"}),
            "m1-stock-location-disable",
        ))
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_response(response).await.code, "M1_LOCATION_HAS_STOCK");
}
