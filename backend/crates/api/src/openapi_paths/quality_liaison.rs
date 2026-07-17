#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/quality-liaisons/types/{type_code}",
    tag = "quality-liaison",
    params(("type_code" = String, Path, description = "质量联系单类型编码")),
    responses(
        (status = 200, description = "当前货主的类型与 H4 审批模板配置", body = QualityLiaisonTypeConfig),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "类型不存在或不属于当前货主", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_quality_liaison_type() {}

#[utoipa::path(
    put,
    path = "/api/v1/quality-liaisons/types/{type_code}",
    tag = "quality-liaison",
    params(
        ("type_code" = String, Path, description = "质量联系单类型编码"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = UpsertQualityLiaisonTypeRequest,
    responses(
        (status = 200, description = "类型与 H4 审批模板配置已保存", body = QualityLiaisonTypeConfig),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "类型、审批人或超时时长非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn upsert_quality_liaison_type() {}

#[utoipa::path(
    post,
    path = "/api/v1/quality-liaisons",
    tag = "quality-liaison",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateQualityLiaisonRequest,
    responses(
        (status = 200, description = "质量联系单与 H4 审批记录已原子创建", body = QualityLiaisonOrder),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "类型未启用或必填字段缺失", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_quality_liaison() {}

#[utoipa::path(
    get,
    path = "/api/v1/quality-liaisons/{id}",
    tag = "quality-liaison",
    params(("id" = Uuid, Path, description = "质量联系单 ID")),
    responses(
        (status = 200, description = "质量联系单详情与审批链路", body = QualityLiaisonOrder),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "联系单不存在或不属于当前货主", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_quality_liaison() {}

#[utoipa::path(
    post,
    path = "/api/v1/quality-liaisons/{id}/approval-callback",
    tag = "quality-liaison",
    params(
        ("id" = Uuid, Path, description = "质量联系单 ID"),
        ("Idempotency-Key" = String, Header, description = "企业微信审批回调幂等键")
    ),
    request_body = QualityLiaisonApprovalCallbackRequest,
    responses(
        (status = 200, description = "H4 与质量联系单审批状态已原子回写", body = QualityLiaisonOrder),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "非指定审批人", body = ErrorResponse),
        (status = 404, description = "联系单不存在", body = ErrorResponse),
        (status = 409, description = "联系单已审批或幂等冲突", body = ErrorResponse),
        (status = 422, description = "审批结论或意见非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn apply_quality_liaison_approval() {}
