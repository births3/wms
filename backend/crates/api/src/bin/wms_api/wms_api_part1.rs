use std::sync::{Arc, Mutex, OnceLock};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    audit::{append_event, AuditWriteRequest},
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthRevocationStore,
        AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
    },
    feature_flags::FeatureFlagRegistry,
};
use wms_domain::{AuditEventListResponse, CurrentUser, LoginRequest, LoginResponse};

use super::*;

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

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

fn config_center_state() -> ConfigCenterAppState {
    let registry = FeatureFlagRegistry::from_toml_str(
        r#"
            [[flags]]
            key = "m3_inventory_batches_config_center_smoke"
            owner = "platform"
            created_at = 2026-06-04
            cleanup_by = 2026-08-31
            enabled = true
            "#,
    )
    .expect("test registry should parse");
    ConfigCenterAppState::from_registry(registry)
}

fn bearer_token(owner_id: Uuid) -> String {
    bearer_token_with_permissions(owner_id, vec!["audit.read".to_string()])
}

fn bearer_token_with_permissions(owner_id: Uuid, permissions: Vec<String>) -> String {
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "audit-reader",
        permissions,
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-secret").expect("token should encode")
}

fn with_env_lock<T>(f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock should not be poisoned");
    f()
}

async fn seed_auth_user(pool: &PgPool, owner_id: Uuid, user_id: Uuid, role_id: Uuid) {
    let password_hash = bcrypt::hash("CorrectHorse1!", 4).expect("password should hash");
    sqlx::query(
        r#"
            INSERT INTO auth_owners (id, owner_code, owner_name)
            VALUES ($1, 'PY_OWNER', '鹏鹞药业')
            "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("owner should insert");
    sqlx::query(
        r#"
            INSERT INTO auth_users (id, username, display_name, password_hash, status)
            VALUES ($1, 'admin', '系统管理员', $2, 'active')
            "#,
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(pool)
    .await
    .expect("user should insert");
    sqlx::query(
        r#"
            INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary)
            VALUES ($1, $2, true, true)
            "#,
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("user owner binding should insert");
    sqlx::query(
        r#"
            INSERT INTO auth_roles (id, owner_id, role_code, role_name)
            VALUES ($1, $2, 'audit_reader', '审计查询员')
            "#,
    )
    .bind(role_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("role should insert");
    sqlx::query(
        r#"
            INSERT INTO auth_user_roles (user_id, owner_id, role_id)
            VALUES ($1, $2, $3)
            "#,
    )
    .bind(user_id)
    .bind(owner_id)
    .bind(role_id)
    .execute(pool)
    .await
    .expect("user role should insert");
    sqlx::query(
        r#"
            INSERT INTO auth_role_permissions (role_id, permission_id)
            SELECT $1, id
              FROM auth_permissions
             WHERE lower(permission_code) = 'audit.read'
            "#,
    )
    .bind(role_id)
    .execute(pool)
    .await
    .expect("role permission should insert");
}

#[tokio::test]
async fn audit_events_route_requires_auth_and_permission() {
    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/wms")
        .expect("lazy pool should not connect during auth rejection test");
    let app = app(
        config_center_state(),
        AuthAppState::new(pool.clone()),
        Wave3AppState::default(),
        Wave4AppState::with_postgres(pool.clone()),
        Wave5AppState::with_postgres(pool.clone()),
        ExpressAppState::with_postgres(pool.clone()),
        AuditQueryState { pool: pool.clone() },
        MasterDataAppState::default(),
        SystemDictionaryAppState::with_postgres(pool),
    )
    .layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
        AllowAllRevocationStore,
    ))));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit/events")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit/events")
                .header(
                    "authorization",
                    format!(
                        "Bearer {}",
                        bearer_token_with_permissions(Uuid::new_v4(), vec![])
                    ),
                )
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn h2_lifecycle_routes_require_auth_context() {
    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/wms")
        .expect("lazy pool should not connect during auth rejection test");
    let app = app(
        config_center_state(),
        AuthAppState::new(pool.clone()),
        Wave3AppState::default(),
        Wave4AppState::with_postgres(pool.clone()),
        Wave5AppState::with_postgres(pool.clone()),
        ExpressAppState::with_postgres(pool.clone()),
        AuditQueryState { pool: pool.clone() },
        MasterDataAppState::default(),
        SystemDictionaryAppState::with_postgres(pool),
    )
    .layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
        AllowAllRevocationStore,
    ))));

    for uri in [
        "/api/v1/audit/archive/partitions",
        "/api/v1/event-bus/deliveries/pending",
        "/api/v1/business-retention/policies",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
    }
}

