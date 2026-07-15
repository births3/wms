#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/alert-definitions",
    tag = "alert-engine",
    params(AlertDefinitionListQuery),
    responses(
        (status = 200, description = "货主范围内告警定义列表", body = AlertDefinitionListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 422, description = "查询条件非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_alert_definitions() {}

#[utoipa::path(
    get,
    path = "/api/v1/alert-definitions/{id}",
    tag = "alert-engine",
    params(("id" = Uuid, Path, description = "告警定义 ID")),
    responses(
        (status = 200, description = "告警定义详情", body = AlertDefinition),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "告警定义不存在或不属于当前货主", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_alert_definition() {}

#[utoipa::path(
    post,
    path = "/api/v1/alert-definitions/change-requests",
    tag = "alert-engine",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = SubmitAlertDefinitionChangeRequest,
    responses(
        (status = 200, description = "告警定义变更已提交 M-QL 审批", body = QualityLiaisonOrder),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "目标告警定义不存在", body = ErrorResponse),
        (status = 409, description = "版本或幂等冲突", body = ErrorResponse),
        (status = 422, description = "字段非法或未配置 M-QL 审批类型", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn submit_alert_definition_change() {}
