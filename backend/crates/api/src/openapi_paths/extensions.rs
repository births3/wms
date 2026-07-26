#[allow(unused_imports)]
use super::*;

#[utoipa::path(get, path = "/api/v1/state-machines", tag = "state-machine", responses((status = 200, description = "H6 状态机定义列表", body = StateMachineDefinitionListResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_state_machines() {}

#[utoipa::path(get, path = "/api/v1/state-machines/{machine_code}", tag = "state-machine", params(("machine_code" = String, Path, description = "状态机编码")), responses((status = 200, description = "H6 状态机定义", body = StateMachineDefinition), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 404, description = "状态机不存在", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn get_state_machine() {}

#[utoipa::path(get, path = "/api/v1/state-machines/{machine_code}/transition-validation", tag = "state-machine", params(("machine_code" = String, Path, description = "状态机编码"), ("from_state" = String, Query, description = "当前状态"), ("to_state" = String, Query, description = "目标状态"), ("event_code" = Option<String>, Query, description = "可选触发事件")), responses((status = 200, description = "H6 状态迁移校验结果", body = StateTransitionValidationResponse), (status = 400, description = "查询参数缺失或格式错误", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 404, description = "状态机不存在", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn validate_state_transition() {}

#[utoipa::path(get, path = "/api/v1/wechat-notify/configs", tag = "wechat-notify", params(("event_type" = Option<String>, Query, description = "按通知事件过滤")), responses((status = 200, description = "H4 通知配置列表", body = H4NotificationConfigListResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_h4_notification_configs() {}

#[utoipa::path(post, path = "/api/v1/wechat-notify/configs", tag = "wechat-notify", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = UpsertH4NotificationConfigRequest, responses((status = 200, description = "创建或更新 H4 通知配置", body = H4NotificationConfig), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 409, description = "幂等冲突", body = ErrorResponse), (status = 422, description = "配置非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn upsert_h4_notification_config() {}

#[utoipa::path(get, path = "/api/v1/wechat-notify/settings", tag = "wechat-notify", responses((status = 200, description = "H4 企业微信通道参数", body = H4WechatSettingsResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn get_h4_wechat_settings() {}

#[utoipa::path(post, path = "/api/v1/wechat-notify/settings", tag = "wechat-notify", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = UpsertH4WechatSettingsRequest, responses((status = 200, description = "创建或更新 H4 企业微信通道参数", body = H4WechatSettings), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 409, description = "幂等冲突", body = ErrorResponse), (status = 422, description = "参数非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn upsert_h4_wechat_settings() {}

#[utoipa::path(post, path = "/api/v1/wechat-notify/settings/test", tag = "wechat-notify", responses((status = 200, description = "测试 H4 企业微信通道参数", body = H4WechatSettingsTestResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 404, description = "企业微信参数未配置", body = ErrorResponse), (status = 422, description = "参数非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn test_h4_wechat_settings() {}

#[utoipa::path(post, path = "/api/v1/wechat-notify/send", tag = "wechat-notify", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = SendH4NotificationRequest, responses((status = 200, description = "发送企业微信通知", body = [H4NotificationRecord]), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 404, description = "事件未配置", body = ErrorResponse), (status = 409, description = "幂等冲突", body = ErrorResponse), (status = 422, description = "模板或接收人非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn send_h4_notification() {}

#[utoipa::path(post, path = "/api/v1/wechat-notify/approvals", tag = "wechat-notify", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateH4ApprovalRequest, responses((status = 200, description = "创建企业微信审批记录", body = H4ApprovalRecord), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 409, description = "幂等冲突", body = ErrorResponse), (status = 422, description = "审批请求非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_h4_approval() {}

#[utoipa::path(post, path = "/api/v1/wechat-notify/approvals/{approval_id}/callback", tag = "wechat-notify", params(("approval_id" = uuid::Uuid, Path, description = "审批记录 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = H4ApprovalCallbackRequest, responses((status = 200, description = "回写企业微信审批结果", body = H4ApprovalRecord), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 404, description = "审批记录不存在", body = ErrorResponse), (status = 409, description = "幂等冲突", body = ErrorResponse), (status = 422, description = "审批结论非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn callback_h4_approval() {}

#[utoipa::path(get, path = "/api/v1/wechat-notify/records", tag = "wechat-notify", params(("event_type" = Option<String>, Query, description = "按事件过滤"), ("recipient" = Option<String>, Query, description = "按接收人模糊过滤"), ("status" = Option<String>, Query, description = "按发送状态过滤"), ("from" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "创建时间起点"), ("to" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "创建时间终点"), ("limit" = Option<u32>, Query, description = "返回条数，默认 50，最大 100")), responses((status = 200, description = "H4 通知发送记录列表", body = H4NotificationRecordListResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_h4_notification_records() {}

