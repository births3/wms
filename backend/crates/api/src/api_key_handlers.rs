use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::ErrorResponse;

use crate::{
    api_key_service::{ApiKeyService, ApiKeyServiceError, API_KEY_MANAGE_PERMISSION},
    auth::{AuthContext, AuthError},
};

#[derive(Clone)]
pub struct ApiKeyManagementState {
    service: ApiKeyService,
}

impl ApiKeyManagementState {
    pub fn new(pool: PgPool) -> Self {
        Self {
            service: ApiKeyService::new(pool),
        }
    }
}

pub fn api_key_router(state: ApiKeyManagementState) -> Router {
    Router::new()
        .route(
            "/api/v1/auth/api-keys",
            get(list_api_keys).post(create_api_key),
        )
        .route(
            "/api/v1/auth/api-keys/:api_key_id/rotate",
            post(rotate_api_key),
        )
        .route(
            "/api/v1/auth/api-keys/:api_key_id/revoke",
            post(revoke_api_key),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub struct ApiKeyListParams {
    pub q: Option<String>,
    pub status: Option<String>,
}

async fn list_api_keys(
    ctx: AuthContext,
    State(state): State<ApiKeyManagementState>,
    Query(params): Query<ApiKeyListParams>,
) -> Result<Json<wms_domain::ApiKeyListResponse>, ApiKeyHandlerError> {
    ctx.require_permission(API_KEY_MANAGE_PERMISSION)?;
    state
        .service
        .list(&ctx, params.q, params.status)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn create_api_key(
    ctx: AuthContext,
    State(state): State<ApiKeyManagementState>,
    headers: HeaderMap,
    Json(request): Json<wms_domain::CreateApiKeyRequest>,
) -> Result<Json<wms_domain::ApiKey>, ApiKeyHandlerError> {
    ctx.require_permission(API_KEY_MANAGE_PERMISSION)?;
    let idempotency_key = idempotency_key(&headers)?;
    state
        .service
        .create(&ctx, request, &idempotency_key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn rotate_api_key(
    ctx: AuthContext,
    State(state): State<ApiKeyManagementState>,
    Path(api_key_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<wms_domain::RotateApiKeyRequest>,
) -> Result<Json<wms_domain::ApiKeyRotationResponse>, ApiKeyHandlerError> {
    ctx.require_permission(API_KEY_MANAGE_PERMISSION)?;
    let idempotency_key = idempotency_key(&headers)?;
    state
        .service
        .rotate(&ctx, api_key_id, request, &idempotency_key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn revoke_api_key(
    ctx: AuthContext,
    State(state): State<ApiKeyManagementState>,
    Path(api_key_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<wms_domain::ApiKey>, ApiKeyHandlerError> {
    ctx.require_permission(API_KEY_MANAGE_PERMISSION)?;
    let idempotency_key = idempotency_key(&headers)?;
    state
        .service
        .revoke(&ctx, api_key_id, &idempotency_key)
        .await
        .map(Json)
        .map_err(Into::into)
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, ApiKeyHandlerError> {
    headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or(ApiKeyHandlerError::IdempotencyRequired)
}

enum ApiKeyHandlerError {
    Auth(AuthError),
    IdempotencyRequired,
    Service(ApiKeyServiceError),
}

impl From<AuthError> for ApiKeyHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<ApiKeyServiceError> for ApiKeyHandlerError {
    fn from(value: ApiKeyServiceError) -> Self {
        Self::Service(value)
    }
}

impl IntoResponse for ApiKeyHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message, retry_hint) = match self {
            Self::IdempotencyRequired => (
                StatusCode::BAD_REQUEST,
                "H1_APIKEY_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
                None,
            ),
            Self::Service(ApiKeyServiceError::InvalidRequest) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_APIKEY_INVALID_REQUEST",
                "API Key 请求字段非法",
                None,
            ),
            Self::Service(ApiKeyServiceError::InvalidScope) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_APIKEY_INVALID_SCOPE",
                "API Key 作用域非法",
                None,
            ),
            Self::Service(ApiKeyServiceError::InvalidExpiry) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_APIKEY_INVALID_EXPIRY",
                "API Key 过期时间非法",
                None,
            ),
            Self::Service(ApiKeyServiceError::InvalidGracePeriod) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_APIKEY_INVALID_GRACE_PERIOD",
                "API Key 轮换宽限期非法",
                None,
            ),
            Self::Service(ApiKeyServiceError::Repository(
                crate::api_key_repository::ApiKeyRepositoryError::NotFound,
            )) => (
                StatusCode::NOT_FOUND,
                "H1_APIKEY_NOT_FOUND",
                "API Key 不存在",
                None,
            ),
            Self::Service(ApiKeyServiceError::Repository(
                crate::api_key_repository::ApiKeyRepositoryError::IdempotencyConflict,
            )) => (
                StatusCode::CONFLICT,
                "H1_APIKEY_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
                None,
            ),
            Self::Service(ApiKeyServiceError::Repository(
                crate::api_key_repository::ApiKeyRepositoryError::Revoked,
            )) => (
                StatusCode::CONFLICT,
                "H1_APIKEY_REVOKED",
                "API Key 已吊销，不能轮换",
                None,
            ),
            Self::Service(ApiKeyServiceError::Repository(
                crate::api_key_repository::ApiKeyRepositoryError::ResponsibleUser,
            )) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_APIKEY_INVALID_RESPONSIBLE_USER",
                "负责人不属于当前货主",
                None,
            ),
            Self::Service(ApiKeyServiceError::Repository(
                crate::api_key_repository::ApiKeyRepositoryError::WarehouseScope,
            )) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_APIKEY_INVALID_WAREHOUSE_SCOPE",
                "仓库范围不属于当前货主",
                None,
            ),
            Self::Service(ApiKeyServiceError::Repository(
                crate::api_key_repository::ApiKeyRepositoryError::Database(_),
            ))
            | Self::Service(ApiKeyServiceError::Repository(
                crate::api_key_repository::ApiKeyRepositoryError::Audit(_),
            ))
            | Self::Service(ApiKeyServiceError::Repository(
                crate::api_key_repository::ApiKeyRepositoryError::Serialize(_),
            ))
            | Self::Service(ApiKeyServiceError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H1_APIKEY_INTERNAL",
                "API Key 处理失败",
                None,
            ),
            Self::Service(ApiKeyServiceError::Repository(_)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_APIKEY_REQUEST_REJECTED",
                "API Key 请求被拒绝",
                None,
            ),
            Self::Auth(error) => return error.into_response(),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: retry_hint.map(str::to_string),
            }),
        )
            .into_response()
    }
}
