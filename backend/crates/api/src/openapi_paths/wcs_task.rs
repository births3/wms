#[allow(unused_imports)]
use super::*;

#[allow(unused_imports)]
use crate::wcs_task_service::{
    ConfirmSkipRequest, CreateWcsTaskRequest, DeviceDashboardSummary, DeviceEventLog,
    DeviceEventRequest, ReceiptRequest, ResendRequest, VoidRequest, WcsTaskResponse,
};

#[utoipa::path(
    get,
    path = "/api/v1/device-dashboard",
    tag = "wcs_task",
    params(("warehouse_id" = Uuid, Query, description = "仓库 ID")),
    responses(
        (status = 200, description = "设备大盘汇总", body = DeviceDashboardSummary),
        (status = 400, description = "查询参数非法", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "仓库范围不足", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn device_dashboard() {}

#[utoipa::path(
    get,
    path = "/api/v1/iot-events",
    tag = "wcs_task",
    params(
        ("warehouse_id" = Uuid, Query, description = "仓库 ID"),
        ("device_id" = Option<Uuid>, Query, description = "设备 ID"),
        ("event_type" = Option<String>, Query, description = "事件类型"),
        ("limit" = Option<i64>, Query, description = "条数上限"),
    ),
    responses(
        (status = 200, description = "事件流", body = Vec<DeviceEventLog>),
        (status = 400, description = "查询参数非法", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "仓库范围不足", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_iot_events() {}

#[utoipa::path(
    post,
    path = "/api/v1/wcs-tasks",
    tag = "wcs_task",
    params(("Idempotency-Key" = String, Header, description = "指令生成幂等键")),
    request_body = CreateWcsTaskRequest,
    responses(
        (status = 201, description = "指令任务生成成功", body = WcsTaskResponse),
        (status = 200, description = "幂等重放已有指令任务", body = WcsTaskResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或仓库范围不足", body = ErrorResponse),
        (status = 404, description = "设备不存在", body = ErrorResponse),
        (status = 409, description = "幂等键或任务互斥冲突", body = ErrorResponse),
        (status = 422, description = "任务参数或设备状态非法", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_wcs_task() {}

#[utoipa::path(
    get,
    path = "/api/v1/wcs-tasks",
    tag = "wcs_task",
    params(
        ("status" = Option<String>, Query, description = "状态筛选"),
        ("task_type" = Option<String>, Query, description = "指令类型筛选"),
    ),
    responses(
        (status = 200, description = "指令任务列表", body = Vec<WcsTaskResponse>),
        (status = 400, description = "查询参数非法", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_wcs_tasks() {}

#[utoipa::path(
    get,
    path = "/api/v1/wcs-tasks/{id}",
    tag = "wcs_task",
    params(("id" = Uuid, Path, description = "任务 ID")),
    responses(
        (status = 200, description = "任务详情", body = WcsTaskResponse),
        (status = 400, description = "任务 ID 非法", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "仓库范围不足", body = ErrorResponse),
        (status = 404, description = "任务或设备不存在", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_wcs_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/wcs-tasks/{id}/resend",
    tag = "wcs_task",
    params(
        ("id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "重发幂等键"),
    ),
    request_body = ResendRequest,
    responses(
        (status = 200, description = "人工重发", body = WcsTaskResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或仓库范围不足", body = ErrorResponse),
        (status = 404, description = "任务或设备不存在", body = ErrorResponse),
        (status = 409, description = "幂等键冲突", body = ErrorResponse),
        (status = 422, description = "任务状态不允许重发", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn resend_wcs_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/wcs-tasks/{id}/dispatch",
    tag = "wcs_task",
    params(
        ("id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "模拟派发幂等键"),
    ),
    responses(
        (status = 200, description = "模拟网关已派发", body = WcsTaskResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "任务不存在", body = ErrorResponse),
        (status = 409, description = "幂等键冲突", body = ErrorResponse),
        (status = 422, description = "任务状态不允许派发", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn dispatch_wcs_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/wcs-tasks/{id}/receipt",
    tag = "wcs_task",
    params(
        ("id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "模拟回执幂等键"),
    ),
    request_body = ReceiptRequest,
    responses(
        (status = 200, description = "模拟网关回执已处理", body = WcsTaskResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "任务不存在", body = ErrorResponse),
        (status = 409, description = "幂等键冲突", body = ErrorResponse),
        (status = 422, description = "回执或任务状态非法", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn apply_wcs_task_receipt() {}

#[utoipa::path(
    post,
    path = "/api/v1/wcs-tasks/{id}/void",
    tag = "wcs_task",
    params(
        ("id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "作废幂等键"),
    ),
    request_body = VoidRequest,
    responses(
        (status = 200, description = "人工作废", body = WcsTaskResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或仓库范围不足", body = ErrorResponse),
        (status = 404, description = "任务或设备不存在", body = ErrorResponse),
        (status = 409, description = "幂等键冲突", body = ErrorResponse),
        (status = 422, description = "任务状态不允许作废", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn void_wcs_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/wcs-tasks/{id}/confirm-skip",
    tag = "wcs_task",
    params(
        ("id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "跳过确认幂等键"),
    ),
    request_body = ConfirmSkipRequest,
    responses(
        (status = 200, description = "跳过确认并补录账务", body = WcsTaskResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或仓库范围不足", body = ErrorResponse),
        (status = 404, description = "任务或设备不存在", body = ErrorResponse),
        (status = 409, description = "幂等键冲突", body = ErrorResponse),
        (status = 422, description = "任务状态、证据或落账条件非法", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn confirm_skip_wcs_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/iot-devices/{id}/events",
    tag = "wcs_task",
    params(
        ("id" = Uuid, Path, description = "设备 ID"),
        ("Idempotency-Key" = String, Header, description = "事件上报幂等键"),
    ),
    request_body = DeviceEventRequest,
    responses(
        (status = 204, description = "事件已接收"),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或仓库范围不足", body = ErrorResponse),
        (status = 404, description = "设备或任务不存在", body = ErrorResponse),
        (status = 409, description = "幂等键冲突", body = ErrorResponse),
        (status = 422, description = "事件与任务不匹配", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn report_device_event() {}
