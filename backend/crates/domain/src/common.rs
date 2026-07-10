use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{
    openapi::schema::{AdditionalProperties, Object, ObjectBuilder},
    ToSchema,
};
use uuid::Uuid;

pub(crate) fn audit_diff_schema() -> Object {
    ObjectBuilder::new()
        .description(Some("变更详情。"))
        .additional_properties(Some(AdditionalProperties::FreeForm(true)))
        .build()
}

pub(crate) fn error_details_schema() -> Object {
    ObjectBuilder::new()
        .description(Some("关联详情。"))
        .additional_properties(Some(AdditionalProperties::FreeForm(true)))
        .build()
}

pub(crate) fn free_form_json_schema() -> Object {
    ObjectBuilder::new()
        .description(Some("自由结构 JSON 对象。"))
        .additional_properties(Some(AdditionalProperties::FreeForm(true)))
        .build()
}

/// 分页信息。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PageMeta {
    /// 下一页游标；为空表示无更多数据。
    pub next_cursor: Option<String>,
    /// 本页数量。
    pub count: u32,
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

/// H3 API 韧性保护状态。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ResilienceStatus {
    /// 全局令牌桶容量。
    pub rate_limit_capacity: u32,
    /// 当前可用令牌数。
    pub rate_limit_available: u32,
    /// 限流或熔断拒绝总次数。
    pub rate_limit_rejected_total: u64,
    /// 熔断状态：closed / open。
    pub circuit_state: String,
    /// 熔断剩余秒数。
    pub circuit_open_remaining_seconds: u64,
    /// 熔断打开总次数。
    pub circuit_opened_total: u64,
    /// 当前连续失败次数。
    pub consecutive_failures: u32,
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
