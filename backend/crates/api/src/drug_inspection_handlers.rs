use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch},
    Json, Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use wms_domain::{
    ChangeDrugInspectionPlatformStatusRequest, DrugInspectionConfigValidationError,
    DrugInspectionPlatformListResponse, ErrorResponse, UpsertDrugInspectionPlatformRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    drug_inspection_repository::{DrugInspectionRepositoryError, PgDrugInspectionRepository},
};

pub const DRUG_INSPECTION_READ_PERMISSION: &str = "m-di.platform.read";
pub const DRUG_INSPECTION_WRITE_PERMISSION: &str = "m-di.platform.write";

#[derive(Clone)]
pub struct DrugInspectionAppState {
    pub repository: Arc<PgDrugInspectionRepository>,
}

impl DrugInspectionAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgDrugInspectionRepository::new(pool)),
        }
    }
}

pub fn drug_inspection_router(state: DrugInspectionAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/drug-inspection/platforms",
            get(list_platforms).post(upsert_platform),
        )
        .route(
            "/api/v1/drug-inspection/platforms/:platform_id/status",
            patch(change_platform_status),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct PlatformListQuery {
    status: Option<String>,
}

async fn list_platforms(
    ctx: AuthContext,
    State(state): State<DrugInspectionAppState>,
    Query(query): Query<PlatformListQuery>,
) -> Result<Json<DrugInspectionPlatformListResponse>, DrugInspectionHandlerError> {
    ctx.require_permission(DRUG_INSPECTION_READ_PERMISSION)?;
    state
        .repository
        .list(ctx.owner_id, query.status)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn upsert_platform(
    ctx: AuthContext,
    State(state): State<DrugInspectionAppState>,
    headers: HeaderMap,
    Json(request): Json<UpsertDrugInspectionPlatformRequest>,
) -> Result<Json<wms_domain::DrugInspectionPlatform>, DrugInspectionHandlerError> {
    ctx.require_permission(DRUG_INSPECTION_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key(&headers)?;
    state
        .repository
        .upsert(&ctx, request, &idempotency_key)
        .await
        .map(|result| Json(result.value))
        .map_err(Into::into)
}

async fn change_platform_status(
    ctx: AuthContext,
    State(state): State<DrugInspectionAppState>,
    Path(platform_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ChangeDrugInspectionPlatformStatusRequest>,
) -> Result<Json<wms_domain::DrugInspectionPlatform>, DrugInspectionHandlerError> {
    ctx.require_permission(DRUG_INSPECTION_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key(&headers)?;
    state
        .repository
        .change_status(&ctx, platform_id, request, &idempotency_key)
        .await
        .map(|result| Json(result.value))
        .map_err(Into::into)
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, DrugInspectionHandlerError> {
    headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or(DrugInspectionHandlerError::IdempotencyRequired)
}

enum DrugInspectionHandlerError {
    Auth(AuthError),
    IdempotencyRequired,
    Repository(DrugInspectionRepositoryError),
}

impl From<AuthError> for DrugInspectionHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<DrugInspectionRepositoryError> for DrugInspectionHandlerError {
    fn from(value: DrugInspectionRepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl IntoResponse for DrugInspectionHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::IdempotencyRequired => (
                StatusCode::BAD_REQUEST,
                "M_DI_PLATFORM_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::Repository(DrugInspectionRepositoryError::Invalid(error)) => {
                validation_error_response(error)
            }
            Self::Repository(DrugInspectionRepositoryError::NotFound) => (
                StatusCode::NOT_FOUND,
                "M_DI_PLATFORM_NOT_FOUND",
                "药检平台不存在",
            ),
            Self::Repository(DrugInspectionRepositoryError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "M_DI_PLATFORM_IDEMPOTENCY_CONFLICT",
                "幂等键已用于不同请求",
            ),
            Self::Repository(DrugInspectionRepositoryError::Audit(_))
            | Self::Repository(DrugInspectionRepositoryError::Database(_))
            | Self::Repository(DrugInspectionRepositoryError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M_DI_PLATFORM_PERSISTENCE_FAILED",
                "药检平台配置持久化或审计失败",
            ),
            Self::Auth(_) => unreachable!("auth error returned above"),
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

fn validation_error_response(
    error: DrugInspectionConfigValidationError,
) -> (StatusCode, &'static str, &'static str) {
    match error {
        DrugInspectionConfigValidationError::FieldRequired(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "M_DI_PLATFORM_FIELD_REQUIRED",
            "药检平台配置必填字段缺失",
        ),
        DrugInspectionConfigValidationError::FieldTooLong(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "M_DI_PLATFORM_FIELD_TOO_LONG",
            "药检平台配置字段超长",
        ),
        DrugInspectionConfigValidationError::InvalidApiUrl => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "M_DI_PLATFORM_API_URL_INVALID",
            "API 地址必须是带主机的 HTTP 或 HTTPS 地址",
        ),
        DrugInspectionConfigValidationError::InvalidAuthMethod => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "M_DI_PLATFORM_AUTH_METHOD_INVALID",
            "认证方式必须是 API Key 或账号密码",
        ),
        DrugInspectionConfigValidationError::InvalidCredentialReference => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "M_DI_PLATFORM_CREDENTIAL_REF_INVALID",
            "认证凭证必须使用 Vault 引用",
        ),
        DrugInspectionConfigValidationError::InvalidCredentialCombination => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "M_DI_PLATFORM_CREDENTIAL_COMBINATION_INVALID",
            "认证参数与认证方式不匹配",
        ),
        DrugInspectionConfigValidationError::InvalidTimeout => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "M_DI_PLATFORM_TIMEOUT_INVALID",
            "超时必须在 1 到 300 秒之间",
        ),
        DrugInspectionConfigValidationError::InvalidStatus => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "M_DI_PLATFORM_STATUS_INVALID",
            "平台状态必须是 connected、testing 或 disabled",
        ),
    }
}
