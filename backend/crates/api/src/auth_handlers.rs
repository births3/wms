//! Wave 1 H1 auth HTTP handlers.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use sqlx::PgPool;
use wms_domain::ErrorResponse;

use crate::{
    auth::AuthContext,
    auth_repository::AuthRepository,
    auth_service::{AuthService, AuthServiceError},
};

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
        .with_state(state)
}

async fn login_handler(
    State(state): State<AuthAppState>,
    Json(request): Json<wms_domain::LoginRequest>,
) -> Result<Json<wms_domain::LoginResponse>, AuthHandlerError> {
    state
        .service
        .login(request)
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

struct AuthHandlerError(AuthServiceError);

impl From<AuthServiceError> for AuthHandlerError {
    fn from(value: AuthServiceError) -> Self {
        Self(value)
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
