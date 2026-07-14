//! Wave 1 H1 auth HTTP handlers.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Extension, Json, Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::ErrorResponse;

use crate::{
    auth::{AuthContext, AuthRuntimePolicy, LogoutContext},
    auth_repository::AuthRepository,
    auth_service::{AuthService, AuthServiceError, LoginMetadata},
};

pub const SESSION_MANAGE_PERMISSION: &str = "h1.sessions.manage";

#[derive(Clone)]
pub struct AuthAppState {
    service: AuthService,
}

impl AuthAppState {
    pub fn new(pool: PgPool) -> Self {
        Self {
            service: AuthService::new(AuthRepository::new(pool)),
        }
    }
}

pub fn auth_router(state: AuthAppState) -> Router {
    Router::new()
        .route("/api/v1/auth/login", post(login_handler))
        .route("/api/v1/auth/me", get(me_handler))
        .route("/api/v1/auth/logout", post(logout_handler))
        .route("/api/v1/auth/me/password", put(change_password_handler))
        .route("/api/v1/auth/sessions", get(list_sessions_handler))
        .route(
            "/api/v1/auth/sessions/:session_id/revoke",
            post(revoke_session_handler),
        )
        .route(
            "/api/v1/auth/sessions/revoke-others",
            post(revoke_other_sessions_handler),
        )
        .route("/api/v1/auth/users/:user_id/kick", post(kick_user_handler))
        .route(
            "/api/v1/auth/users/:user_id/status",
            put(change_user_status_handler),
        )
        .with_state(state)
}

