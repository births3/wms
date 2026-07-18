use chrono::{TimeZone, Utc};
use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header::AUTHORIZATION, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthContext,
        AuthRevocationStore, AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
    },
    master_data_handlers::{master_data_router, MasterDataAppState},
    master_data_postgres::PgMasterDataReadRepository,
};
use wms_domain::{
    Customer, CustomerListResponse, ErrorResponse, Location, LocationListResponse, Product,
    ProductListResponse, SpecialDrugCategoryListResponse, Supplier, SupplierListResponse,
    Warehouse, WarehouseZone,
};

struct AllowAllRevocationStore;

#[axum::async_trait]
impl AuthRevocationStore for AllowAllRevocationStore {
    async fn jti_is_blacklisted(&self, _jti: &str) -> Result<bool, AuthRevocationStoreError> {
        Ok(false)
    }

    async fn permissions_changed_at(
        &self,
        _user_id: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError> {
        Ok(None)
    }

    async fn blacklist_jti(
        &self,
        _jti: &str,
        _ttl_seconds: u64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }

    async fn set_permissions_changed_at(
        &self,
        _user_id: Uuid,
        _changed_at_unix: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }
}

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "master-data-postgres-test".to_string(),
        permissions: vec!["m1.master_data.read".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn bearer_token(owner_id: Uuid) -> String {
    bearer_token_with_permissions(owner_id, &["m1.master_data.read"])
}

fn bearer_token_with_permissions(owner_id: Uuid, permissions: &[&str]) -> String {
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "master-data-reader",
        permissions
            .iter()
            .map(|permission| permission.to_string())
            .collect(),
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-secret").expect("token should encode")
}

fn writer_token(owner_id: Uuid) -> String {
    bearer_token_with_permissions(owner_id, &["m1.master_data.read", "m1.master_data.write"])
}

#[sqlx::test(migrations = "../../migrations")]
async fn products_are_read_from_postgres_by_owner(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 6, 29, 9, 0, 0)
        .single()
        .expect("valid time");
    seed_product(&pool, owner_id, "P-M1-001", "冷藏胰岛素", "cold", now).await;
    seed_product(
        &pool,
        other_owner_id,
        "P-M1-002",
        "其他货主商品",
        "normal",
        now,
    )
    .await;

    let rows = PgMasterDataReadRepository::new(pool)
        .list_products(&ctx(owner_id))
        .await
        .expect("owner products should load");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].product_code, "P-M1-001");
    assert_eq!(rows[0].spec.as_deref(), Some("10ml*1支"));
    assert_eq!(
        rows[0].special_drug_category_code.as_deref(),
        Some("none")
    );
    assert_eq!(
        rows[0].attrs,
        json!({"storage_condition": "cold", "source": "api_import"})
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_list_route_reads_postgres_by_owner(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 6, 29, 9, 30, 0)
        .single()
        .expect("valid time");
    seed_product(&pool, owner_id, "P-M1-101", "接口冷藏胰岛素", "cold", now).await;
    seed_product(
        &pool,
        other_owner_id,
        "P-M1-102",
        "其他货主接口商品",
        "normal",
        now,
    )
    .await;
    let token = bearer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/master-data/products")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let payload: ProductListResponse =
        serde_json::from_slice(&body).expect("response should be product list");
    assert_eq!(payload.page.count, 1);
    assert_eq!(payload.data.len(), 1);
    assert_eq!(payload.data[0].product_code, "P-M1-101");
    assert_eq!(payload.data[0].attrs["source"], "api_import");
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_create_route_writes_source_and_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let token = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let product_body = json!({
        "product_code": "P-M1-CREATE",
        "product_name": "新建冷链商品",
        "approval_no": "国药准字H-CREATE",
        "spec": "10ml*1支",
        "dosage_form": "注射剂",
        "manufacturer": "示例药业",
        "special_drug_category_code": "none",
        "attrs": {
            "storage_condition": "cold",
            "source": "manual",
            "middle_package": "10盒/中包"
        }
    });
    let response = app
        .clone()
        .oneshot(request_json_with_key(
            "POST",
            "/api/v1/master-data/products",
            &token,
            product_body.clone(),
            "m1-product-create-source",
        ))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::OK);
    let product: Product =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
            .expect("product response");
    assert_eq!(product.product_code, "P-M1-CREATE");
    assert_eq!(product.attrs["source"], "manual");
    assert_eq!(product.attrs["storage_condition"], "cold");
    assert_eq!(product.attrs["middle_package"], "10盒/中包");

    let replayed_response = app
        .oneshot(request_json_with_key(
            "POST",
            "/api/v1/master-data/products",
            &token,
            product_body,
            "m1-product-create-source",
        ))
        .await
        .expect("replay should respond");
    assert_eq!(replayed_response.status(), StatusCode::OK);
    let replayed: Product = serde_json::from_slice(
        &to_bytes(replayed_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .expect("replayed product response");
    assert_eq!(replayed.id, product.id);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'create_product'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit count");
    assert_eq!(audit_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_routes_accept_custom_enabled_special_drug_category(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let custom_code = "custom_antineoplastic";
    let now = Utc
        .with_ymd_and_hms(2026, 7, 13, 10, 0, 0)
        .single()
        .expect("valid time");
    sqlx::query(
        "INSERT INTO system_dictionary_items (id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at) VALUES ($1, 'special_drug_category', $2, '自定义抗肿瘤药品', TRUE, NULL, $3, 'global', $4, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(custom_code)
    .bind(json!({ "requires_dual_sign": true }))
    .bind(now)
    .execute(&pool)
    .await
    .expect("custom special drug category should be seeded");

    let token = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let create_response = app
        .clone()
        .oneshot(request_json_with_key(
            "POST",
            "/api/v1/master-data/products",
            &token,
            json!({
                "product_code": "P-M1-CUSTOM-001",
                "product_name": "自定义特殊药品",
                "special_drug_category_code": custom_code,
                "attrs": { "storage_condition": "normal" }
            }),
            "m1-product-custom-category-create",
        ))
        .await
        .expect("custom category create should respond");
    assert_eq!(create_response.status(), StatusCode::OK);
    let created: Product = serde_json::from_slice(
        &to_bytes(create_response.into_body(), usize::MAX)
            .await
            .expect("create response body"),
    )
    .expect("custom category product response");
    assert_eq!(
        created.special_drug_category_code.as_deref(),
        Some(custom_code)
    );

    let update_response = app
        .oneshot(request_json_with_key(
            "PATCH",
            &format!("/api/v1/master-data/products/{}", created.id),
            &token,
            json!({ "special_drug_category_code": custom_code }),
            "m1-product-custom-category-update",
        ))
        .await
        .expect("custom category update should respond");
    assert_eq!(update_response.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn supplier_and_customer_routes_return_source(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    seed_supplier(&pool, owner_id, "S-M1-001", "手工供应商", "manual").await;
    seed_supplier(
        &pool,
        other_owner_id,
        "S-M1-002",
        "其他供应商",
        "api_import",
    )
    .await;
    seed_customer(&pool, owner_id, "C-M1-001", "批量客户", "batch_import").await;
    seed_customer(&pool, other_owner_id, "C-M1-002", "其他客户", "api_import").await;

    let token = bearer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let suppliers = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/master-data/suppliers")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(suppliers.status(), StatusCode::OK);
    let supplier_body = to_bytes(suppliers.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let supplier_payload: SupplierListResponse =
        serde_json::from_slice(&supplier_body).expect("response should be supplier list");
    assert_eq!(supplier_payload.page.count, 1);
    assert_eq!(supplier_payload.data[0].source, "manual");

    let customers = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/master-data/customers")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(customers.status(), StatusCode::OK);
    let customer_body = to_bytes(customers.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let customer_payload: CustomerListResponse =
        serde_json::from_slice(&customer_body).expect("response should be customer list");
    assert_eq!(customer_payload.page.count, 1);
    assert_eq!(customer_payload.data[0].source, "batch_import");
}

#[sqlx::test(migrations = "../../migrations")]
async fn supplier_and_customer_create_routes_write_source_and_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let token = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let supplier_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/master-data/suppliers")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header("idempotency-key", "supplier-create-source")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "supplier_code": "S-M1-CREATE",
                        "supplier_name": "新建供应商",
                        "license_no": "91350100M000100Y43",
                        "contact_name": "王供应",
                        "source": "manual"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(supplier_response.status(), StatusCode::OK);
    let supplier: Supplier = serde_json::from_slice(
        &to_bytes(supplier_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .expect("supplier response");
    assert_eq!(supplier.source, "manual");

    let customer_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/master-data/customers")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header("idempotency-key", "customer-create-source")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "customer_code": "C-M1-CREATE",
                        "customer_name": "新建客户",
                        "license_no": "LIC-CREATE",
                        "source": "batch_import"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(customer_response.status(), StatusCode::OK);
    let customer: Customer = serde_json::from_slice(
        &to_bytes(customer_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .expect("customer response");
    assert_eq!(customer.source, "batch_import");

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action IN ('create_supplier', 'create_customer')",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit count");
    assert_eq!(audit_count, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn special_drug_category_route_reads_system_dictionary(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let token = bearer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/master-data/special-drug-categories")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let payload: SpecialDrugCategoryListResponse =
        serde_json::from_slice(&body).expect("response should be category list");
    assert_eq!(payload.page.count, 8);
    assert_eq!(payload.data.len(), 8);
    assert!(payload.data.iter().any(|category| {
        category.owner_id == owner_id
            && category.category_code == "narcotic"
            && category.category_name == "麻醉药品"
            && category.requires_dual_sign
            && category.status == "active"
    }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn location_batch_create_route_writes_postgres_and_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_zone(&pool, owner_id).await;
    let token = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .oneshot(batch_create_request(
            &token,
            Some("loc-batch-30"),
            json!({
                "warehouse_id": warehouse_id,
                "zone_id": zone_id,
                "area_code": "a01",
                "row_start": 1,
                "row_end": 2,
                "column_start": 1,
                "column_end": 3,
                "layer_start": 1,
                "layer_end": 5,
                "max_volume_cm3": 5_000_000,
                "max_sku_count": 2,
                "location_type": "storage",
                "bound_owner_id": owner_id
            }),
        ))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = location_list_response(response).await;
    assert_eq!(payload.page.count, 30);
    assert!(payload.data.iter().any(|location| {
        location.location_code == "A01-02-03-05"
            && location.row_no == 2
            && location.column_no == 3
            && location.layer_no == 5
            && location.owner_id == owner_id
            && location.used_volume_cm3 == 0
            && location.status == "available"
    }));

    let location_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM warehouse_locations WHERE owner_id = $1 AND zone_id = $2",
    )
    .bind(owner_id)
    .bind(zone_id)
    .fetch_one(&pool)
    .await
    .expect("location count");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'batch_create_locations'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit count");

    assert_eq!(location_count, 30);
    assert_eq!(audit_count, 1);
}
