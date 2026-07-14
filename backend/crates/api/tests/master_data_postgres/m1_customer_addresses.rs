use wms_domain::{CustomerAddress, CustomerAddressListResponse, CustomerProfile};

#[sqlx::test(migrations = "../../migrations")]
async fn customer_addresses_support_multiple_defaults_idempotency_and_owner_scope(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let writer = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let customer: Customer = json_response(
        app.clone(),
        request_json_with_key(
            "POST",
            "/api/v1/master-data/customers",
            &writer,
            json!({
                "customer_code":"C-ADDRESS-01",
                "customer_name":"多地址客户",
                "license_no":null,
                "source":"manual"
            }),
            "customer-address-owner",
        ),
    )
    .await;
    let address_request = json!({
        "province":"上海市",
        "city":"上海市",
        "district":"浦东新区",
        "detail_address":"张江路 1 号",
        "contact_name":"张三",
        "contact_phone":"13800000001",
        "is_default":true
    });
    let first: CustomerAddress = json_response(
        app.clone(),
        request_json_with_key(
            "POST",
            &format!("/api/v1/master-data/customers/{}/addresses", customer.id),
            &writer,
            address_request.clone(),
            "customer-address-create-1",
        ),
    )
    .await;
    let replayed: CustomerAddress = json_response(
        app.clone(),
        request_json_with_key(
            "POST",
            &format!("/api/v1/master-data/customers/{}/addresses", customer.id),
            &writer,
            address_request,
            "customer-address-create-1",
        ),
    )
    .await;
    assert_eq!(first.id, replayed.id);

    let second: CustomerAddress = json_response(
        app.clone(),
        request_json_with_key(
            "POST",
            &format!("/api/v1/master-data/customers/{}/addresses", customer.id),
            &writer,
            json!({
                "province":"江苏省",
                "city":"南京市",
                "district":"玄武区",
                "detail_address":"中山路 2 号",
                "contact_name":"李四",
                "contact_phone":"13900000002",
                "is_default":true
            }),
            "customer-address-create-2",
        ),
    )
    .await;
    assert_ne!(first.id, second.id);

    let addresses: CustomerAddressListResponse = json_response(
        app.clone(),
        request_json(
            "GET",
            &format!("/api/v1/master-data/customers/{}/addresses", customer.id),
            &writer,
            json!({}),
        ),
    )
    .await;
    assert_eq!(addresses.data.len(), 2);
    assert_eq!(addresses.data.iter().filter(|item| item.is_default).count(), 1);
    assert_eq!(
        addresses
            .data
            .iter()
            .find(|item| item.id == second.id)
            .map(|item| item.is_default),
        Some(true)
    );

    let moved_default: CustomerAddress = json_response(
        app.clone(),
        request_json(
            "PATCH",
            &format!(
                "/api/v1/master-data/customers/{}/addresses/{}",
                customer.id, first.id
            ),
            &writer,
            json!({"is_default":true}),
        ),
    )
    .await;
    assert!(moved_default.is_default);

    let cross_owner = app
        .oneshot(request_json(
            "GET",
            &format!("/api/v1/master-data/customers/{}/addresses", customer.id),
            &bearer_token(other_owner_id),
            json!({}),
        ))
        .await
        .expect("router should respond");
    assert_eq!(cross_owner.status(), StatusCode::NOT_FOUND);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id=$1 AND action IN ('create_customer_address','update_customer_address')",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("address audit count");
    assert_eq!(audit_count, 3);
    let reset_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id=$1 AND action='unset_customer_address_default'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("default reset audit count");
    assert_eq!(reset_audit_count, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn customer_profile_supports_store_fields_validation_idempotency_and_owner_scope(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let writer = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let customer: Customer = json_response(
        app.clone(),
        request_json_with_key(
            "POST",
            "/api/v1/master-data/customers",
            &writer,
            json!({
                "customer_code":"C-PROFILE-01",
                "customer_name":"门店档案",
                "license_no":null,
                "source":"manual"
            }),
            "customer-profile-create",
        ),
    )
    .await;
    let profile_request = json!({
        "customer_type":"store",
        "contact_name":"门店联系人",
        "contact_phone":"13800000003",
        "business_scope":["处方药","医疗器械"],
        "qualification_certificates":[{"certificate_type":"经营许可证","certificate_no":"LIC-001","expires_at":"2027-12-31"}],
        "chain_name":"示例连锁"
    });
    let profile: CustomerProfile = json_response(
        app.clone(),
        request_json_with_key(
            "PATCH",
            &format!("/api/v1/master-data/customers/{}/profile", customer.id),
            &writer,
            profile_request.clone(),
            "customer-profile-upsert",
        ),
    )
    .await;
    assert_eq!(profile.customer_type, "store");
    assert_eq!(profile.business_scope, vec!["处方药", "医疗器械"]);
    assert_eq!(profile.qualification_certificates.len(), 1);

    let replayed: CustomerProfile = json_response(
        app.clone(),
        request_json_with_key(
            "PATCH",
            &format!("/api/v1/master-data/customers/{}/profile", customer.id),
            &writer,
            profile_request,
            "customer-profile-upsert",
        ),
    )
    .await;
    assert_eq!(replayed.updated_at, profile.updated_at);

    let invalid = app
        .clone()
        .oneshot(request_json_with_key(
            "PATCH",
            &format!("/api/v1/master-data/customers/{}/profile", customer.id),
            &writer,
            json!({
                "customer_type":"store",
                "contact_name":"门店联系人",
                "contact_phone":"13800000003",
                "business_scope":[],
                "qualification_certificates":[],
                "chain_name":null
            }),
            "customer-profile-invalid",
        ))
        .await
        .expect("router should respond");
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let cross_owner = app
        .oneshot(request_json(
            "GET",
            &format!("/api/v1/master-data/customers/{}/profile", customer.id),
            &bearer_token(other_owner_id),
            json!({}),
        ))
        .await
        .expect("router should respond");
    assert_eq!(cross_owner.status(), StatusCode::NOT_FOUND);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id=$1 AND action='update_customer_profile'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("customer profile audit count");
    assert_eq!(audit_count, 1);
}
