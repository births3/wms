#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/dock-appointments",
    tag = "h-dock",
    params(
        ("warehouse_id" = uuid::Uuid, Query, description = "仓库 ID"),
        ("dock_id" = Option<uuid::Uuid>, Query, description = "月台 ID"),
        ("from" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "查询窗口起点（RFC3339）"),
        ("to" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "查询窗口终点（RFC3339）"),
        ("status" = Option<String>, Query, description = "预约状态")
    ),
    responses(
        (status = 200, description = "查询月台预约列表", body = [DockAppointment]),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "仓库或月台不存在", body = ErrorResponse),
        (status = 422, description = "时间窗非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_dock_appointments() {}

#[utoipa::path(
    post,
    path = "/api/v1/dock-appointments",
    tag = "h-dock",
    params(("Idempotency-Key" = String, Header, description = "预约创建幂等键")),
    request_body = CreateDockAppointmentRequest,
    responses(
        (status = 200, description = "创建月台预约", body = DockAppointment),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "月台或仓库不存在", body = ErrorResponse),
        (status = 409, description = "预约冲突", body = ErrorResponse),
        (status = 422, description = "预约字段或时间窗非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_dock_appointment() {}

#[utoipa::path(
    patch,
    path = "/api/v1/dock-appointments/{id}",
    tag = "h-dock",
    params(
        ("id" = uuid::Uuid, Path, description = "预约 ID"),
        ("Idempotency-Key" = String, Header, description = "预约变更幂等键")
    ),
    request_body = UpdateDockAppointmentRequest,
    responses(
        (status = 200, description = "变更月台预约并创建新版本", body = DockAppointment),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "预约或月台不存在", body = ErrorResponse),
        (status = 409, description = "预约冲突或状态不允许", body = ErrorResponse),
        (status = 422, description = "预约字段或时间窗非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn update_dock_appointment() {}

#[utoipa::path(
    post,
    path = "/api/v1/dock-appointments/{id}/cancel",
    tag = "h-dock",
    params(
        ("id" = uuid::Uuid, Path, description = "预约 ID"),
        ("Idempotency-Key" = String, Header, description = "预约取消幂等键")
    ),
    request_body = CancelDockAppointmentRequest,
    responses(
        (status = 200, description = "取消月台预约", body = DockAppointment),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "预约不存在", body = ErrorResponse),
        (status = 409, description = "预约状态不允许取消", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn cancel_dock_appointment() {}

#[utoipa::path(
    post,
    path = "/api/v1/dock-appointments/{id}/arrive",
    tag = "h-dock",
    params(
        ("id" = uuid::Uuid, Path, description = "预约 ID"),
        ("Idempotency-Key" = String, Header, description = "预约到达核对幂等键")
    ),
    request_body = ArriveDockAppointmentRequest,
    responses(
        (status = 200, description = "预约到达核对", body = DockAppointment),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "预约不存在", body = ErrorResponse),
        (status = 409, description = "到达核对冲突", body = ErrorResponse),
        (status = 422, description = "到达核对字段非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn arrive_dock_appointment() {}
