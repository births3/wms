#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/task-engine/task-types",
    tag = "task-engine",
    responses(
        (status = 200, description = "当前货主任务类型", body = TaskTypeListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_task_types() {}

#[utoipa::path(
    put,
    path = "/api/v1/task-engine/task-types/{task_type_code}",
    tag = "task-engine",
    params(
        ("task_type_code" = String, Path, description = "任务类型编码"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = UpsertTaskTypeRequest,
    responses(
        (status = 200, description = "保存任务类型", body = TaskType),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "任务类型参数非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn upsert_task_type() {}

#[utoipa::path(
    patch,
    path = "/api/v1/task-engine/task-types/{task_type_code}/enabled",
    tag = "task-engine",
    params(
        ("task_type_code" = String, Path, description = "任务类型编码"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = SetTaskTypeEnabledRequest,
    responses(
        (status = 200, description = "更新任务类型启停状态", body = TaskType),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "任务类型不存在", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn set_task_type_enabled() {}
