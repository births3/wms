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
    Customer, CustomerListResponse, ErrorResponse, LocationListResponse, Product,
    ProductListResponse, SpecialDrugCategoryListResponse, Supplier, SupplierListResponse,
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
        Some("normal")
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

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/master-data/products")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "product_code": "P-M1-CREATE",
                        "product_name": "新建冷链商品",
                        "approval_no": "国药准字H-CREATE",
                        "spec": "10ml*1支",
                        "dosage_form": "注射剂",
                        "manufacturer": "示例药业",
                        "special_drug_category_code": "normal",
                        "attrs": {
                            "storage_condition": "cold",
                            "source": "manual"
                        }
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::OK);
    let product: Product =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
            .expect("product response");
    assert_eq!(product.product_code, "P-M1-CREATE");
    assert_eq!(product.attrs["source"], "manual");
    assert_eq!(product.attrs["storage_condition"], "cold");

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
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "supplier_code": "S-M1-CREATE",
                        "supplier_name": "新建供应商",
                        "license_no": "USCC-CREATE",
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

#[sqlx::test(migrations = "../../migrations")]
async fn location_batch_create_requires_master_data_write_permission(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_zone(&pool, owner_id).await;
    let token = bearer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .oneshot(batch_create_request(
            &token,
            Some("loc-no-write-permission"),
            json!({
                "warehouse_id": warehouse_id,
                "zone_id": zone_id,
                "area_code": "P01",
                "row_start": 1,
                "row_end": 1,
                "column_start": 1,
                "column_end": 1,
                "layer_start": 1,
                "layer_end": 1,
                "max_volume_cm3": 1_000,
                "max_sku_count": 1,
                "location_type": "storage",
                "bound_owner_id": null
            }),
        ))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let error = error_response(response).await;
    assert_eq!(error.code, "AUTH-005");

    let location_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM warehouse_locations WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("location count");
    assert_eq!(location_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn location_batch_create_rejects_batches_over_limit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_zone(&pool, owner_id).await;
    let token = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .oneshot(batch_create_request(
            &token,
            Some("loc-over-limit"),
            json!({
                "warehouse_id": warehouse_id,
                "zone_id": zone_id,
                "area_code": "L01",
                "row_start": 1,
                "row_end": 10,
                "column_start": 1,
                "column_end": 10,
                "layer_start": 1,
                "layer_end": 6,
                "max_volume_cm3": 1_000,
                "max_sku_count": 1,
                "location_type": "storage",
                "bound_owner_id": null
            }),
        ))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error = error_response(response).await;
    assert_eq!(error.code, "M1_LOCATION_BATCH_INVALID");

    let location_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM warehouse_locations WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("location count");
    assert_eq!(location_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn location_batch_create_replays_same_idempotency_key_without_duplicates(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_zone(&pool, owner_id).await;
    let token = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let body = json!({
        "warehouse_id": warehouse_id,
        "zone_id": zone_id,
        "area_code": "B02",
        "row_start": 1,
        "row_end": 1,
        "column_start": 1,
        "column_end": 1,
        "layer_start": 1,
        "layer_end": 1,
        "max_volume_cm3": 1_000,
        "max_sku_count": 1,
        "location_type": "piece_pick",
        "bound_owner_id": null
    });

    let first = app
        .clone()
        .oneshot(batch_create_request(
            &token,
            Some("loc-replay-1"),
            body.clone(),
        ))
        .await
        .expect("first response");
    let replay = app
        .oneshot(batch_create_request(&token, Some("loc-replay-1"), body))
        .await
        .expect("replay response");

    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(replay.status(), StatusCode::OK);
    let first_payload = location_list_response(first).await;
    let replay_payload = location_list_response(replay).await;
    assert_eq!(first_payload.data.len(), 1);
    assert_eq!(replay_payload.data.len(), 1);
    assert_eq!(first_payload.data[0].id, replay_payload.data[0].id);

    let (location_count, audit_count, idempotency_count): (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*)::BIGINT FROM warehouse_locations WHERE owner_id = $1),
            (SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'batch_create_locations'),
            (SELECT COUNT(*)::BIGINT FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2)
        "#,
    )
    .bind(owner_id)
    .bind("loc-replay-1")
    .fetch_one(&pool)
    .await
    .expect("replay counts");

    assert_eq!(location_count, 1);
    assert_eq!(audit_count, 1);
    assert_eq!(idempotency_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn location_batch_create_duplicate_location_rolls_back_whole_batch(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_zone(&pool, owner_id).await;
    seed_location(&pool, owner_id, warehouse_id, zone_id, "C03-01-01-02", 2).await;
    let token = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .oneshot(batch_create_request(
            &token,
            Some("loc-duplicate-1"),
            json!({
                "warehouse_id": warehouse_id,
                "zone_id": zone_id,
                "area_code": "C03",
                "row_start": 1,
                "row_end": 1,
                "column_start": 1,
                "column_end": 1,
                "layer_start": 1,
                "layer_end": 3,
                "max_volume_cm3": 1_000,
                "max_sku_count": 1,
                "location_type": "storage",
                "bound_owner_id": null
            }),
        ))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let error = error_response(response).await;
    assert_eq!(error.code, "M1_LOCATION_DUPLICATE");

    let (location_count, audit_count, idempotency_count): (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*)::BIGINT FROM warehouse_locations WHERE owner_id = $1),
            (SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'batch_create_locations'),
            (SELECT COUNT(*)::BIGINT FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2)
        "#,
    )
    .bind(owner_id)
    .bind("loc-duplicate-1")
    .fetch_one(&pool)
    .await
    .expect("rollback counts");

    assert_eq!(location_count, 1);
    assert_eq!(audit_count, 0);
    assert_eq!(idempotency_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn location_batch_create_requires_idempotency_key(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_zone(&pool, owner_id).await;
    let token = writer_token(owner_id);
    let app = master_data_router(MasterDataAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .oneshot(batch_create_request(
            &token,
            None,
            json!({
                "warehouse_id": warehouse_id,
                "zone_id": zone_id,
                "area_code": "D04",
                "row_start": 1,
                "row_end": 1,
                "column_start": 1,
                "column_end": 1,
                "layer_start": 1,
                "layer_end": 1,
                "max_volume_cm3": 1_000,
                "max_sku_count": 1,
                "location_type": "storage",
                "bound_owner_id": null
            }),
        ))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error = error_response(response).await;
    assert_eq!(error.code, "M1_LOCATION_IDEMPOTENCY_REQUIRED");
}

async fn seed_product(
    pool: &PgPool,
    owner_id: Uuid,
    product_code: &str,
    product_name: &str,
    storage_condition: &str,
    now: chrono::DateTime<Utc>,
) {
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification,
            storage_condition, special_drug_category, approval_no, manufacturer,
            status, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, '10ml*1支', $5, 'normal', '国药准字H-M1', '示例药业', 'active', $6, $6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(product_code)
    .bind(product_name)
    .bind(storage_condition)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed product");
}

async fn seed_supplier(
    pool: &PgPool,
    owner_id: Uuid,
    supplier_code: &str,
    supplier_name: &str,
    source: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO suppliers (
            id, owner_id, supplier_code, supplier_name, uscc, contact_name, source, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, '张供应', $6, 'active', now(), now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(supplier_code)
    .bind(supplier_name)
    .bind(format!("USCC-{}", &Uuid::new_v4().to_string()[..8]))
    .bind(source)
    .execute(pool)
    .await
    .expect("seed supplier");
}

async fn seed_customer(
    pool: &PgPool,
    owner_id: Uuid,
    customer_code: &str,
    customer_name: &str,
    source: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO customers (
            id, owner_id, customer_code, customer_name, customer_type, source, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, 'customer', $5, 'active', now(), now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(customer_code)
    .bind(customer_name)
    .bind(source)
    .execute(pool)
    .await
    .expect("seed customer");
}

async fn seed_warehouse_zone(pool: &PgPool, owner_id: Uuid) -> (Uuid, Uuid) {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, '测试仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'A01', '合格区', 'cold', 'qualified_green', 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("seed warehouse zone");
    (warehouse_id, zone_id)
}

async fn seed_location(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    zone_id: Uuid,
    location_code: &str,
    layer_no: i32,
) {
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, 1, 1, $6, 1000, 0, 1, 'storage', 'available', now(), now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(location_code)
    .bind(layer_no)
    .execute(pool)
    .await
    .expect("seed location");
}

fn batch_create_request(
    token: &str,
    idempotency_key: Option<&str>,
    body: serde_json::Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/v1/master-data/locations/batch-create")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("content-type", "application/json");
    if let Some(idempotency_key) = idempotency_key {
        builder = builder.header("idempotency-key", idempotency_key);
    }
    builder
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

async fn location_list_response(response: axum::response::Response) -> LocationListResponse {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    serde_json::from_slice(&body).expect("response should be location list")
}

async fn error_response(response: axum::response::Response) -> ErrorResponse {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    serde_json::from_slice(&body).expect("response should be error")
}
