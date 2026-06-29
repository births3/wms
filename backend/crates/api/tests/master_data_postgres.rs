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
use wms_domain::{ProductListResponse, SpecialDrugCategoryListResponse};

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
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "master-data-reader",
        vec!["m1.master_data.read".to_string()],
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-secret").expect("token should encode")
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
    assert_eq!(rows[0].attrs, json!({"storage_condition": "cold"}));
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