#[utoipa::path(post, path = "/api/v1/wechat-notify/records/{record_id}/resend", tag = "wechat-notify", params(("record_id" = uuid::Uuid, Path, description = "通知记录 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), responses((status = 200, description = "重发 H4 通知记录", body = H4NotificationRecord), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 404, description = "通知记录不存在", body = ErrorResponse), (status = 409, description = "幂等冲突", body = ErrorResponse), (status = 422, description = "通知记录状态不可重发", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn resend_h4_notification_record() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-templates/field-libraries",
    tag = "print-template",
    responses(
        (status = 200, description = "打印字段库列表", body = PrintFieldLibraryListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_print_field_libraries() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-templates/field-libraries/drafts",
    tag = "print-template",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = GeneratePrintFieldLibraryDraftRequest,
    responses(
        (status = 200, description = "从当前 OpenAPI schema 生成字段库草稿", body = PrintFieldLibraryVersion),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "schema 不存在或请求非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn generate_print_field_library_draft() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-templates/field-libraries/{version_id}/fields",
    tag = "print-template",
    params(("version_id" = uuid::Uuid, Path, description = "字段库版本 ID")),
    responses(
        (status = 200, description = "打印字段定义列表", body = PrintFieldDefinitionListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_print_field_definitions() {}

#[utoipa::path(
    patch,
    path = "/api/v1/print-templates/field-libraries/{version_id}/fields/{field_id}",
    tag = "print-template",
    params(
        ("version_id" = uuid::Uuid, Path, description = "字段库版本 ID"),
        ("field_id" = uuid::Uuid, Path, description = "字段定义 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = UpdatePrintFieldDefinitionRequest,
    responses(
        (status = 200, description = "更新草稿字段元数据", body = PrintFieldDefinition),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "字段库版本或字段不存在", body = ErrorResponse),
        (status = 409, description = "已发布版本不可修改或幂等冲突", body = ErrorResponse),
        (status = 422, description = "字段元数据非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn update_print_field_definition() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-templates/field-libraries/{version_id}/publish",
    tag = "print-template",
    params(
        ("version_id" = uuid::Uuid, Path, description = "字段库版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "发布字段库草稿", body = PrintFieldLibraryVersion),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "字段库版本不存在", body = ErrorResponse),
        (status = 409, description = "已发布版本不可重复改写或幂等冲突", body = ErrorResponse),
        (status = 422, description = "字段路径已不在当前 OpenAPI schema", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn publish_print_field_library() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-templates/templates",
    tag = "print-template",
    responses(
        (status = 200, description = "打印模板列表", body = PrintTemplateListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_print_templates() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-templates/templates/{template_id}/versions",
    tag = "print-template",
    params(("template_id" = uuid::Uuid, Path, description = "打印模板 ID")),
    responses(
        (status = 200, description = "打印模板版本历史", body = PrintTemplateVersionListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_print_template_versions() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-templates/templates",
    tag = "print-template",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = SavePrintTemplateRequest,
    responses(
        (status = 200, description = "保存打印模板版本", body = PrintTemplateVersion),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "字段库未发布或幂等冲突", body = ErrorResponse),
        (status = 422, description = "模板 JSON 或字段绑定非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn save_print_template() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-templates/resolve",
    tag = "print-template",
    request_body = ResolvePrintTemplateRequest,
    responses(
        (status = 200, description = "解析业务打印模板", body = ResolvePrintTemplateResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "模板不存在", body = ErrorResponse),
        (status = 409, description = "模板已停用", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn resolve_print_template() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-templates/preview",
    tag = "print-template",
    request_body = PrintTemplatePreviewRequest,
    responses(
        (status = 200, description = "预览打印模板", body = PrintTemplatePreviewResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "模板不存在", body = ErrorResponse),
        (status = 422, description = "打印数据缺少必填字段", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn preview_print_template() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-templates/print",
    tag = "print-template",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = PrintTemplatePrintRequest,
    responses(
        (status = 200, description = "记录浏览器打印结果", body = PrintRecord),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "模板不存在", body = ErrorResponse),
        (status = 422, description = "打印数据缺少必填字段", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn record_print_template() {}

#[utoipa::path(get, path = "/api/v1/inbound/receiving-orders", tag = "inbound", responses((status = 200, description = "收货单列表", body = ReceivingOrderListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_receiving_orders() {}

#[utoipa::path(get, path = "/api/v1/inbound/receiving-dashboard", tag = "inbound", params(("supplier_id" = Option<uuid::Uuid>, Query, description = "供应商"), ("product_code" = Option<String>, Query, description = "商品编码"), ("from" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "预计到货时间起点"), ("to" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "预计到货时间终点")), responses((status = 200, description = "入库进度看板", body = ReceivingDashboardResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_receiving_dashboard() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders", tag = "inbound", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateReceivingOrderRequest, responses((status = 200, description = "创建收货单", body = ReceivingOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 409, description = "幂等键冲突", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_receiving_order() {}

#[utoipa::path(get, path = "/api/v1/inbound/receiving-orders/{id}", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID")), responses((status = 200, description = "收货单详情", body = ReceivingOrder), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn get_receiving_order() {}

#[utoipa::path(get, path = "/api/v1/inbound/receiving-orders/{id}/print-data", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID")), responses((status = 200, description = "收货单打印业务数据", body = ReceivingOrderPrintData), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "收货单不存在", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn get_receiving_order_print_data() {}

#[utoipa::path(patch, path = "/api/v1/inbound/receiving-orders/{id}", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = UpdateReceivingOrderRequest, responses((status = 200, description = "更新收货单", body = ReceivingOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 409, description = "幂等键冲突", body = ErrorResponse), (status = 422, description = "状态或字段校验失败", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn update_receiving_order() {}

#[utoipa::path(delete, path = "/api/v1/inbound/receiving-orders/{id}", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID")), responses((status = 200, description = "删除收货单", body = ReceivingOrder), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn delete_receiving_order() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/release", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), responses((status = 200, description = "草稿 ASN 校验通过并放行收货", body = ReceivingOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 422, description = "状态或供应商校验失败", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn release_receiving_order() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/cancel", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CancelReceivingOrderRequest, responses((status = 200, description = "ASN 审批作废", body = ReceivingOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 422, description = "状态或审批校验失败", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn cancel_receiving_order() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/force-close-shortage", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ForceCloseShortageRequest, responses((status = 200, description = "短少强制关闭", body = ReceivingOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 422, description = "状态或短少数校验失败", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn force_close_shortage_receiving_order() {}

#[utoipa::path(get, path = "/api/v1/inbound/putaway-strategy-profiles", tag = "inbound", responses((status = 200, description = "上架策略方案列表", body = PutawayStrategyProfileListResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "无上架权限", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_putaway_strategy_profiles() {}

#[utoipa::path(put, path = "/api/v1/inbound/putaway-strategy-profiles", tag = "inbound", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = UpsertPutawayStrategyProfileRequest, responses((status = 200, description = "创建或更新上架策略方案", body = PutawayStrategyProfile), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "无上架权限", body = ErrorResponse), (status = 422, description = "方案字段校验失败", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn upsert_putaway_strategy_profile() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/receive", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ReceiveReceivingOrderRequest, responses((status = 200, description = "PDA 收货闭环记录", body = ReceivingOrderReceipt), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn receive_receiving_order() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/reject", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = RejectReceivingOrderRequest, responses((status = 200, description = "整单拒收闭环记录", body = ReceivingOrderReceipt), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn reject_receiving_order() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/inspect", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = InspectReceivingOrderRequest, responses((status = 200, description = "PDA 验收记录", body = ReceivingInspectionRecord), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn inspect_receiving_order() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/sign", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = SignInspectionRequest, responses((status = 200, description = "双人验收签字记录", body = InspectionSignatureRecord), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn sign_receiving_order_inspection() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/putaway", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = PutawayRequest, responses((status = 200, description = "PDA 上架记录", body = PutawayRecord), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn putaway_receiving_order() {}

#[utoipa::path(get, path = "/api/v1/inbound/receiving-orders/{id}/putaway-recommendations", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("product_code" = String, Query, description = "商品编码"), ("batch_no" = String, Query, description = "批号"), ("qty" = i64, Query, description = "待上架数量"), ("quality_status" = String, Query, description = "质量状态"), ("limit" = Option<u32>, Query, description = "推荐条数，默认 5，最大 50")), responses((status = 200, description = "智能上架库位推荐", body = PutawayRecommendationResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "无上架确认权限", body = ErrorResponse), (status = 404, description = "收货单或批次不存在", body = ErrorResponse), (status = 422, description = "库位、数量或商品体积校验失败", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn recommend_putaway_locations() {}

#[utoipa::path(get, path = "/api/v1/inventory/batches", tag = "inventory", params(("q" = Option<String>, Query, description = "商品名称/编码、批号、库位或容器模糊匹配"), ("product_code" = Option<String>, Query, description = "商品编码模糊匹配"), ("batch_no" = Option<String>, Query, description = "批号模糊匹配"), ("location_code" = Option<String>, Query, description = "库位编码模糊匹配"), ("location_type" = Option<String>, Query, description = "库位类型精确匹配"), ("zone_code" = Option<String>, Query, description = "库区编码精确匹配"), ("temperature_zone" = Option<String>, Query, description = "温区精确匹配"), ("quality_status" = Option<String>, Query, description = "质量状态精确匹配"), ("production_from" = Option<String>, Query, description = "生产日期起始日，格式 YYYY-MM-DD"), ("production_to" = Option<String>, Query, description = "生产日期截止日，格式 YYYY-MM-DD"), ("expiry_from" = Option<String>, Query, description = "有效期起始日，格式 YYYY-MM-DD"), ("expiry_to" = Option<String>, Query, description = "有效期截止日，格式 YYYY-MM-DD"), ("created_from" = Option<String>, Query, description = "创建时间起点，RFC3339"), ("created_to" = Option<String>, Query, description = "创建时间终点，RFC3339")), responses((status = 200, description = "库存批次列表", body = InventoryBatchListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_inventory_batches() {}

#[utoipa::path(get, path = "/api/v1/inventory/batches/near-expiry-report", tag = "inventory", params(("as_of" = Option<String>, Query, description = "报告基准日，格式 YYYY-MM-DD"), ("warning_days" = Option<i64>, Query, description = "预警阈值天数，缺省读取货主覆盖或全局 inventory_policy")), responses((status = 200, description = "库存批次近效期预警报表", body = InventoryBatchListResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "日期或阈值非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn near_expiry_report() {}

#[utoipa::path(get, path = "/api/v1/inventory/batches/{id}/trace", tag = "inventory", params(("id" = uuid::Uuid, Path, description = "库存批次 ID")), responses((status = 200, description = "库存批次流转追溯", body = InventoryBatchTrace), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "库存批次不存在", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn get_inventory_batch_trace() {}

#[utoipa::path(get, path = "/api/v1/inventory/locations/history", tag = "inventory", params(("location_code" = Option<String>, Query, description = "库位编码，必填"), ("from" = Option<String>, Query, description = "开始时间，RFC3339 或 YYYY-MM-DD"), ("to" = Option<String>, Query, description = "结束时间，RFC3339 或 YYYY-MM-DD"), ("movement_type" = Option<String>, Query, description = "操作类型精确匹配"), ("product_code" = Option<String>, Query, description = "商品编码模糊匹配"), ("batch_no" = Option<String>, Query, description = "批号模糊匹配"), ("days" = Option<i64>, Query, description = "默认回溯天数，缺省 30，最大 3650")), responses((status = 200, description = "库位历史追踪", body = LocationHistoryResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "库位不存在", body = ErrorResponse), (status = 422, description = "查询参数非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_location_history() {}

#[utoipa::path(post, path = "/api/v1/inventory/relocations", tag = "inventory", params(("Idempotency-Key" = String, Header, description = "幂等键")), request_body = RelocateInventoryRequest, responses((status = 200, description = "库内移库完成", body = InventoryRelocation), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "移库校验失败", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn relocate_inventory() {}

#[utoipa::path(get, path = "/api/v1/inventory/alerts", tag = "inventory", params(("alert_type" = Option<String>, Query, description = "预警类型"), ("lifecycle_status" = Option<String>, Query, description = "生命周期状态"), ("product_code" = Option<String>, Query, description = "商品编码")), responses((status = 200, description = "库存预警事件列表", body = InventoryAlertListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_inventory_alerts() {}

#[utoipa::path(post, path = "/api/v1/inventory/alerts/{id}/handle", tag = "inventory", params(("id" = uuid::Uuid, Path, description = "预警 ID")), request_body = HandleInventoryAlertRequest, responses((status = 200, description = "预警已处理", body = InventoryAlertEvent), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "预警不存在", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn handle_inventory_alert() {}

#[utoipa::path(post, path = "/api/v1/inventory/alerts/generate-near-expiry", tag = "inventory", responses((status = 200, description = "近效期预警已生成"), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn generate_near_expiry_alerts() {}

#[utoipa::path(get, path = "/api/v1/inventory/abc", tag = "inventory", params(("abc_class" = Option<String>, Query, description = "ABC 分类"), ("product_code" = Option<String>, Query, description = "商品编码")), responses((status = 200, description = "ABC 分类列表", body = InventoryAbcListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_inventory_abc() {}

#[utoipa::path(post, path = "/api/v1/inventory/abc", tag = "inventory", request_body = RecomputeInventoryAbcRequest, responses((status = 200, description = "ABC 重算结果", body = InventoryAbcListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn recompute_inventory_abc() {}

#[utoipa::path(post, path = "/api/v1/inventory/abc/override", tag = "inventory", request_body = OverrideInventoryAbcRequest, responses((status = 200, description = "ABC 人工覆盖", body = InventoryAbcClassification), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "参数非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn override_inventory_abc() {}

#[utoipa::path(get, path = "/api/v1/inventory/batches/{id}/shipped-customers", tag = "inventory", params(("id" = uuid::Uuid, Path, description = "库存批次 ID")), responses((status = 200, description = "召回已发货客户提示", body = InventoryRecallImpact), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "批次不存在", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_shipped_customers_for_batch() {}

#[utoipa::path(post, path = "/api/v1/inventory/status-erp-outbox/process", tag = "inventory", responses((status = 200, description = "ERP 反馈 outbox 处理结果"), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn process_status_erp_outbox() {}

#[utoipa::path(get, path = "/api/v1/inventory/counts", tag = "inventory", responses((status = 200, description = "盘点单列表"), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_inventory_counts() {}

#[utoipa::path(post, path = "/api/v1/inventory/maintenance/tasks/generate", tag = "inventory", responses((status = 200, description = "生成养护计划"), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn generate_maintenance_tasks() {}

#[utoipa::path(get, path = "/api/v1/inventory/relocations", tag = "inventory", responses((status = 200, description = "移库记录列表"), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_inventory_relocations() {}

#[utoipa::path(post, path = "/api/v1/inventory/batches/putaway", tag = "inventory", request_body = PutawayInventoryRequest, responses((status = 200, description = "入库上架增加库存", body = InventoryBatch), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn putaway_inventory_batch() {}

#[utoipa::path(post, path = "/api/v1/inventory/batches/status", tag = "inventory", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ChangeInventoryStatusRequest, responses((status = 200, description = "库存状态变更", body = InventoryBatch), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn change_inventory_batch_status() {}

#[utoipa::path(get, path = "/api/v1/inventory/status-transitions", tag = "inventory", responses((status = 200, description = "库存状态转换规则", body = InventoryStatusTransitionListResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_inventory_status_transitions() {}

#[utoipa::path(put, path = "/api/v1/inventory/status-transitions/{from_status}/{to_status}", tag = "inventory", params(("from_status" = String, Path, description = "起始库存状态"), ("to_status" = String, Path, description = "目标库存状态"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = UpsertInventoryStatusTransitionRequest, responses((status = 200, description = "库存状态转换规则已保存", body = InventoryStatusTransition), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 409, description = "幂等冲突", body = ErrorResponse), (status = 422, description = "规则非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn upsert_inventory_status_transition() {}

#[utoipa::path(post, path = "/api/v1/inventory/batches/recall", tag = "inventory", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = MarkInventoryRecallRequest, responses((status = 200, description = "标记库存批次召回并隔离", body = InventoryBatch), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "审批源或状态非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn mark_inventory_batch_recall() {}

#[utoipa::path(post, path = "/api/v1/inventory/batches/recall/cancel", tag = "inventory", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CancelInventoryRecallRequest, responses((status = 200, description = "双人审批取消库存批次召回", body = InventoryBatch), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 409, description = "召回状态已被其他流程改变", body = ErrorResponse), (status = 422, description = "双人审批或召回状态非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn cancel_inventory_batch_recall() {}

#[utoipa::path(post, path = "/api/v1/inventory/batches/expire", tag = "inventory", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ExpireInventoryBatchesRequest, responses((status = 200, description = "按日期隔离过期库存批次", body = InventoryBatchListResponse), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "日期格式非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn isolate_expired_inventory_batches() {}

#[utoipa::path(post, path = "/api/v1/outbound/orders", tag = "outbound", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateOutboundOrderRequest, responses((status = 200, description = "创建出库订单", body = OutboundOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 409, description = "单号或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_outbound_order() {}

#[utoipa::path(
    get,
    path = "/api/v1/outbound/orders",
    tag = "outbound",
    params(
        ("status" = Option<String>, Query, description = "按出库订单状态过滤"),
        ("q" = Option<String>, Query, description = "按 WMS/ERP 单号模糊查询"),
        ("limit" = Option<u32>, Query, description = "返回条数，默认 50，最大 200"),
    ),
    responses(
        (status = 200, description = "出库订单列表", body = OutboundOrderListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_outbound_orders() {}

#[utoipa::path(
    get,
    path = "/api/v1/outbound/orders/{id}",
    tag = "outbound",
    params(("id" = uuid::Uuid, Path, description = "出库订单 ID")),
    responses(
        (status = 200, description = "出库订单详情", body = OutboundOrder),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "出库订单不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_outbound_order() {}

#[utoipa::path(post, path = "/api/v1/outbound/waves", tag = "outbound", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateOutboundWaveRequest, responses((status = 200, description = "创建并下发出库波次", body = OutboundWave), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "订单状态不可入波次", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_outbound_wave() {}

#[utoipa::path(
    get,
    path = "/api/v1/outbound/waves",
    tag = "outbound",
    params(
        ("status" = Option<String>, Query, description = "按波次状态过滤"),
        ("q" = Option<String>, Query, description = "按波次号模糊查询"),
        ("limit" = Option<u32>, Query, description = "返回条数，默认 50，最大 200"),
    ),
    responses(
        (status = 200, description = "出库波次列表", body = OutboundWaveListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_outbound_waves() {}

#[utoipa::path(
    get,
    path = "/api/v1/outbound/waves/{wave_id}",
    tag = "outbound",
    params(("wave_id" = uuid::Uuid, Path, description = "出库波次 ID")),
    responses(
        (status = 200, description = "出库波次详情", body = OutboundWave),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "出库波次不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_outbound_wave() {}

#[utoipa::path(
    post,
    path = "/api/v1/outbound/waves/{wave_id}/cancel",
    tag = "outbound",
    params(
        ("wave_id" = uuid::Uuid, Path, description = "出库波次 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "取消未开始拣选的出库波次并释放库存锁定", body = OutboundWave),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无出库写权限", body = ErrorResponse),
        (status = 404, description = "出库波次不存在", body = ErrorResponse),
        (status = 422, description = "波次状态不允许取消", body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn cancel_outbound_wave() {}

#[utoipa::path(post, path = "/api/v1/outbound/pick-tasks/{id}/complete", tag = "outbound", params(("id" = uuid::Uuid, Path, description = "出库订单 ID；当前最小闭环按订单行完成拣选"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CompletePickTaskRequest, responses((status = 200, description = "完成拣选任务，短拣时订单进入待补齐状态", body = OutboundOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "数量或状态非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn complete_outbound_pick_task() {}

#[utoipa::path(
    get,
    path = "/api/v1/outbound/orders/{id}/review",
    tag = "outbound",
    params(("id" = uuid::Uuid, Path, description = "出库订单 ID")),
    responses(
        (status = 200, description = "查询出库复核明细", body = OutboundOrder),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "出库订单不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_outbound_review() {}

#[utoipa::path(post, path = "/api/v1/outbound/orders/{id}/review", tag = "outbound", params(("id" = uuid::Uuid, Path, description = "出库订单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ReviewOutboundOrderRequest, responses((status = 200, description = "完成出库复核", body = OutboundOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "订单状态或复核明细不一致", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn review_outbound_order() {}

#[utoipa::path(post, path = "/api/v1/outbound/orders/{id}/ship", tag = "outbound", params(("id" = uuid::Uuid, Path, description = "出库订单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ShipOutboundOrderRequest, responses((status = 200, description = "发货交接并扣减库存", body = OutboundOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "短拣未补齐或库存不足", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn ship_outbound_order() {}

#[utoipa::path(post, path = "/api/v1/reports/query", tag = "reports", request_body = ReportQueryRequest, responses((status = 200, description = "报表查询结果", body = ReportQueryResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn query_report() {}

#[utoipa::path(post, path = "/api/v1/reports/gsp/inbound-ledger", tag = "reports", request_body = ReportQueryRequest, responses((status = 200, description = "GSP 入库验收台账", body = GspLedgerReport), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn query_gsp_inbound_ledger() {}

#[utoipa::path(post, path = "/api/v1/reports/gsp/outbound-ledger", tag = "reports", request_body = ReportQueryRequest, responses((status = 200, description = "GSP 出库复核台账", body = GspLedgerReport), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn query_gsp_outbound_ledger() {}

#[utoipa::path(post, path = "/api/v1/reports/gsp/inventory-ledger", tag = "reports", request_body = ReportQueryRequest, responses((status = 200, description = "GSP 库存流水台账", body = GspLedgerReport), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn query_gsp_inventory_ledger() {}

#[utoipa::path(post, path = "/api/v1/traceability/outbound-reports", tag = "traceability", request_body = TraceabilityOutboundReportRequest, responses((status = 200, description = "追溯码出库核销待上报记录", body = TraceabilityOutboundReport), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "追溯码状态变更三元组不完整", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_traceability_outbound_report() {}

#[utoipa::path(get, path = "/api/v1/driver/tasks/today", tag = "driver", responses((status = 200, description = "司机今日配送任务", body = DriverTaskListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_driver_today_tasks() {}

#[utoipa::path(get, path = "/api/v1/store/dashboard", tag = "store", responses((status = 200, description = "门店首页业务概览", body = StoreDashboardResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn get_store_dashboard() {}

#[utoipa::path(post, path = "/api/v1/parameter-mapping/map", tag = "parameter-mapping", params(("Idempotency-Key" = String, Header, description = "跨重试保持不变的幂等键")), request_body = MapParameterRequest, responses((status = 200, description = "执行参数对照", body = MapParameterResponse), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "字典不存在", body = ErrorResponse), (status = 409, description = "幂等键冲突", body = ErrorResponse), (status = 422, description = "请求无效", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn map_parameter() {}

#[utoipa::path(post, path = "/api/v1/config-center/feature-flags/migrate", tag = "config-center", responses((status = 200, description = "迁移文件版 Feature Flag", body = FeatureFlagMigrationResult), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn migrate_feature_flags() {}

#[utoipa::path(get, path = "/api/v1/config-center/feature-flags/reconcile", tag = "config-center", responses((status = 200, description = "Feature Flag 对账报告", body = FeatureFlagReconcileReport), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn reconcile_feature_flags() {}

#[utoipa::path(get, path = "/api/v1/config-center/feature-flags/export", tag = "config-center", responses((status = 200, description = "导出配置中心 Feature Flag", body = FeatureFlagExportResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn export_feature_flags() {}

#[utoipa::path(post, path = "/api/v1/config-center/feature-flags/import", tag = "config-center", request_body = FeatureFlagBatchImportRequest, responses((status = 200, description = "批量导入配置中心 Feature Flag", body = FeatureFlagBatchImportResult), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn import_feature_flags() {}

#[utoipa::path(post, path = "/api/v1/config-center/feature-flags/source", tag = "config-center", request_body = FeatureFlagSourceSwitchRequest, responses((status = 200, description = "切换 Feature Flag 读取源", body = FeatureFlagSourceSwitchResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn switch_feature_flag_source() {}

#[utoipa::path(post, path = "/api/v1/config-center/feature-flags/archive-file-source", tag = "config-center", request_body = FeatureFlagArchiveRequest, responses((status = 200, description = "归档 W1 文件版 Feature Flag", body = FeatureFlagArchiveResult), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn archive_feature_flag_file_source() {}

#[utoipa::path(post, path = "/api/v1/cold-chain/devices", tag = "cold-chain", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateColdChainDeviceRequest, responses((status = 200, description = "创建冷链设备台账", body = ColdChainDevice), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 409, description = "设备编码重复或幂等冲突", body = ErrorResponse), (status = 422, description = "设备类型非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_cold_chain_device() {}

#[utoipa::path(get, path = "/api/v1/cold-chain/devices", tag = "cold-chain", responses((status = 200, description = "按货主查询冷链设备台账", body = [ColdChainDevice]), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "无冷链设备读取权限", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_cold_chain_devices() {}

#[utoipa::path(patch, path = "/api/v1/cold-chain/devices/{device_code}", tag = "cold-chain", params(("device_code" = String, Path, description = "设备编码"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = UpdateColdChainDeviceRequest, responses((status = 200, description = "更新冷链设备台账", body = ColdChainDevice), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "设备不存在", body = ErrorResponse), (status = 422, description = "设备类型非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn update_cold_chain_device() {}

#[utoipa::path(post, path = "/api/v1/cold-chain/devices/{device_code}/disable", tag = "cold-chain", params(("device_code" = String, Path, description = "设备编码"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), responses((status = 200, description = "停用冷链设备台账", body = ColdChainDevice), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "设备不存在", body = ErrorResponse), (status = 422, description = "设备仍处于监控中", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn disable_cold_chain_device() {}

#[utoipa::path(post, path = "/api/v1/cold-chain/readings", tag = "cold-chain", params(("Idempotency-Key" = String, Header, description = "外部系统生成的幂等键"), ("X-WMS-API-Key" = String, Header, description = "外部冷链系统 API Key")), request_body = IngestTemperatureReadingRequest, responses((status = 200, description = "接收外部温控数据", body = TemperatureReading), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "外部系统 API Key 缺失或无效", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn ingest_temperature_reading() {}

#[utoipa::path(post, path = "/api/v1/cold-chain/excursions", tag = "cold-chain", params(("Idempotency-Key" = String, Header, description = "外部系统生成的幂等键"), ("X-WMS-API-Key" = String, Header, description = "外部冷链系统 API Key")), request_body = IngestTemperatureExcursionRequest, responses((status = 200, description = "接收外部温度超标事件", body = TemperatureExcursionEvent), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "外部系统 API Key 缺失或无效", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn ingest_temperature_excursion() {}

#[utoipa::path(get, path = "/api/v1/cold-chain/excursions/pending-disposition", tag = "cold-chain", responses((status = 200, description = "温度超标待处置列表", body = TemperatureExcursionEventListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_pending_temperature_excursions() {}

#[utoipa::path(post, path = "/api/v1/cold-chain/excursions/{external_event_id}/dispose", tag = "cold-chain", params(("external_event_id" = String, Path, description = "外部冷链系统事件 ID")), request_body = DisposeTemperatureExcursionRequest, responses((status = 200, description = "温度超标处置并隔离批次", body = TemperatureExcursionDispositionResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "温度超标事件不存在", body = ErrorResponse), (status = 422, description = "批次不在影响范围或事件状态不可处置", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn dispose_temperature_excursion() {}

#[utoipa::path(post, path = "/api/v1/billing/accounts", tag = "billing", request_body = CreateBillingAccountRequest, responses((status = 200, description = "创建计费账户", body = BillingAccount), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_billing_account() {}

#[utoipa::path(post, path = "/api/v1/billing/contracts", tag = "billing", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateBillingContractRequest, responses((status = 200, description = "创建计费合同", body = BillingContract), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "货主下的计费账户不存在", body = ErrorResponse), (status = 409, description = "幂等冲突或合同重复", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_billing_contract() {}

#[utoipa::path(post, path = "/api/v1/billing/rules", tag = "billing", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateBillingRuleRequest, responses((status = 200, description = "创建计费规则", body = BillingRule), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "货主下的计费合同不存在", body = ErrorResponse), (status = 409, description = "幂等冲突或规则生效窗口重复", body = ErrorResponse), (status = 422, description = "规则项、单位、周期、费率或生效窗口校验失败", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_billing_rule() {}

#[utoipa::path(post, path = "/api/v1/packing/stations", tag = "packing", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreatePackingStationRequest, responses((status = 200, description = "创建包装工位", body = PackingStation), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 409, description = "工位或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_packing_station() {}

#[utoipa::path(post, path = "/api/v1/packing/jobs", tag = "packing", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreatePackJobRequest, responses((status = 200, description = "创建装箱任务", body = PackJob), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "出库订单或工位不存在", body = ErrorResponse), (status = 409, description = "装箱任务或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_pack_job() {}

#[utoipa::path(post, path = "/api/v1/packing/jobs/{id}/weigh", tag = "packing", params(("id" = uuid::Uuid, Path, description = "装箱任务 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = WeighPackJobRequest, responses((status = 200, description = "记录装箱称重", body = PackJob), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "装箱任务不存在", body = ErrorResponse), (status = 422, description = "称重数据非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn weigh_pack_job() {}

#[utoipa::path(post, path = "/api/v1/packing/jobs/{id}/waybill", tag = "packing", params(("id" = uuid::Uuid, Path, description = "装箱任务 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = PrintWaybillRequest, responses((status = 200, description = "记录面单打印结果", body = PackJob), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "装箱任务不存在", body = ErrorResponse), (status = 422, description = "面单数据非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn print_pack_job_waybill() {}

#[utoipa::path(get, path = "/api/v1/express/carriers", tag = "express", params(("q" = Option<String>, Query, description = "快递商编码或名称"), ("enabled" = Option<bool>, Query, description = "启停状态"), ("limit" = Option<u32>, Query, description = "返回条数")), responses((status = 200, description = "快递商配置列表", body = ExpressCarrierListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_express_carriers() {}

#[utoipa::path(post, path = "/api/v1/express/carriers", tag = "express", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = UpsertExpressCarrierRequest, responses((status = 200, description = "新增或更新快递商配置", body = ExpressCarrier), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "快递商配置非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn upsert_express_carrier() {}

#[utoipa::path(get, path = "/api/v1/express/routing-rules", tag = "express", params(("q" = Option<String>, Query, description = "规则编码或名称"), ("delivery_provider_type" = Option<String>, Query, description = "配送方式"), ("enabled" = Option<bool>, Query, description = "启停状态"), ("limit" = Option<u32>, Query, description = "返回条数")), responses((status = 200, description = "快递选择规则列表", body = ExpressRoutingRuleListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_express_routing_rules() {}

#[utoipa::path(post, path = "/api/v1/express/routing-rules", tag = "express", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = UpsertExpressRoutingRuleRequest, responses((status = 200, description = "新增或更新快递选择规则", body = ExpressRoutingRule), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "快递规则非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn upsert_express_routing_rule() {}

#[utoipa::path(post, path = "/api/v1/express/waybills", tag = "express", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateExpressWaybillRequest, responses((status = 200, description = "快递下单并生成运单", body = ExpressWaybill), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "快递商不存在或未启用", body = ErrorResponse), (status = 422, description = "快递下单字段非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_express_waybill() {}

#[utoipa::path(post, path = "/api/v1/express/waybills/{waybill_no}/cancel", tag = "express", params(("waybill_no" = String, Path, description = "快递运单号"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CancelExpressWaybillRequest, responses((status = 200, description = "取消快递运单", body = ExpressWaybill), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "运单不存在", body = ErrorResponse), (status = 422, description = "运单状态不可取消", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn cancel_express_waybill() {}

#[utoipa::path(get, path = "/api/v1/express/waybills/{waybill_no}/tracking", tag = "express", params(("waybill_no" = String, Path, description = "快递运单号")), responses((status = 200, description = "快递轨迹缓存", body = ExpressTrackingResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "运单不存在", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn get_express_tracking() {}

#[utoipa::path(post, path = "/api/v1/retail/replenishment-suggestions", tag = "retail", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateRetailReplenishmentSuggestionRequest, responses((status = 200, description = "生成门店补货建议", body = RetailReplenishmentSuggestion), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 409, description = "建议或幂等冲突", body = ErrorResponse), (status = 422, description = "补货水位非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_retail_replenishment_suggestion() {}

#[utoipa::path(post, path = "/api/v1/retail/crossdock-plans", tag = "retail", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateCrossdockPlanRequest, responses((status = 200, description = "创建门店越库计划", body = CrossdockPlan), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "出库订单不存在", body = ErrorResponse), (status = 422, description = "越库数量非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_retail_crossdock_plan() {}

#[utoipa::path(post, path = "/api/v1/billing/charges/calculate", tag = "billing", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CalculateBillingChargesRequest, responses((status = 200, description = "计算周期计费明细", body = BillingChargeCalculation), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "计费合同不存在", body = ErrorResponse), (status = 409, description = "计费明细或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn calculate_billing_charges() {}

#[utoipa::path(post, path = "/api/v1/billing/statements", tag = "billing", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = GenerateBillingStatementRequest, responses((status = 200, description = "生成月结账单", body = BillingStatement), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "计费合同不存在", body = ErrorResponse), (status = 409, description = "账单或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn generate_billing_statement() {}

#[utoipa::path(post, path = "/api/v1/billing/statements/{id}/confirm", tag = "billing", params(("id" = uuid::Uuid, Path, description = "月结账单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ConfirmBillingStatementRequest, responses((status = 200, description = "确认月结账单", body = BillingStatement), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "账单不存在", body = ErrorResponse), (status = 422, description = "账单状态不可确认", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn confirm_billing_statement() {}

#[utoipa::path(post, path = "/api/v1/tms/dispatches", tag = "tms", params(("Idempotency-Key" = String, Header, description = "外部 TMS 生成的幂等键")), request_body = ReceiveTmsDispatchRequest, responses((status = 200, description = "接收 TMS 调度", body = TmsDispatch), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "出库订单不存在", body = ErrorResponse), (status = 409, description = "调度或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn receive_tms_dispatch() {}

#[utoipa::path(post, path = "/api/v1/tms/route-plans", tag = "tms", params(("Idempotency-Key" = String, Header, description = "外部 TMS 生成的幂等键")), request_body = ReceiveTmsRoutePlanRequest, responses((status = 200, description = "接收 TMS 路径规划结果", body = TmsRoutePlan), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "出库订单或司机不存在", body = ErrorResponse), (status = 409, description = "路径规划结果或幂等冲突", body = ErrorResponse), (status = 422, description = "路径规划结果非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn receive_tms_route_plan() {}

#[utoipa::path(post, path = "/api/v1/tms/transit-temperature-readings", tag = "tms", params(("Idempotency-Key" = String, Header, description = "外部 TMS 生成的幂等键")), request_body = IngestTransitTemperatureRequest, responses((status = 200, description = "接收在途温控读数", body = TransitTemperatureReading), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "TMS 调度不存在", body = ErrorResponse), (status = 422, description = "温控数据非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn ingest_transit_temperature() {}

#[utoipa::path(post, path = "/api/v1/tms/container-recoveries", tag = "tms", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ConfirmContainerRecoveryRequest, responses((status = 200, description = "确认周转容器回收", body = ContainerRecovery), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "TMS 调度不存在", body = ErrorResponse), (status = 409, description = "容器回收或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn confirm_container_recovery() {}

#[utoipa::path(get, path = "/api/v1/auth/api-keys", tag = "auth", params(("q" = Option<String>, Query, description = "调用方名称或用途"), ("status" = Option<String>, Query, description = "active / revoked / temporarily_disabled")), responses((status = 200, description = "当前货主 API Key 列表", body = ApiKeyListResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "仅系统管理员可访问", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_api_keys() {}

#[utoipa::path(post, path = "/api/v1/auth/api-keys", tag = "auth", params(("Idempotency-Key" = String, Header, description = "创建幂等键；明文只在首次响应展示")), request_body = CreateApiKeyRequest, responses((status = 200, description = "创建 API Key，secret 只展示一次", body = ApiKey), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "仅系统管理员可操作", body = ErrorResponse), (status = 422, description = "作用域或过期时间非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_api_key() {}

#[utoipa::path(post, path = "/api/v1/auth/api-keys/{api_key_id}/rotate", tag = "auth", params(("api_key_id" = uuid::Uuid, Path, description = "API Key ID"), ("Idempotency-Key" = String, Header, description = "轮换幂等键；新 secret 只展示一次")), request_body = RotateApiKeyRequest, responses((status = 200, description = "轮换 API Key 并返回旧 Key 宽限期", body = ApiKeyRotationResponse), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "仅系统管理员可操作", body = ErrorResponse), (status = 404, description = "API Key 不存在", body = ErrorResponse), (status = 422, description = "宽限期或过期时间非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn rotate_api_key() {}

#[utoipa::path(post, path = "/api/v1/auth/api-keys/{api_key_id}/revoke", tag = "auth", params(("api_key_id" = uuid::Uuid, Path, description = "API Key ID"), ("Idempotency-Key" = String, Header, description = "吊销幂等键")), responses((status = 200, description = "吊销 API Key；重复吊销幂等", body = ApiKey), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "仅系统管理员可操作", body = ErrorResponse), (status = 404, description = "API Key 不存在", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn revoke_api_key() {}
