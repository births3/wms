#[allow(unused_imports)]
use super::{
    AggregationFieldCatalogResponse, AggregationRuleTestResult, AggregationRuleVersion,
    AggregationRuleVersionListResponse, CreateAggregationRuleDraftRequest, CreateCutoffPlanRequest,
    CreatePrintSuiteDraftRequest, CutoffPlan, CutoffPlanListResponse,
    DeliveryNoteCandidateListResponse, DeliveryNoteGroup, DeliveryNoteGroupListResponse,
    ErrorResponse, ManualDeliveryNoteCutoffRequest, PrintDocumentCategoryListResponse,
    PrintSuiteInstanceListResponse, PrintSuiteTestResult, PrintSuiteVersion,
    PrintSuiteVersionListResponse, PublishRouteBindingRequest, RouteBinding,
    RouteBindingListResponse, TestAggregationRuleRequest, TestPrintSuiteRequest,
};

#[utoipa::path(
    get,
    path = "/api/v1/print-orchestration/delivery-note-candidates",
    params(("warehouse_id" = Option<uuid::Uuid>, Query, description = "可选仓库筛选")),
    responses(
        (status = 200, description = "待截单真实出库订单列表", body = DeliveryNoteCandidateListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn list_delivery_note_candidates() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-orchestration/delivery-note-groups",
    params(("warehouse_id" = Option<uuid::Uuid>, Query, description = "可选仓库筛选")),
    responses(
        (status = 200, description = "随货同行单归集结果列表", body = DeliveryNoteGroupListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn list_delivery_note_groups() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/delivery-note-groups/manual-cutoff",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = ManualDeliveryNoteCutoffRequest,
    responses(
        (status = 200, description = "人工截单成功或幂等重放", body = DeliveryNoteGroup),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 404, description = "订单不存在或尚未冻结线路", body = ErrorResponse),
        (status = 409, description = "订单已截单或编号规则未配置", body = ErrorResponse),
        (status = 422, description = "参数或归集硬边界非法", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn manual_delivery_note_cutoff() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/route-bindings",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = PublishRouteBindingRequest,
    responses(
        (status = 200, description = "线路绑定发布成功或幂等重放", body = RouteBinding),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 409, description = "同一地址的有效期重叠", body = ErrorResponse),
        (status = 422, description = "线路绑定参数或主数据非法", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn publish_route_binding() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-orchestration/route-bindings",
    params(("warehouse_id" = Option<uuid::Uuid>, Query, description = "可选仓库筛选")),
    responses(
        (status = 200, description = "线路绑定列表", body = RouteBindingListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn list_route_bindings() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-orchestration/cutoff-plans",
    params(("warehouse_id" = Option<uuid::Uuid>, Query, description = "可选仓库筛选")),
    responses(
        (status = 200, description = "截单计划列表", body = CutoffPlanListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn list_cutoff_plans() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/cutoff-plans",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateCutoffPlanRequest,
    responses(
        (status = 200, description = "截单计划草稿创建成功或幂等重放", body = CutoffPlan),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 422, description = "截单计划参数或主数据非法", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn create_cutoff_plan() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-orchestration/aggregation-fields",
    responses(
        (status = 200, description = "已登记的归集维度字段目录（仅等值归组）", body = AggregationFieldCatalogResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn list_aggregation_fields() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-orchestration/aggregation-rules/versions",
    responses(
        (status = 200, description = "归集规则版本列表（按版本号倒序）", body = AggregationRuleVersionListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn list_aggregation_rules() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/aggregation-rules/versions",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateAggregationRuleDraftRequest,
    responses(
        (status = 200, description = "规则草稿创建成功或幂等重放", body = AggregationRuleVersion),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 422, description = "维度未登记或参数非法", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn create_aggregation_rule_draft() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/aggregation-rules/versions/{version_id}/test",
    params(
        ("version_id" = uuid::Uuid, Path, description = "规则版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = TestAggregationRuleRequest,
    responses(
        (status = 200, description = "样本订单测试成功或幂等重放，返回分组键与预计归集结果", body = AggregationRuleTestResult),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 404, description = "规则版本或样本订单不存在", body = ErrorResponse),
        (status = 409, description = "规则状态不允许测试", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn test_aggregation_rule() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/aggregation-rules/versions/{version_id}/publish",
    params(
        ("version_id" = uuid::Uuid, Path, description = "规则版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "规则发布成功或幂等重放；同货主旧发布版本自动停用", body = AggregationRuleVersion),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 404, description = "规则版本不存在", body = ErrorResponse),
        (status = 409, description = "仅已测试版本可发布", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn publish_aggregation_rule() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/aggregation-rules/versions/{version_id}/disable",
    params(
        ("version_id" = uuid::Uuid, Path, description = "规则版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "规则停用成功或幂等重放", body = AggregationRuleVersion),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 404, description = "规则版本不存在", body = ErrorResponse),
        (status = 409, description = "仅已发布版本可停用", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn disable_aggregation_rule() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/cutoff-plans/{plan_id}/publish",
    params(
        ("plan_id" = uuid::Uuid, Path, description = "截单计划 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "截单计划发布成功或幂等重放", body = CutoffPlan),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 404, description = "截单计划不存在", body = ErrorResponse),
        (status = 409, description = "状态非法或同级有效期重叠", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn publish_cutoff_plan() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-orchestration/print-document-categories",
    responses(
        (status = 200, description = "M1 系统字典 print_document_category 受控分类（含 source_mode）", body = PrintDocumentCategoryListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn list_print_document_categories() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-orchestration/print-suites/versions",
    responses(
        (status = 200, description = "打印组套版本列表（含有序打印项，按版本号倒序）", body = PrintSuiteVersionListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn list_print_suites() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/print-suites/versions",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreatePrintSuiteDraftRequest,
    responses(
        (status = 200, description = "组套草稿创建成功或幂等重放", body = PrintSuiteVersion),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 422, description = "分类未登记、绑定非法或参数非法", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn create_print_suite_draft() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/print-suites/versions/{version_id}/test",
    params(
        ("version_id" = uuid::Uuid, Path, description = "组套版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = TestPrintSuiteRequest,
    responses(
        (status = 200, description = "样本归集组测试成功或幂等重放，返回解析层级与逐项就绪预检", body = PrintSuiteTestResult),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 404, description = "组套版本或样本归集组不存在", body = ErrorResponse),
        (status = 409, description = "组套状态不允许测试", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn test_print_suite() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/print-suites/versions/{version_id}/publish",
    params(
        ("version_id" = uuid::Uuid, Path, description = "组套版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "组套发布成功或幂等重放", body = PrintSuiteVersion),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 404, description = "组套版本不存在", body = ErrorResponse),
        (status = 409, description = "仅已测试版本可发布，或同级同对象有效期重叠", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn publish_print_suite() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-orchestration/print-suites/versions/{version_id}/disable",
    params(
        ("version_id" = uuid::Uuid, Path, description = "组套版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "组套停用成功或幂等重放", body = PrintSuiteVersion),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排维护权限", body = ErrorResponse),
        (status = 404, description = "组套版本不存在", body = ErrorResponse),
        (status = 409, description = "仅已发布版本可停用", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn disable_print_suite() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-orchestration/suite-instances",
    params(("group_id" = Option<uuid::Uuid>, Query, description = "可选随货同行单归集组筛选")),
    responses(
        (status = 200, description = "冻结组套实例列表（含组套版本、规则版本、源单据快照与逐项策略）", body = PrintSuiteInstanceListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印编排读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-orchestration"
)]
#[allow(dead_code)]
pub(crate) fn list_print_suite_instances() {}
