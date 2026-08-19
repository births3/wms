use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::PageMeta;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertInstance {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub alert_definition_id: Uuid,
    pub alert_code: String,
    pub alert_name: String,
    pub severity: String,
    pub event_type: String,
    pub resource_type: String,
    pub resource_id: String,
    pub resource_path: Option<String>,
    pub warehouse_id: Option<Uuid>,
    pub event_payload: serde_json::Value,
    pub recipients: Vec<String>,
    pub status: String,
    pub escalation_level: i32,
    pub action_description: Option<String>,
    pub ignored_reason: Option<String>,
    pub close_reason: Option<String>,
    pub triggered_at: DateTime<Utc>,
    pub notified_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub handled_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AlertInstanceListQuery {
    pub warehouse_id: Option<Uuid>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub alert_code: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub active_only: Option<bool>,
    pub limit: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertInstanceListResponse {
    pub data: Vec<AlertInstance>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertActionRequest {
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct AlertEscalationLevelDraft {
    pub level: i32,
    pub threshold_seconds: i64,
    pub recipient_roles: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct UpsertAlertEscalationRuleRequest {
    pub rule_code: String,
    pub rule_name: String,
    pub notify_lower_levels: bool,
    pub off_hours_start: String,
    pub off_hours_end: String,
    pub off_hours_handler_roles: Vec<String>,
    pub holiday_dates: Vec<chrono::NaiveDate>,
    pub enabled: bool,
    pub levels: Vec<AlertEscalationLevelDraft>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertEscalationRule {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub rule_code: String,
    pub rule_name: String,
    pub notify_lower_levels: bool,
    pub off_hours_start: String,
    pub off_hours_end: String,
    pub off_hours_handler_roles: Vec<String>,
    pub holiday_dates: Vec<chrono::NaiveDate>,
    pub enabled: bool,
    pub levels: Vec<AlertEscalationLevelDraft>,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertEscalationRuleListResponse {
    pub data: Vec<AlertEscalationRule>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertMonthlyMetric {
    pub month: String,
    pub triggered_count: i64,
    pub acknowledgement_rate: f64,
    pub average_response_seconds: Option<f64>,
    pub escalation_rate: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertRankingItem {
    pub key: String,
    pub count: i64,
    pub average_response_seconds: Option<f64>,
    pub unacknowledged_count: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertStatisticsResponse {
    pub generated_at: DateTime<Utc>,
    pub possibly_stale: bool,
    pub monthly: Vec<AlertMonthlyMetric>,
    pub alert_type_top10: Vec<AlertRankingItem>,
    pub recipient_top10: Vec<AlertRankingItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct GspAlertLifecycleRecord {
    pub alert: AlertInstance,
    pub lifecycle_events: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct GspAlertLifecycleReport {
    pub generated_at: DateTime<Utc>,
    pub data: Vec<GspAlertLifecycleRecord>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateAlertExportRequest {
    pub format: String,
    pub filters: AlertInstanceListQuery,
    pub recipient_email: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertExportJob {
    pub id: Uuid,
    pub status: String,
    pub format: String,
    pub row_count: i64,
    pub download_url: Option<String>,
    pub email_notification_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertChangeEvent {
    pub id: Uuid,
    pub alert_instance_id: Uuid,
    pub to_status: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AlertChangeListResponse {
    pub data: Vec<AlertChangeEvent>,
    pub server_time: DateTime<Utc>,
}
