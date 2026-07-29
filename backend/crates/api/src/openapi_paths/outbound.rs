#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    post,
    path = "/api/v1/outbound/orders/{id}/revalidate",
    tag = "outbound",
    params(
        ("id" = uuid::Uuid, Path, description = "出库订单 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "重新校验出库订单；通过置 confirmed，失败置 validation_exception", body = OutboundOrder),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无出库写权限", body = ErrorResponse),
        (status = 404, description = "出库订单不存在", body = ErrorResponse),
        (status = 409, description = "幂等键已用于不同请求", body = ErrorResponse),
        (status = 422, description = "订单状态不允许重新校验", body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn revalidate_outbound_order() {}

#[utoipa::path(
    post,
    path = "/api/v1/outbound/orders/{id}/void-request",
    tag = "outbound",
    params(
        ("id" = uuid::Uuid, Path, description = "出库订单 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "提交作废申请，订单置 void_requested", body = OutboundOrder),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无出库写权限", body = ErrorResponse),
        (status = 404, description = "出库订单不存在", body = ErrorResponse),
        (status = 409, description = "幂等键已用于不同请求", body = ErrorResponse),
        (status = 422, description = "订单已进入波次或后续状态，不允许作废申请", body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn void_request_outbound_order() {}

#[utoipa::path(
    post,
    path = "/api/v1/outbound/waves/{wave_id}/release",
    tag = "outbound",
    params(
        ("wave_id" = uuid::Uuid, Path, description = "出库波次 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "下发草稿波次：锁定订单库存并生成拣选任务", body = OutboundWave),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无出库写权限", body = ErrorResponse),
        (status = 404, description = "出库波次不存在", body = ErrorResponse),
        (status = 409, description = "幂等键已用于不同请求", body = ErrorResponse),
        (status = 422, description = "波次或订单状态不允许下发", body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn release_outbound_wave() {}

#[utoipa::path(
    post,
    path = "/api/v1/outbound/purchase-returns",
    tag = "outbound",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreatePurchaseReturnRequest,
    responses(
        (status = 200, description = "创建采购退货出库单，初始状态 pending_approval", body = PurchaseReturnOrder),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无出库写权限", body = ErrorResponse),
        (status = 409, description = "退货单号重复或幂等键已用于不同请求", body = ErrorResponse),
        (status = 422, description = "数量非法", body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn create_purchase_return() {}

#[utoipa::path(
    get,
    path = "/api/v1/outbound/purchase-returns",
    tag = "outbound",
    params(
        ("status" = Option<String>, Query, description = "按退货单状态过滤"),
        ("q" = Option<String>, Query, description = "按退货单号/来源采购单号/供应商模糊查询"),
        ("limit" = Option<u32>, Query, description = "返回条数，默认 50，最大 200"),
    ),
    responses(
        (status = 200, description = "采购退货出库单列表", body = PurchaseReturnOrderListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_purchase_returns() {}

#[utoipa::path(
    get,
    path = "/api/v1/outbound/purchase-returns/{id}",
    tag = "outbound",
    params(("id" = uuid::Uuid, Path, description = "采购退货出库单 ID")),
    responses(
        (status = 200, description = "采购退货出库单详情", body = PurchaseReturnOrder),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "采购退货出库单不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_purchase_return() {}

#[utoipa::path(
    post,
    path = "/api/v1/outbound/purchase-returns/{id}/approve",
    tag = "outbound",
    params(
        ("id" = uuid::Uuid, Path, description = "采购退货出库单 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "审批通过：pending_approval → approved", body = PurchaseReturnOrder),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无出库写权限", body = ErrorResponse),
        (status = 404, description = "采购退货出库单不存在", body = ErrorResponse),
        (status = 409, description = "幂等键已用于不同请求", body = ErrorResponse),
        (status = 422, description = "当前状态不允许审批", body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn approve_purchase_return() {}

#[utoipa::path(
    post,
    path = "/api/v1/outbound/purchase-returns/{id}/reject",
    tag = "outbound",
    params(
        ("id" = uuid::Uuid, Path, description = "采购退货出库单 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = RejectPurchaseReturnRequest,
    responses(
        (status = 200, description = "审批驳回：pending_approval → cancelled，驳回原因必填", body = PurchaseReturnOrder),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无出库写权限", body = ErrorResponse),
        (status = 404, description = "采购退货出库单不存在", body = ErrorResponse),
        (status = 409, description = "幂等键已用于不同请求", body = ErrorResponse),
        (status = 422, description = "当前状态不允许驳回或驳回原因缺失", body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn reject_purchase_return() {}

#[utoipa::path(
    post,
    path = "/api/v1/outbound/purchase-returns/{id}/pick",
    tag = "outbound",
    params(
        ("id" = uuid::Uuid, Path, description = "采购退货出库单 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "退货拣货：approved → picking", body = PurchaseReturnOrder),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无出库写权限", body = ErrorResponse),
        (status = 404, description = "采购退货出库单不存在", body = ErrorResponse),
        (status = 409, description = "幂等键已用于不同请求", body = ErrorResponse),
        (status = 422, description = "当前状态不允许拣货", body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn pick_purchase_return() {}

#[utoipa::path(
    post,
    path = "/api/v1/outbound/purchase-returns/{id}/review",
    tag = "outbound",
    params(
        ("id" = uuid::Uuid, Path, description = "采购退货出库单 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "退货复核：picking → reviewed", body = PurchaseReturnOrder),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无出库写权限", body = ErrorResponse),
        (status = 404, description = "采购退货出库单不存在", body = ErrorResponse),
        (status = 409, description = "幂等键已用于不同请求", body = ErrorResponse),
        (status = 422, description = "当前状态不允许复核", body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn review_purchase_return() {}

#[utoipa::path(
    post,
    path = "/api/v1/outbound/purchase-returns/{id}/ship",
    tag = "outbound",
    params(
        ("id" = uuid::Uuid, Path, description = "采购退货出库单 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "出库交接：reviewed → shipped，记录交接时间与操作人", body = PurchaseReturnOrder),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无出库写权限", body = ErrorResponse),
        (status = 404, description = "采购退货出库单不存在", body = ErrorResponse),
        (status = 409, description = "幂等键已用于不同请求", body = ErrorResponse),
        (status = 422, description = "当前状态不允许出库交接", body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn ship_purchase_return() {}
