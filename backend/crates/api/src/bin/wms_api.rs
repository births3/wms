use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    middleware::from_fn_with_state,
    response::{Html, IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use std::{env, error::Error, io, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use utoipa::OpenApi;
use wms_api::ApiDoc;
use wms_api::{
    admin_menu_handlers::{admin_menu_router, AdminMenuAppState},
    auth::{auth_runtime_layer, AuthRuntimePolicy, RedisAuthRevocationStore, JWT_SECRET_ENV},
    auth_handlers::{auth_router, AuthAppState},
    config_center::{config_center_router, ConfigCenterAppState},
    document_numbering_handlers::{document_numbering_router, DocumentNumberingAppState},
    express::{express_router, ExpressAppState},
    feature_flags::FeatureFlagRegistry,
    h2_lifecycle_handlers::{h2_lifecycle_router, H2LifecycleAppState},
    master_data_handlers::{master_data_router, MasterDataAppState},
    print_template_handlers::{print_template_router, PrintTemplateAppState},
    reports_handlers::mount_reports,
    resilience::{resilience_middleware, resilience_status, ResilienceState},
    state_machine::state_machine_router,
    system_dictionary_handlers::{system_dictionary_router, SystemDictionaryAppState},
    wave3_handlers::{wave3_router, Wave3AppState},
    wave4_handlers::{wave4_router, Wave4AppState},
    wave5_handlers::{wave5_router, Wave5AppState},
    wechat_notify::{wechat_notify_router, WechatNotifyAppState},
};
use wms_domain::HealthzResponse;

#[path = "wms_api/audit_query.rs"]
mod wms_api_audit_query;
use wms_api_audit_query::{audit_query_router, AuditQueryState};
const BIND_ADDR_ENV: &str = "WMS_BIND_ADDR";
const REDIS_URL_ENV: &str = "WMS_REDIS_URL";
const DATABASE_URL_ENV: &str = "DATABASE_URL";
const WMS_DB_URL_ENV: &str = "WMS_DB_URL";
const DB_MAX_CONNECTIONS_ENV: &str = "WMS_DB_MAX_CONNECTIONS";
const FEATURE_FLAGS_FILE_ENV: &str = "WMS_FEATURE_FLAGS_FILE";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:8080";
const DEFAULT_DB_MAX_CONNECTIONS: u32 = 32;
const DEFAULT_FEATURE_FLAGS_FILE: &str = "deploy/feature_flags.toml";
const API_DOCS_MODE_ENV: &str = "WMS_API_DOCS_MODE";
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bind_addr = env::var(BIND_ADDR_ENV)
        .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string())
        .parse::<SocketAddr>()?;
    let jwt_secret = env::var(JWT_SECRET_ENV).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{JWT_SECRET_ENV} is required"),
        )
    })?;
    if jwt_secret.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{JWT_SECRET_ENV} must not be empty"),
        )
        .into());
    }
    let redis_url = env::var(REDIS_URL_ENV).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{REDIS_URL_ENV} is required"),
        )
    })?;
    let database_url = database_url()?;
    let feature_flags_file = PathBuf::from(
        env::var(FEATURE_FLAGS_FILE_ENV).unwrap_or_else(|_| DEFAULT_FEATURE_FLAGS_FILE.to_string()),
    );
    let file_registry = FeatureFlagRegistry::from_file(&feature_flags_file).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "failed to load feature flags from {}: {error:?}",
                feature_flags_file.display()
            ),
        )
    })?;
    let revocation_store = RedisAuthRevocationStore::from_url(&redis_url)
        .await
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("failed to configure Redis auth revocation store: {error:?}"),
            )
        })?;
    let pool = PgPoolOptions::new()
        .max_connections(database_max_connections()?)
        .after_connect(|connection, _metadata| {
            Box::pin(async move {
                sqlx::query("SET jit = off").execute(connection).await?;
                Ok(())
            })
        })
        .connect(&database_url)
        .await
        .map_err(|error| io::Error::other(format!("failed to connect PostgreSQL: {error:?}")))?;
    let config_center_state = ConfigCenterAppState::from_registry(file_registry);
    let auth_state = AuthAppState::new(pool.clone());
    let audit_query_state = AuditQueryState { pool: pool.clone() };
    let master_data_state = MasterDataAppState::with_postgres(pool.clone());
    let system_dictionary_state = SystemDictionaryAppState::with_postgres(pool.clone());
    let wave3_state =
        Wave3AppState::with_postgres(pool.clone()).with_config_center(config_center_state.clone());
    let wave4_state = Wave4AppState::with_postgres(pool.clone());
    let wave5_state = Wave5AppState::with_postgres(pool.clone());
    let express_state = ExpressAppState::with_postgres(pool.clone());
    let app = app(
        config_center_state,
        auth_state,
        wave3_state,
        wave4_state,
        wave5_state,
        express_state,
        audit_query_state,
        master_data_state,
        system_dictionary_state,
    )
    .layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
        revocation_store,
    ))));

    let listener = TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn database_url() -> Result<String, io::Error> {
    match env::var(DATABASE_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        Some(value) => Ok(value),
        None => env::var(WMS_DB_URL_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{DATABASE_URL_ENV} or {WMS_DB_URL_ENV} is required"),
                )
            }),
    }
}

