use std::{env, error::Error, io, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::net::TcpListener;
use uuid::Uuid;
use wms_api::{
    audit::{
        list_events, AuditError, AuditEventPage, AuditEventQuery, AuditEventQueryCursor,
        AuditEventRecord, DEFAULT_AUDIT_EVENT_QUERY_LIMIT, MAX_AUDIT_EVENT_QUERY_LIMIT,
    },
    auth::{
        auth_runtime_layer, AuthContext, AuthRuntimePolicy, RedisAuthRevocationStore,
        JWT_SECRET_ENV,
    },
    auth_handlers::{auth_router, AuthAppState},
    config_center::{config_center_router, ConfigCenterAppState},
    feature_flags::FeatureFlagRegistry,
    master_data_handlers::{master_data_router, MasterDataAppState},
    system_dictionary_handlers::{system_dictionary_router, SystemDictionaryAppState},
    wave3_handlers::{wave3_router, Wave3AppState},
};
use wms_domain::{AuditActor, AuditEvent, AuditEventListResponse, ErrorResponse, HealthzResponse};

const BIND_ADDR_ENV: &str = "WMS_BIND_ADDR";
const REDIS_URL_ENV: &str = "WMS_REDIS_URL";
const DATABASE_URL_ENV: &str = "DATABASE_URL";
const WMS_DB_URL_ENV: &str = "WMS_DB_URL";
const DB_MAX_CONNECTIONS_ENV: &str = "WMS_DB_MAX_CONNECTIONS";
const FEATURE_FLAGS_FILE_ENV: &str = "WMS_FEATURE_FLAGS_FILE";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:8080";
const DEFAULT_DB_MAX_CONNECTIONS: u32 = 32;
const DEFAULT_FEATURE_FLAGS_FILE: &str = "deploy/feature_flags.toml";

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
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("failed to connect PostgreSQL: {error:?}"),
            )
        })?;
    let config_center_state = ConfigCenterAppState::from_registry(file_registry);
    let auth_state = AuthAppState::new(pool.clone());
    let audit_query_state = AuditQueryState { pool: pool.clone() };
    let master_data_state = MasterDataAppState::with_postgres(pool.clone());
    let system_dictionary_state = SystemDictionaryAppState::with_postgres(pool.clone());
    let wave3_state =
        Wave3AppState::with_postgres(pool.clone()).with_config_center(config_center_state.clone());
    let app = app(
        config_center_state,
        auth_state,
        wave3_state,
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

#[derive(Clone)]
struct AuditQueryState {
    pool: PgPool,
}

fn app(
    config_center_state: ConfigCenterAppState,
    auth_state: AuthAppState,
    wave3_state: Wave3AppState,
    audit_query_state: AuditQueryState,
    master_data_state: MasterDataAppState,
    system_dictionary_state: SystemDictionaryAppState,
) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(healthz))
        .route("/api/v1/healthz", get(healthz))
        .merge(auth_router(auth_state))
        .merge(audit_query_router(audit_query_state))
        .merge(config_center_router(config_center_state))
        .merge(master_data_router(master_data_state))
        .merge(system_dictionary_router(system_dictionary_state))
        .merge(wave3_router(wave3_state))
}

async fn healthz() -> Json<HealthzResponse> {
    Json(HealthzResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at: Utc::now(),
    })
}

fn audit_query_router(state: AuditQueryState) -> Router {
    Router::new()
        .route("/api/v1/audit/events", get(list_audit_events_handler))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct AuditEventQueryParams {
    resource_type: Option<String>,
    actor_id: Option<Uuid>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: Option<u32>,
    cursor: Option<String>,
}

#[derive(Debug)]
enum AuditQueryError {
    InvalidCursor,
    Query,
}

impl From<AuditError> for AuditQueryError {
    fn from(_value: AuditError) -> Self {
        Self::Query
    }
}

impl IntoResponse for AuditQueryError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AuditQueryError::InvalidCursor => (
                StatusCode::BAD_REQUEST,
                "H2_AUDIT_QUERY_CURSOR_INVALID",
                "审计查询游标格式无效",
            ),
            AuditQueryError::Query => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H2_AUDIT_QUERY_FAILED",
                "审计查询失败",
            ),
        };

        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

