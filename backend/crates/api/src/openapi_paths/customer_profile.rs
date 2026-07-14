#[utoipa::path(
    get,
    path = "/api/v1/master-data/customers/{customer_id}/profile",
    tag = "master-data",
    params(("customer_id" = uuid::Uuid, Path, description = "客户 ID")),
    responses(
        (status = 200, description = "客户与门店档案扩展信息", body = CustomerProfile),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "客户不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_customer_profile() {}

#[utoipa::path(
    patch,
    path = "/api/v1/master-data/customers/{customer_id}/profile",
    tag = "master-data",
    params(
        ("customer_id" = uuid::Uuid, Path, description = "客户 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = UpsertCustomerProfileRequest,
    responses(
        (status = 200, description = "保存客户与门店档案扩展信息", body = CustomerProfile),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "客户不存在", body = ErrorResponse),
        (status = 422, description = "客户档案字段非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn upsert_customer_profile() {}
