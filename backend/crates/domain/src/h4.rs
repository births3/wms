use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::{free_form_json_schema, PageMeta};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H4NotificationConfig {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub event_type: String,
    pub enabled: bool,
    pub template: String,
    #[schema(schema_with = free_form_json_schema)]
    pub recipient_rule: serde_json::Value,
    pub channels: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertH4NotificationConfigRequest {
    pub event_type: String,
    pub enabled: bool,
    pub template: String,
    #[schema(schema_with = free_form_json_schema)]
    pub recipient_rule: serde_json::Value,
    pub channels: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H4NotificationConfigListResponse {
    pub data: Vec<H4NotificationConfig>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H4WechatSettings {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub corp_id: String,
    pub agent_id: String,
    pub secret_alias: String,
    pub callback_token_alias: String,
    pub aes_key_alias: String,
    pub callback_url: String,
    pub approval_callback_path: String,
    pub enabled: bool,
    pub retry_max_attempts: i32,
    pub retry_interval_seconds: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertH4WechatSettingsRequest {
    pub corp_id: String,
    pub agent_id: String,
    pub secret_alias: String,
    pub callback_token_alias: String,
    pub aes_key_alias: String,
    pub callback_url: String,
    pub approval_callback_path: String,
    pub enabled: bool,
    pub retry_max_attempts: i32,
    pub retry_interval_seconds: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H4WechatSettingsResponse {
    pub data: Option<H4WechatSettings>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H4WechatSettingsTestResponse {
    pub status: String,
    pub message: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SendH4NotificationRequest {
    pub event_type: String,
    pub dedupe_key: String,
    pub recipients: Vec<String>,
    #[schema(schema_with = free_form_json_schema)]
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H4NotificationRecord {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub config_id: Option<Uuid>,
    pub event_type: String,
    pub dedupe_key: String,
    pub recipient: String,
    pub channel: String,
    pub content_summary: String,
    pub status: String,
    pub retry_count: i32,
    pub failure_reason: Option<String>,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H4NotificationRecordListResponse {
    pub data: Vec<H4NotificationRecord>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateH4ApprovalRequest {
    pub scenario: String,
    pub business_ref: String,
    pub dedupe_key: String,
    pub approver_user: String,
    pub process_id: String,
    pub callback_path: String,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H4ApprovalCallbackRequest {
    pub conclusion: String,
    pub opinion: Option<String>,
    pub approved_by: String,
    pub external_approval_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H4ApprovalRecord {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub scenario: String,
    pub business_ref: String,
    pub dedupe_key: String,
    pub approver_user: String,
    pub process_id: String,
    pub callback_path: String,
    pub summary: String,
    pub status: String,
    pub opinion: Option<String>,
    pub external_approval_id: Option<String>,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
