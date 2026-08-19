#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/docks",
    tag = "master-data",
    params(("warehouse_id" = uuid::Uuid, Query, description = "物理仓库 ID")),
    responses(
        (status = 200, description = "月台档案列表", body = [Dock]),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_docks() {}

#[utoipa::path(
    post,
    path = "/api/v1/docks",
    tag = "master-data",
    params(("Idempotency-Key" = String, Header, description = "创建幂等键")),
    request_body = CreateDockRequest,
    responses(
        (status = 200, description = "创建月台档案", body = Dock),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "月台编号已存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_dock() {}

#[utoipa::path(
    post,
    path = "/api/v1/docks/import",
    tag = "master-data",
    params(("Idempotency-Key" = String, Header, description = "批量导入幂等键")),
    request_body = CreateDockImportRequest,
    responses(
        (status = 200, description = "批量导入月台档案", body = [Dock]),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "月台编号已存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn import_docks() {}

#[utoipa::path(
    patch,
    path = "/api/v1/docks/{id}",
    tag = "master-data",
    params(("id" = uuid::Uuid, Path, description = "月台档案 ID"), ("Idempotency-Key" = String, Header, description = "更新幂等键")),
    request_body = UpdateDockRequest,
    responses(
        (status = 200, description = "更新月台状态", body = Dock),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "月台档案不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn update_dock() {}

#[utoipa::path(
    delete,
    path = "/api/v1/docks/{id}",
    tag = "master-data",
    params(("id" = uuid::Uuid, Path, description = "月台档案 ID"), ("Idempotency-Key" = String, Header, description = "删除幂等键")),
    responses(
        (status = 204, description = "删除月台档案"),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "月台档案不存在", body = ErrorResponse),
        (status = 409, description = "月台存在关联预约", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn delete_dock() {}
