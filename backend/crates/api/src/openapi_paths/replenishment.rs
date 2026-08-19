#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/replenishment/strategies",
    tag = "replenishment",
    params(
        ("keyword" = Option<String>, Query, description = "策略编码或名称"),
        ("enabled" = Option<bool>, Query, description = "启停"),
        ("scope_type" = Option<String>, Query, description = "范围类型"),
    ),
    responses((status = 200, description = "策略列表", body = ReplenishmentStrategyListResponse)),
)]
#[allow(dead_code)]
pub(crate) fn list_replenishment_strategies() {}

#[utoipa::path(
    get,
    path = "/api/v1/replenishment/strategies/{id}",
    tag = "replenishment",
    params(("id" = Uuid, Path, description = "策略 ID")),
    responses((status = 200, description = "策略详情", body = ReplenishmentStrategy)),
)]
#[allow(dead_code)]
pub(crate) fn get_replenishment_strategy() {}

#[utoipa::path(
    put,
    path = "/api/v1/replenishment/strategies/{id}",
    tag = "replenishment",
    params(
        ("id" = Uuid, Path, description = "策略 ID"),
        ("Idempotency-Key" = String, Header, description = "更新幂等键"),
    ),
    request_body = UpsertReplenishmentStrategyRequest,
    responses((status = 200, description = "更新补货策略", body = ReplenishmentStrategy)),
)]
#[allow(dead_code)]
pub(crate) fn update_replenishment_strategy() {}

#[utoipa::path(
    post,
    path = "/api/v1/replenishment/strategies/{id}/disable",
    tag = "replenishment",
    params(
        ("id" = Uuid, Path, description = "策略 ID"),
        ("Idempotency-Key" = String, Header, description = "停用幂等键"),
    ),
    responses((status = 200, description = "停用补货策略", body = ReplenishmentStrategy)),
)]
#[allow(dead_code)]
pub(crate) fn disable_replenishment_strategy() {}

#[utoipa::path(
    get,
    path = "/api/v1/replenishment/location-groups",
    tag = "replenishment",
    responses((status = 200, description = "库位组列表", body = ReplenishmentLocationGroupListResponse)),
)]
#[allow(dead_code)]
pub(crate) fn list_replenishment_location_groups() {}

#[utoipa::path(
    post,
    path = "/api/v1/replenishment/strategies",
    tag = "replenishment",
    params(("Idempotency-Key" = String, Header, description = "创建幂等键")),
    request_body = UpsertReplenishmentStrategyRequest,
    responses(
        (status = 200, description = "创建补货策略", body = ReplenishmentStrategy),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "范围引用不存在", body = ErrorResponse),
        (status = 422, description = "动线或水位非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_replenishment_strategy() {}

#[utoipa::path(
    put,
    path = "/api/v1/replenishment/strategies/{id}/locations",
    tag = "replenishment",
    params(
        ("id" = Uuid, Path, description = "策略 ID"),
        ("Idempotency-Key" = String, Header, description = "挂接幂等键"),
    ),
    request_body = BindReplenishmentLocationsRequest,
    responses(
        (status = 200, description = "挂接拣选位", body = BindReplenishmentLocationsResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "库位已挂其他策略", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn bind_replenishment_locations() {}

#[utoipa::path(
    get,
    path = "/api/v1/replenishment/strategies/{id}/preview",
    tag = "replenishment",
    params(("id" = Uuid, Path, description = "策略 ID")),
    responses(
        (status = 200, description = "命中预览", body = ReplenishmentPreviewResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "策略不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn preview_replenishment_strategy() {}

#[utoipa::path(
    post,
    path = "/api/v1/replenishment/location-groups",
    tag = "replenishment",
    params(("Idempotency-Key" = String, Header, description = "创建幂等键")),
    request_body = UpsertReplenishmentLocationGroupRequest,
    responses(
        (status = 200, description = "创建库位组", body = ReplenishmentLocationGroup),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_replenishment_location_group() {}

#[utoipa::path(
    get,
    path = "/api/v1/replenishment/location-groups/{id}",
    tag = "replenishment",
    params(("id" = Uuid, Path, description = "库位组 ID")),
    responses((status = 200, description = "库位组详情", body = ReplenishmentLocationGroup)),
)]
#[allow(dead_code)]
pub(crate) fn get_replenishment_location_group() {}

#[utoipa::path(
    put,
    path = "/api/v1/replenishment/location-groups/{id}",
    tag = "replenishment",
    params(
        ("id" = Uuid, Path, description = "库位组 ID"),
        ("Idempotency-Key" = String, Header, description = "更新幂等键"),
    ),
    request_body = UpsertReplenishmentLocationGroupRequest,
    responses((status = 200, description = "更新库位组", body = ReplenishmentLocationGroup)),
)]
#[allow(dead_code)]
pub(crate) fn update_replenishment_location_group() {}

#[utoipa::path(
    post,
    path = "/api/v1/replenishment/location-groups/{id}/disable",
    tag = "replenishment",
    params(
        ("id" = Uuid, Path, description = "库位组 ID"),
        ("Idempotency-Key" = String, Header, description = "停用幂等键"),
    ),
    responses((status = 200, description = "停用库位组", body = ReplenishmentLocationGroup)),
)]
#[allow(dead_code)]
pub(crate) fn disable_replenishment_location_group() {}

