#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/m-vr/dual-person-policy",
    tag = "validation-rules",
    params(ResolveDualPersonPolicyQuery),
    responses(
        (status = 200, description = "当前商品在指定流程节点的双人策略", body = DualPersonPolicyResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足或跨货主", body = ErrorResponse),
        (status = 404, description = "商品或仓库不存在", body = ErrorResponse),
        (status = 422, description = "流程与节点不匹配", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn resolve_dual_person_policy() {}

#[utoipa::path(
    get,
    path = "/api/v1/m-vr/dual-person-policy/rules",
    tag = "validation-rules",
    params(DualPersonPolicyRuleListQuery),
    responses(
        (status = 200, description = "当前货主可见的双人策略规则", body = DualPersonPolicyRuleListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "仓库不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_dual_person_policy_rules() {}

#[utoipa::path(
    put,
    path = "/api/v1/m-vr/dual-person-policy/rules",
    tag = "validation-rules",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = UpsertDualPersonPolicyRuleRequest,
    responses(
        (status = 200, description = "双人确认后保存的矩阵规则", body = DualPersonPolicyRule),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "分类或仓库不存在", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "规则非法、确认人相同或无资格", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn upsert_dual_person_policy_rule() {}
