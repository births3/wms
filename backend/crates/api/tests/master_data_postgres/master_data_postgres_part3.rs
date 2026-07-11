#[sqlx::test(migrations = "../../migrations")]
async fn warehouse_zone_location_routes_persist_update_disable_and_isolate_owner(pool: PgPool) {
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
            json!({
                "warehouse_code": "WH-REAL-01", "warehouse_name": "真实一号仓"
            }),
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
                "warehouse_id": warehouse.id, "zone_code": "A01", "zone_name": "合格区",
                "temperature_zone": "normal", "quality_color": "qualified_green"
            }),
        ),
    )
    .await;
    let location: Location = json_response(
        app.clone(),
        request_json(
            "POST",
            "/api/v1/master-data/locations",
            &writer,
            json!({
                "warehouse_id": warehouse.id, "zone_id": zone.id, "location_code": "A01-01-01-01",
                "row_no": 1, "column_no": 1, "layer_no": 1, "max_volume_cm3": 1000,
                "max_sku_count": 1, "location_type": "storage", "bound_owner_id": null
            }),
        ),
    )
    .await;

    let warehouse: Warehouse = json_response(
        app.clone(),
        request_json(
            "PATCH",
            &format!("/api/v1/master-data/warehouses/{}", warehouse.id),
            &writer,
            json!({
                "warehouse_name": "真实一号仓（更新）", "status": "disabled"
            }),
        ),
    )
    .await;
    let zone: WarehouseZone = json_response(
        app.clone(),
        request_json(
            "PATCH",
            &format!("/api/v1/master-data/warehouse-zones/{}", zone.id),
            &writer,
            json!({
                "zone_name": "待验区", "status": "disabled"
            }),
        ),
    )
    .await;
    let location: Location = json_response(
        app.clone(),
        request_json(
            "PATCH",
            &format!("/api/v1/master-data/locations/{}", location.id),
            &writer,
            json!({
                "status": "disabled"
            }),
        ),
    )
    .await;

    assert_eq!(warehouse.status, "disabled");
    assert_eq!(zone.status, "disabled");
    assert_eq!(location.status, "disabled");
    let persisted: (String, String, String) = sqlx::query_as(
        "SELECT w.status, z.status, l.status FROM warehouses w JOIN warehouse_zones z ON z.warehouse_id=w.id JOIN warehouse_locations l ON l.zone_id=z.id WHERE w.owner_id=$1 AND w.id=$2",
    ).bind(owner_id).bind(warehouse.id).fetch_one(&pool).await.expect("records persist");
    assert_eq!(
        persisted,
        ("disabled".into(), "disabled".into(), "disabled".into())
    );

    let other_writer = writer_token(Uuid::new_v4());
    let response = app
        .oneshot(request_json(
            "PATCH",
            &format!("/api/v1/master-data/warehouse-zones/{}", zone.id),
            &other_writer,
            json!({"zone_name": "越权修改"}),
        ))
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id=$1 AND resource_type IN ('warehouse','warehouse_zone','location')",
    ).bind(owner_id).fetch_one(&pool).await.expect("audit count");
    assert_eq!(audit_count, 6);
    let diff: serde_json::Value = sqlx::query_scalar(
        "SELECT diff FROM audit_event WHERE owner_id=$1 AND resource_type='warehouse_zone' AND action='update_warehouse_zone'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("zone update audit diff");
    assert_eq!(diff["before"]["zone_name"], "合格区");
    assert_eq!(diff["after"]["zone_name"], "待验区");
    assert!(diff["changed_keys"]
        .as_array()
        .expect("changed keys")
        .contains(&json!("zone_name")));
}

