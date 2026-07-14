use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// 货主范围内可配置的告警定义。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertDefinition {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub alert_code: String,
    pub name: String,
    pub event_type: String,
    pub condition_expression: String,
    pub default_severity: String,
    pub recipient_roles: Vec<String>,
    pub escalation_ref: Option<String>,
    pub silence_period_seconds: i64,
    pub is_disable_allowed: bool,
    pub message_template: String,
    pub is_gsp_forced: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 新增告警定义请求。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateAlertDefinitionRequest {
    pub alert_code: String,
    pub name: String,
    pub event_type: String,
    pub condition_expression: String,
    pub default_severity: String,
    pub recipient_roles: Vec<String>,
    pub escalation_ref: Option<String>,
    pub silence_period_seconds: i64,
    pub is_disable_allowed: bool,
    pub message_template: String,
    pub is_gsp_forced: bool,
}

/// 告警定义启停策略变更请求。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateAlertDefinitionPolicyRequest {
    pub is_disable_allowed: bool,
}
