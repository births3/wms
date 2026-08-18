#[allow(unused_imports)]
use super::*;

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
