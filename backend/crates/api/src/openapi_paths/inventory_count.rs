#[allow(unused_imports)]
use super::*;
// Utoipa consumes these short names from attribute-macro tokens. Keeping them
// unqualified preserves refs such as `#/components/schemas/PdaLocationInfo`.
#[allow(unused_imports)]
use wms_domain::{PdaLocationInfo, QuickSpotCountRequest, QuickSpotCountResponse};

#[utoipa::path(
    post,
    path = "/api/v1/inventory/counts",
    tag = "inventory-count",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateInventoryCountRequest,
    responses(
        (status = 200, description = "创建库存盘点单及盘点明细", body = InventoryCount),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无盘点执行权限", body = ErrorResponse),
        (status = 409, description = "范围已被盘点锁占用", body = ErrorResponse),
        (status = 422, description = "盘点类型非法或范围无库存", body = ErrorResponse),
    )
)]
#[allow(dead_code)]
pub(crate) fn create_inventory_count() {}

#[utoipa::path(
    get,
    path = "/api/v1/inventory/counts/{id}",
    tag = "inventory-count",
    params(("id" = uuid::Uuid, Path, description = "盘点单 ID")),
    responses(
        (status = 200, description = "库存盘点单详情", body = InventoryCount),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无盘点查询权限", body = ErrorResponse),
        (status = 404, description = "盘点单不存在或不属于当前货主", body = ErrorResponse),
    )
)]
#[allow(dead_code)]
pub(crate) fn get_inventory_count() {}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/counts/{id}/lines/{line_id}/submit",
    tag = "inventory-count",
    params(
        ("id" = uuid::Uuid, Path, description = "盘点单 ID"),
        ("line_id" = uuid::Uuid, Path, description = "盘点明细 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = SubmitInventoryCountLineRequest,
    responses(
        (status = 200, description = "提交盲盘实盘数量并计算差异", body = InventoryCountLine),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无盘点执行权限", body = ErrorResponse),
        (status = 404, description = "盘点明细不存在", body = ErrorResponse),
        (status = 422, description = "数量或盘点状态非法", body = ErrorResponse),
    )
)]
#[allow(dead_code)]
pub(crate) fn submit_inventory_count_line() {}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/counts/{id}/approve",
    tag = "inventory-count",
    params(
        ("id" = uuid::Uuid, Path, description = "盘点单 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = ApproveInventoryCountRequest,
    responses(
        (status = 200, description = "审批盘点差异并原子调整库存", body = InventoryCount),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无盘点审批权限", body = ErrorResponse),
        (status = 404, description = "盘点单不存在", body = ErrorResponse),
        (status = 422, description = "审批源、数量或盘点状态非法", body = ErrorResponse),
    )
)]
#[allow(dead_code)]
pub(crate) fn approve_inventory_count() {}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/counts/quick-spot-count",
    tag = "inventory-count",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = QuickSpotCountRequest,
    responses(
        (status = 200, description = "按库位与商品执行快速抽盘并返回差异", body = QuickSpotCountResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无盘点执行权限", body = ErrorResponse),
        (status = 409, description = "幂等键冲突", body = ErrorResponse),
        (status = 422, description = "库位、商品、批号或数量非法", body = ErrorResponse),
    )
)]
#[allow(dead_code)]
pub(crate) fn quick_spot_count() {}

#[utoipa::path(
    get,
    path = "/api/v1/master-data/locations/by-code/{location_code}",
    tag = "master-data",
    params(("location_code" = String, Path, description = "库位编码")),
    responses(
        (status = 200, description = "按编码获取 PDA 库位信息", body = PdaLocationInfo),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无基础档案读取权限", body = ErrorResponse),
        (status = 404, description = "库位不存在", body = ErrorResponse),
    )
)]
#[allow(dead_code)]
pub(crate) fn get_location_by_code() {}
