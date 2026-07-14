#[sqlx::test(migrations = "../../migrations")]
async fn location_capacity_cannot_exceed_maximum(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let writer = writer_token(owner_id);
    let (warehouse_id, zone_id) = seed_warehouse_zone(&pool, owner_id).await;
    let app = master_data_router(MasterDataAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let location: Location = json_response(
        app.clone(),
        request_json_with_key(
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
                "bound_owner_id": null
            }),
            "m1-capacity-location-create",
        ),
    )
    .await;

    let valid_update = app
        .clone()
        .oneshot(request_json_with_key(
            "PATCH",
            &format!("/api/v1/master-data/locations/{}", location.id),
            &writer,
            json!({"used_volume_cm3": 900}),
            "m1-capacity-used-update",
        ))
        .await
        .expect("router should respond");
    assert_eq!(valid_update.status(), StatusCode::OK);

    let over_capacity = app
        .clone()
        .oneshot(request_json_with_key(
            "PATCH",
            &format!("/api/v1/master-data/locations/{}", location.id),
            &writer,
            json!({"max_volume_cm3": 800}),
            "m1-capacity-max-update",
        ))
        .await
        .expect("router should respond");
    assert_eq!(over_capacity.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_response(over_capacity).await.code, "M1_LOCATION_CAPACITY_INVALID");

    let negative_used = app
        .oneshot(request_json_with_key(
            "PATCH",
            &format!("/api/v1/master-data/locations/{}", location.id),
            &writer,
            json!({"used_volume_cm3": -1}),
            "m1-capacity-negative-used",
        ))
        .await
        .expect("router should respond");
    assert_eq!(negative_used.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_response(negative_used).await.code, "M1_LOCATION_CAPACITY_INVALID");
}
