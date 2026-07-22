#[allow(unused_imports)]
use wms_domain::{
    ClaimH8ErpMessageRequest, CreateH8ErpConnectorRequest, ErrorResponse, H8DecryptedPayload,
    H8ErpConnector, H8ErpConnectorListResponse, H8ErpConnectorRuntimeConfig,
    H8ErpConnectorTestResult, H8ErpInterfaceTableConnectorOption, H8ErpInterfaceTableDetail,
    H8ErpInterfaceTableListResponse, H8ErpMessage, H8ErpMessageDetail, H8ErpMessageListResponse,
    H8ErpMessageStats, H8PayloadRetentionPolicy, H8WorkerClaimControl, H8WorkerClaimDecision,
    H8WorkerHeartbeatRequest, H8WorkerRuntimeResponse, H8WorkerStatus, PurgeH8ErpMessagesRequest,
    PurgeH8ErpMessagesResponse, ReplayH8ErpMessageRequest, SetH8WorkerClaimControlRequest,
    UpdateH8ErpConnectorRequest, UpdateH8PayloadRetentionPolicyRequest,
    UpsertH8ErpMessageLifecycleRequest,
};

#[utoipa::path(
    get,
    path = "/api/v1/h8/erp-interface-tables/connectors",
    tag = "h8-erp",
    responses(
        (status = 200, description = "当前货主可用于接口表探查的连接最小投影", body = [H8ErpInterfaceTableConnectorOption]),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "缺少接口表探查权限", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_h8_erp_interface_table_connectors() {}

#[utoipa::path(
    get,
    path = "/api/v1/h8/erp-interface-tables/rows",
    tag = "h8-erp",
    params(
        ("connector_id" = uuid::Uuid, Query, description = "当前货主的 H8 连接"),
        ("table_key" = String, Query, description = "受控接口表白名单"),
        ("sync_status" = Option<String>, Query, description = "当前表允许的一个或多个同步状态，多个值以逗号分隔"),
        ("time_from" = Option<String>, Query, description = "updated_at 起始时间"),
        ("time_to" = Option<String>, Query, description = "updated_at 结束时间"),
        ("warehouse_id" = Option<uuid::Uuid>, Query, description = "仓库精确匹配"),
        ("external_doc_no" = Option<String>, Query, description = "入站外部单号精确匹配"),
        ("source_outbox_id" = Option<String>, Query, description = "出站 outbox ID 精确匹配"),
        ("event_type" = Option<String>, Query, description = "出站事件类型精确匹配"),
        ("external_ref" = Option<String>, Query, description = "外部引用精确匹配"),
        ("wms_resource_id" = Option<String>, Query, description = "WMS 资源 ID 精确匹配"),
        ("idempotency_key" = Option<String>, Query, description = "幂等键精确匹配"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页 1..=100"),
    ),
    responses(
        (status = 200, description = "接口表只读行列表", body = H8ErpInterfaceTableListResponse),
        (status = 400, description = "表名/过滤条件非法", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或数据范围不足", body = ErrorResponse),
        (status = 409, description = "探查凭据未配置", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_h8_erp_interface_table_rows() {}

#[utoipa::path(
    get,
    path = "/api/v1/h8/erp-interface-tables/rows/{row_id}",
    tag = "h8-erp",
    params(
        ("row_id" = String, Path, description = "接口表行 ID"),
        ("connector_id" = uuid::Uuid, Query, description = "当前货主的 H8 连接"),
        ("table_key" = String, Query, description = "受控接口表白名单"),
    ),
    responses(
        (status = 200, description = "接口表只读行详情", body = H8ErpInterfaceTableDetail),
        (status = 400, description = "联合身份参数缺失", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或数据范围不足", body = ErrorResponse),
        (status = 404, description = "行不存在", body = ErrorResponse),
        (status = 409, description = "探查凭据未配置", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_h8_erp_interface_table_row() {}

#[utoipa::path(
    get,
    path = "/api/v1/config/erp-connectors",
    tag = "h8-erp",
    responses(
        (status = 200, description = "ERP 连接列表", body = H8ErpConnectorListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_h8_erp_connectors() {}

#[utoipa::path(
    post,
    path = "/api/v1/config/erp-connectors",
    tag = "h8-erp",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateH8ErpConnectorRequest,
    responses(
        (status = 201, description = "新建 ERP 连接（testing）", body = H8ErpConnector),
        (status = 400, description = "校验失败或缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "编码冲突", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_h8_erp_connector() {}

#[utoipa::path(
    get,
    path = "/api/v1/config/erp-connectors/{id}",
    tag = "h8-erp",
    params(("id" = uuid::Uuid, Path, description = "连接 ID")),
    responses(
        (status = 200, description = "连接详情", body = H8ErpConnector),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_h8_erp_connector() {}

#[utoipa::path(
    get,
    path = "/api/v1/config/erp-connectors/{id}/versions/{version}",
    tag = "h8-erp",
    params(
        ("id" = uuid::Uuid, Path, description = "连接 ID"),
        ("version" = i64, Path, description = "处理消息时绑定的配置版本"),
    ),
    responses(
        (status = 200, description = "不可变连接运行配置快照", body = H8ErpConnectorRuntimeConfig),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "版本不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_h8_erp_connector_version() {}

#[utoipa::path(
    patch,
    path = "/api/v1/config/erp-connectors/{id}",
    tag = "h8-erp",
    params(
        ("id" = uuid::Uuid, Path, description = "连接 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = UpdateH8ErpConnectorRequest,
    responses(
        (status = 200, description = "更新连接", body = H8ErpConnector),
        (status = 400, description = "校验失败", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "不存在", body = ErrorResponse),
        (status = 409, description = "版本冲突", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn update_h8_erp_connector() {}

#[utoipa::path(
    post,
    path = "/api/v1/config/erp-connectors/{id}/test",
    tag = "h8-erp",
    params(
        ("id" = uuid::Uuid, Path, description = "连接 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    responses(
        (status = 200, description = "连接测试结果（不写业务单据）", body = H8ErpConnectorTestResult),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn test_h8_erp_connector() {}

#[utoipa::path(
    post,
    path = "/api/v1/config/erp-connectors/{id}/activate",
    tag = "h8-erp",
    params(
        ("id" = uuid::Uuid, Path, description = "连接 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    responses(
        (status = 200, description = "启用连接", body = H8ErpConnector),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "不存在", body = ErrorResponse),
        (status = 409, description = "路由重叠", body = ErrorResponse),
        (status = 422, description = "需先通过当前版本测试", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn activate_h8_erp_connector() {}

#[utoipa::path(
    post,
    path = "/api/v1/config/erp-connectors/{id}/disable",
    tag = "h8-erp",
    params(
        ("id" = uuid::Uuid, Path, description = "连接 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    responses(
        (status = 200, description = "停用连接", body = H8ErpConnector),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "不存在", body = ErrorResponse),
        (status = 422, description = "状态非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn disable_h8_erp_connector() {}

#[utoipa::path(
    delete,
    path = "/api/v1/config/erp-connectors/{id}",
    tag = "h8-erp",
    params(
        ("id" = uuid::Uuid, Path, description = "连接 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    responses(
        (status = 204, description = "物理删除（仅从未启用且无引用）"),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "不存在", body = ErrorResponse),
        (status = 422, description = "不允许删除", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn delete_h8_erp_connector() {}

#[utoipa::path(
    get,
    path = "/api/v1/integration/erp-messages",
    tag = "h8-erp",
    params(
        ("direction" = Option<String>, Query, description = "inbound/outbound"),
        ("message_type" = Option<String>, Query, description = "受控消息类型"),
        ("status" = Option<String>, Query, description = "sync_status"),
        ("connector_code" = Option<String>, Query, description = "连接编码精确筛选"),
        ("channel" = Option<String>, Query, description = "通道精确筛选"),
        ("warehouse_id" = Option<uuid::Uuid>, Query, description = "仓库精确筛选"),
        ("external_ref" = Option<String>, Query, description = "外部业务标识精确筛选"),
        ("idempotency_key" = Option<String>, Query, description = "幂等键精确筛选"),
        ("correlation_id" = Option<String>, Query, description = "关联标识精确筛选"),
        ("created_from" = Option<String>, Query, description = "开始时间 ISO8601"),
        ("created_to" = Option<String>, Query, description = "结束时间 ISO8601"),
        ("cursor" = Option<String>, Query, description = "上一页返回的稳定游标"),
        ("limit" = Option<u32>, Query, description = "每页 1..=200，默认 50"),
    ),
    responses(
        (status = 200, description = "ERP 消息列表", body = H8ErpMessageListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_h8_erp_messages() {}

#[utoipa::path(
    get,
    path = "/api/v1/integration/erp-messages/stats",
    tag = "h8-erp",
    params(
        ("connector_code" = Option<String>, Query, description = "连接编码精确筛选"),
        ("channel" = Option<String>, Query, description = "通道精确筛选"),
        ("message_type" = Option<String>, Query, description = "消息类型精确筛选"),
    ),
    responses(
        (status = 200, description = "消息统计", body = H8ErpMessageStats),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn stats_h8_erp_messages() {}

#[utoipa::path(
    get,
    path = "/api/v1/integration/erp-messages/payload-retention",
    tag = "h8-erp",
    responses(
        (status = 200, description = "当前货主的完整报文保留策略", body = [H8PayloadRetentionPolicy]),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_h8_payload_retention_policies() {}

#[utoipa::path(
    post,
    path = "/api/v1/integration/erp-messages/payload-retention",
    tag = "h8-erp",
    request_body = UpdateH8PayloadRetentionPolicyRequest,
    responses(
        (status = 200, description = "更新连接的完整报文保留策略", body = H8PayloadRetentionPolicy),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 503, description = "加密主密钥不可用", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn update_h8_payload_retention_policy() {}

#[utoipa::path(
    get,
    path = "/api/v1/integration/erp-messages/{id}/payload",
    tag = "h8-erp",
    params(("id" = uuid::Uuid, Path, description = "消息 ID")),
    responses(
        (status = 200, description = "按需解密的完整报文", body = H8DecryptedPayload),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 410, description = "报文已到期", body = ErrorResponse),
        (status = 503, description = "加密主密钥不可用", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn decrypt_h8_erp_message_payload() {}

#[utoipa::path(
    get,
    path = "/api/v1/integration/erp-messages/worker-runtime",
    tag = "h8-erp",
    responses(
        (status = 200, description = "Worker 心跳与认领控制", body = H8WorkerRuntimeResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_h8_worker_runtime() {}

#[utoipa::path(
    post,
    path = "/api/v1/integration/erp-messages/worker-runtime/heartbeat",
    tag = "h8-erp",
    request_body = H8WorkerHeartbeatRequest,
    responses(
        (status = 200, description = "记录 Worker 心跳", body = H8WorkerStatus),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn record_h8_worker_heartbeat() {}

#[utoipa::path(
    post,
    path = "/api/v1/integration/erp-messages/worker-runtime/control",
    tag = "h8-erp",
    request_body = SetH8WorkerClaimControlRequest,
    responses(
        (status = 200, description = "暂停或恢复连接方向的消息认领", body = H8WorkerClaimControl),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn set_h8_worker_claim_control() {}

#[utoipa::path(
    get,
    path = "/api/v1/integration/erp-messages/worker-runtime/claim-decision",
    tag = "h8-erp",
    params(
        ("connector_id" = uuid::Uuid, Query, description = "连接 ID"),
        ("direction" = String, Query, description = "inbound/outbound"),
    ),
    responses(
        (status = 200, description = "当前是否允许认领", body = H8WorkerClaimDecision),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_h8_worker_claim_decision() {}

#[utoipa::path(
    post,
    path = "/api/v1/integration/erp-messages/lifecycle",
    tag = "h8-erp",
    request_body = UpsertH8ErpMessageLifecycleRequest,
    responses(
        (status = 200, description = "按幂等键记录 Worker 交换阶段", body = H8ErpMessage),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 503, description = "完整报文保留密钥不可用", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn upsert_h8_erp_message_lifecycle() {}

#[utoipa::path(
    get,
    path = "/api/v1/integration/erp-messages/{id}",
    tag = "h8-erp",
    params(("id" = uuid::Uuid, Path, description = "消息 ID")),
    responses(
        (status = 200, description = "消息详情与尝试", body = H8ErpMessageDetail),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_h8_erp_message() {}

#[utoipa::path(
    post,
    path = "/api/v1/integration/erp-messages/{id}/replay",
    tag = "h8-erp",
    params(("id" = uuid::Uuid, Path, description = "消息 ID")),
    request_body = ReplayH8ErpMessageRequest,
    responses(
        (status = 200, description = "已接受重放", body = H8ErpMessage),
        (status = 400, description = "缺少原因或未确认", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "状态不允许重放", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn replay_h8_erp_message() {}

#[utoipa::path(
    post,
    path = "/api/v1/integration/erp-messages/{id}/claim",
    tag = "h8-erp",
    params(("id" = uuid::Uuid, Path, description = "消息 ID")),
    request_body = ClaimH8ErpMessageRequest,
    responses(
        (status = 200, description = "认领成功", body = H8ErpMessage),
        (status = 409, description = "租约冲突", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn claim_h8_erp_message() {}

#[utoipa::path(
    post,
    path = "/api/v1/integration/erp-messages/purge",
    tag = "h8-erp",
    request_body = PurgeH8ErpMessagesRequest,
    responses(
        (status = 200, description = "按保留策略清理终态消息", body = PurgeH8ErpMessagesResponse),
        (status = 400, description = "未配置保留策略", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn purge_h8_erp_messages() {}
