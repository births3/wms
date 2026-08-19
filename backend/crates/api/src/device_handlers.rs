//! T02：设备中台 HTTP 层（注册/列表/详情/维护/心跳/绑定/解绑）。

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
use crate::device_service::{
    BindDeviceRequest, DeviceBindingResponse, DeviceError, DeviceResponse, DeviceService,
    RegisterDeviceRequest, UnbindRequest, UpdateDeviceRequest,
};

#[derive(Clone)]
pub struct DeviceAppState {
    pub service: DeviceService,
}

impl DeviceAppState {
    pub fn with_postgres(pool: sqlx::PgPool) -> Self {
        Self {
            service: DeviceService::new(pool),
        }
    }
}

pub fn device_router(state: DeviceAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/iot-devices",
            get(list_devices_handler).post(register_device_handler),
        )
        .route(
            "/api/v1/iot-devices/:id",
            get(get_device_handler).patch(update_device_handler),
        )
        .route("/api/v1/iot-devices/:id/heartbeat", post(heartbeat_handler))
        .route(
            "/api/v1/location-device-bindings",
            post(bind_device_handler),
        )
        .route(
            "/api/v1/location-device-bindings/:id/unbind",
            post(unbind_device_handler),
        )
        .with_state(state)
}

#[derive(Deserialize)]
pub struct DeviceListQuery {
    #[serde(default)]
    pub device_type: Option<String>,
    #[serde(default)]
    pub online_status: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

async fn register_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    headers: HeaderMap,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<Json<DeviceResponse>, DeviceHandlerError> {
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.register(&ctx, req, &key).await?))
}

async fn list_devices_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    Query(query): Query<DeviceListQuery>,
) -> Result<Json<Vec<DeviceResponse>>, DeviceHandlerError> {
    require_monitor(&ctx)?;
    Ok(Json(
        state
            .service
            .list(&ctx, query.device_type, query.online_status, query.enabled)
            .await?,
    ))
}

async fn get_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeviceResponse>, DeviceHandlerError> {
    require_monitor(&ctx)?;
    Ok(Json(state.service.get(&ctx, id).await?))
}

async fn update_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDeviceRequest>,
) -> Result<Json<DeviceResponse>, DeviceHandlerError> {
    require_manage(&ctx)?;
    Ok(Json(state.service.update(&ctx, id, req).await?))
}

async fn heartbeat_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeviceResponse>, DeviceHandlerError> {
    require_manage(&ctx)?;
    Ok(Json(state.service.heartbeat(&ctx, id).await?))
}

async fn bind_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    headers: HeaderMap,
    Json(req): Json<BindDeviceRequest>,
) -> Result<Json<DeviceBindingResponse>, DeviceHandlerError> {
    require_bind_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.bind(&ctx, req, &key).await?))
}

async fn unbind_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UnbindRequest>,
) -> Result<StatusCode, DeviceHandlerError> {
    require_bind_manage(&ctx)?;
    state.service.unbind(&ctx, id, req).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn require_manage(ctx: &AuthContext) -> Result<(), DeviceHandlerError> {
    if ctx.permissions.iter().any(|p| p == "m1.device.manage") {
        Ok(())
    } else {
        Err(DeviceHandlerError::PermissionDenied)
    }
}

fn require_monitor(ctx: &AuthContext) -> Result<(), DeviceHandlerError> {
    if ctx
        .permissions
        .iter()
        .any(|p| p == "m1.device.manage" || p == "m1.device.monitor")
    {
        Ok(())
    } else {
        Err(DeviceHandlerError::PermissionDenied)
    }
}

fn require_bind_manage(ctx: &AuthContext) -> Result<(), DeviceHandlerError> {
    if ctx
        .permissions
        .iter()
        .any(|p| p == "m1.device-bind.manage" || p == "m1.device.manage")
    {
        Ok(())
    } else {
        Err(DeviceHandlerError::PermissionDenied)
    }
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, DeviceHandlerError> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(DeviceHandlerError::MissingIdempotencyKey)
}

pub enum DeviceHandlerError {
    PermissionDenied,
    MissingIdempotencyKey,
    Service(DeviceError),
}

impl From<DeviceError> for DeviceHandlerError {
    fn from(error: DeviceError) -> Self {
        DeviceHandlerError::Service(error)
    }
}

impl IntoResponse for DeviceHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            DeviceHandlerError::PermissionDenied => (
                StatusCode::FORBIDDEN,
                "M1_DEVICE_PERMISSION_DENIED",
                "设备中台权限不足",
            ),
            DeviceHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "M1_DEVICE_MISSING_IDEMPOTENCY_KEY",
                "缺少 Idempotency-Key",
            ),
            DeviceHandlerError::Service(error) => match error {
                DeviceError::DuplicateCode => (
                    StatusCode::CONFLICT,
                    "M1_DEVICE_DUPLICATE_CODE",
                    "设备编码在仓库内已存在",
                ),
                DeviceError::TypeInvalid => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "M1_DEVICE_TYPE_INVALID",
                    "设备类型非法",
                ),
                DeviceError::NotFound => {
                    (StatusCode::NOT_FOUND, "M1_DEVICE_NOT_FOUND", "设备不存在")
                }
                DeviceError::Disabled => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "M1_DEVICE_DISABLED",
                    "设备已停用",
                ),
                DeviceError::Offline => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "M1_DEVICE_OFFLINE",
                    "设备离线",
                ),
                DeviceError::BindConflict => (
                    StatusCode::CONFLICT,
                    "M1_BIND_CONFLICT",
                    "同一库位同一角色已有生效绑定",
                ),
                DeviceError::BindDeviceMismatch => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "M1_BIND_DEVICE_MISMATCH",
                    "绑定角色与设备类型不匹配",
                ),
                DeviceError::BindNotFound => (
                    StatusCode::NOT_FOUND,
                    "M1_BIND_NOT_FOUND",
                    "绑定不存在或已解绑",
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
                DeviceError::Database(_) => (
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
