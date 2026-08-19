#[allow(unused_imports)]
use super::*;

#[allow(unused_imports)]
use crate::wcs_task_service::{
    CreateWcsTaskRequest, DeviceEventRequest, ResendRequest, VoidRequest, WcsTaskResponse,
};

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
    path = "/api/v1/iot-devices/{id}/events",
    tag = "wcs_task",
    params(("id" = Uuid, Path, description = "设备 ID")),
    request_body = DeviceEventRequest,
    responses((status = 204, description = "事件已接收")),
)]
#[allow(dead_code)]
pub(crate) fn report_device_event() {}