#[tokio::test]
async fn master_data_route_is_mounted_under_auth_context() {
    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/wms")
        .expect("lazy pool should not connect during auth rejection test");
    let app = app(
        config_center_state(),
        AuthAppState::new(pool.clone()),
        Wave3AppState::default(),
        Wave4AppState::with_postgres(pool.clone()),
        Wave5AppState::with_postgres(pool.clone()),
        ExpressAppState::with_postgres(pool.clone()),
        AuditQueryState { pool: pool.clone() },
        MasterDataAppState::default(),
        SystemDictionaryAppState::with_postgres(pool),
    )
    .layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
        AllowAllRevocationStore,
    ))));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/master-data/products")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn runtime_mounts_wave4_and_wave5_routes() {
    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/wms")
        .expect("lazy pool should not connect during runtime route test");
    let app = app(
        config_center_state(),
        AuthAppState::new(pool.clone()),
        Wave3AppState::default(),
        Wave4AppState::with_postgres(pool.clone()),
        Wave5AppState::with_postgres(pool.clone()),
        ExpressAppState::with_postgres(pool.clone()),
        AuditQueryState { pool: pool.clone() },
        MasterDataAppState::default(),
        SystemDictionaryAppState::with_postgres(pool),
    );

    for uri in ["/api/v1/outbound/orders", "/api/v1/packing/stations"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
    }

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/code-generator/document-number-allocations")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn h3_docs_routes_are_public() {
    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/wms")
        .expect("lazy pool should not connect during docs route test");
    let app = with_env_lock(|| {
        std::env::remove_var("WMS_API_DOCS_MODE");
        app(
            config_center_state(),
            AuthAppState::new(pool.clone()),
            Wave3AppState::default(),
            Wave4AppState::with_postgres(pool.clone()),
            Wave5AppState::with_postgres(pool.clone()),
            ExpressAppState::with_postgres(pool.clone()),
            AuditQueryState { pool: pool.clone() },
            MasterDataAppState::default(),
            SystemDictionaryAppState::with_postgres(pool),
        )
    });

    for uri in ["/openapi.json", "/api-docs", "/api/v1/resilience/status"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK, "{uri}");
    }
}

#[tokio::test]
async fn h3_resilience_layer_rate_limits_with_retry_after() {
    let app = Router::new()
        .route("/limited", get(healthz))
        .layer(from_fn_with_state(
            wms_api::resilience::ResilienceState::new_for_test(1, 1, 10, 30),
            wms_api::resilience::resilience_middleware,
        ));

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/limited")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(
            Request::builder()
                .uri("/limited")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        second
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok()),
        Some("1")
    );
}

#[tokio::test]
async fn h3_resilience_layer_opens_circuit_after_failures() {
    async fn failing() -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    let app = Router::new()
        .route("/dependency", get(failing))
        .layer(from_fn_with_state(
            wms_api::resilience::ResilienceState::new_for_test(100, 100, 1, 30),
            wms_api::resilience::resilience_middleware,
        ));

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/dependency")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(first.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let second = app
        .oneshot(
            Request::builder()
                .uri("/dependency")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(second.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        second
            .headers()
            .get("x-wms-circuit-state")
            .and_then(|v| v.to_str().ok()),
        Some("open")
    );
}

#[tokio::test]
async fn h3_resilience_half_open_closes_after_recovery() {
    async fn failing() -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    let state = wms_api::resilience::ResilienceState::new_for_test(100, 100, 1, 0);
    let failing_app = Router::new()
        .route("/dependency", get(failing))
        .layer(from_fn_with_state(
            state.clone(),
            wms_api::resilience::resilience_middleware,
        ));
    let failed = failing_app
        .oneshot(
            Request::builder()
                .uri("/dependency")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(state.status().circuit_state, "half_open");

    let recovered_app = Router::new()
        .route("/dependency", get(healthz))
        .layer(from_fn_with_state(
            state.clone(),
            wms_api::resilience::resilience_middleware,
        ));
    let recovered = recovered_app
        .oneshot(
            Request::builder()
                .uri("/dependency")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(recovered.status(), StatusCode::OK);
    assert_eq!(state.status().circuit_state, "closed");
}
