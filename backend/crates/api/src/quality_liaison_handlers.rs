use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    CreateQualityLiaisonRequest, ErrorResponse, QualityLiaisonApprovalCallbackRequest,
    QualityLiaisonOrder, QualityLiaisonTypeConfig, UpsertQualityLiaisonTypeRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    quality_liaison::{PgQualityLiaisonRepository, QualityLiaisonError},
};

const READ_PERMISSION: &str = "mql.quality-liaison.read";
const WRITE_PERMISSION: &str = "mql.quality-liaison.write";
const CONFIG_PERMISSION: &str = "mql.quality-liaison.config";
const APPROVE_PERMISSION: &str = "mql.quality-liaison.approve";

#[derive(Clone, Debug)]
pub struct QualityLiaisonAppState {
    repository: PgQualityLiaisonRepository,
}

impl QualityLiaisonAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: PgQualityLiaisonRepository::new(pool),
        }
    }
}

#[derive(Debug)]
pub enum QualityLiaisonHandlerError {
    Auth(AuthError),
    QualityLiaison(QualityLiaisonError),
    MissingIdempotencyKey,
}

impl From<AuthError> for QualityLiaisonHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<QualityLiaisonError> for QualityLiaisonHandlerError {
    fn from(value: QualityLiaisonError) -> Self {
        Self::QualityLiaison(value)
    }
}

impl IntoResponse for QualityLiaisonHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "QL_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::QualityLiaison(QualityLiaisonError::NotFound) => {
                (StatusCode::NOT_FOUND, "QL_NOT_FOUND", "质量联系单不存在")
            }
            Self::QualityLiaison(QualityLiaisonError::TypeNotFound) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "QL_INVALID_TYPE",
                "质量联系单类型不存在或已停用",
            ),
            Self::QualityLiaison(QualityLiaisonError::InvalidRequest) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "QL_REQUEST_INVALID",
                "质量联系单请求参数不完整",
            ),
            Self::QualityLiaison(QualityLiaisonError::ApprovalOpinionRequired) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "QL_APPROVAL_OPINION_REQUIRED",
                "审批意见不能为空",
            ),
            Self::QualityLiaison(QualityLiaisonError::UnauthorizedApprover) => (
                StatusCode::FORBIDDEN,
                "QL_APPROVER_UNAUTHORIZED",
                "当前用户不是指定审批人",
            ),
            Self::QualityLiaison(QualityLiaisonError::AlreadyClosed) => (
                StatusCode::CONFLICT,
                "QL_ALREADY_CLOSED",
                "质量联系单已完成审批",
            ),
            Self::QualityLiaison(QualityLiaisonError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "QL_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            Self::QualityLiaison(QualityLiaisonError::BusinessActionInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "QL_BUSINESS_ACTION_FAILED",
                "审批通过后的业务联动未完成",
            ),
            Self::QualityLiaison(QualityLiaisonError::BusinessAction(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "QL_BUSINESS_ACTION_INTERNAL",
                "审批通过后的业务联动处理失败",
            ),
            Self::QualityLiaison(
                QualityLiaisonError::DocumentNumbering(_)
                | QualityLiaisonError::Audit(_)
                | QualityLiaisonError::Database(_)
                | QualityLiaisonError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "QL_INTERNAL",
                "质量联系单处理失败",
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

pub fn quality_liaison_router(state: QualityLiaisonAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/quality-liaisons/types/:type_code",
            put(upsert_type_handler),
        )
        .route("/api/v1/quality-liaisons", post(create_handler))
        .route("/api/v1/quality-liaisons/:id", get(get_handler))
        .route(
            "/api/v1/quality-liaisons/:id/approval-callback",
            post(approval_callback_handler),
        )
        .with_state(state)
}

async fn upsert_type_handler(
    ctx: AuthContext,
    State(state): State<QualityLiaisonAppState>,
    Path(type_code): Path<String>,
    headers: HeaderMap,
    Json(mut request): Json<UpsertQualityLiaisonTypeRequest>,
) -> Result<Json<QualityLiaisonTypeConfig>, QualityLiaisonHandlerError> {
    ctx.require_permission(CONFIG_PERMISSION)?;
    if request.type_code.trim() != type_code.trim() {
        return Err(QualityLiaisonError::InvalidRequest.into());
    }
    request.type_code = type_code;
    let result = state
        .repository
        .upsert_type(&ctx, request, Utc::now(), idempotency_key(&headers)?)
        .await?;
    Ok(Json(result.value))
}

async fn create_handler(
    ctx: AuthContext,
    State(state): State<QualityLiaisonAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateQualityLiaisonRequest>,
) -> Result<Json<QualityLiaisonOrder>, QualityLiaisonHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .repository
        .create(&ctx, request, Utc::now(), idempotency_key(&headers)?)
        .await?;
    Ok(Json(result.value))
}

async fn get_handler(
    ctx: AuthContext,
    State(state): State<QualityLiaisonAppState>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<QualityLiaisonOrder>, QualityLiaisonHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(state.repository.get(&ctx, order_id).await?))
}

async fn approval_callback_handler(
    ctx: AuthContext,
    State(state): State<QualityLiaisonAppState>,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<QualityLiaisonApprovalCallbackRequest>,
) -> Result<Json<QualityLiaisonOrder>, QualityLiaisonHandlerError> {
    ctx.require_permission(APPROVE_PERMISSION)?;
    let result = state
        .repository
        .apply_approval_callback(
            &ctx,
            order_id,
            request,
            Utc::now(),
            idempotency_key(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

fn idempotency_key(headers: &HeaderMap) -> Result<&str, QualityLiaisonHandlerError> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(QualityLiaisonHandlerError::MissingIdempotencyKey)
}
