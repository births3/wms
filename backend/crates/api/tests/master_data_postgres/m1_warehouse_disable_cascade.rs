#[sqlx::test(migrations = "../../migrations")]
async fn disabling_warehouse_cascades_to_zones_and_locations(pool: PgPool) {
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
            json!({"warehouse_code": "WH-CASCADE-01", "warehouse_name": "级联停用仓"}),
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
                "zone_name": "级联库区",
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
            "m1-cascade-location-create",
        ),
    )
    .await;

    let disabled: Warehouse = json_response(
        app,
        request_json(
            "PATCH",
            &format!("/api/v1/master-data/warehouses/{}", warehouse.id),
            &writer,
            json!({"status": "disabled"}),
        ),
    )
    .await;
    assert_eq!(disabled.status, "disabled");

    let statuses: (String, String) = sqlx::query_as(
        "SELECT z.status, l.status FROM warehouse_zones z JOIN warehouse_locations l ON l.zone_id = z.id WHERE z.owner_id = $1 AND z.id = $2 AND l.id = $3",
    )
    .bind(owner_id)
    .bind(zone.id)
    .bind(location.id)
    .fetch_one(&pool)
    .await
    .expect("cascaded statuses should exist");
    assert_eq!(statuses, ("disabled".to_string(), "disabled".to_string()));

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'cascade_disable_warehouse'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("cascade audit count");
    assert_eq!(audit_count, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn disabling_warehouse_with_stock_is_rejected_and_rolled_back(pool: PgPool) {
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
            json!({"warehouse_code": "WH-CASCADE-STOCK", "warehouse_name": "库存级联保护仓"}),
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
                "zone_name": "库存库区",
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
            "m1-cascade-stock-location-create",
        ),
    )
    .await;
    sqlx::query(
        "INSERT INTO inventory_batches (id, owner_id, product_code, batch_no, production_date, expiry_date, qty_on_hand, qty_locked, quality_status, location_id, location_code, created_at, updated_at) VALUES ($1, $2, 'P-M1-CASCADE', 'B-M1-CASCADE', DATE '2026-01-01', DATE '2028-01-01', 1, 0, 'qualified', $3, 'A01-01-01-01', now(), now())",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(location.id)
    .execute(&pool)
    .await
    .expect("inventory should be seeded");

    let response = app
        .oneshot(request_json(
            "PATCH",
            &format!("/api/v1/master-data/warehouses/{}", warehouse.id),
            &writer,
            json!({"status": "disabled"}),
        ))
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_response(response).await.code, "M1_LOCATION_HAS_STOCK");

    let statuses: (String, String) = sqlx::query_as(
        "SELECT w.status, l.status FROM warehouses w JOIN warehouse_locations l ON l.warehouse_id = w.id WHERE w.owner_id = $1 AND w.id = $2 AND l.id = $3",
    )
    .bind(owner_id)
    .bind(warehouse.id)
    .bind(location.id)
    .fetch_one(&pool)
    .await
    .expect("warehouse and location should remain");
    assert_eq!(statuses, ("active".to_string(), "available".to_string()));
}
