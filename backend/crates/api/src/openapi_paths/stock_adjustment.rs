#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    post,
    path = "/api/v1/stock-adjustments/loss-orders",
    tag = "stock-adjustment",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateStockLossOrderRequest,
    responses(
        (status = 201, description = "报损单已创建并由 M-CG 分配单号", body = StockLossOrder),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "仓库或库存批次不存在", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "数量、来源或召回销毁参数非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_stock_loss_order() {}

#[utoipa::path(
    get,
    path = "/api/v1/stock-adjustments/loss-orders/{id}",
    tag = "stock-adjustment",
    params(("id" = Uuid, Path, description = "报损单 ID")),
    responses(
        (status = 200, description = "报损单详情与执行证据摘要", body = StockLossOrder),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "报损单不存在或不属于当前货主", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_stock_loss_order() {}

#[utoipa::path(
    post,
    path = "/api/v1/stock-adjustments/loss-orders/{id}/quality-approval",
    tag = "stock-adjustment",
    params(
        ("id" = Uuid, Path, description = "报损单 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = StockLossQualityApprovalRequest,
    responses(
        (status = 200, description = "质量联系单审批结果已回写", body = StockLossOrder),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "报损单不存在", body = ErrorResponse),
        (status = 409, description = "状态或幂等冲突", body = ErrorResponse),
        (status = 422, description = "质量联系单 ID 为空", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn approve_stock_loss_order() {}

#[utoipa::path(
    post,
    path = "/api/v1/stock-adjustments/loss-orders/{id}/start",
    tag = "stock-adjustment",
    params(
        ("id" = Uuid, Path, description = "报损单 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "报损单进入执行中", body = StockLossOrder),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "报损单不存在", body = ErrorResponse),
        (status = 409, description = "状态或幂等冲突", body = ErrorResponse),
        (status = 422, description = "第一操作人无有效保管员资格", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn start_stock_loss_order() {}

#[utoipa::path(
    post,
    path = "/api/v1/stock-adjustments/loss-orders/{id}/execute",
    tag = "stock-adjustment",
    params(
        ("id" = Uuid, Path, description = "报损单 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = ExecuteStockLossOrderRequest,
    responses(
        (status = 200, description = "按 M-VR 双人策略完成报损并原子扣减库存", body = StockLossOrder),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "报损单不存在", body = ErrorResponse),
        (status = 409, description = "状态、主管审批或幂等冲突", body = ErrorResponse),
        (status = 422, description = "数量超限、第二操作人缺失/相同/无资格", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn execute_stock_loss_order() {}
