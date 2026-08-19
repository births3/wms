#[sqlx::test(migrations = "../../migrations")]
async fn warehouse_type_is_persisted_and_validated(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let writer = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let invalid = app
        .clone()
        .oneshot(request_json(
            "POST",
            "/api/v1/master-data/warehouses",
            &writer,
            json!({
                "warehouse_code": "WH-TYPE-INVALID",
                "warehouse_name": "非法仓库",
                "warehouse_type": "invalid"
            }),
        ))
        .await
        .expect("router should respond");
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_response(invalid).await.code, "M1_WAREHOUSE_TYPE_INVALID");

    let created: serde_json::Value = json_response(
        app,
        request_json(
            "POST",
            "/api/v1/master-data/warehouses",
            &writer,
            json!({
                "warehouse_code": "WH-TYPE-LOGICAL",
                "warehouse_name": "逻辑仓",
                "warehouse_type": "logical"
            }),
        ),
    )
    .await;
    assert_eq!(created["warehouse_type"], "logical");

    let persisted: String = sqlx::query_scalar(
        "SELECT warehouse_type FROM warehouses WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(Uuid::parse_str(created["id"].as_str().expect("warehouse id")).expect("valid uuid"))
    .fetch_one(&pool)
    .await
    .expect("warehouse type should persist");
    assert_eq!(persisted, "logical");
}