async fn list_audit_events_handler(
    ctx: AuthContext,
    State(state): State<AuditQueryState>,
    Query(params): Query<AuditEventQueryParams>,
) -> Result<Json<AuditEventListResponse>, AuditQueryError> {
    let query = AuditEventQuery {
        owner_id: ctx.owner_id,
        resource_type: params.resource_type,
        actor_id: params.actor_id,
        from: params.from,
        to: params.to,
        cursor: params
            .cursor
            .as_deref()
            .map(parse_audit_cursor)
            .transpose()?,
        limit: params
            .limit
            .unwrap_or(DEFAULT_AUDIT_EVENT_QUERY_LIMIT)
            .clamp(1, MAX_AUDIT_EVENT_QUERY_LIMIT),
    };
    let page = list_events(&state.pool, &query).await?;
    Ok(Json(audit_event_response(page)?))
}

fn audit_event_response(page: AuditEventPage) -> Result<AuditEventListResponse, AuditQueryError> {
    Ok(AuditEventListResponse {
        data: page.events.into_iter().map(audit_event_dto).collect(),
        next_cursor: page.next_cursor.map(format_audit_cursor),
    })
}

fn audit_event_dto(record: AuditEventRecord) -> AuditEvent {
    let trace_id = record
        .request_id
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    AuditEvent {
        id: record.id,
        owner_id: record.owner_id,
        resource_type: record.resource_type,
        resource_id: record.resource_id,
        action: record.action,
        trace_id,
        occurred_at: record.occurred_at,
        actor: AuditActor {
            actor_id: record.actor_id,
            actor_name: record.actor_name,
            owner_id: record.owner_id,
            jti: record.jti,
        },
        diff: record
            .diff
            .map(|value| serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!({})))
            .unwrap_or_else(|| serde_json::json!({})),
    }
}

fn parse_audit_cursor(value: &str) -> Result<AuditEventQueryCursor, AuditQueryError> {
    let (micros, id) = value
        .split_once(':')
        .ok_or(AuditQueryError::InvalidCursor)?;
    let timestamp_micros = micros
        .parse::<i64>()
        .map_err(|_| AuditQueryError::InvalidCursor)?;
    let id = id
        .parse::<i64>()
        .map_err(|_| AuditQueryError::InvalidCursor)?;
    let occurred_at = Utc
        .timestamp_micros(timestamp_micros)
        .single()
        .ok_or(AuditQueryError::InvalidCursor)?;
    Ok(AuditEventQueryCursor { occurred_at, id })
}

fn format_audit_cursor(cursor: AuditEventQueryCursor) -> String {
    format!("{}:{}", cursor.occurred_at.timestamp_micros(), cursor.id)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
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
        std::env::set_var(JWT_SECRET_ENV, "test-secret");
        let claims = build_access_claims(
            Uuid::new_v4(),
            owner_id,
            "audit-reader",
            vec![],
            Uuid::new_v4().to_string(),
            Utc::now(),
        );
        encode_access_token(&claims, "test-secret").expect("token should encode")
    }

    async fn seed_auth_user(
        pool: &PgPool,
        owner_id: Uuid,
        user_id: Uuid,
        role_id: Uuid,
        permission_id: Uuid,
    ) {
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
            VALUES ($1, $2, 'system_admin', '系统管理员')
            "#,
        )
        .bind(role_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("role should insert");
        sqlx::query(
            r#"
            INSERT INTO auth_permissions (id, permission_code, permission_name)
            VALUES ($1, 'audit.read', '审计查询')
            "#,
        )
        .bind(permission_id)
        .execute(pool)
        .await
        .expect("permission should insert");
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
            VALUES ($1, $2)
            "#,
        )
        .bind(role_id)
        .bind(permission_id)
        .execute(pool)
        .await
        .expect("role permission should insert");
    }

    #[tokio::test]
    async fn audit_events_route_requires_auth_context() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during auth rejection test");
        let app = app(
            config_center_state(),
            AuthAppState::new(pool.clone()),
            Wave3AppState::default(),
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
                    .uri("/api/v1/audit/events")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn master_data_route_is_mounted_under_auth_context() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during auth rejection test");
        let app = app(
            config_center_state(),
            AuthAppState::new(pool.clone()),
            Wave3AppState::default(),
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
        let permission_id = Uuid::new_v4();
        seed_auth_user(&pool, owner_id, user_id, role_id, permission_id).await;
        std::env::set_var(JWT_SECRET_ENV, "test-secret");
        let app = app(
            config_center_state(),
            AuthAppState::new(pool.clone()),
            Wave3AppState::default(),
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
        assert_eq!(login.user.roles, vec!["system_admin"]);
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
        assert_eq!(current_user.roles, vec!["system_admin"]);
        assert_eq!(current_user.permissions, vec!["audit.read"]);
    }
}
