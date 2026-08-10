#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/task-engine/task-groups",
    tag = "task-engine",
    responses(
        (status = 200, description = "当前货主任务组", body = TaskGroupListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_task_groups() {}

#[utoipa::path(
    get,
    path = "/api/v1/task-engine/workers",
    tag = "task-engine",
    params(
        ("page" = Option<u32>, Query, description = "页码，从 1 开始；缺省为 1"),
        ("page_size" = Option<u32>, Query, description = "每页条数；缺省为 20，上限 200"),
    ),
    responses(
        (status = 200, description = "当前货主可加入任务组的有效人员", body = TaskWorkerListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_task_workers() {}

#[utoipa::path(
    put,
    path = "/api/v1/task-engine/task-groups/{task_group_code}",
    tag = "task-engine",
    params(
        ("task_group_code" = String, Path, description = "任务组编码"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = UpsertTaskGroupRequest,
    responses(
        (status = 200, description = "保存任务组及成员资格", body = TaskGroup),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "任务组参数或引用非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn upsert_task_group() {}

#[utoipa::path(
    get,
    path = "/api/v1/task-engine/priority-rule",
    tag = "task-engine",
    responses(
        (status = 200, description = "当前货主任务优先级权重", body = TaskPriorityRule),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_task_priority_rule() {}

#[utoipa::path(
    put,
    path = "/api/v1/task-engine/priority-rule",
    tag = "task-engine",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = UpsertTaskPriorityRuleRequest,
    responses(
        (status = 200, description = "保存任务优先级权重", body = TaskPriorityRule),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "规则参数非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn upsert_task_priority_rule() {}

#[utoipa::path(
    get,
    path = "/api/v1/task-engine/tasks",
    tag = "task-engine",
    params(TaskListQuery),
    responses(
        (status = 200, description = "任务队列；mine_only=true 为 PDA 统一待办", body = WarehouseTaskListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_warehouse_tasks() {}

#[utoipa::path(
    post,
    path = "/api/v1/task-engine/tasks",
    tag = "task-engine",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateWarehouseTaskRequest,
    responses(
        (status = 201, description = "按任务类型释放规则创建待释放或待分配任务", body = WarehouseTask),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "任务类型不存在或未启用", body = ErrorResponse),
        (status = 409, description = "业务触发源或幂等冲突", body = ErrorResponse),
        (status = 422, description = "任务参数或引用非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_warehouse_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/task-engine/tasks/{task_id}/transitions",
    tag = "task-engine",
    params(
        ("task_id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = TransitionWarehouseTaskRequest,
    responses(
        (status = 200, description = "任务状态迁移结果", body = WarehouseTask),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足或非任务执行人", body = ErrorResponse),
        (status = 404, description = "任务不存在", body = ErrorResponse),
        (status = 409, description = "状态或幂等冲突", body = ErrorResponse),
        (status = 422, description = "释放条件、资格或执行结果非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn transition_warehouse_task() {}
