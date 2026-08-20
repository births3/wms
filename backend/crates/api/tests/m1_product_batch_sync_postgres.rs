use axum::{
    body::{to_bytes, Body},
    http::{Request as HttpRequest, StatusCode},
    middleware::from_fn_with_state,
};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    api_key_auth::{api_key_auth_middleware, ApiKeyAuthState},
    api_key_service::ApiKeyService,
    auth::AuthContext,
    master_data::MasterDataError,
    master_data_handlers::{master_data_router, MasterDataAppState},
    master_data_postgres::PgMasterDataReadRepository,
};
use wms_domain::{
    CreateApiKeyRequest, CreateProductRequest, ErrorResponse, ProductPackagingLevelInput,
};

fn context(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m1-product-batch-test".to_string(),
        permissions: vec![
            "m1.master_data.read".to_string(),
            "m1.master_data.write".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn request(code: &str) -> CreateProductRequest {
    CreateProductRequest {
        product_code: code.to_string(),
        product_name: format!("商品 {code}"),
        approval_no: None,
        spec: "10ml*1支".to_string(),
        dosage_form: Some("注射剂".to_string()),
        manufacturer: Some("测试药业".to_string()),
        special_drug_category_code: Some("none".to_string()),
        is_external_use: None,
        is_fragrant: None,
        udi_code: None,
        electronic_regulatory_code: None,
        barcode_69: None,
        length_mm: None,
        width_mm: None,
        height_mm: None,
        volume_cm3: None,
        weight_g: None,
        packaging_levels: vec![ProductPackagingLevelInput {
            unit_code: "piece".to_string(),
            unit_name: "支".to_string(),
            ratio_to_base: 1,
            is_base: true,
            is_default: true,
            sort_order: 1,
        }],
        attrs: json!({"storage_condition": "normal_10_30", "source": "api_import"}),
    }
}

async fn seed_owner_and_user(pool: &PgPool, owner_id: Uuid, user_id: Uuid) {
    sqlx::query("INSERT INTO auth_owners(id, owner_code, owner_name) VALUES ($1, $2, $3)")
        .bind(owner_id)
        .bind(format!("OWNER-{}", &owner_id.to_string()[..8]))
        .bind("商品批量同步测试货主")
        .execute(pool)
        .await
        .expect("owner should seed");
    sqlx::query(
        "INSERT INTO auth_users(id, username, display_name, password_hash) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(format!("product-batch-{}", &user_id.to_string()[..8]))
    .bind("商品批量同步测试用户")
    .bind("test-hash")
    .execute(pool)
    .await
    .expect("user should seed");
    sqlx::query("INSERT INTO auth_user_owner_bindings(user_id, owner_id) VALUES ($1, $2)")
        .bind(user_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("owner binding should seed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_batch_sync_is_atomic_owner_scoped_and_idempotent(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let repo = PgMasterDataReadRepository::new(pool.clone());
    let owner = context(owner_id);
    let other_owner = context(other_owner_id);
    let requests = vec![request("P-BATCH-001"), request("P-BATCH-002")];

    let created = repo
        .batch_create_products(&owner, requests.clone(), Utc::now(), "batch-key-1")
        .await
        .expect("batch should create");
    assert_eq!(created.len(), 2);

    let replay = repo
        .batch_create_products(&owner, requests, Utc::now(), "batch-key-1")
        .await
        .expect("same batch should replay");
    assert_eq!(
        replay.iter().map(|row| row.id).collect::<Vec<_>>(),
        created.iter().map(|row| row.id).collect::<Vec<_>>()
    );

    let owner_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("owner product count");
    assert_eq!(owner_count, 2);

    let failed = repo
        .batch_create_products(
            &owner,
            vec![request("P-BATCH-003"), request("P-BATCH-001")],
            Utc::now(),
            "batch-key-2",
        )
        .await;
    assert!(failed.is_err(), "duplicate code must fail the whole batch");
    let owner_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("owner product count after rollback");
    assert_eq!(
        owner_count_after, 2,
        "failed batch must roll back its first row"
    );

    let same_code_other_owner = repo
        .batch_create_products(
            &other_owner,
            vec![request("P-BATCH-001")],
            Utc::now(),
            "batch-key-other-owner",
        )
        .await
        .expect("same product code is isolated by owner");
    assert_eq!(same_code_other_owner[0].owner_id, other_owner_id);

    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone()));
    let mut http_request = HttpRequest::builder()
        .method("POST")
        .uri("/api/v1/master-data/products/batch-sync")
        .header("content-type", "application/json")
        .header("Idempotency-Key", "batch-route-key")
        .body(Body::from(
            serde_json::to_vec(&vec![request("P-BATCH-ROUTE")]).expect("request json"),
        ))
        .expect("batch request should build");
    http_request.extensions_mut().insert(owner);
    let response = app
        .oneshot(http_request)
        .await
        .expect("batch route should respond");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let route_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("route must not create products");
    assert_eq!(route_count, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_batch_sync_rejects_missing_required_fields_without_partial_writes(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let repo = PgMasterDataReadRepository::new(pool.clone());
    let owner = context(owner_id);
    let mut invalid = request("P-BATCH-INVALID");
    invalid.spec = " ".to_string();

    let result = repo
        .batch_create_products(
            &owner,
            vec![request("P-BATCH-BEFORE"), invalid, request("P-BATCH-AFTER")],
            Utc::now(),
            "batch-required-fields",
        )
        .await;

    assert!(matches!(result, Err(MasterDataError::InvalidProductFields)));
    let product_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("product count should query");
    assert_eq!(product_count, 0, "invalid batch must roll back every row");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'batch_create_product'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should query");
    assert_eq!(audit_count, 0, "invalid batch must not retain audit rows");
    let idempotency_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'batch-required-fields'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("idempotency count should query");
    assert_eq!(
        idempotency_count, 0,
        "invalid batch must not retain idempotency state"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_patch_rejects_special_drug_category_change_without_side_effects(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let product = PgMasterDataReadRepository::new(pool.clone())
        .create_product(
            &ctx,
            request("P-CONTROLLED-CATEGORY"),
            Utc::now(),
            "controlled-category-create",
        )
        .await
        .expect("product should create");
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone()));
    let mut patch = HttpRequest::builder()
        .method("PATCH")
        .uri(format!("/api/v1/master-data/products/{}", product.id))
        .header("content-type", "application/json")
        .header("Idempotency-Key", "controlled-category-change")
        .body(Body::from(
            json!({"special_drug_category_code": "narcotic"}).to_string(),
        ))
        .expect("patch request should build");
    patch.extensions_mut().insert(ctx);

    let response = app.oneshot(patch).await.expect("patch should respond");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let error: ErrorResponse = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("error body should read"),
    )
    .expect("error body should decode");
    assert_eq!(error.code, "AUTH-005");

    let evidence: (String, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT special_drug_category, version,
               (SELECT COUNT(*) FROM audit_event
                 WHERE owner_id = $1 AND action = 'update_product'
                   AND resource_id = $2),
               (SELECT COUNT(*) FROM idempotency_request
                 WHERE owner_id = $1 AND idempotency_key = $3)
          FROM products
         WHERE owner_id = $1 AND id = $4
        "#,
    )
    .bind(owner_id)
    .bind(product.id.to_string())
    .bind("controlled-category-change")
    .bind(product.id)
    .fetch_one(&pool)
    .await
    .expect("rejected change evidence should query");
    assert_eq!(evidence, ("none".to_string(), 1, 0, 0));
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_patch_cannot_activate_pending_mapping_product(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let mut pending_request = request("P-PENDING-MAPPING");
    pending_request.packaging_levels.clear();
    pending_request.special_drug_category_code = None;
    pending_request.attrs = json!({"source": "erp_rest"});
    let product = PgMasterDataReadRepository::new(pool.clone())
        .create_product_with_mapping_traces_status(
            &ctx,
            pending_request,
            Vec::new(),
            "pending_mapping",
            Utc::now(),
            "pending-mapping-create",
        )
        .await
        .expect("pending product should create");
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone()));
    let mut patch = HttpRequest::builder()
        .method("PATCH")
        .uri(format!("/api/v1/master-data/products/{}", product.id))
        .header("content-type", "application/json")
        .header("Idempotency-Key", "pending-mapping-activate")
        .body(Body::from(json!({"status": "active"}).to_string()))
        .expect("patch request should build");
    patch.extensions_mut().insert(ctx);

    let response = app.oneshot(patch).await.expect("patch should respond");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let error: ErrorResponse = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("error body should read"),
    )
    .expect("error body should decode");
    assert_eq!(error.code, "AUTH-005");

    let evidence: (String, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT status, version,
               (SELECT COUNT(*) FROM audit_event
                 WHERE owner_id = $1 AND action = 'update_product'
                   AND resource_id = $2),
               (SELECT COUNT(*) FROM idempotency_request
                 WHERE owner_id = $1 AND idempotency_key = $3)
          FROM products
         WHERE owner_id = $1 AND id = $4
        "#,
    )
    .bind(owner_id)
    .bind(product.id.to_string())
    .bind("pending-mapping-activate")
    .bind(product.id)
    .fetch_one(&pool)
    .await
    .expect("rejected transition evidence should query");
    assert_eq!(evidence, ("pending_mapping".to_string(), 1, 0, 0));
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_routes_reject_external_api_key_writes_but_allow_reads(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_owner_and_user(&pool, owner_id, user_id).await;
    let repository = PgMasterDataReadRepository::new(pool.clone());
    let internal = AuthContext {
        user_id,
        owner_id,
        actor_name: "内部商品维护测试".to_string(),
        permissions: vec!["m1.master_data.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    };
    let existing = repository
        .create_product(
            &internal,
            request("P-API-KEY-TARGET"),
            Utc::now(),
            "product-api-key-target",
        )
        .await
        .expect("target product should seed");
    let service = ApiKeyService::new(pool.clone());
    let key = service
        .create(
            &AuthContext {
                user_id,
                owner_id,
                actor_name: "商品批量同步 API Key 测试".to_string(),
                permissions: vec!["h1.api_keys.manage".to_string()],
                jti: Uuid::new_v4().to_string(),
                warehouse_scope: None,
            },
            CreateApiKeyRequest {
                caller_name: "ERP 商品同步".to_string(),
                purpose: "商品档案批量同步".to_string(),
                warehouse_ids: Vec::new(),
                scopes: vec!["master-data:write".to_string()],
                expires_at: Some(Utc::now() + Duration::days(1)),
                responsible_user_id: user_id,
            },
            "product-batch-api-key",
        )
        .await
        .expect("api key should create")
        .secret
        .expect("api key secret should be returned once");

    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        from_fn_with_state(ApiKeyAuthState::new(pool.clone()), api_key_auth_middleware),
    );
    let write_cases = [
        (
            "POST",
            "/api/v1/master-data/products".to_string(),
            json!(request("P-DIRECT-API-KEY")),
        ),
        (
            "PATCH",
            format!("/api/v1/master-data/products/{}", existing.id),
            json!({"product_name": "API Key 不得改名"}),
        ),
        (
            "DELETE",
            format!("/api/v1/master-data/products/{}", existing.id),
            Value::Null,
        ),
        (
            "POST",
            "/api/v1/master-data/products/batch-sync".to_string(),
            json!([request("P-BATCH-API-KEY")]),
        ),
    ];
    for (index, (method, uri, body)) in write_cases.iter().enumerate() {
        let response = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method(*method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .header("X-WMS-API-Key", &key)
                    .header("Idempotency-Key", format!("product-api-key-write-{index}"))
                    .body(if body.is_null() {
                        Body::empty()
                    } else {
                        Body::from(serde_json::to_vec(body).expect("request json"))
                    })
                    .expect("request should build"),
            )
            .await
            .expect("product route should respond");
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{method} {uri} must reject external API keys"
        );
    }

    let read = app
        .oneshot(
            HttpRequest::builder()
                .method("GET")
                .uri("/api/v1/master-data/products")
                .header("X-WMS-API-Key", key)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("product list should respond");
    assert_eq!(read.status(), StatusCode::OK);

    let product_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("external product count");
    assert_eq!(product_count, 1);
    let unchanged: (String, String) =
        sqlx::query_as("SELECT product_name, status FROM products WHERE id = $1")
            .bind(existing.id)
            .fetch_one(&pool)
            .await
            .expect("target product should remain");
    assert_eq!(
        unchanged,
        ("商品 P-API-KEY-TARGET".to_string(), "active".to_string())
    );

    let audited: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND resource_type = 'api_key_request'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("api key request audits should exist");
    assert_eq!(audited, 5);
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_routes_reject_internal_jwt_writes_but_allow_reads(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let existing = PgMasterDataReadRepository::new(pool.clone())
        .create_product(
            &ctx,
            request("P-ERP-ONLY-TARGET"),
            Utc::now(),
            "erp-only-target",
        )
        .await
        .expect("target product should seed through the controlled repository");
    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone()));
    let write_cases = [
        (
            "POST",
            "/api/v1/master-data/products".to_string(),
            json!(request("P-DIRECT-JWT")),
        ),
        (
            "PATCH",
            format!("/api/v1/master-data/products/{}", existing.id),
            json!({"product_name": "JWT 不得直接改名"}),
        ),
        (
            "DELETE",
            format!("/api/v1/master-data/products/{}", existing.id),
            Value::Null,
        ),
        (
            "POST",
            "/api/v1/master-data/products/batch-sync".to_string(),
            json!([request("P-BATCH-JWT")]),
        ),
    ];
    for (index, (method, uri, body)) in write_cases.iter().enumerate() {
        let mut request = HttpRequest::builder()
            .method(*method)
            .uri(uri)
            .header("content-type", "application/json")
            .header("Idempotency-Key", format!("product-jwt-write-{index}"))
            .body(if body.is_null() {
                Body::empty()
            } else {
                Body::from(serde_json::to_vec(body).expect("request json"))
            })
            .expect("request should build");
        request.extensions_mut().insert(ctx.clone());
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("product route should respond");
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{method} {uri} must reject direct JWT product writes"
        );
    }

    let mut read = HttpRequest::builder()
        .method("GET")
        .uri("/api/v1/master-data/products")
        .body(Body::empty())
        .expect("request should build");
    read.extensions_mut().insert(ctx);
    let response = app
        .oneshot(read)
        .await
        .expect("product list should respond");
    assert_eq!(response.status(), StatusCode::OK);

    let stored: (String, String) =
        sqlx::query_as("SELECT product_name, status FROM products WHERE id = $1")
            .bind(existing.id)
            .fetch_one(&pool)
            .await
            .expect("target product should remain");
    assert_eq!(
        stored,
        ("商品 P-ERP-ONLY-TARGET".to_string(), "active".to_string())
    );
}
