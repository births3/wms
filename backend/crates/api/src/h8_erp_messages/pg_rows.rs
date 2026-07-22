//! H8 消息 PostgreSQL 行映射。

use chrono::{DateTime, Utc};
use uuid::Uuid;
use wms_domain::{H8ErpMessage, H8ErpMessageAttempt};

#[derive(sqlx::FromRow)]
pub(super) struct MessageRow {
    id: Uuid,
    owner_id: Uuid,
    warehouse_id: Option<Uuid>,
    pub(super) connector_id: Option<Uuid>,
    connector_code: Option<String>,
    config_version: Option<i64>,
    pub(super) direction: String,
    message_type: String,
    schema_version: String,
    pub(super) channel: String,
    external_ref: String,
    wms_resource_id: Option<String>,
    pub(super) idempotency_key: String,
    correlation_id: String,
    pub(super) sync_status: String,
    pub(super) retry_count: i32,
    next_retry_at: Option<DateTime<Utc>>,
    last_error_summary: Option<String>,
    payload_digest: String,
    pub(super) claimed_by: Option<String>,
    pub(super) lease_expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    pub(super) updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    acked_at: Option<DateTime<Utc>>,
}

impl From<MessageRow> for H8ErpMessage {
    fn from(row: MessageRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            warehouse_id: row.warehouse_id,
            connector_id: row.connector_id,
            connector_code: row.connector_code,
            config_version: row.config_version,
            direction: row.direction,
            message_type: row.message_type,
            schema_version: row.schema_version,
            channel: row.channel,
            external_ref: row.external_ref,
            wms_resource_id: row.wms_resource_id,
            idempotency_key: row.idempotency_key,
            correlation_id: row.correlation_id,
            sync_status: row.sync_status,
            retry_count: row.retry_count,
            next_retry_at: row.next_retry_at,
            last_error_summary: row.last_error_summary,
            payload_digest: row.payload_digest,
            claimed_by: row.claimed_by,
            lease_expires_at: row.lease_expires_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            completed_at: row.completed_at,
            acked_at: row.acked_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct AttemptRow {
    id: Uuid,
    message_id: Uuid,
    attempt_no: i32,
    channel: String,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    result: String,
    error_summary: Option<String>,
    actor: String,
}

impl From<AttemptRow> for H8ErpMessageAttempt {
    fn from(row: AttemptRow) -> Self {
        Self {
            id: row.id,
            message_id: row.message_id,
            attempt_no: row.attempt_no,
            channel: row.channel,
            started_at: row.started_at,
            finished_at: row.finished_at,
            result: row.result,
            error_summary: row.error_summary,
            actor: row.actor,
        }
    }
}

#[derive(sqlx::FromRow)]
pub(super) struct StatsRow {
    pub(super) total: i64,
    pub(super) succeeded: i64,
    pub(super) failed: i64,
    pub(super) dead: i64,
    pub(super) processing: i64,
    pub(super) pending: i64,
    pub(super) retry_total: i64,
}
