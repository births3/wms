use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::{audit_diff_schema, PageMeta};

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
    pub diff: Value,
}

/// 审计事件分页响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuditEventListResponse {
    /// 事件列表。
    pub data: Vec<AuditEvent>,
    /// 下一页游标；为空表示无更多数据。
    pub next_cursor: Option<String>,
}

/// 审计归档分区状态。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuditArchivePartitionState {
    pub partition_name: String,
    pub partition_start: NaiveDate,
    pub partition_end: NaiveDate,
    pub storage_tier: String,
    pub target_tier: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuditArchivePartitionStateListResponse {
    pub data: Vec<AuditArchivePartitionState>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuditArchiveRunRequest {
    pub reference_date: Option<NaiveDate>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuditArchiveRunResponse {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub reference_date: NaiveDate,
    pub partitions_seen: i32,
    pub partitions_archived: i32,
    pub created_at: DateTime<Utc>,
}

/// H2 事件投递记录。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EventDelivery {
    pub id: Uuid,
    pub event_id: Uuid,
    pub status: String,
    pub attempt_count: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EventDeliveryListResponse {
    pub data: Vec<EventDelivery>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EventDeliveryNackRequest {
    pub error: String,
}

/// H2 业务数据留存策略。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BusinessRetentionPolicy {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub policy_code: String,
    pub retention_years: Option<i32>,
    pub online_retention_months: i32,
    pub permanent: bool,
    pub special_drug: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BusinessRetentionPolicyListResponse {
    pub data: Vec<BusinessRetentionPolicy>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PlanBusinessArchiveJobRequest {
    pub policy_code: String,
    pub table_name: String,
    pub reference_date: Option<NaiveDate>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BusinessArchiveJob {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub policy_code: String,
    pub table_name: String,
    pub target_layer: String,
    pub status: String,
    pub cutoff_date: Option<NaiveDate>,
    pub delete_allowed: bool,
}
