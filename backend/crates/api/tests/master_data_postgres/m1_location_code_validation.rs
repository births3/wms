#[sqlx::test(migrations = "../../migrations")]
async fn location_code_format_is_enforced_on_create_and_update(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let writer = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let warehouse: Warehouse = json_response(
        app.clone(),
        request_json(
            "POST",
            "/api/v1/master-data/warehouses",
            &writer,
            json!({"warehouse_code": "WH-CODE-01", "warehouse_name": "编码测试仓"}),
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
                "temperature_zone": "normal_10_30",
                "quality_color": "qualified_green"
            }),
        ),
    )
    .await;

    let invalid_create = app
        .clone()
        .oneshot(request_json_with_key(
            "POST",
            "/api/v1/master-data/locations",
            &writer,
            json!({
                "warehouse_id": warehouse.id,
                "zone_id": zone.id,
                "location_code": "A01-1-01-01",
                "row_no": 1,
                "column_no": 1,
                "layer_no": 1,
                "max_volume_cm3": 1000,
                "max_sku_count": 1,
                "location_type": "storage",
                "current_owner_id": null
            }),
            "m1-invalid-location-create",
        ))
        .await
        .expect("router should respond");
    assert_eq!(invalid_create.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_response(invalid_create).await.code, "M1_LOCATION_BATCH_INVALID");

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
                "current_owner_id": null
            }),
            "m1-valid-location-create",
        ),
    )
    .await;
    let invalid_update = app
        .clone()
        .oneshot(request_json_with_key(
            "PATCH",
            &format!("/api/v1/master-data/locations/{}", location.id),
            &writer,
            json!({"location_code": "A01-01-01-02", "row_no": 1, "column_no": 1, "layer_no": 1}),
            "m1-invalid-location-update",
        ))
        .await
        .expect("router should respond");
    assert_eq!(invalid_update.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_response(invalid_update).await.code, "M1_LOCATION_BATCH_INVALID");
}