#[sqlx::test(migrations = "../../migrations")]
async fn warehouse_zone_create_replays_idempotency_key_and_rejects_changed_request(pool: PgPool) {
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
            json!({"warehouse_code":"WH-IDEM-01","warehouse_name":"幂等测试仓"}),
        ),
    )
    .await;
    let body = json!({
        "warehouse_id": warehouse.id, "zone_code":"IDEM-01", "zone_name":"幂等库区",
        "temperature_zone":"normal", "quality_color":"qualified_green"
    });
    let key = "warehouse-zone-create-idempotency";
    let first: WarehouseZone = json_response(
        app.clone(),
        request_json_with_key("POST", "/api/v1/master-data/warehouse-zones", &writer, body.clone(), key),
    )
    .await;
    let replayed: WarehouseZone = json_response(
        app.clone(),
        request_json_with_key("POST", "/api/v1/master-data/warehouse-zones", &writer, body, key),
    )
    .await;
    assert_eq!(first.id, replayed.id);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM warehouse_zones WHERE owner_id=$1")
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("zone count");
    assert_eq!(count, 1);

    let response = app
        .oneshot(request_json_with_key(
            "POST",
            "/api/v1/master-data/warehouse-zones",
            &writer,
            json!({
                "warehouse_id": warehouse.id, "zone_code":"IDEM-02", "zone_name":"冲突库区",
                "temperature_zone":"normal", "quality_color":"qualified_green"
            }),
            key,
        ))
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_supplier_customer_updates_persist_and_append_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let writer = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let product: Product = json_response(app.clone(), request_json("POST", "/api/v1/master-data/products", &writer, json!({
        "product_code":"P-LIFE-01", "product_name":"原商品", "approval_no":null, "spec":"1盒", "dosage_form":null,
        "manufacturer":null, "special_drug_category_code":"normal", "attrs":{"storage_condition":"normal","source":"manual"}
    }))).await;
    let supplier: Supplier = json_response(app.clone(), request_json("POST", "/api/v1/master-data/suppliers", &writer, json!({
        "supplier_code":"S-LIFE-01", "supplier_name":"原供应商", "license_no":"USCC-LIFE-01", "contact_name":null, "source":"manual"
    }))).await;
    let customer: Customer = json_response(app.clone(), request_json("POST", "/api/v1/master-data/customers", &writer, json!({
        "customer_code":"C-LIFE-01", "customer_name":"原客户", "license_no":"LIC-LIFE-01", "source":"manual"
    }))).await;

    let product: Product = json_response(
        app.clone(),
        request_json(
            "PATCH",
            &format!("/api/v1/master-data/products/{}", product.id),
            &writer,
            json!({"product_name":"新商品","status":"disabled"}),
        ),
    )
    .await;
    let supplier: Supplier = json_response(
        app.clone(),
        request_json(
            "PATCH",
            &format!("/api/v1/master-data/suppliers/{}", supplier.id),
            &writer,
            json!({"supplier_name":"新供应商","status":"disabled"}),
        ),
    )
    .await;
    let customer: Customer = json_response(
        app,
        request_json(
            "PATCH",
            &format!("/api/v1/master-data/customers/{}", customer.id),
            &writer,
            json!({"customer_name":"新客户","status":"disabled"}),
        ),
    )
    .await;
    assert_eq!(
        (product.product_name.as_str(), product.status.as_str()),
        ("新商品", "disabled")
    );
    assert_eq!(
        (supplier.supplier_name.as_str(), supplier.status.as_str()),
        ("新供应商", "disabled")
    );
    assert_eq!(
        (customer.customer_name.as_str(), customer.status.as_str()),
        ("新客户", "disabled")
    );

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id=$1 AND action LIKE 'update_%'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit count");
    assert_eq!(audit_count, 3);
}

#[sqlx::test(migrations = "../../migrations")]
async fn warehouse_zone_rejects_disabled_dictionary_value(pool: PgPool) {
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
            json!({
                "warehouse_code":"WH-DICT-01", "warehouse_name":"字典校验仓"
            }),
        ),
    )
    .await;
    sqlx::query("UPDATE system_dictionary_items SET enabled=FALSE WHERE dict_code='temperature_zone' AND item_code='normal' AND owner_id IS NULL")
        .execute(&pool).await.expect("disable temperature dictionary item");
    let response = app
        .oneshot(request_json(
            "POST",
            "/api/v1/master-data/warehouse-zones",
            &writer,
            json!({
                "warehouse_id":warehouse.id, "zone_code":"A01", "zone_name":"不可创建区",
                "temperature_zone":"normal", "quality_color":"qualified_green"
            }),
        ))
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM warehouse_zones WHERE owner_id=$1")
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("zone count");
    assert_eq!(count, 0);
}

fn request_json(method: &str, uri: &str, token: &str, body: serde_json::Value) -> Request<Body> {
    request_json_with_key(method, uri, token, body, &Uuid::new_v4().to_string())
}

fn request_json_with_key(
    method: &str,
    uri: &str,
    token: &str,
    body: serde_json::Value,
    idempotency_key: &str,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("idempotency-key", idempotency_key)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

async fn json_response<T: serde::de::DeserializeOwned>(
    app: axum::Router,
    request: Request<Body>,
) -> T {
    let response = app.oneshot(request).await.expect("router should respond");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    assert_eq!(
        status,
        StatusCode::OK,
        "response: {}",
        String::from_utf8_lossy(&body)
    );
    serde_json::from_slice(&body).expect("response should deserialize")
}
