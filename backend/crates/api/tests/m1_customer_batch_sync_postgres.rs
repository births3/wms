use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    middleware::from_fn_with_state,
};
use chrono::{Duration, Utc};
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
use wms_domain::{CreateApiKeyRequest, CreateCustomerRequest, CustomerListResponse};

fn context(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m1-customer-batch-test".to_string(),
        permissions: vec!["m1.master_data.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn request(code: &str) -> CreateCustomerRequest {
    CreateCustomerRequest {
        customer_code: code.to_string(),
        customer_name: format!("客户 {code}"),
        license_no: Some(format!("LIC-{code}")),
        source: Some("api_import".to_string()),
    }
}

async fn seed_owner_and_user(pool: &PgPool, owner_id: Uuid, user_id: Uuid) {
    sqlx::query("INSERT INTO auth_owners(id, owner_code, owner_name) VALUES ($1, $2, $3)")
        .bind(owner_id)
        .bind(format!("OWNER-{}", &owner_id.to_string()[..8]))
        .bind("客户批量同步测试货主")
        .execute(pool)
        .await
        .expect("owner should seed");
    sqlx::query(
        "INSERT INTO auth_users(id, username, display_name, password_hash) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(format!("customer-batch-{}", &user_id.to_string()[..8]))
    .bind("客户批量同步测试用户")
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
async fn customer_batch_sync_is_atomic_owner_scoped_idempotent_and_routed(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let repo = PgMasterDataReadRepository::new(pool.clone());
    let owner = context(owner_id);
    let other_owner = context(other_owner_id);
    let requests = vec![request("C-BATCH-001"), request("C-BATCH-002")];

    let created = repo
        .batch_create_customers(&owner, requests.clone(), Utc::now(), "customer-batch-key-1")
        .await
        .expect("batch should create");
    assert_eq!(created.len(), 2);

    let replay = repo
        .batch_create_customers(&owner, requests, Utc::now(), "customer-batch-key-1")
        .await
        .expect("same batch should replay");
    assert_eq!(
        replay.iter().map(|row| row.id).collect::<Vec<_>>(),
        created.iter().map(|row| row.id).collect::<Vec<_>>()
    );

    sqlx::query(
        "UPDATE idempotency_request SET method = 'PATCH', path = '/wrong-path' WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_id)
    .bind("customer-batch-key-1")
    .execute(&pool)
    .await
    .expect("idempotency metadata should be mutable for the regression check");
    let metadata_conflict = repo
        .batch_create_customers(
            &owner,
            vec![request("C-BATCH-001"), request("C-BATCH-002")],
            Utc::now(),
            "customer-batch-key-1",
        )
        .await;
    assert!(matches!(
        metadata_conflict,
        Err(MasterDataError::IdempotencyConflict)
    ));
    sqlx::query(
        "UPDATE idempotency_request SET method = 'POST', path = '/api/v1/master-data/customers/batch-sync' WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_id)
    .bind("customer-batch-key-1")
    .execute(&pool)
    .await
    .expect("idempotency metadata should be restored for the hash regression check");

    let conflict = repo
        .batch_create_customers(
            &owner,
            vec![request("C-BATCH-DIFFERENT")],
            Utc::now(),
            "customer-batch-key-1",
        )
        .await;
    assert!(matches!(
        conflict,
        Err(MasterDataError::IdempotencyConflict)
    ));

    let failed = repo
        .batch_create_customers(
            &owner,
            vec![request("C-BATCH-003"), request("C-BATCH-001")],
            Utc::now(),
            "customer-batch-key-2",
        )
        .await;
    assert!(failed.is_err(), "duplicate code must fail the whole batch");

    let owner_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM customers WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("owner customer count");
    assert_eq!(owner_count, 2, "failed batch must roll back its first row");

    let same_code_other_owner = repo
        .batch_create_customers(
            &other_owner,
            vec![request("C-BATCH-001")],
            Utc::now(),
            "customer-batch-key-other-owner",
        )
        .await
        .expect("same customer code is isolated by owner");
    assert_eq!(same_code_other_owner[0].owner_id, other_owner_id);

    let audited: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND resource_type = 'customer' AND action = 'batch_create_customer'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("customer batch audit count");
    assert_eq!(audited, 2);

    let app = master_data_router(MasterDataAppState::with_postgres(pool));
    let mut http_request = Request::builder()
        .method("POST")
        .uri("/api/v1/master-data/customers/batch-sync")
        .header("content-type", "application/json")
        .header("Idempotency-Key", "customer-batch-route-key")
        .body(Body::from(
            serde_json::to_vec(&vec![request("C-BATCH-ROUTE")]).expect("request json"),
        ))
        .expect("batch request should build");
    http_request.extensions_mut().insert(owner);
    let response = app
        .oneshot(http_request)
        .await
        .expect("batch route should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let response: CustomerListResponse = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("batch response body"),
    )
    .expect("batch response should decode");
    assert_eq!(response.data.len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn customer_batch_sync_accepts_api_key_and_records_request_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_owner_and_user(&pool, owner_id, user_id).await;
    let service = ApiKeyService::new(pool.clone());
    let key = service
        .create(
            &AuthContext {
                user_id,
                owner_id,
                actor_name: "客户批量同步 API Key 测试".to_string(),
                permissions: vec!["h1.api_keys.manage".to_string()],
                jti: Uuid::new_v4().to_string(),
                warehouse_scope: None,
            },
            CreateApiKeyRequest {
                caller_name: "ERP 客户同步".to_string(),
                purpose: "客户档案批量同步".to_string(),
                warehouse_ids: Vec::new(),
                scopes: vec!["master-data:write".to_string()],
                expires_at: Some(Utc::now() + Duration::days(1)),
                responsible_user_id: user_id,
            },
            "customer-batch-api-key-create",
        )
        .await
        .expect("api key should create")
        .secret
        .expect("api key secret should be returned once");

    let app = master_data_router(MasterDataAppState::with_postgres(pool.clone())).layer(
        from_fn_with_state(ApiKeyAuthState::new(pool.clone()), api_key_auth_middleware),
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/master-data/customers/batch-sync")
                .header("content-type", "application/json")
                .header("X-WMS-API-Key", key)
                .header("Idempotency-Key", "customer-batch-api-key-request")
                .body(Body::from(
                    serde_json::to_vec(&vec![request("C-BATCH-API-KEY")]).expect("request json"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("batch route should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let response: CustomerListResponse = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("batch response body"),
    )
    .expect("batch response should decode");
    assert_eq!(response.data[0].owner_id, owner_id);

    let audited: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND resource_type = 'api_key_request' AND diff->'after'->>'path' = '/api/v1/master-data/customers/batch-sync'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("api key request audit should exist");
    assert_eq!(audited, 1);
}
