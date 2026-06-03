//! Wave 1 W1.C 主仓 OpenAPI 契约骨架使用的最小 schema。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{
    openapi::schema::{AdditionalProperties, Object, ObjectBuilder},
    ToSchema,
};
use uuid::Uuid;

fn audit_diff_schema() -> Object {
    ObjectBuilder::new()
        .description(Some("变更详情。"))
        .additional_properties(Some(AdditionalProperties::FreeForm(true)))
        .build()
}

fn error_details_schema() -> Object {
    ObjectBuilder::new()
        .description(Some("关联详情。"))
        .additional_properties(Some(AdditionalProperties::FreeForm(true)))
        .build()
}

/// 健康检查响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct HealthzResponse {
    /// 服务状态。
    pub status: String,
    /// 契约版本。
    pub version: String,
    /// 文档生成时间。
    pub generated_at: DateTime<Utc>,
}

/// 登录请求。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LoginRequest {
    /// 货主编码。
    pub owner_code: String,
    /// 登录账号。
    pub username: String,
    /// 登录密码。
    pub password: String,
}

/// 当前登录用户摘要。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CurrentUser {
    /// 用户 ID。
    pub user_id: Uuid,
    /// 货主 ID。
    pub owner_id: Uuid,
    /// 货主编码。
    pub owner_code: String,
    /// 用户名。
    pub username: String,
    /// 展示名。
    pub display_name: String,
    /// 当前角色列表。
    pub roles: Vec<String>,
    /// 当前权限码列表。
    pub permissions: Vec<String>,
}

/// 登录成功响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LoginResponse {
    /// Bearer token。
    pub access_token: String,
    /// token 类型。
    pub token_type: String,
    /// 过期时间。
    pub expires_at: DateTime<Utc>,
    /// 当前用户。
    pub user: CurrentUser,
}

/// 审计事件操作者摘要。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuditActor {
    /// 操作者 ID。
    pub actor_id: Uuid,
    /// 操作者名称。
    pub actor_name: String,
    /// 操作者所属货主 ID。
    pub owner_id: Uuid,
    /// JWT jti，用于追溯登录态。
    pub jti: String,
}

/// 审计事件。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuditEvent {
    /// 审计事件 ID。
    pub id: i64,
    /// 被审计记录所属货主 ID。
    pub owner_id: Uuid,
    /// 资源类型。
    pub resource_type: String,
    /// 资源实例 ID。
    pub resource_id: String,
    /// 事件动作。
    pub action: String,
    /// 审计 trace ID。
    pub trace_id: String,
    /// 发生时间。
    pub occurred_at: DateTime<Utc>,
    /// 操作者摘要。
    pub actor: AuditActor,
    /// 变更详情。
    #[schema(schema_with = audit_diff_schema)]
    pub diff: serde_json::Value,
}

/// 审计事件分页响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuditEventListResponse {
    /// 事件列表。
    pub data: Vec<AuditEvent>,
    /// 下一页游标；为空表示无更多数据。
    pub next_cursor: Option<String>,
}

/// 统一错误响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// 业务错误码。
    pub code: String,
    /// 中文错误消息。
    pub message: String,
    /// 严重度。
    pub severity: String,
    /// 关联详情。
    #[schema(schema_with = error_details_schema)]
    pub details: serde_json::Value,
    /// 链路追踪 ID。
    pub trace_id: String,
    /// 重试提示。
    pub retry_hint: Option<String>,
}
