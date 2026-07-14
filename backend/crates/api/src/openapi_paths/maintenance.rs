#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/inventory/maintenance/tasks",
    tag = "inventory-maintenance",
    params(
        ("task_id" = Option<uuid::Uuid>, Query, description = "养护任务 ID"),
        ("batch_id" = Option<uuid::Uuid>, Query, description = "库存批次 ID"),
        ("status" = Option<String>, Query, description = "任务状态：pending/completed"),
    ),
    responses(
        (status = 200, description = "养护任务列表", body = MaintenanceTaskListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    )
)]
#[allow(dead_code)]
pub(crate) fn list_maintenance_tasks() {}

#[utoipa::path(
    get,
    path = "/api/v1/inventory/maintenance/records",
    tag = "inventory-maintenance",
    params(
        ("task_id" = Option<uuid::Uuid>, Query, description = "养护任务 ID"),
        ("batch_id" = Option<uuid::Uuid>, Query, description = "库存批次 ID"),
    ),
    responses(
        (status = 200, description = "养护记录列表", body = MaintenanceRecordListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    )
)]
#[allow(dead_code)]
pub(crate) fn list_maintenance_records() {}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/maintenance/records",
    tag = "inventory-maintenance",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateMaintenanceRecordRequest,
    responses(
        (status = 200, description = "写入一次养护结果", body = MaintenanceRecord),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "仅养护员可写入", body = ErrorResponse),
        (status = 404, description = "养护任务不存在或不属于当前货主", body = ErrorResponse),
        (status = 409, description = "幂等键冲突", body = ErrorResponse),
        (status = 422, description = "批号、效期、库存状态或养护字段校验失败", body = ErrorResponse),
    )
)]
#[allow(dead_code)]
pub(crate) fn create_maintenance_record() {}