fn database_max_connections() -> Result<u32, io::Error> {
    match env::var(DB_MAX_CONNECTIONS_ENV) {
        Ok(value) => value.trim().parse::<u32>().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{DB_MAX_CONNECTIONS_ENV} must be a positive integer"),
            )
        }),
        Err(env::VarError::NotPresent) => Ok(DEFAULT_DB_MAX_CONNECTIONS),
        Err(error) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("failed to read {DB_MAX_CONNECTIONS_ENV}: {error}"),
        )),
    }
    .and_then(|value| {
        if value == 0 {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{DB_MAX_CONNECTIONS_ENV} must be greater than 0"),
            ))
        } else {
            Ok(value)
        }
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ApiDocsMode {
    Development,
    Production,
}

impl ApiDocsMode {
    fn from_env() -> Self {
        match env::var(API_DOCS_MODE_ENV)
            .unwrap_or_else(|_| "development".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "production" => Self::Production,
            _ => Self::Development,
        }
    }

    fn requires_internal_ip(self) -> bool {
        matches!(self, Self::Production)
    }
}
fn app(
    config_center_state: ConfigCenterAppState,
    auth_state: AuthAppState,
    wave3_state: Wave3AppState,
    wave4_state: Wave4AppState,
    wave5_state: Wave5AppState,
    express_state: ExpressAppState,
    audit_query_state: AuditQueryState,
    master_data_state: MasterDataAppState,
    system_dictionary_state: SystemDictionaryAppState,
) -> Router {
    let document_numbering_state =
        DocumentNumberingAppState::with_postgres(audit_query_state.pool.clone());
    let print_template_state = PrintTemplateAppState::with_postgres(audit_query_state.pool.clone());
    let admin_menu_state = AdminMenuAppState::with_postgres(audit_query_state.pool.clone());
    let h2_lifecycle_state = H2LifecycleAppState::with_postgres(audit_query_state.pool.clone());
    let wechat_notify_state = WechatNotifyAppState::with_postgres(audit_query_state.pool.clone());
    let resilience_state =
        ResilienceState::from_env().with_audit_pool(audit_query_state.pool.clone());
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(healthz))
        .route("/api/v1/healthz", get(healthz))
        .route("/openapi.json", get(openapi_json))
        .route("/api-docs", get(api_docs))
        .route("/redoc", get(redoc_docs))
        .merge(
            Router::new()
                .route("/api/v1/resilience/status", get(resilience_status))
                .route("/metrics", get(metrics))
                .with_state(resilience_state.clone()),
        )
        .merge(auth_router(auth_state))
        .merge(mount_reports(audit_query_state.pool.clone()))
        .merge(audit_query_router(audit_query_state))
        .merge(h2_lifecycle_router(h2_lifecycle_state))
        .merge(config_center_router(config_center_state))
        .merge(master_data_router(master_data_state))
        .merge(admin_menu_router(admin_menu_state))
        .merge(system_dictionary_router(system_dictionary_state))
        .merge(state_machine_router())
        .merge(document_numbering_router(document_numbering_state))
        .merge(print_template_router(print_template_state))
        .merge(wechat_notify_router(wechat_notify_state))
        .merge(express_router(express_state))
        .merge(wave3_router(wave3_state))
        .merge(wave4_router(wave4_state))
        .merge(wave5_router(wave5_state))
        .layer(Extension(ApiDocsMode::from_env()))
        .layer(from_fn_with_state(resilience_state, resilience_middleware))
}

async fn healthz() -> Json<HealthzResponse> {
    Json(HealthzResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at: Utc::now(),
    })
}

