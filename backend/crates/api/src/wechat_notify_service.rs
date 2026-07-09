//! H4 企业微信通知与审批服务。

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    CreateH4ApprovalRequest, H4ApprovalCallbackRequest, H4ApprovalRecord, H4NotificationConfig,
    H4NotificationConfigListResponse, H4NotificationRecord, H4NotificationRecordListResponse,
    H4WechatSettings, H4WechatSettingsResponse, PageMeta, SendH4NotificationRequest,
    UpsertH4NotificationConfigRequest, UpsertH4WechatSettingsRequest,
};

use crate::{
    auth::AuthContext,
    wechat_notify_idempotency::{
        finish_mutation, json_request_hash, lock_idempotency_key, replay_idempotency,
    },
};

pub const DEFAULT_H4_QUERY_LIMIT: u32 = 50;
pub const MAX_H4_QUERY_LIMIT: u32 = 100;

#[derive(Clone, Debug)]
pub struct PgWechatNotifyService;

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WechatNotifyError {
    EventNotFound,
    TemplateInvalid,
    NoRecipients,
    ApprovalNotFound,
    InvalidRequest,
    InvalidStatus,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, Deserialize)]
pub struct H4RecordQuery {
    pub event_type: Option<String>,
    pub recipient: Option<String>,
    pub status: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, FromRow)]
