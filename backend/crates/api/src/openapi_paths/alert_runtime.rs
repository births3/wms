#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get, path = "/api/v1/alerts", tag = "alert-engine", params(AlertInstanceListQuery),
    responses(
        (status = 200, description = "货主范围内告警实例列表", body = AlertInstanceListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_alert_instances() {}

#[utoipa::path(
    get, path = "/api/v1/alerts/{id}", tag = "alert-engine",
    params(("id" = Uuid, Path, description = "告警实例 ID")),
    responses(
        (status = 200, description = "告警实例详情", body = AlertInstance),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "告警实例不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_alert_instance() {}

#[utoipa::path(
    post, path = "/api/v1/alerts/{id}/acknowledge", tag = "alert-engine",
    params(("id" = Uuid, Path, description = "告警实例 ID")),
    responses(
        (status = 200, description = "确认接警", body = AlertInstance),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "告警实例不存在", body = ErrorResponse),
        (status = 409, description = "状态不允许确认", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn acknowledge_alert_instance() {}

#[utoipa::path(
    post, path = "/api/v1/alerts/{id}/handling", tag = "alert-engine",
    params(("id" = Uuid, Path, description = "告警实例 ID")),
    request_body = AlertActionRequest,
    responses(
        (status = 200, description = "记录处理过程", body = AlertInstance),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "告警实例不存在", body = ErrorResponse),
        (status = 422, description = "处理说明为空", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn handle_alert_instance() {}

#[utoipa::path(
    post, path = "/api/v1/alerts/{id}/close", tag = "alert-engine",
    params(("id" = Uuid, Path, description = "告警实例 ID")),
    request_body = AlertActionRequest,
    responses(
        (status = 200, description = "关闭告警", body = AlertInstance),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "告警实例不存在", body = ErrorResponse),
        (status = 422, description = "关闭原因为空", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn close_alert_instance() {}

#[utoipa::path(
    post, path = "/api/v1/alerts/{id}/ignore", tag = "alert-engine",
    params(("id" = Uuid, Path, description = "告警实例 ID")),
    request_body = AlertActionRequest,
    responses(
        (status = 200, description = "忽略告警", body = AlertInstance),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "告警实例不存在", body = ErrorResponse),
        (status = 422, description = "忽略原因为空", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn ignore_alert_instance() {}

#[utoipa::path(
    get, path = "/api/v1/alert-escalation-rules", tag = "alert-engine",
    responses(
        (status = 200, description = "升级规则列表", body = AlertEscalationRuleListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_alert_escalation_rules() {}

#[utoipa::path(
    put, path = "/api/v1/alert-escalation-rules/{rule_code}", tag = "alert-engine",
    params(("rule_code" = String, Path, description = "升级规则编码")),
    request_body = UpsertAlertEscalationRuleRequest,
    responses(
        (status = 200, description = "升级规则已保存", body = AlertEscalationRule),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 422, description = "规则字段非法或超过三级", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn upsert_alert_escalation_rule() {}

#[utoipa::path(
    get, path = "/api/v1/alerts/active", tag = "alert-engine", params(AlertInstanceListQuery),
    responses(
        (status = 200, description = "当前用户有权查看的活动告警", body = AlertInstanceListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "仓库范围越权", body = ErrorResponse),
        (status = 422, description = "仓库主管未选择授权仓库", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_active_alerts() {}

#[utoipa::path(
    get, path = "/api/v1/alerts/statistics", tag = "alert-engine", params(AlertInstanceListQuery),
    responses(
        (status = 200, description = "月度趋势及告警和接收人排行", body = AlertStatisticsResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 422, description = "查询范围超过一年", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_alert_statistics() {}

#[utoipa::path(
    get, path = "/api/v1/alerts/gsp-report", tag = "alert-engine", params(AlertInstanceListQuery),
    responses(
        (status = 200, description = "GSP 强制告警完整生命周期报表", body = GspAlertLifecycleReport),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_gsp_alert_report() {}

#[utoipa::path(
    get, path = "/api/v1/alerts/changes", tag = "alert-engine",
    params(("since" = Option<String>, Query, description = "ISO-8601 增量起点，默认最近五秒")),
    responses(
        (status = 200, description = "看板增量变更", body = AlertChangeListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_alert_changes() {}

#[utoipa::path(
    post, path = "/api/v1/alerts/exports", tag = "alert-engine",
    request_body = CreateAlertExportRequest,
    responses(
        (status = 201, description = "十万行以内同步生成", body = AlertExportJob),
        (status = 202, description = "超过十万行进入异步队列", body = AlertExportJob),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 422, description = "格式或过滤条件非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn create_alert_export() {}

#[utoipa::path(
    get, path = "/api/v1/alerts/exports/{id}", tag = "alert-engine",
    params(("id" = Uuid, Path, description = "导出任务 ID")),
    responses(
        (status = 200, description = "导出任务状态", body = AlertExportJob),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "导出任务不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_alert_export() {}

#[utoipa::path(
    get, path = "/api/v1/alerts/exports/{token}/download", tag = "alert-engine",
    params(("token" = Uuid, Path, description = "七天有效下载令牌")),
    responses(
        (status = 200, description = "下载 Excel 或 PDF 文件", content_type = "application/octet-stream"),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "导出文件不存在或已过期", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn download_alert_export() {}
