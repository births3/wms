#[sqlx::test(migrations = "../../migrations")]
async fn m1_master_data_write_is_provisioned_for_warehouse_manager(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners(id, owner_code, owner_name) VALUES ($1, $2, $2)",
    )
    .bind(owner_id)
    .bind(format!("M1-PERM-{owner_id}"))
    .execute(&pool)
    .await
    .expect("owner should seed default roles");

    let permission_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_permissions WHERE permission_code = 'm1.master_data.write'",
    )
    .fetch_one(&pool)
    .await
    .expect("M1 write permission should exist");
    let granted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM auth_role_permissions rp JOIN auth_roles role ON role.id = rp.role_id WHERE role.owner_id = $1 AND role.role_code = 'warehouse_manager' AND rp.permission_id = $2",
    )
    .bind(owner_id)
    .bind(permission_id)
    .fetch_one(&pool)
    .await
    .expect("warehouse manager permission should be queryable");

    assert_eq!(granted, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn m1_warehouse_reads_require_master_data_read_permission(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let token = bearer_token_with_permissions(owner_id, &[]);
    let app = master_data_router(MasterDataAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/master-data/warehouses")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn m1_product_reads_require_master_data_read_permission(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, special_drug_category, status) VALUES ($1,$2,'P-READ-PERM','权限测试商品','1盒','normal_10_30','none','active')",
    )
    .bind(product_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("product should seed");
    let token = bearer_token_with_permissions(owner_id, &[]);
    let app = master_data_router(MasterDataAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    for uri in [
        "/api/v1/master-data/products".to_string(),
        format!("/api/v1/master-data/products/{product_id}"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header(AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