async fn openapi_json(Extension(mode): Extension<ApiDocsMode>, headers: HeaderMap) -> Response {
    if !docs_internal_access_allowed(&headers, mode) {
        return StatusCode::FORBIDDEN.into_response();
    }
    Json(ApiDoc::openapi()).into_response()
}

async fn api_docs(Extension(mode): Extension<ApiDocsMode>, headers: HeaderMap) -> Response {
    if mode == ApiDocsMode::Production {
        return StatusCode::NOT_FOUND.into_response();
    }
    if !docs_internal_access_allowed(&headers, mode) {
        return StatusCode::FORBIDDEN.into_response();
    }
    Html(
        r##"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>WMS API Docs</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>window.ui = SwaggerUIBundle({ url: "/openapi.json", dom_id: "#swagger-ui" });</script>
</body>
</html>"##,
    )
    .into_response()
}

async fn redoc_docs(Extension(mode): Extension<ApiDocsMode>, headers: HeaderMap) -> Response {
    if !docs_internal_access_allowed(&headers, mode) {
        return StatusCode::FORBIDDEN.into_response();
    }
    Html(
        r##"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>WMS API ReDoc</title>
</head>
<body>
  <redoc spec-url="/openapi.json"></redoc>
  <script src="https://cdn.jsdelivr.net/npm/redoc@next/bundles/redoc.standalone.js"></script>
</body>
</html>"##,
    )
    .into_response()
}

async fn metrics(
    Extension(mode): Extension<ApiDocsMode>,
    State(state): State<ResilienceState>,
    headers: HeaderMap,
) -> Response {
    if !docs_internal_access_allowed(&headers, mode) {
        return StatusCode::FORBIDDEN.into_response();
    }
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics_text(),
    )
        .into_response()
}

fn docs_internal_access_allowed(headers: &HeaderMap, mode: ApiDocsMode) -> bool {
    let Some(ip) = docs_request_ip(headers) else {
        return !mode.requires_internal_ip();
    };
    ip_is_internal(&ip)
}

fn docs_request_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .map(str::to_string)
}

fn ip_is_internal(ip: &str) -> bool {
    ip.starts_with("10.")
        || ip.starts_with("192.168.")
        || ip.starts_with("127.")
        || ip == "::1"
        || ip
            .strip_prefix("172.")
            .and_then(|rest| rest.split('.').next())
            .and_then(|octet| octet.parse::<u8>().ok())
            .is_some_and(|octet| (16..=31).contains(&octet))
}

#[cfg(test)]
mod tests {
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
        let failing_app =
            Router::new()
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

        let recovered_app =
            Router::new()
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

