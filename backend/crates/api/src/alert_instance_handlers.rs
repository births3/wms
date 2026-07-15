use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    AlertActionRequest, AlertInstance, AlertInstanceListQuery, AlertInstanceListResponse,
    ErrorResponse, PageMeta,
};

use crate::{
    alert_instance_repository::{AlertInstanceRepositoryError, PgAlertInstanceRepository},
    alert_lifecycle_service::{AlertLifecycleError, PgAlertLifecycleService},
    auth::{AuthContext, AuthError},
};

const READ_PERMISSION: &str = "hal.alert.read";
const HANDLE_PERMISSION: &str = "hal.alert.handle";

#[derive(Clone, Debug)]
pub struct AlertInstanceAppState {
    repository: PgAlertInstanceRepository,
    lifecycle: PgAlertLifecycleService,
}

impl AlertInstanceAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: PgAlertInstanceRepository::new(pool.clone()),
            lifecycle: PgAlertLifecycleService::new(pool),
        }
    }
}

#[derive(Debug)]
pub enum AlertInstanceHandlerError {
    Auth(AuthError),
    Repository(AlertInstanceRepositoryError),
    Lifecycle(AlertLifecycleError),
}

impl From<AuthError> for AlertInstanceHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<AlertInstanceRepositoryError> for AlertInstanceHandlerError {
    fn from(value: AlertInstanceRepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl From<AlertLifecycleError> for AlertInstanceHandlerError {
    fn from(value: AlertLifecycleError) -> Self {
        Self::Lifecycle(value)
    }
}

impl IntoResponse for AlertInstanceHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::Repository(AlertInstanceRepositoryError::NotFound)
            | Self::Lifecycle(AlertLifecycleError::NotFound) => (
                StatusCode::NOT_FOUND,
                "HAL_ALERT_INSTANCE_NOT_FOUND",
                "告警实例不存在",
            ),
            Self::Lifecycle(AlertLifecycleError::InvalidTransition) => (
                StatusCode::CONFLICT,
                "HAL_ALERT_STATUS_INVALID",
                "当前告警状态不允许该操作",
            ),
            Self::Lifecycle(AlertLifecycleError::ReasonRequired) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_ALERT_REASON_REQUIRED",
                "处理、关闭或忽略原因不能为空",
            ),
            Self::Repository(AlertInstanceRepositoryError::Database(_))
            | Self::Lifecycle(AlertLifecycleError::Database(_) | AlertLifecycleError::Audit(_)) => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "HAL_ALERT_INTERNAL",
                    "告警实例处理失败",
                )
            }
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

pub fn alert_instance_router(state: AlertInstanceAppState) -> Router {
    Router::new()
        .route("/api/v1/alerts", get(list_handler))
        .route("/api/v1/alerts/:id", get(get_handler))
        .route("/api/v1/alerts/:id/acknowledge", post(acknowledge_handler))
        .route("/api/v1/alerts/:id/handling", post(handling_handler))
        .route("/api/v1/alerts/:id/close", post(close_handler))
        .route("/api/v1/alerts/:id/ignore", post(ignore_handler))
        .with_state(state)
}

async fn list_handler(
    ctx: AuthContext,
    State(state): State<AlertInstanceAppState>,
    Query(query): Query<AlertInstanceListQuery>,
) -> Result<Json<AlertInstanceListResponse>, AlertInstanceHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    let data = state.repository.list(ctx.owner_id, &query).await?;
    Ok(Json(AlertInstanceListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len().min(u32::MAX as usize) as u32,
        },
        data,
    }))
}

async fn get_handler(
    ctx: AuthContext,
    State(state): State<AlertInstanceAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AlertInstance>, AlertInstanceHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(state.repository.get(ctx.owner_id, id).await?))
}

async fn acknowledge_handler(
    ctx: AuthContext,
    State(state): State<AlertInstanceAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AlertInstance>, AlertInstanceHandlerError> {
    ctx.require_permission(HANDLE_PERMISSION)?;
    state.lifecycle.acknowledge(&ctx, id, Utc::now()).await?;
    Ok(Json(state.repository.get(ctx.owner_id, id).await?))
}

async fn handling_handler(
    ctx: AuthContext,
    State(state): State<AlertInstanceAppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<AlertActionRequest>,
) -> Result<Json<AlertInstance>, AlertInstanceHandlerError> {
    ctx.require_permission(HANDLE_PERMISSION)?;
    state
        .lifecycle
        .record_handling(&ctx, id, request.description, Utc::now())
        .await?;
    Ok(Json(state.repository.get(ctx.owner_id, id).await?))
}

async fn close_handler(
    ctx: AuthContext,
    State(state): State<AlertInstanceAppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<AlertActionRequest>,
) -> Result<Json<AlertInstance>, AlertInstanceHandlerError> {
    ctx.require_permission(HANDLE_PERMISSION)?;
    state
        .lifecycle
        .close(&ctx, id, request.description, Utc::now())
        .await?;
    Ok(Json(state.repository.get(ctx.owner_id, id).await?))
}

async fn ignore_handler(
    ctx: AuthContext,
    State(state): State<AlertInstanceAppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<AlertActionRequest>,
) -> Result<Json<AlertInstance>, AlertInstanceHandlerError> {
    ctx.require_permission(HANDLE_PERMISSION)?;
    state
        .lifecycle
        .ignore(&ctx, id, request.description, Utc::now())
        .await?;
    Ok(Json(state.repository.get(ctx.owner_id, id).await?))
}
