#[sqlx::test(migrations = "../../migrations")]
async fn location_bound_owner_must_exist_or_be_shared(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let writer = writer_token(owner_id);
    let (warehouse_id, zone_id) = seed_warehouse_zone(&pool, owner_id).await;
    let app = master_data_router(MasterDataAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let response = app
        .clone()
        .oneshot(request_json_with_key(
            "POST",
            "/api/v1/master-data/locations",
            &writer,
            json!({
                "warehouse_id": warehouse_id,
                "zone_id": zone_id,
                "location_code": "A01-01-01-01",
                "row_no": 1,
                "column_no": 1,
                "layer_no": 1,
                "max_volume_cm3": 1000,
                "max_sku_count": 1,
                "location_type": "storage",
                "bound_owner_id": Uuid::new_v4()
            }),
            "m1-invalid-bound-owner",
        ))
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_response(response).await.code, "M1_LOCATION_OWNER_INVALID");

    let location: Location = json_response(
        app,
        request_json_with_key(
            "POST",
            "/api/v1/master-data/locations",
            &writer,
            json!({
                "warehouse_id": warehouse_id,
                "zone_id": zone_id,
                "location_code": "A01-01-01-02",
                "row_no": 1,
                "column_no": 1,
                "layer_no": 2,
                "max_volume_cm3": 1000,
                "max_sku_count": 1,
                "location_type": "storage",
                "bound_owner_id": null
            }),
            "m1-shared-location",
        ),
    )
    .await;
    assert_eq!(location.bound_owner_id, None);
}