#[utoipa::path(
    get,
    path = "/api/v1/replenishment/tasks",
    tag = "replenishment",
    params(
        ("status" = Option<String>, Query, description = "任务状态"),
        ("trigger_mode" = Option<String>, Query, description = "触发模式"),
        ("priority" = Option<String>, Query, description = "优先级"),
        ("source_location_id" = Option<Uuid>, Query, description = "来源库位"),
        ("target_location_id" = Option<Uuid>, Query, description = "目标库位"),
        ("location_id" = Option<Uuid>, Query, description = "来源或目标库位"),
        ("operator_id" = Option<Uuid>, Query, description = "作业员"),
        ("wave_id" = Option<Uuid>, Query, description = "波次"),
        ("keyword" = Option<String>, Query, description = "任务号"),
        ("created_from" = Option<String>, Query, description = "创建起始时间"),
        ("created_to" = Option<String>, Query, description = "创建截止时间"),
        ("limit" = Option<u32>, Query, description = "每页条数"),
        ("cursor" = Option<String>, Query, description = "分页游标"),
    ),
    responses((status = 200, description = "补货任务列表", body = ReplenishmentTaskListResponse)),
)]
#[allow(dead_code)]
pub(crate) fn list_replenishment_tasks() {}

#[utoipa::path(
    post,
    path = "/api/v1/replenishment/tasks",
    tag = "replenishment",
    params(("Idempotency-Key" = String, Header, description = "手工发起幂等键")),
    request_body = CreateReplenishmentTaskRequest,
    responses(
        (status = 200, description = "手工发起补货任务", body = ReplenishmentTask),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 422, description = "编号不可用、包装非整或来源不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_replenishment_task() {}

#[utoipa::path(
    get,
    path = "/api/v1/replenishment/tasks/{id}",
    tag = "replenishment",
    params(("id" = Uuid, Path, description = "任务 ID")),
    responses((status = 200, description = "补货任务详情", body = ReplenishmentTask)),
)]
#[allow(dead_code)]
pub(crate) fn get_replenishment_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/replenishment/tasks/{id}/claim",
    tag = "replenishment",
    params(
        ("id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "领取幂等键"),
    ),
    request_body = ClaimReplenishmentTaskRequest,
    responses(
        (status = 200, description = "领取补货任务", body = ReplenishmentTask),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "领取冲突", body = ErrorResponse),
        (status = 422, description = "目标库区不在作业员班组", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn claim_replenishment_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/replenishment/tasks/{id}/pick",
    tag = "replenishment",
    params(
        ("id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "下架幂等键"),
    ),
    request_body = PickReplenishmentTaskRequest,
    responses(
        (status = 200, description = "补货下架登记", body = ReplenishmentTask),
        (status = 422, description = "扫码不符或超量", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn pick_replenishment_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/replenishment/tasks/{id}/confirm",
    tag = "replenishment",
    params(
        ("id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "确认幂等键"),
    ),
    request_body = ConfirmReplenishmentTaskRequest,
    responses(
        (status = 200, description = "补货送达确认", body = ReplenishmentTask),
        (status = 422, description = "状态非法或上架阻断", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn confirm_replenishment_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/replenishment/tasks/{id}/cancel",
    tag = "replenishment",
    params(
        ("id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "取消幂等键"),
    ),
    request_body = CancelReplenishmentTaskRequest,
    responses(
        (status = 200, description = "取消补货任务", body = ReplenishmentTask),
        (status = 422, description = "已下架不可取消", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn cancel_replenishment_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/replenishment/tasks/{id}/reassign",
    tag = "replenishment",
    params(
        ("id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "改派幂等键"),
    ),
    request_body = ReassignReplenishmentTaskRequest,
    responses((status = 200, description = "改派回池", body = ReplenishmentTask)),
)]
#[allow(dead_code)]
pub(crate) fn reassign_replenishment_task() {}

#[utoipa::path(
    post,
    path = "/api/v1/replenishment/tasks/{id}/return",
    tag = "replenishment",
    params(
        ("id" = Uuid, Path, description = "任务 ID"),
        ("Idempotency-Key" = String, Header, description = "退回幂等键"),
    ),
    request_body = ReturnReplenishmentTaskRequest,
    responses(
        (status = 200, description = "退回待领", body = ReplenishmentTask),
        (status = 422, description = "已下架不可退回", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn return_replenishment_task() {}
