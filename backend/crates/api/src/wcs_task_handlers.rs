//! T03：指令任务 HTTP 层（事件上报 / 任务列表 / 重发 / 作废）。

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::device_service::DeviceError;
use crate::wcs_task_service::{
    ConfirmSkipRequest, CreateWcsTaskRequest, DeviceEventRequest, ResendRequest, VoidRequest,
    WcsTaskResponse, WcsTaskService,
};

#[derive(Clone)]
pub struct WcsTaskAppState {
    pub service: WcsTaskService,
}

impl WcsTaskAppState {
    pub fn with_postgres(pool: sqlx::PgPool) -> Self {
        Self {
            service: WcsTaskService::new(pool),
        }
    }
}

pub fn wcs_task_router(state: WcsTaskAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/wcs-tasks",
            get(list_tasks_handler).post(create_task_handler),
        )
        .route("/api/v1/wcs-tasks/:id", get(get_task_handler))
        .route("/api/v1/wcs-tasks/:id/resend", post(resend_task_handler))
        .route("/api/v1/wcs-tasks/:id/void", post(void_task_handler))
        .route(
            "/api/v1/wcs-tasks/:id/confirm-skip",
            post(confirm_skip_handler),
        )
        .route("/api/v1/iot-devices/:id/events", post(device_event_handler))
        .with_state(state)
}

#[derive(Deserialize)]
pub struct TaskListQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub task_type: Option<String>,
}

async fn create_task_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateWcsTaskRequest>,
) -> Result<Json<WcsTaskResponse>, WcsTaskHandlerError> {
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.create_task(&ctx, req, &key).await?))
}

async fn list_tasks_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    Query(query): Query<TaskListQuery>,
) -> Result<Json<Vec<WcsTaskResponse>>, WcsTaskHandlerError> {
    require_monitor(&ctx)?;
    Ok(Json(
        state
            .service
            .list(&ctx, query.status, query.task_type)
            .await?,
    ))
}

async fn get_task_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<WcsTaskResponse>, WcsTaskHandlerError> {
    require_monitor(&ctx)?;
    Ok(Json(state.service.get(&ctx, id).await?))
}

async fn resend_task_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    Path(id): Path<Uuid>,
    Json(_req): Json<ResendRequest>,
) -> Result<Json<WcsTaskResponse>, WcsTaskHandlerError> {
    require_manage(&ctx)?;
    Ok(Json(state.service.resend(&ctx, id).await?))
}

async fn void_task_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<VoidRequest>,
) -> Result<Json<WcsTaskResponse>, WcsTaskHandlerError> {
    require_manage(&ctx)?;
    Ok(Json(state.service.void(&ctx, id, req).await?))
}

async fn confirm_skip_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    Path(id): Path<Uuid>,
    Json(_req): Json<ConfirmSkipRequest>,
) -> Result<Json<WcsTaskResponse>, WcsTaskHandlerError> {
    require_manage(&ctx)?;
    Ok(Json(state.service.get(&ctx, id).await?))
}

async fn device_event_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<DeviceEventRequest>,
) -> Result<StatusCode, WcsTaskHandlerError> {
    require_manage(&ctx)?;
    state.service.handle_event(&ctx, id, req).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn require_manage(ctx: &AuthContext) -> Result<(), WcsTaskHandlerError> {
    if ctx.permissions.iter().any(|p| p == "m1.device.manage") {
        Ok(())
    } else {
        Err(WcsTaskHandlerError::PermissionDenied)
    }
}

fn require_monitor(ctx: &AuthContext) -> Result<(), WcsTaskHandlerError> {
    if ctx
        .permissions
        .iter()
        .any(|p| p == "m1.device.manage" || p == "m1.device.monitor")
    {
        Ok(())
    } else {
        Err(WcsTaskHandlerError::PermissionDenied)
    }
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, WcsTaskHandlerError> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(WcsTaskHandlerError::MissingIdempotencyKey)
}

pub enum WcsTaskHandlerError {
    PermissionDenied,
    MissingIdempotencyKey,
    Service(DeviceError),
}

impl From<DeviceError> for WcsTaskHandlerError {
    fn from(error: DeviceError) -> Self {
        WcsTaskHandlerError::Service(error)
    }
}

impl IntoResponse for WcsTaskHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            WcsTaskHandlerError::PermissionDenied => (
                StatusCode::FORBIDDEN,
                "M1_DEVICE_PERMISSION_DENIED",
                "设备中台权限不足",
            ),
            WcsTaskHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "M1_DEVICE_MISSING_IDEMPOTENCY_KEY",
                "缺少 Idempotency-Key",
            ),
            WcsTaskHandlerError::Service(error) => match error {
                DeviceError::NotFound => {
                    (StatusCode::NOT_FOUND, "M1_DEVICE_NOT_FOUND", "设备不存在")
                }
                DeviceError::Disabled => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "M1_DEVICE_DISABLED",
                    "设备已停用",
                ),
                DeviceError::TaskNotFound => (
                    StatusCode::NOT_FOUND,
                    "M1_WCS_TASK_NOT_FOUND",
                    "指令任务不存在",
                ),
                DeviceError::TaskStateInvalid => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "M1_WCS_TASK_STATE_INVALID",
                    "指令任务状态迁移非法",
                ),
                DeviceError::TaskVoidBlocked => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "M1_WCS_TASK_VOID_BLOCKED",
                    "已落账指令任务不可作废",
                ),
                DeviceError::PtLightBusy => (
                    StatusCode::CONFLICT,
                    "M1_PTL_LIGHT_BUSY",
                    "同一 PTL 已有未完成亮灯任务",
                ),
                DeviceError::PtQtyDiffExceeded => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "M1_PTL_QTY_DIFF_EXCEEDED",
                    "拍灯数量差异超阈值",
                ),
                DeviceError::PodMoveActive => (
                    StatusCode::CONFLICT,
                    "M1_POD_MOVE_ACTIVE",
                    "同一货架已有未完成搬运任务",
                ),
                DeviceError::EventTaskMismatch => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "M1_EVENT_TASK_MISMATCH",
                    "设备事件与指令任务不匹配",
                ),
                _other => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "M1_DEVICE_INTERNAL",
                    "设备中台内部错误",
                ),
            },
        };
        (
            status,
            Json(serde_json::json!({
                "code": code,
                "message": message,
            })),
        )
            .into_response()
    }
}
