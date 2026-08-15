#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/master-data/lpn-containers",
    tag = "master-data",
    params(
        ("keyword" = Option<String>, Query, description = "LPN 码关键字"),
        ("type" = Option<String>, Query, description = "容器类型"),
        ("status" = Option<String>, Query, description = "容器状态"),
    ),
    responses(
        (status = 200, description = "LPN 容器列表", body = LpnContainerListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_lpn_containers() {}

#[utoipa::path(
    post,
    path = "/api/v1/master-data/lpn-containers",
    tag = "master-data",
    params(("Idempotency-Key" = String, Header, description = "创建幂等键")),
    request_body = CreateLpnContainerRequest,
    responses(
        (status = 200, description = "创建 LPN 容器", body = LpnContainer),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "LPN 码已存在", body = ErrorResponse),
        (status = 422, description = "容器类型非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_lpn_container() {}

#[utoipa::path(
    patch,
    path = "/api/v1/master-data/lpn-containers/{id}",
    tag = "master-data",
    params(
        ("id" = uuid::Uuid, Path, description = "LPN 容器 ID"),
        ("Idempotency-Key" = String, Header, description = "更新幂等键"),
    ),
    request_body = UpdateLpnContainerRequest,
    responses(
        (status = 200, description = "更新 LPN 容器", body = LpnContainer),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "LPN 容器不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn update_lpn_container() {}

#[utoipa::path(
    get,
    path = "/api/v1/master-data/lpn-container-type-policies",
    tag = "master-data",
    responses(
        (status = 200, description = "容器类型混装策略", body = Vec<LpnContainerTypePolicy>),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_lpn_container_type_policies() {}

#[utoipa::path(
    put,
    path = "/api/v1/master-data/lpn-container-type-policies",
    tag = "master-data",
    request_body = UpsertLpnContainerTypePolicyRequest,
    responses(
        (status = 200, description = "保存容器类型混装策略", body = LpnContainerTypePolicy),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 422, description = "容器类型非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn upsert_lpn_container_type_policy() {}