struct ConfigRow {
    id: Uuid,
    owner_id: Uuid,
    event_type: String,
    enabled: bool,
    template: String,
    recipient_rule: Value,
    channels: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

#[derive(Clone, Debug, FromRow)]
struct WechatSettingsRow {
    id: Uuid,
    owner_id: Uuid,
    corp_id: String,
    agent_id: String,
    secret_alias: String,
    callback_token_alias: String,
    aes_key_alias: String,
    callback_url: String,
    approval_callback_path: String,
    enabled: bool,
    retry_max_attempts: i32,
    retry_interval_seconds: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

#[derive(Clone, Debug, FromRow)]
struct RecordRow {
    id: Uuid,
    owner_id: Uuid,
    config_id: Option<Uuid>,
    event_type: String,
    dedupe_key: String,
    recipient: String,
    channel: String,
    content_summary: String,
    status: String,
    retry_count: i32,
    failure_reason: Option<String>,
    sent_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow)]
struct ApprovalRow {
    id: Uuid,
    owner_id: Uuid,
    scenario: String,
    business_ref: String,
    dedupe_key: String,
    approver_user: String,
    process_id: String,
    callback_path: String,
    summary: String,
    status: String,
    opinion: Option<String>,
    external_approval_id: Option<String>,
    approved_by: Option<String>,
    approved_at: Option<DateTime<Utc>>,
    failure_reason: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PgWechatNotifyService {
    pub fn new() -> Self {
        Self
    }

    pub async fn list_configs(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        event_type: Option<&str>,
    ) -> Result<H4NotificationConfigListResponse, WechatNotifyError> {
        let rows = sqlx::query_as::<_, ConfigRow>(
            r#"
            SELECT id, owner_id, event_type, enabled, template, recipient_rule,
                   channels, created_at, updated_at, version
              FROM h4_notification_configs
             WHERE owner_id = $1
               AND ($2::TEXT IS NULL OR event_type = $2)
             ORDER BY event_type ASC
            "#,
        )
        .bind(ctx.owner_id)
        .bind(event_type)
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;
        Ok(H4NotificationConfigListResponse {
            page: PageMeta {
                next_cursor: None,
                count: rows.len() as u32,
            },
            data: rows.into_iter().map(Into::into).collect(),
        })
    }

    pub async fn upsert_config(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: UpsertH4NotificationConfigRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<H4NotificationConfig>, WechatNotifyError> {
        validate_config_request(&req)?;
        let request_hash = json_request_hash(&req)?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let row = sqlx::query_as::<_, ConfigRow>(
            r#"
            INSERT INTO h4_notification_configs (
                id, owner_id, event_type, enabled, template, recipient_rule,
                channels, created_by, updated_by, created_at, updated_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $9, $9, 1)
            ON CONFLICT (owner_id, event_type)
            DO UPDATE SET
                enabled = EXCLUDED.enabled,
                template = EXCLUDED.template,
                recipient_rule = EXCLUDED.recipient_rule,
                channels = EXCLUDED.channels,
                updated_by = EXCLUDED.updated_by,
                updated_at = EXCLUDED.updated_at,
                version = h4_notification_configs.version + 1
            RETURNING id, owner_id, event_type, enabled, template, recipient_rule,
                      channels, created_at, updated_at, version
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(req.event_type.trim())
        .bind(req.enabled)
        .bind(req.template.trim())
        .bind(req.recipient_rule)
        .bind(normalize_channels(req.channels))
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let value: H4NotificationConfig = row.into();
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/wechat-notify/configs",
            "h4_notification_config",
            value.id.to_string(),
            &value,
            "h4.config.upserted",
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }

    pub async fn get_wechat_settings(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
    ) -> Result<H4WechatSettingsResponse, WechatNotifyError> {
        let row = sqlx::query_as::<_, WechatSettingsRow>(
            r#"
            SELECT id, owner_id, corp_id, agent_id, secret_alias,
                   callback_token_alias, aes_key_alias, callback_url,
                   approval_callback_path, enabled, retry_max_attempts,
                   retry_interval_seconds, created_at, updated_at, version
              FROM h4_wechat_settings
             WHERE owner_id = $1
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_optional(pool)
        .await
        .map_err(map_db_error)?;
        Ok(H4WechatSettingsResponse {
            data: row.map(Into::into),
        })
    }

    pub async fn upsert_wechat_settings(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: UpsertH4WechatSettingsRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<H4WechatSettings>, WechatNotifyError> {
        validate_wechat_settings_request(&req)?;
        let request_hash = json_request_hash(&req)?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let row = sqlx::query_as::<_, WechatSettingsRow>(
            r#"
            INSERT INTO h4_wechat_settings (
                id, owner_id, corp_id, agent_id, secret_alias, callback_token_alias,
                aes_key_alias, callback_url, approval_callback_path, enabled,
                retry_max_attempts, retry_interval_seconds, created_by, updated_by,
                created_at, updated_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $13, $14, $14, 1)
            ON CONFLICT (owner_id)
            DO UPDATE SET
                corp_id = EXCLUDED.corp_id,
                agent_id = EXCLUDED.agent_id,
                secret_alias = EXCLUDED.secret_alias,
                callback_token_alias = EXCLUDED.callback_token_alias,
                aes_key_alias = EXCLUDED.aes_key_alias,
                callback_url = EXCLUDED.callback_url,
                approval_callback_path = EXCLUDED.approval_callback_path,
                enabled = EXCLUDED.enabled,
                retry_max_attempts = EXCLUDED.retry_max_attempts,
                retry_interval_seconds = EXCLUDED.retry_interval_seconds,
                updated_by = EXCLUDED.updated_by,
                updated_at = EXCLUDED.updated_at,
                version = h4_wechat_settings.version + 1
            RETURNING id, owner_id, corp_id, agent_id, secret_alias,
                      callback_token_alias, aes_key_alias, callback_url,
                      approval_callback_path, enabled, retry_max_attempts,
                      retry_interval_seconds, created_at, updated_at, version
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(req.corp_id.trim())
        .bind(req.agent_id.trim())
        .bind(req.secret_alias.trim())
        .bind(req.callback_token_alias.trim())
        .bind(req.aes_key_alias.trim())
        .bind(req.callback_url.trim())
        .bind(req.approval_callback_path.trim())
        .bind(req.enabled)
        .bind(req.retry_max_attempts)
        .bind(req.retry_interval_seconds)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let value: H4WechatSettings = row.into();
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/wechat-notify/settings",
            "h4_wechat_settings",
            value.id.to_string(),
            &value,
            "h4.wechat_settings.upserted",
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }

    pub async fn send_notification(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: SendH4NotificationRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<Vec<H4NotificationRecord>>, WechatNotifyError> {
        validate_send_request(&req)?;
        let request_hash = json_request_hash(&req)?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let config = load_enabled_config(&mut tx, ctx.owner_id, &req.event_type).await?;
        let content = render_template(&config.template, &req.payload)?;
        let summary = summarize(&content);
        let mut records = Vec::new();
        for recipient in req
            .recipients
            .iter()
            .map(|value| value.trim())
            .filter(|v| !v.is_empty())
        {
            let row = sqlx::query_as::<_, RecordRow>(
                r#"
                INSERT INTO h4_notification_records (
                    id, owner_id, config_id, event_type, dedupe_key, recipient, channel,
                    content, content_summary, status, retry_count, failure_reason,
                    sent_at, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, 'wechat', $7, $8,
                        'success', 0, NULL, $9, $9, $9)
                ON CONFLICT (owner_id, event_type, recipient, dedupe_key)
                DO UPDATE SET updated_at = h4_notification_records.updated_at
                RETURNING id, owner_id, config_id, event_type, dedupe_key, recipient,
                          channel, content_summary, status, retry_count, failure_reason,
                          sent_at, created_at, updated_at
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(config.id)
            .bind(&req.event_type)
            .bind(&req.dedupe_key)
            .bind(recipient)
            .bind(&content)
            .bind(&summary)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            records.push(row.into());
        }
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/wechat-notify/send",
            "h4_notification_record",
            req.dedupe_key.clone(),
            &records,
            "h4.notify.sent",
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: records,
            replayed: false,
        })
    }

    pub async fn create_approval(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: CreateH4ApprovalRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<H4ApprovalRecord>, WechatNotifyError> {
        validate_approval_request(&req)?;
        let request_hash = json_request_hash(&req)?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let approval_id = Uuid::new_v4();
        let row = sqlx::query_as::<_, ApprovalRow>(
            r#"
            INSERT INTO h4_approval_records (
                id, owner_id, scenario, business_ref, dedupe_key, approver_user,
                process_id, callback_path, summary, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', $10, $10)
            ON CONFLICT (owner_id, scenario, business_ref, dedupe_key)
            DO UPDATE SET updated_at = h4_approval_records.updated_at
            RETURNING id, owner_id, scenario, business_ref, dedupe_key, approver_user,
                      process_id, callback_path, summary, status, opinion,
                      external_approval_id, approved_by, approved_at, failure_reason,
                      created_at, updated_at
            "#,
        )
        .bind(approval_id)
        .bind(ctx.owner_id)
        .bind(req.scenario.trim())
        .bind(req.business_ref.trim())
        .bind(req.dedupe_key.trim())
        .bind(req.approver_user.trim())
        .bind(req.process_id.trim())
        .bind(req.callback_path.trim())
        .bind(req.summary.trim())
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let value: H4ApprovalRecord = row.into();
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/wechat-notify/approvals",
            "h4_approval_record",
            value.id.to_string(),
            &value,
            "h4.approval.created",
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }

    pub async fn apply_approval_callback(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        approval_id: Uuid,
        req: H4ApprovalCallbackRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<H4ApprovalRecord>, WechatNotifyError> {
        let status = match req.conclusion.trim() {
            "approved" | "同意" => "approved",
            "rejected" | "拒绝" => "rejected",
            _ => return Err(WechatNotifyError::InvalidStatus),
        };
        let request_hash = json_request_hash(&serde_json::json!({
            "approval_id": approval_id,
            "request": &req,
        }))?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let row = sqlx::query_as::<_, ApprovalRow>(
            r#"
            UPDATE h4_approval_records
               SET status = CASE WHEN status = 'pending' THEN $3 ELSE status END,
                   opinion = CASE WHEN status = 'pending' THEN $4 ELSE opinion END,
                   external_approval_id = COALESCE(external_approval_id, $5),
                   approved_by = CASE WHEN status = 'pending' THEN $6 ELSE approved_by END,
                   approved_at = CASE WHEN status = 'pending' THEN $7 ELSE approved_at END,
                   updated_at = $7
             WHERE owner_id = $1 AND id = $2
             RETURNING id, owner_id, scenario, business_ref, dedupe_key, approver_user,
                       process_id, callback_path, summary, status, opinion,
                       external_approval_id, approved_by, approved_at, failure_reason,
                       created_at, updated_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(approval_id)
        .bind(status)
        .bind(req.opinion)
        .bind(req.external_approval_id)
        .bind(req.approved_by)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(WechatNotifyError::ApprovalNotFound)?;
        let value: H4ApprovalRecord = row.into();
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/wechat-notify/approvals/{approval_id}/callback",
            "h4_approval_record",
            value.id.to_string(),
            &value,
            "h4.approval.callback_applied",
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }

    pub async fn list_records(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        query: H4RecordQuery,
    ) -> Result<H4NotificationRecordListResponse, WechatNotifyError> {
        let limit = query
            .limit
            .unwrap_or(DEFAULT_H4_QUERY_LIMIT)
            .min(MAX_H4_QUERY_LIMIT);
        let rows = sqlx::query_as::<_, RecordRow>(
            r#"
            SELECT id, owner_id, config_id, event_type, dedupe_key, recipient,
                   channel, content_summary, status, retry_count, failure_reason,
                   sent_at, created_at, updated_at
              FROM h4_notification_records
             WHERE owner_id = $1
               AND ($2::TEXT IS NULL OR event_type = $2)
               AND ($3::TEXT IS NULL OR recipient ILIKE '%' || $3 || '%')
               AND ($4::TEXT IS NULL OR status = $4)
               AND ($5::TIMESTAMPTZ IS NULL OR created_at >= $5)
               AND ($6::TIMESTAMPTZ IS NULL OR created_at <= $6)
             ORDER BY created_at DESC, id DESC
             LIMIT $7
            "#,
        )
        .bind(ctx.owner_id)
        .bind(query.event_type)
        .bind(query.recipient)
        .bind(query.status)
        .bind(query.from)
        .bind(query.to)
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;
        Ok(H4NotificationRecordListResponse {
            page: PageMeta {
                next_cursor: None,
                count: rows.len() as u32,
            },
            data: rows.into_iter().map(Into::into).collect(),
        })
    }

    pub async fn resend_record(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        record_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<H4NotificationRecord>, WechatNotifyError> {
        let request_hash = json_request_hash(&serde_json::json!({ "record_id": record_id }))?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let row = sqlx::query_as::<_, RecordRow>(
            r#"
            UPDATE h4_notification_records
               SET status = 'success',
                   retry_count = retry_count + 1,
                   failure_reason = NULL,
                   sent_at = $3,
                   updated_at = $3
             WHERE owner_id = $1 AND id = $2
             RETURNING id, owner_id, config_id, event_type, dedupe_key, recipient,
                       channel, content_summary, status, retry_count, failure_reason,
                       sent_at, created_at, updated_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(record_id)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(WechatNotifyError::EventNotFound)?;
        let value: H4NotificationRecord = row.into();
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/wechat-notify/records/{record_id}/resend",
            "h4_notification_record",
            value.id.to_string(),
            &value,
            "h4.notify.resent",
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }
}

async fn load_enabled_config(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    event_type: &str,
) -> Result<ConfigRow, WechatNotifyError> {
    sqlx::query_as::<_, ConfigRow>(
        r#"
        SELECT id, owner_id, event_type, enabled, template, recipient_rule,
               channels, created_at, updated_at, version
          FROM h4_notification_configs
         WHERE owner_id = $1 AND event_type = $2 AND enabled = TRUE
        "#,
    )
    .bind(owner_id)
    .bind(event_type)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(WechatNotifyError::EventNotFound)
}

fn validate_config_request(
    req: &UpsertH4NotificationConfigRequest,
) -> Result<(), WechatNotifyError> {
    if req.event_type.trim().is_empty() || req.template.trim().is_empty() {
        return Err(WechatNotifyError::InvalidRequest);
    }
    if !req.recipient_rule.is_object()
        || !normalize_channels(req.channels.clone())
            .iter()
            .any(|v| v == "wechat")
    {
        return Err(WechatNotifyError::NoRecipients);
    }
    Ok(())
}

fn validate_wechat_settings_request(
    req: &UpsertH4WechatSettingsRequest,
) -> Result<(), WechatNotifyError> {
    if [
        req.corp_id.as_str(),
        req.agent_id.as_str(),
        req.secret_alias.as_str(),
        req.callback_token_alias.as_str(),
        req.aes_key_alias.as_str(),
        req.callback_url.as_str(),
        req.approval_callback_path.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(WechatNotifyError::InvalidRequest);
    }
    if req.retry_max_attempts < 0
        || req.retry_max_attempts > 10
        || req.retry_interval_seconds < 1
        || req.retry_interval_seconds > 3600
    {
        return Err(WechatNotifyError::InvalidRequest);
    }
    Ok(())
}

fn validate_send_request(req: &SendH4NotificationRequest) -> Result<(), WechatNotifyError> {
    if req.event_type.trim().is_empty() || req.dedupe_key.trim().is_empty() {
        return Err(WechatNotifyError::InvalidRequest);
    }
    if req.recipients.iter().all(|value| value.trim().is_empty()) {
        return Err(WechatNotifyError::NoRecipients);
    }
    Ok(())
}

fn validate_approval_request(req: &CreateH4ApprovalRequest) -> Result<(), WechatNotifyError> {
    if [
        req.scenario.as_str(),
        req.business_ref.as_str(),
        req.dedupe_key.as_str(),
        req.approver_user.as_str(),
        req.process_id.as_str(),
        req.callback_path.as_str(),
        req.summary.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(WechatNotifyError::InvalidRequest);
    }
    Ok(())
}

fn render_template(template: &str, payload: &Value) -> Result<String, WechatNotifyError> {
    let mut content = template.to_string();
    let object = payload
        .as_object()
        .ok_or(WechatNotifyError::TemplateInvalid)?;
    for (key, value) in object {
        let rendered = value
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| value.to_string());
        content = content.replace(&format!("{{{{{key}}}}}"), &rendered);
    }
    if content.contains("{{") || content.contains("}}") {
        return Err(WechatNotifyError::TemplateInvalid);
    }
    Ok(content)
}

fn summarize(content: &str) -> String {
    content.chars().take(80).collect()
}

fn normalize_channels(channels: Vec<String>) -> Vec<String> {
    let mut normalized: Vec<String> = channels
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect();
    if !normalized.iter().any(|value| value == "wechat") {
        normalized.push("wechat".to_string());
    }
    normalized.sort();
    normalized.dedup();
    normalized
}

fn map_db_error(error: sqlx::Error) -> WechatNotifyError {
    WechatNotifyError::Database(error.to_string())
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
