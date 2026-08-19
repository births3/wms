use axum::{
    body::{to_bytes, Body},
    extract::Json,
    http::{Request, StatusCode},
    middleware::from_fn_with_state,
    routing::get,
    Router,
};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    api_key_auth::{api_key_auth_middleware, ApiKeyAuthState},
    api_key_expiry::notify_expiring_api_keys,
    api_key_handlers::{api_key_router, ApiKeyManagementState},
    api_key_service::{ApiKeyAuthError, ApiKeyService},
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthContext,
        AuthRevocationStore, AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
    },
};
use wms_domain::{
    ApiKey, ApiKeyListResponse, ApiKeyRotationResponse, CreateApiKeyRequest, ErrorResponse,
    RotateApiKeyRequest,
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

fn bearer_token(owner_id: Uuid, permissions: &[&str]) -> String {
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "system-admin",
        permissions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-secret").expect("test token should encode")
}

async fn seed_owner_and_user(pool: &PgPool, owner_id: Uuid, user_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners(id, owner_code, owner_name) VALUES ($1, 'OWNER_A', '货主 A')",
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("owner should seed");
    sqlx::query("INSERT INTO auth_users(id, username, display_name, password_hash) VALUES ($1, 'admin', '系统管理员', 'test-hash')")
        .bind(user_id).execute(pool).await.expect("user should seed");
    sqlx::query("INSERT INTO auth_user_owner_bindings(user_id, owner_id) VALUES ($1, $2)")
        .bind(user_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("binding should seed");
}

fn app(pool: PgPool) -> Router {
    api_key_router(ApiKeyManagementState::new(pool)).layer(auth_runtime_layer(
        AuthRuntimePolicy::new(std::sync::Arc::new(AllowAllRevocationStore)),
    ))
}

fn create_request(
    responsible_user_id: Uuid,
    expires_at: Option<chrono::DateTime<Utc>>,
) -> CreateApiKeyRequest {
    CreateApiKeyRequest {
        caller_name: "ERP 测试系统".to_string(),
        purpose: "推送入库单".to_string(),
        warehouse_ids: Vec::new(),
        scopes: vec!["inbound:push".to_string()],
        expires_at,
        responsible_user_id,
    }
}

async fn json_body<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read");
    serde_json::from_slice(&body).expect("response body should be json")
}

#[sqlx::test(migrations = "../../migrations")]
async fn api_key_lifecycle_uses_hash_once_idempotency_rotation_revoke_and_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_owner_and_user(&pool, owner_id, user_id).await;
    let token = bearer_token(owner_id, &["h1.api_keys.manage"]);
    let request = create_request(user_id, Some(Utc::now() + Duration::days(30)));
    let body = serde_json::to_vec(&request).expect("request should encode");
    let response = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/api-keys")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "create-1")
                .body(Body::from(body.clone()))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let created: ApiKey = json_body(response).await;
    let secret = created.secret.clone().expect("secret should be shown once");

    let stored: (String,) = sqlx::query_as("SELECT key_hash FROM auth_api_keys WHERE id = $1")
        .bind(created.key_id)
        .fetch_one(&pool)
        .await
        .expect("key should persist");
    assert_ne!(stored.0, secret);
    assert!(!stored.0.contains(&secret));
    let plaintext_columns: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM information_schema.columns WHERE table_name = 'auth_api_keys' AND column_name IN ('secret', 'plaintext_secret')")
        .fetch_one(&pool).await.expect("schema should be queryable");
    assert_eq!(
        plaintext_columns, 0,
        "database must not have a plaintext secret column"
    );

    let replay = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/api-keys")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "create-1")
                .body(Body::from(body.clone()))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    let replayed: ApiKey = json_body(replay).await;
    assert_eq!(replayed.key_id, created.key_id);
    assert!(
        replayed.secret.is_none(),
        "idempotent replay must not redisplay secret"
    );
    sqlx::query(
        "UPDATE idempotency_request SET method = 'PATCH', path = '/wrong-path' WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_id)
    .bind("create-1")
    .execute(&pool)
    .await
    .expect("idempotency metadata should be mutable for the regression check");
    let metadata_conflict = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/api-keys")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "create-1")
                .body(Body::from(body))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(metadata_conflict.status(), StatusCode::CONFLICT);

    let listed: ApiKeyListResponse = json_body(
        app(pool.clone())
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/api-keys")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond"),
    )
    .await;
    assert_eq!(listed.data.len(), 1);
    assert!(listed.data[0].secret.is_none());

    let rotate = RotateApiKeyRequest {
        grace_period_days: Some(2),
        expires_at: None,
    };
    let rotated: ApiKeyRotationResponse = json_body(
        app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/auth/api-keys/{}/rotate", created.key_id))
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .header("Idempotency-Key", "rotate-1")
                    .body(Body::from(
                        serde_json::to_vec(&rotate).expect("rotate should encode"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond"),
    )
    .await;
    assert_eq!(rotated.previous_key_id, created.key_id);
    assert_eq!(rotated.new_key.owner_id, owner_id);
    assert!(rotated.new_key.secret.is_some());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM auth_api_keys WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("keys should count"),
        2
    );

    let revoke_uri = format!("/api/v1/auth/api-keys/{}/revoke", rotated.new_key.key_id);
    for key in ["revoke-1", "revoke-2"] {
        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&revoke_uri)
                    .header("authorization", format!("Bearer {token}"))
                    .header("Idempotency-Key", key)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H1' AND resource_type = 'api_key'").bind(owner_id).fetch_one(&pool).await.expect("audit should count"), 3);
}

#[sqlx::test(migrations = "../../migrations")]
async fn api_key_rejects_invalid_scope_expiry_and_non_admin(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_owner_and_user(&pool, owner_id, user_id).await;
    let request = create_request(user_id, Some(Utc::now() - Duration::minutes(1)));
    let token = bearer_token(owner_id, &["h1.api_keys.manage"]);
    let mut invalid_scope = request.clone();
    invalid_scope.scopes = vec!["not-allowed".to_string()];
    for (body, expected) in [
        (invalid_scope, "H1_APIKEY_INVALID_SCOPE"),
        (request, "H1_APIKEY_INVALID_EXPIRY"),
    ] {
        let response = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/api-keys")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .header("Idempotency-Key", Uuid::new_v4().to_string())
                    .body(Body::from(
                        serde_json::to_vec(&body).expect("body should encode"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let error: ErrorResponse = json_body(response).await;
        assert_eq!(error.code, expected);
    }
    let response = app(pool)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/api-keys")
                .header(
                    "authorization",
                    format!("Bearer {}", bearer_token(owner_id, &["audit.read"])),
                )
                .header("content-type", "application/json")
                .header("Idempotency-Key", "not-admin")
                .body(Body::from(
                    serde_json::to_vec(&create_request(user_id, None)).expect("body should encode"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn api_key_authentication_uses_postgres_failures_rate_limit_owner_and_scope(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_owner_and_user(&pool, owner_id, user_id).await;
    let service = ApiKeyService::new(pool.clone());
    let context = AuthContext {
        user_id,
        owner_id,
        actor_name: "system-admin".to_string(),
        permissions: vec!["h1.api_keys.manage".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    };
    let created = service
        .create(&context, create_request(user_id, None), "test-create")
        .await
        .expect("key should create");
    let secret = created
        .secret
        .clone()
        .expect("test secret should be returned");
    let wrong = format!("{secret}-wrong");
    for _ in 0..9 {
        assert!(matches!(
            service
                .authenticate(&wrong, owner_id, "inbound:push", None, None)
                .await,
            Err(ApiKeyAuthError::Invalid)
        ));
    }
    assert!(matches!(
        service
            .authenticate(&wrong, owner_id, "inbound:push", None, None)
            .await,
        Err(ApiKeyAuthError::TemporarilyDisabled)
    ));
    let scope_key = service
        .create(&context, create_request(user_id, None), "scope-create")
        .await
        .expect("scope key should create")
        .secret
        .expect("scope key secret should be returned");
    assert!(matches!(
        service
            .authenticate(&scope_key, Uuid::new_v4(), "inbound:push", None, None)
            .await,
        Err(ApiKeyAuthError::CrossOwner)
    ));
    assert!(matches!(
        service
            .authenticate(&scope_key, owner_id, "master-data:write", None, None)
            .await,
        Err(ApiKeyAuthError::InvalidScope)
    ));
}

async fn external_owner_handler(ctx: AuthContext) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "owner_id": ctx.owner_id,
        "actor": ctx.actor_name,
    }))
}

/// 与生产 `required_scope` 对齐：仅 ASN 创建路径 `POST/GET /api/v1/inbound/receiving-orders`
/// 接受 `inbound:push`；测试用 GET 探针验证鉴权注入与仓库范围。
const ASN_PUSH_PATH: &str = "/api/v1/inbound/receiving-orders";

fn external_app(pool: PgPool) -> Router {
    Router::new()
        .route(ASN_PUSH_PATH, get(external_owner_handler))
        .layer(from_fn_with_state(
            ApiKeyAuthState::new(pool),
            api_key_auth_middleware,
        ))
}

fn external_resilience_app(pool: PgPool) -> Router {
    let resilience =
        wms_api::resilience::ResilienceState::new(wms_api::resilience::ResilienceConfig {
            global_qps: 100,
            global_burst: 100,
            user_qps: 100,
            user_burst: 100,
            api_key_qps: 1,
            api_key_burst: 1,
            retry_after_seconds: 1,
            circuit_failures: 10,
            circuit_open_seconds: 30,
        })
        .with_audit_pool(pool.clone());
    Router::new()
        .route(ASN_PUSH_PATH, get(external_owner_handler))
        .layer(from_fn_with_state(
            resilience,
            wms_api::resilience::resilience_middleware,
        ))
        .layer(from_fn_with_state(
            ApiKeyAuthState::new(pool),
            api_key_auth_middleware,
        ))
}

#[sqlx::test(migrations = "../../migrations")]
async fn external_api_key_auth_injects_owner_and_audits_request(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_owner_and_user(&pool, owner_id, user_id).await;
    let service = ApiKeyService::new(pool.clone());
    let context = AuthContext {
        user_id,
        owner_id,
        actor_name: "system-admin".to_string(),
        permissions: vec!["h1.api_keys.manage".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    };
    let key = service
        .create(
            &context,
            create_request(user_id, None),
            "external-auth-create",
        )
        .await
        .expect("key should create")
        .secret
        .expect("secret should return once");
    let response = external_app(pool.clone())
        .oneshot(
            Request::builder()
                .uri(ASN_PUSH_PATH)
                .header("X-WMS-API-Key", &key)
                .header("X-Forwarded-For", "10.0.0.8, 10.0.0.9")
                .header("User-Agent", "external-test")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = json_body(response).await;
    assert_eq!(body["owner_id"], owner_id.to_string());
    let audit: (String, String, String, Option<String>) = sqlx::query_as(
        "SELECT action, resource_type, diff->'after'->>'path', host(ip) FROM audit_event WHERE owner_id = $1 AND resource_type = 'api_key_request' ORDER BY id DESC LIMIT 1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("request audit should exist");
    assert_eq!(audit.0, "auth.api_key.request");
    assert_eq!(audit.1, "api_key_request");
    assert_eq!(audit.2, ASN_PUSH_PATH);
    assert_eq!(audit.3.as_deref(), Some("10.0.0.8"));

    // 作业路径不得再映射 inbound:push（中间件不认证 → 无 AuthContext → 401）
    let receive_path = format!(
        "/api/v1/inbound/receiving-orders/{}/receive",
        Uuid::new_v4()
    );
    let blocked = Router::new()
        .route(&receive_path, get(external_owner_handler))
        .layer(from_fn_with_state(
            ApiKeyAuthState::new(pool.clone()),
            api_key_auth_middleware,
        ))
        .oneshot(
            Request::builder()
                .uri(&receive_path)
                .header("X-WMS-API-Key", &key)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(blocked.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn external_api_key_resilience_audit_uses_real_key_context(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_owner_and_user(&pool, owner_id, user_id).await;
    let service = ApiKeyService::new(pool.clone());
    let context = AuthContext {
        user_id,
        owner_id,
        actor_name: "system-admin".to_string(),
        permissions: vec!["h1.api_keys.manage".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    };
    let created = service
        .create(
            &context,
            create_request(user_id, None),
            "external-resilience-create",
        )
        .await
        .expect("key should create");
    let key_id = created.key_id;
    let secret = created.secret.expect("secret should return once");
    let app = external_resilience_app(pool.clone());

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(ASN_PUSH_PATH)
                .header("X-WMS-API-Key", &secret)
                .header("X-Forwarded-For", "10.0.0.8, 10.0.0.9")
                .header("User-Agent", "resilience-e2e")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let rejected = app
        .oneshot(
            Request::builder()
                .uri(ASN_PUSH_PATH)
                .header("X-WMS-API-Key", &secret)
                .header("X-Forwarded-For", "10.0.0.8, 10.0.0.9")
                .header("User-Agent", "resilience-e2e")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(rejected.status(), StatusCode::TOO_MANY_REQUESTS);

    let audit: (Uuid, String, String, String, String, String, String, Option<String>) =
        sqlx::query_as(
            "SELECT actor_id, actor_name, jti, diff->'after'->>'path', diff->'after'->>'status_code', host(ip), user_agent, action FROM audit_event WHERE owner_id = $1 AND module = 'H3' AND resource_type = 'api_resilience' ORDER BY id DESC LIMIT 1",
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("H3 audit should exist");
    assert_eq!(audit.0, key_id);
    assert_eq!(audit.1, "API Key / ERP 测试系统");
    assert_eq!(audit.2, format!("api-key:{key_id}"));
    assert_eq!(audit.3, ASN_PUSH_PATH);
    assert_eq!(audit.4, "429");
    assert_eq!(audit.5, "10.0.0.8");
    assert_eq!(audit.6, "resilience-e2e");
    assert_eq!(audit.7.as_deref(), Some("h3.rate_limited"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn api_key_expiry_reminder_is_deduplicated_through_h4(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_owner_and_user(&pool, owner_id, user_id).await;
    let service = ApiKeyService::new(pool.clone());
    let context = AuthContext {
        user_id,
        owner_id,
        actor_name: "system-admin".to_string(),
        permissions: vec!["h1.api_keys.manage".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    };
    let key = service
        .create(
            &context,
            create_request(user_id, Some(Utc::now() + Duration::days(5))),
            "expiry-create",
        )
        .await
        .expect("key should create");
    let now = Utc::now();
    assert_eq!(
        notify_expiring_api_keys(&pool, now)
            .await
            .expect("expiry task should run"),
        1
    );
    notify_expiring_api_keys(&pool, now)
        .await
        .expect("replayed expiry task should run");
    let records: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM h4_notification_records WHERE owner_id = $1 AND event_type = 'auth.api_key.expiring'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("H4 record should exist");
    assert_eq!(records, 1);
    let config_template: String = sqlx::query_scalar(
        "SELECT template FROM h4_notification_configs WHERE owner_id = $1 AND event_type = 'auth.api_key.expiring'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("default expiry config should exist");
    assert!(config_template.contains("{{caller_name}}"));
    assert!(key.secret.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn external_api_key_enforces_configured_warehouse_scope(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    seed_owner_and_user(&pool, owner_id, user_id).await;
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, 'E2E-WH', 'E2E 仓库', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("warehouse should seed");
    let service = ApiKeyService::new(pool.clone());
    let context = AuthContext {
        user_id,
        owner_id,
        actor_name: "system-admin".to_string(),
        permissions: vec!["h1.api_keys.manage".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    };
    let mut request = create_request(user_id, None);
    request.warehouse_ids = vec![warehouse_id];
    let key = service
        .create(&context, request, "warehouse-scope-create")
        .await
        .expect("scoped key should create")
        .secret
        .expect("secret should return once");

    let missing_header = external_app(pool.clone())
        .oneshot(
            Request::builder()
                .uri(ASN_PUSH_PATH)
                .header("X-WMS-API-Key", &key)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(missing_header.status(), StatusCode::FORBIDDEN);

    let foreign_warehouse = Uuid::new_v4();
    let cross_warehouse = external_app(pool.clone())
        .oneshot(
            Request::builder()
                .uri(ASN_PUSH_PATH)
                .header("X-WMS-API-Key", &key)
                .header("X-WMS-Warehouse-ID", foreign_warehouse.to_string())
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(cross_warehouse.status(), StatusCode::FORBIDDEN);

    let allowed = external_app(pool)
        .oneshot(
            Request::builder()
                .uri(ASN_PUSH_PATH)
                .header("X-WMS-API-Key", &key)
                .header("X-WMS-Warehouse-ID", warehouse_id.to_string())
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(allowed.status(), StatusCode::OK);
    let allowed_body: serde_json::Value = json_body(allowed).await;
    assert_eq!(allowed_body["owner_id"], owner_id.to_string());
}