    #[tokio::test]
    async fn h3_resilience_limits_user_and_api_key_independently() {
        let state =
            wms_api::resilience::ResilienceState::new(wms_api::resilience::ResilienceConfig {
                global_qps: 100,
                global_burst: 100,
                user_qps: 1,
                user_burst: 1,
                api_key_qps: 1,
                api_key_burst: 1,
                retry_after_seconds: 1,
                circuit_failures: 10,
                circuit_open_seconds: 30,
            });
        let app = Router::new()
            .route("/limited", get(healthz))
            .layer(from_fn_with_state(
                state,
                wms_api::resilience::resilience_middleware,
            ));
        let owner_id = Uuid::new_v4();
        let user_one = bearer_token(owner_id);
        let user_two = bearer_token(owner_id);

        let first_user_one = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/limited")
                    .header("authorization", format!("Bearer {user_one}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(first_user_one.status(), StatusCode::OK);

        let second_user_one = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/limited")
                    .header("authorization", format!("Bearer {user_one}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(second_user_one.status(), StatusCode::TOO_MANY_REQUESTS);

        let first_user_two = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/limited")
                    .header("authorization", format!("Bearer {user_two}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(first_user_two.status(), StatusCode::OK);

        let first_api_key = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/limited")
                    .header("x-wms-api-key", "external-key-a")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(first_api_key.status(), StatusCode::OK);

        let second_api_key = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/limited")
                    .header("x-wms-api-key", "external-key-a")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(second_api_key.status(), StatusCode::TOO_MANY_REQUESTS);

        let other_api_key = app
            .oneshot(
                Request::builder()
                    .uri("/limited")
                    .header("x-wms-api-key", "external-key-b")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(other_api_key.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn h3_resilience_metrics_expose_rate_limit_and_degraded_counters() {
        async fn failing() -> StatusCode {
            StatusCode::INTERNAL_SERVER_ERROR
        }

        let state =
            wms_api::resilience::ResilienceState::new(wms_api::resilience::ResilienceConfig {
                global_qps: 1,
                global_burst: 1,
                user_qps: 100,
                user_burst: 100,
                api_key_qps: 100,
                api_key_burst: 100,
                retry_after_seconds: 1,
                circuit_failures: 1,
                circuit_open_seconds: 30,
            });
        let app = Router::new()
            .route("/limited", get(healthz))
            .route("/dependency", get(failing))
            .route("/metrics", get(wms_api::resilience::resilience_metrics))
            .with_state(state.clone())
            .layer(from_fn_with_state(
                state.clone(),
                wms_api::resilience::resilience_middleware,
            ));

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/limited")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        let limited = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/limited")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);

        state.reset_rate_limit_for_test();
        let failed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/dependency")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let degraded = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/dependency")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(degraded.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            degraded
                .headers()
                .get("x-wms-degraded-response")
                .and_then(|value| value.to_str().ok()),
            Some("true")
        );

        let metrics = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(metrics.status(), StatusCode::OK);
        let body = to_bytes(metrics.into_body(), usize::MAX)
            .await
            .expect("metrics body should read");
        let body = String::from_utf8(body.to_vec()).expect("metrics should be utf8");
        assert!(body.contains("wms_h3_rate_limit_rejected_total 1"));
        assert!(body.contains("wms_h3_circuit_opened_total 1"));
        assert!(body.contains("wms_h3_degraded_responses_total 1"));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn h3_resilience_rejections_write_h2_audit_for_bearer_actor(pool: PgPool) {
        let state =
            wms_api::resilience::ResilienceState::new(wms_api::resilience::ResilienceConfig {
                global_qps: 1,
                global_burst: 1,
                user_qps: 100,
                user_burst: 100,
                api_key_qps: 100,
                api_key_burst: 100,
                retry_after_seconds: 1,
                circuit_failures: 10,
                circuit_open_seconds: 30,
            })
            .with_audit_pool(pool.clone());
        let app = Router::new()
            .route("/limited", get(healthz))
            .layer(from_fn_with_state(
                state,
                wms_api::resilience::resilience_middleware,
            ));
        let owner_id = Uuid::new_v4();
        let token = bearer_token(owner_id);

        for expected in [StatusCode::OK, StatusCode::TOO_MANY_REQUESTS] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/limited")
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .expect("request should build"),
                )
                .await
                .expect("router should respond");
            assert_eq!(response.status(), expected);
        }

        let row: (i64, Option<String>, Option<String>) = sqlx::query_as(
            r#"
            SELECT COUNT(*), MIN(action), MIN(actor_name)
              FROM audit_event
             WHERE owner_id = $1
               AND module = 'H3'
               AND resource_type = 'api_resilience'
            "#,
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("audit row should query");
        assert_eq!(row.0, 1);
        assert_eq!(row.1.as_deref(), Some("h3.rate_limited"));
        assert_eq!(row.2.as_deref(), Some("audit-reader"));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn h3_resilience_rejections_write_h2_audit_for_api_key(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let state =
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
            .with_api_key_audit_owner_id(owner_id)
            .with_audit_pool(pool.clone());
        let app = Router::new()
            .route("/limited", get(healthz))
            .layer(from_fn_with_state(
                state,
                wms_api::resilience::resilience_middleware,
            ));

        for expected in [StatusCode::OK, StatusCode::TOO_MANY_REQUESTS] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/limited")
                        .header("x-wms-api-key", "external-key-a")
                        .body(Body::empty())
                        .expect("request should build"),
                )
                .await
                .expect("router should respond");
            assert_eq!(response.status(), expected);
        }

        let row: (i64, Option<String>, Option<String>) = sqlx::query_as(
            r#"
            SELECT COUNT(*), MIN(action), MIN(actor_name)
              FROM audit_event
             WHERE owner_id = $1
               AND module = 'H3'
               AND resource_type = 'api_resilience'
            "#,
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("audit row should query");
        assert_eq!(row.0, 1);
        assert_eq!(row.1.as_deref(), Some("h3.rate_limited"));
        assert!(row
            .2
            .as_deref()
            .is_some_and(|actor_name| actor_name.starts_with("api-key:")));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn h3_resilience_circuit_events_write_h2_audit(pool: PgPool) {
        async fn failing() -> StatusCode {
            StatusCode::INTERNAL_SERVER_ERROR
        }

        let state =
            wms_api::resilience::ResilienceState::new(wms_api::resilience::ResilienceConfig {
                global_qps: 100,
                global_burst: 100,
                user_qps: 100,
                user_burst: 100,
                api_key_qps: 100,
                api_key_burst: 100,
                retry_after_seconds: 1,
                circuit_failures: 1,
                circuit_open_seconds: 30,
            })
            .with_audit_pool(pool.clone());
        let app = Router::new()
            .route("/dependency", get(failing))
            .layer(from_fn_with_state(
                state,
                wms_api::resilience::resilience_middleware,
            ));
        let owner_id = Uuid::new_v4();
        let token = bearer_token(owner_id);

        let failed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/dependency")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let degraded = app
            .oneshot(
                Request::builder()
                    .uri("/dependency")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(degraded.status(), StatusCode::SERVICE_UNAVAILABLE);

        let actions: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT action
              FROM audit_event
             WHERE owner_id = $1
               AND module = 'H3'
               AND resource_type = 'api_resilience'
             ORDER BY action
            "#,
        )
        .bind(owner_id)
        .fetch_all(&pool)
        .await
        .expect("audit actions should query");
        assert_eq!(
            actions,
            vec![
                "h3.circuit_degraded".to_string(),
                "h3.circuit_opened".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn h3_docs_routes_follow_environment_mode() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during docs mode test");
        let app = with_env_lock(|| {
            std::env::set_var("WMS_API_DOCS_MODE", "production");
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
            std::env::remove_var("WMS_API_DOCS_MODE");
            app
        });

        let swagger = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api-docs")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(swagger.status(), StatusCode::NOT_FOUND);

        let blocked_redoc = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/redoc")
                    .header("x-forwarded-for", "8.8.8.8")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(blocked_redoc.status(), StatusCode::FORBIDDEN);

        let redoc_without_forwarded_ip = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/redoc")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(redoc_without_forwarded_ip.status(), StatusCode::FORBIDDEN);

        let metrics_without_forwarded_ip = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(metrics_without_forwarded_ip.status(), StatusCode::FORBIDDEN);

        let openapi_without_forwarded_ip = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/openapi.json")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(openapi_without_forwarded_ip.status(), StatusCode::FORBIDDEN);

        let redoc = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/redoc")
                    .header("x-forwarded-for", "10.0.0.8")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(redoc.status(), StatusCode::OK);
        let body = to_bytes(redoc.into_body(), usize::MAX)
            .await
            .expect("redoc body should read");
        let body = String::from_utf8(body.to_vec()).expect("redoc should be utf8");
        assert!(body.contains("redoc"));

        let metrics = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .header("x-forwarded-for", "10.0.0.8")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(metrics.status(), StatusCode::OK);
    }

    #[test]
    fn database_max_connections_defaults_and_rejects_invalid_values() {
        std::env::remove_var(DB_MAX_CONNECTIONS_ENV);
        assert_eq!(
            database_max_connections().expect("default max connections"),
            DEFAULT_DB_MAX_CONNECTIONS
        );

        std::env::set_var(DB_MAX_CONNECTIONS_ENV, "64");
        assert_eq!(
            database_max_connections().expect("configured max connections"),
            64
        );

        std::env::set_var(DB_MAX_CONNECTIONS_ENV, "0");
        assert!(database_max_connections().is_err());

        std::env::set_var(DB_MAX_CONNECTIONS_ENV, "not-a-number");
        assert!(database_max_connections().is_err());

        std::env::remove_var(DB_MAX_CONNECTIONS_ENV);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn audit_events_query_filters_by_auth_owner(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let other_owner_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let request_id = Uuid::new_v4();
        append_event(
            &pool,
            &AuditWriteRequest {
                occurred_at: Utc::now(),
                actor_id,
                actor_name: "owner-a-user".to_string(),
                owner_id,
                jti: "owner-a-jti".to_string(),
                action: "receive".to_string(),
                module: "M2".to_string(),
                resource_type: "receiving_order".to_string(),
                resource_id: "ASN-001".to_string(),
                diff: None,
                request_id: Some(request_id),
                ip: None,
                user_agent: None,
            },
        )
        .await
        .expect("owner audit should insert");
        append_event(
            &pool,
            &AuditWriteRequest {
                occurred_at: Utc::now(),
                actor_id,
                actor_name: "owner-a-user".to_string(),
                owner_id,
                jti: "owner-a-second-jti".to_string(),
                action: "putaway".to_string(),
                module: "M2".to_string(),
                resource_type: "putaway".to_string(),
                resource_id: "PUT-001".to_string(),
                diff: None,
                request_id: None,
                ip: None,
                user_agent: None,
            },
        )
        .await
        .expect("second owner audit should insert");
        append_event(
            &pool,
            &AuditWriteRequest {
                occurred_at: Utc::now(),
                actor_id: Uuid::new_v4(),
                actor_name: "other-owner-user".to_string(),
                owner_id: other_owner_id,
                jti: "other-owner-jti".to_string(),
                action: "receive".to_string(),
                module: "M2".to_string(),
                resource_type: "receiving_order".to_string(),
                resource_id: "ASN-002".to_string(),
                diff: None,
                request_id: None,
                ip: None,
                user_agent: None,
            },
        )
        .await
        .expect("other owner audit should insert");
        let app = app(
            config_center_state(),
            AuthAppState::new(pool.clone()),
            Wave3AppState::default(),
            Wave4AppState::with_postgres(pool.clone()),
            Wave5AppState::with_postgres(pool.clone()),
            ExpressAppState::with_postgres(pool.clone()),
            AuditQueryState { pool: pool.clone() },
            MasterDataAppState::default(),
            SystemDictionaryAppState::with_postgres(pool.clone()),
        )
        .layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
            AllowAllRevocationStore,
        ))));

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audit/events?resource_type=receiving_order&limit=10")
                    .header(
                        "authorization",
                        format!("Bearer {}", bearer_token(owner_id)),
                    )
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let payload: AuditEventListResponse =
            serde_json::from_slice(&body).expect("response should be json");
        assert_eq!(payload.data.len(), 1);
        let event = &payload.data[0];
        assert_eq!(event.owner_id, owner_id);
        assert_eq!(event.actor.actor_id, actor_id);
        assert_eq!(event.actor.jti, "owner-a-jti");
        assert_eq!(event.resource_id, "ASN-001");
        assert_eq!(event.trace_id, request_id.to_string());
        assert_eq!(event.diff, serde_json::json!({}));

        let page_one = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audit/events?limit=1")
                    .header(
                        "authorization",
                        format!("Bearer {}", bearer_token(owner_id)),
                    )
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(page_one.status(), StatusCode::OK);
        let body = axum::body::to_bytes(page_one.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let page_one: AuditEventListResponse =
            serde_json::from_slice(&body).expect("response should be json");
        assert_eq!(page_one.data.len(), 1);
        let first_page_id = page_one.data[0].id;
        let cursor = page_one
            .next_cursor
            .expect("first page should include next cursor");

        let page_two = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/audit/events?limit=1&cursor={cursor}"))
                    .header(
                        "authorization",
                        format!("Bearer {}", bearer_token(owner_id)),
                    )
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(page_two.status(), StatusCode::OK);
        let body = axum::body::to_bytes(page_two.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let page_two: AuditEventListResponse =
            serde_json::from_slice(&body).expect("response should be json");
        assert_eq!(page_two.data.len(), 1);
        assert_ne!(page_two.data[0].id, first_page_id);
        assert!(page_two.next_cursor.is_none());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn auth_login_issues_token_and_me_returns_current_user(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        seed_auth_user(&pool, owner_id, user_id, role_id).await;
        std::env::set_var(JWT_SECRET_ENV, "test-secret");
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

        let login_request = LoginRequest {
            owner_code: "PY_OWNER".to_string(),
            username: "admin".to_string(),
            password: "CorrectHorse1!".to_string(),
        };
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&login_request).expect("login request should encode"),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let login: LoginResponse =
            serde_json::from_slice(&body).expect("login response should be json");
        assert_eq!(login.token_type, "Bearer");
        assert_eq!(login.user.user_id, user_id);
        assert_eq!(login.user.owner_id, owner_id);
        assert_eq!(login.user.owner_code, "PY_OWNER");
        assert_eq!(login.user.username, "admin");
        assert_eq!(login.user.roles, vec!["audit_reader"]);
        assert_eq!(login.user.permissions, vec!["audit.read"]);

        let me_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/me")
                    .header("authorization", format!("Bearer {}", login.access_token))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(me_response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(me_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let current_user: CurrentUser =
            serde_json::from_slice(&body).expect("current user response should be json");
        assert_eq!(current_user.user_id, user_id);
        assert_eq!(current_user.owner_id, owner_id);
        assert_eq!(current_user.owner_code, "PY_OWNER");
        assert_eq!(current_user.roles, vec!["audit_reader"]);
        assert_eq!(current_user.permissions, vec!["audit.read"]);
    }
}
