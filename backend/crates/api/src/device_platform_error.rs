//! 设备中台统一 HTTP 错误（单一 IntoResponse 来源；设备与指令两路由共用）。

use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use crate::auth::AuthContext;
use crate::device_service::DeviceError;

pub enum DevicePlatformHandlerError {
    PermissionDenied,
    MissingIdempotencyKey,
    Service(DeviceError),
}

impl From<DeviceError> for DevicePlatformHandlerError {
    fn from(error: DeviceError) -> Self {
        DevicePlatformHandlerError::Service(error)
    }
}

impl IntoResponse for DevicePlatformHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            DevicePlatformHandlerError::PermissionDenied => (
                StatusCode::FORBIDDEN,
                "M1_DEVICE_PERMISSION_DENIED",
                "设备中台权限不足",
            ),
            DevicePlatformHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "M1_DEVICE_MISSING_IDEMPOTENCY_KEY",
                "缺少 Idempotency-Key",
            ),
            DevicePlatformHandlerError::Service(error) => match error {
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
                DeviceError::LocationUnreachable => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "M1_LOCATION_UNREACHABLE",
                    "格口处于 AGV 搬运不可达期",
                ),
                DeviceError::NumberingUnavailable => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "M1_NUMBERING_UNAVAILABLE",
                    "wcs_task 无可用编号规则",
                ),
                DeviceError::VersionConflict => (
                    StatusCode::CONFLICT,
                    "M1_DEVICE_VERSION_CONFLICT",
                    "设备档案已被其他操作更新",
                ),
                DeviceError::IdempotencyConflict => (
                    StatusCode::CONFLICT,
                    "M1_WCS_TASK_IDEMPOTENCY_CONFLICT",
                    "幂等键已用于不同请求",
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

pub(crate) fn require_manage(ctx: &AuthContext) -> Result<(), DevicePlatformHandlerError> {
    if ctx.permissions.iter().any(|p| p == "m1.device.manage") {
        Ok(())
    } else {
        Err(DevicePlatformHandlerError::PermissionDenied)
    }
}

pub(crate) fn require_monitor(ctx: &AuthContext) -> Result<(), DevicePlatformHandlerError> {
    if ctx
        .permissions
        .iter()
        .any(|p| p == "m1.device.manage" || p == "m1.device.monitor")
    {
        Ok(())
    } else {
        Err(DevicePlatformHandlerError::PermissionDenied)
    }
}

pub(crate) fn require_bind_manage(ctx: &AuthContext) -> Result<(), DevicePlatformHandlerError> {
    if ctx
        .permissions
        .iter()
        .any(|p| p == "m1.device-bind.manage" || p == "m1.device.manage")
    {
        Ok(())
    } else {
        Err(DevicePlatformHandlerError::PermissionDenied)
    }
}

pub(crate) fn idempotency_key(headers: &HeaderMap) -> Result<String, DevicePlatformHandlerError> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(DevicePlatformHandlerError::MissingIdempotencyKey)
}
