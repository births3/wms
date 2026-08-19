use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;
use wms_domain::{H4ApprovalRecord, H4NotificationConfig, H4NotificationRecord, H4WechatSettings};

#[derive(Clone, Debug, FromRow)]
pub(crate) struct ConfigRow {
    pub(crate) id: Uuid,
    pub(crate) owner_id: Uuid,
    pub(crate) event_type: String,
    pub(crate) enabled: bool,
    pub(crate) template: String,
    pub(crate) recipient_rule: Value,
    pub(crate) channels: Vec<String>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) version: i64,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct WechatSettingsRow {
    pub(crate) id: Uuid,
    pub(crate) owner_id: Uuid,
    pub(crate) corp_id: String,
    pub(crate) agent_id: String,
    pub(crate) secret_alias: String,
    pub(crate) callback_token_alias: String,
    pub(crate) aes_key_alias: String,
    pub(crate) callback_url: String,
    pub(crate) approval_callback_path: String,
    pub(crate) enabled: bool,
    pub(crate) retry_max_attempts: i32,
    pub(crate) retry_interval_seconds: i32,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) version: i64,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct RecordRow {
    pub(crate) id: Uuid,
    pub(crate) owner_id: Uuid,
    pub(crate) config_id: Option<Uuid>,
    pub(crate) event_type: String,
    pub(crate) dedupe_key: String,
    pub(crate) recipient: String,
    pub(crate) channel: String,
    pub(crate) content_summary: String,
    pub(crate) status: String,
    pub(crate) retry_count: i32,
    pub(crate) failure_reason: Option<String>,
    pub(crate) sent_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct ApprovalRow {
    pub(crate) id: Uuid,
    pub(crate) owner_id: Uuid,
    pub(crate) scenario: String,
    pub(crate) business_ref: String,
    pub(crate) dedupe_key: String,
    pub(crate) approver_user: String,
    pub(crate) process_id: String,
    pub(crate) callback_path: String,
    pub(crate) summary: String,
    pub(crate) status: String,
    pub(crate) opinion: Option<String>,
    pub(crate) external_approval_id: Option<String>,
    pub(crate) approved_by: Option<String>,
    pub(crate) approved_at: Option<DateTime<Utc>>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl From<ConfigRow> for H4NotificationConfig {
    fn from(row: ConfigRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            event_type: row.event_type,
            enabled: row.enabled,
            template: row.template,
            recipient_rule: row.recipient_rule,
            channels: row.channels,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        }
    }
}

impl From<WechatSettingsRow> for H4WechatSettings {
    fn from(row: WechatSettingsRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            corp_id: row.corp_id,
            agent_id: row.agent_id,
            secret_alias: row.secret_alias,
            callback_token_alias: row.callback_token_alias,
            aes_key_alias: row.aes_key_alias,
            callback_url: row.callback_url,
            approval_callback_path: row.approval_callback_path,
            enabled: row.enabled,
            retry_max_attempts: row.retry_max_attempts,
            retry_interval_seconds: row.retry_interval_seconds,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        }
    }
}

impl From<RecordRow> for H4NotificationRecord {
    fn from(row: RecordRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            config_id: row.config_id,
            event_type: row.event_type,
            dedupe_key: row.dedupe_key,
            recipient: row.recipient,
            channel: row.channel,
            content_summary: row.content_summary,
            status: row.status,
            retry_count: row.retry_count,
            failure_reason: row.failure_reason,
            sent_at: row.sent_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<ApprovalRow> for H4ApprovalRecord {
    fn from(row: ApprovalRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            scenario: row.scenario,
            business_ref: row.business_ref,
            dedupe_key: row.dedupe_key,
            approver_user: row.approver_user,
            process_id: row.process_id,
            callback_path: row.callback_path,
            summary: row.summary,
            status: row.status,
            opinion: row.opinion,
            external_approval_id: row.external_approval_id,
            approved_by: row.approved_by,
            approved_at: row.approved_at,
            failure_reason: row.failure_reason,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
