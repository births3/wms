use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, put},
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use wms_domain::{
    AlertEscalationRule, AlertEscalationRuleListResponse, ErrorResponse, PageMeta,
    UpsertAlertEscalationRuleRequest,
};

use crate::{
    alert_escalation::{AlertEscalationError, PgAlertEscalationRepository},
    auth::{AuthContext, AuthError},
};

#[derive(Clone, Debug)]
pub struct AlertEscalationAppState {
    repository: PgAlertEscalationRepository,
}

impl AlertEscalationAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: PgAlertEscalationRepository::new(pool),
        }
    }
}

#[derive(Debug)]
pub enum AlertEscalationHandlerError {
    Auth(AuthError),
    Escalation(AlertEscalationError),
    PathMismatch,
}

impl From<AuthError> for AlertEscalationHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<AlertEscalationError> for AlertEscalationHandlerError {
    fn from(value: AlertEscalationError) -> Self {
        Self::Escalation(value)
    }
}

impl IntoResponse for AlertEscalationHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::Escalation(AlertEscalationError::TooManyLevels) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_ESCALATION_LEVEL_LIMIT",
                "升级规则最多允许三级",
            ),
            Self::Escalation(AlertEscalationError::InvalidRule) | Self::PathMismatch => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_ESCALATION_INVALID",
                "升级规则字段无效",
            ),
            Self::Escalation(
                AlertEscalationError::Database(_)
                | AlertEscalationError::Audit(_)
                | AlertEscalationError::Notification(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "HAL_ESCALATION_INTERNAL",
                "升级规则处理失败",
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

pub fn alert_escalation_router(state: AlertEscalationAppState) -> Router {
    Router::new()
        .route("/api/v1/alert-escalation-rules", get(list_handler))
        .route(
            "/api/v1/alert-escalation-rules/:rule_code",
            put(upsert_handler),
        )
        .with_state(state)
}

async fn list_handler(
    ctx: AuthContext,
    State(state): State<AlertEscalationAppState>,
) -> Result<Json<AlertEscalationRuleListResponse>, AlertEscalationHandlerError> {
    ctx.require_permission("hal.escalation.read")?;
    let data = state.repository.list(ctx.owner_id).await?;
    Ok(Json(AlertEscalationRuleListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len().min(u32::MAX as usize) as u32,
            total: None,
        },
        data,
    }))
}

async fn upsert_handler(
    ctx: AuthContext,
    State(state): State<AlertEscalationAppState>,
    Path(rule_code): Path<String>,
    Json(request): Json<UpsertAlertEscalationRuleRequest>,
) -> Result<Json<AlertEscalationRule>, AlertEscalationHandlerError> {
    ctx.require_permission("hal.escalation.write")?;
    if rule_code != request.rule_code {
        return Err(AlertEscalationHandlerError::PathMismatch);
    }
    Ok(Json(
        state.repository.upsert(&ctx, request, Utc::now()).await?,
    ))
}
