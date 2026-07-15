use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::PageMeta;

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
    pub enabled: bool,
    pub message_template: String,
    pub message_templates: BTreeMap<String, String>,
    pub is_gsp_forced: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct AlertDefinitionDraft {
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
    pub message_templates: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AlertDefinitionChangeOperation {
    Upsert,
    SetEnabled,
    Delete,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct SubmitAlertDefinitionChangeRequest {
    pub operation: AlertDefinitionChangeOperation,
    pub definition_id: Option<Uuid>,
    pub expected_version: Option<i64>,
    pub definition: Option<AlertDefinitionDraft>,
    pub enabled: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AlertDefinitionListQuery {
    pub keyword: Option<String>,
    pub severity: Option<String>,
    pub enabled: Option<bool>,
    pub limit: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertDefinitionListResponse {
    pub data: Vec<AlertDefinition>,
    pub page: PageMeta,
}
