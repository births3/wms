#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/master-data/customers/{customer_id}/addresses",
    tag = "master-data",
    params(("customer_id" = uuid::Uuid, Path, description = "客户 ID")),
    responses(
        (status = 200, description = "客户收货地址列表", body = CustomerAddressListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "客户不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_customer_addresses() {}

#[utoipa::path(
    post,
    path = "/api/v1/master-data/customers/{customer_id}/addresses",
    tag = "master-data",
    params(
        ("customer_id" = uuid::Uuid, Path, description = "客户 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = CreateCustomerAddressRequest,
    responses(
        (status = 200, description = "创建客户收货地址", body = CustomerAddress),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "客户不存在", body = ErrorResponse),
        (status = 422, description = "地址字段非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_customer_address() {}

#[utoipa::path(
    patch,
    path = "/api/v1/master-data/customers/{customer_id}/addresses/{address_id}",
    tag = "master-data",
    params(
        ("customer_id" = uuid::Uuid, Path, description = "客户 ID"),
        ("address_id" = uuid::Uuid, Path, description = "地址 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = UpdateCustomerAddressRequest,
    responses(
        (status = 200, description = "更新客户收货地址", body = CustomerAddress),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "客户或地址不存在", body = ErrorResponse),
        (status = 422, description = "地址字段非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn update_customer_address() {}