async fn login_handler(
    State(state): State<AuthAppState>,
    headers: HeaderMap,
    Json(request): Json<wms_domain::LoginRequest>,
) -> Result<Json<wms_domain::LoginResponse>, AuthHandlerError> {
    state
        .service
        .login_with_metadata(request, login_metadata(&headers))
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn logout_handler(
    State(state): State<AuthAppState>,
    Extension(policy): Extension<AuthRuntimePolicy>,
    ctx: LogoutContext,
) -> Result<Json<wms_domain::AuthRevocationResponse>, AuthHandlerError> {
    state
        .service
        .logout(&ctx, &policy)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn change_password_handler(
    ctx: AuthContext,
    State(state): State<AuthAppState>,
    Extension(policy): Extension<AuthRuntimePolicy>,
    Json(request): Json<wms_domain::PasswordChangeRequest>,
) -> Result<Json<wms_domain::AuthSessionRevokeResponse>, AuthHandlerError> {
    state
        .service
        .change_password(&ctx, request, &policy)
        .await
        .map(Json)
        .map_err(Into::into)
}

#[derive(Debug, Deserialize)]
struct SessionListQuery {
    user_id: Option<Uuid>,
}

async fn list_sessions_handler(
    ctx: AuthContext,
    State(state): State<AuthAppState>,
    Query(query): Query<SessionListQuery>,
) -> Result<Json<wms_domain::AuthSessionListResponse>, AuthHandlerError> {
    let user_id = query.user_id.unwrap_or(ctx.user_id);
    if user_id != ctx.user_id {
        ctx.require_permission(SESSION_MANAGE_PERMISSION)?;
    }
    state
        .service
        .list_sessions(&ctx, user_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn revoke_session_handler(
    ctx: AuthContext,
    State(state): State<AuthAppState>,
    Extension(policy): Extension<AuthRuntimePolicy>,
    Path(session_id): Path<String>,
) -> Result<Json<wms_domain::AuthRevocationResponse>, AuthHandlerError> {
    state
        .service
        .revoke_session(&ctx, &session_id, &policy)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn revoke_other_sessions_handler(
    ctx: AuthContext,
    State(state): State<AuthAppState>,
    Extension(policy): Extension<AuthRuntimePolicy>,
) -> Result<Json<wms_domain::AuthSessionRevokeResponse>, AuthHandlerError> {
    state
        .service
        .revoke_other_sessions(&ctx, &policy)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn kick_user_handler(
    ctx: AuthContext,
    State(state): State<AuthAppState>,
    Extension(policy): Extension<AuthRuntimePolicy>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<wms_domain::AuthSessionRevokeResponse>, AuthHandlerError> {
    ctx.require_permission(SESSION_MANAGE_PERMISSION)?;
    state
        .service
        .kick_user(&ctx, user_id, &policy)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn change_user_status_handler(
    ctx: AuthContext,
    State(state): State<AuthAppState>,
    Extension(policy): Extension<AuthRuntimePolicy>,
    Path(user_id): Path<Uuid>,
    Json(request): Json<wms_domain::AuthUserStatusRequest>,
) -> Result<Json<wms_domain::AuthSessionRevokeResponse>, AuthHandlerError> {
    ctx.require_permission(SESSION_MANAGE_PERMISSION)?;
    state
        .service
        .change_user_status(&ctx, user_id, request, &policy)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn me_handler(
    ctx: AuthContext,
    State(state): State<AuthAppState>,
) -> Result<Json<wms_domain::CurrentUser>, AuthHandlerError> {
    state
        .service
        .current_user(&ctx)
        .await
        .map(Json)
        .map_err(Into::into)
}

fn login_metadata(headers: &HeaderMap) -> LoginMetadata {
    let user_agent = headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.chars().take(240).collect::<String>());
    LoginMetadata {
        device_name: user_agent
            .as_deref()
            .unwrap_or("unknown")
            .chars()
            .take(120)
            .collect(),
        ip: request_ip(headers),
        user_agent,
    }
}

fn request_ip(headers: &HeaderMap) -> Option<String> {
    ["x-forwarded-for", "x-real-ip"]
        .into_iter()
        .filter_map(|name| headers.get(name))
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(',').map(str::trim))
        .find_map(|value| {
            value
                .parse::<std::net::IpAddr>()
                .ok()
                .map(|_| value.to_string())
        })
}

struct AuthHandlerError(AuthServiceError);

impl From<AuthServiceError> for AuthHandlerError {
    fn from(value: AuthServiceError) -> Self {
        Self(value)
    }
}

impl From<crate::auth::AuthError> for AuthHandlerError {
    fn from(value: crate::auth::AuthError) -> Self {
        Self(AuthServiceError::Auth(value))
    }
}

impl IntoResponse for AuthHandlerError {
    fn into_response(self) -> Response {
        if let AuthServiceError::Auth(error) = self.0 {
            return error.into_response();
        }

        let (status, code, message, severity, details) = match self.0 {
            AuthServiceError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "AUTH-008",
                "账号或密码错误",
                "warning",
                serde_json::json!({}),
            ),
            AuthServiceError::AccountLocked(locked_until) => (
                StatusCode::LOCKED,
                "H1_LOGIN_LOCKED",
                "账号已被锁定",
                "warning",
                serde_json::json!({ "locked_until": locked_until }),
            ),
            AuthServiceError::Repository => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H1_AUTH_DATABASE_ERROR",
                "鉴权数据读取失败",
                "error",
                serde_json::json!({}),
            ),
            AuthServiceError::PasswordHash => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H1_AUTH_PASSWORD_HASH_ERROR",
                "密码哈希校验失败",
                "error",
                serde_json::json!({}),
            ),
            AuthServiceError::MissingSecret => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "AUTH-SECRET-MISSING",
                "JWT 密钥未配置",
                "error",
                serde_json::json!({}),
            ),
            AuthServiceError::SessionNotFound => (
                StatusCode::NOT_FOUND,
                "H1_SESSION_NOT_FOUND",
                "登录会话不存在",
                "warning",
                serde_json::json!({}),
            ),
            AuthServiceError::UserNotFound => (
                StatusCode::NOT_FOUND,
                "H1_USER_NOT_FOUND",
                "用户不存在或不属于当前货主",
                "warning",
                serde_json::json!({}),
            ),
            AuthServiceError::InvalidPassword => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_PASSWORD_INVALID",
                "新密码不符合最小长度和复杂度要求",
                "warning",
                serde_json::json!({}),
            ),
            AuthServiceError::InvalidStatus => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_STATUS_INVALID",
                "用户状态只能是 active 或 disabled",
                "warning",
                serde_json::json!({}),
            ),
            AuthServiceError::CannotDisableSelf => (
                StatusCode::CONFLICT,
                "M1_CANNOT_DISABLE_SELF",
                "不能停用当前登录用户",
                "warning",
                serde_json::json!({}),
            ),
            AuthServiceError::Auth(_) => unreachable!("auth error returned above"),
        };

        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: severity.to_string(),
                details,
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}
