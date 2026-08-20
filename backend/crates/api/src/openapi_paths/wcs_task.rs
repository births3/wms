#[allow(unused_imports)]
use super::*;

#[allow(unused_imports)]
use crate::wcs_task_service::{
    ConfirmSkipRequest, CreateWcsTaskRequest, DeviceDashboardSummary, DeviceEventLog,
    DeviceEventRequest, ResendRequest, VoidRequest, WcsTaskResponse,
};

#[utoipa::path(
    get,
    path = "/api/v1/device-dashboard",
    tag = "wcs_task",
    responses((status = 200, description = "设备大盘汇总", body = DeviceDashboardSummary)),
)]
#[allow(dead_code)]
pub(crate) fn device_dashboard() {}

#[utoipa::path(
    get,
    path = "/api/v1/iot-events",
    tag = "wcs_task",
    params(
        ("device_id" = Option<Uuid>, Query, description = "设备 ID"),
        ("event_type" = Option<String>, Query, description = "事件类型"),
        ("limit" = Option<i64>, Query, description = "条数上限"),
    ),
    responses((status = 200, description = "事件流", body = Vec<DeviceEventLog>)),
)]
#[allow(dead_code)]
pub(crate) fn list_iot_events() {}

#[utoipa::path(
    post,
    path = "/api/v1/wcs-tasks",
    tag = "wcs_task",
    request_body = CreateWcsTaskRequest,
    responses(
        (status = 200, description = "指令任务生成成功", body = WcsTaskResponse),
        (status = 409, description = "亮灯互斥/搬运互斥"),
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
    responses((status = 200, description = "指令任务列表", body = Vec<WcsTaskResponse>)),
)]
#[allow(dead_code)]
pub(crate) fn list_wcs_tasks() {}

#[utoipa::path(
    get,
    path = "/api/v1/wcs-tasks/{id}",
    tag = "wcs_task",
    params(("id" = Uuid, Path, description = "任务 ID")),
    responses((status = 200, description = "任务详情", body = WcsTaskResponse)),
)]
#[allow(dead_code)]
pub(crate) fn get_wcs_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/wcs-tasks/{id}/resend",
    tag = "wcs_task",
    params(("id" = Uuid, Path, description = "任务 ID")),
    request_body = ResendRequest,
    responses((status = 200, description = "人工重发", body = WcsTaskResponse)),
)]
#[allow(dead_code)]
pub(crate) fn resend_wcs_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/wcs-tasks/{id}/void",
    tag = "wcs_task",
    params(("id" = Uuid, Path, description = "任务 ID")),
    request_body = VoidRequest,
    responses((status = 200, description = "人工作废", body = WcsTaskResponse)),
)]
#[allow(dead_code)]
pub(crate) fn void_wcs_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/wcs-tasks/{id}/confirm-skip",
    tag = "wcs_task",
    params(("id" = Uuid, Path, description = "任务 ID")),
    request_body = ConfirmSkipRequest,
    responses((status = 200, description = "跳过确认并补录账务", body = WcsTaskResponse)),
)]
#[allow(dead_code)]
pub(crate) fn confirm_skip_wcs_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/iot-devices/{id}/events",
    tag = "wcs_task",
    params(("id" = Uuid, Path, description = "设备 ID")),
    request_body = DeviceEventRequest,
    responses((status = 204, description = "事件已接收")),
)]
#[allow(dead_code)]
pub(crate) fn report_device_event() {}
