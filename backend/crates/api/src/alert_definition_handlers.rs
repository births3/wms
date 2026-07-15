use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    AlertDefinition, AlertDefinitionListQuery, AlertDefinitionListResponse, ErrorResponse,
    PageMeta, QualityLiaisonOrder, SubmitAlertDefinitionChangeRequest,
};

use crate::{
    alert_definition_repository::{AlertDefinitionRepositoryError, PgAlertDefinitionRepository},
    alert_definition_service::{AlertDefinitionService, AlertDefinitionServiceError},
    auth::{AuthContext, AuthError},
    quality_liaison::QualityLiaisonError,
};

const READ_PERMISSION: &str = "hal.alert-definition.read";
const WRITE_PERMISSION: &str = "hal.alert-definition.write";

#[derive(Clone, Debug)]
pub struct AlertDefinitionAppState {
    repository: PgAlertDefinitionRepository,
    service: AlertDefinitionService,
}

impl AlertDefinitionAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: PgAlertDefinitionRepository::new(pool.clone()),
            service: AlertDefinitionService::new(pool),
        }
    }
}

#[derive(Debug)]
pub enum AlertDefinitionHandlerError {
    Auth(AuthError),
    Repository(AlertDefinitionRepositoryError),
    QualityLiaison(QualityLiaisonError),
    MissingIdempotencyKey,
}

impl From<AuthError> for AlertDefinitionHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<AlertDefinitionRepositoryError> for AlertDefinitionHandlerError {
    fn from(value: AlertDefinitionRepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl From<AlertDefinitionServiceError> for AlertDefinitionHandlerError {
    fn from(value: AlertDefinitionServiceError) -> Self {
        match value {
            AlertDefinitionServiceError::Definition(error) => Self::Repository(error),
            AlertDefinitionServiceError::QualityLiaison(error) => Self::QualityLiaison(error),
        }
    }
}

impl IntoResponse for AlertDefinitionHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "HAL_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::Repository(AlertDefinitionRepositoryError::NotFound) => (
                StatusCode::NOT_FOUND,
                "HAL_ALERT_NOT_FOUND",
                "告警定义不存在",
            ),
            Self::Repository(AlertDefinitionRepositoryError::DuplicateCode) => (
                StatusCode::CONFLICT,
                "HAL_ALERT_DUPLICATE",
                "告警编码或名称已存在",
            ),
            Self::Repository(AlertDefinitionRepositoryError::StaleVersion) => (
                StatusCode::CONFLICT,
                "HAL_ALERT_STALE",
                "告警定义已变更，请刷新后重试",
            ),
            Self::Repository(AlertDefinitionRepositoryError::InUse) => (
                StatusCode::CONFLICT,
                "HAL_ALERT_IN_USE",
                "告警定义已有触发记录，不能删除",
            ),
            Self::Repository(
                AlertDefinitionRepositoryError::GspForcedCannotDisable
                | AlertDefinitionRepositoryError::GspForcedCannotDelete,
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_ALERT_GSP_REQUIRED",
                "GSP 强制告警不能停用或删除",
            ),
            Self::Repository(AlertDefinitionRepositoryError::DisableNotAllowed) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_ALERT_DISABLE_NOT_ALLOWED",
                "该告警定义不允许停用",
            ),
            Self::Repository(AlertDefinitionRepositoryError::ConditionInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_CONDITION_INVALID",
                "触发条件必须是有效 JSON",
            ),
            Self::Repository(AlertDefinitionRepositoryError::ChannelNotFound) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_CHANNEL_NOT_FOUND",
                "触发事件尚未配置可用通知通道",
            ),
            Self::Repository(AlertDefinitionRepositoryError::EscalationRuleNotFound) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_ESCALATION_RULE_NOT_FOUND",
                "升级规则不存在或已停用",
            ),
            Self::Repository(AlertDefinitionRepositoryError::Invalid(_)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_ALERT_INVALID",
                "告警定义字段或变更结构非法",
            ),
            Self::QualityLiaison(QualityLiaisonError::TypeNotFound) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_ALERT_APPROVAL_NOT_CONFIGURED",
                "未配置告警定义变更审批类型",
            ),
            Self::QualityLiaison(QualityLiaisonError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "HAL_ALERT_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            Self::QualityLiaison(QualityLiaisonError::InvalidRequest) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_ALERT_INVALID",
                "告警定义变更请求非法",
            ),
            Self::Repository(
                AlertDefinitionRepositoryError::Database(_)
                | AlertDefinitionRepositoryError::Audit(_),
            )
            | Self::QualityLiaison(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "HAL_ALERT_INTERNAL",
                "告警定义处理失败",
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

pub fn alert_definition_router(state: AlertDefinitionAppState) -> Router {
    Router::new()
        .route("/api/v1/alert-definitions", get(list_handler))
        .route("/api/v1/alert-definitions/:id", get(get_handler))
        .route(
            "/api/v1/alert-definitions/change-requests",
            post(submit_change_handler),
        )
        .with_state(state)
}

async fn list_handler(
    ctx: AuthContext,
    State(state): State<AlertDefinitionAppState>,
    Query(query): Query<AlertDefinitionListQuery>,
) -> Result<Json<AlertDefinitionListResponse>, AlertDefinitionHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    let data = state.repository.list(ctx.owner_id, &query).await?;
    Ok(Json(AlertDefinitionListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len().min(u32::MAX as usize) as u32,
        },
        data,
    }))
}

async fn get_handler(
    ctx: AuthContext,
    State(state): State<AlertDefinitionAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AlertDefinition>, AlertDefinitionHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(state.repository.get(ctx.owner_id, id).await?))
}

async fn submit_change_handler(
    ctx: AuthContext,
    State(state): State<AlertDefinitionAppState>,
    headers: HeaderMap,
    Json(request): Json<SubmitAlertDefinitionChangeRequest>,
) -> Result<Json<QualityLiaisonOrder>, AlertDefinitionHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    Ok(Json(
        state
            .service
            .submit_change(&ctx, request, Utc::now(), idempotency_key(&headers)?)
            .await?,
    ))
}

fn idempotency_key(headers: &HeaderMap) -> Result<&str, AlertDefinitionHandlerError> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(AlertDefinitionHandlerError::MissingIdempotencyKey)
}
