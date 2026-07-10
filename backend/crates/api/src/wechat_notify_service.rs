//! H4 企业微信通知与审批服务。

mod models;
mod validation;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    CreateH4ApprovalRequest, H4ApprovalCallbackRequest, H4ApprovalRecord, H4NotificationConfig,
    H4NotificationConfigListResponse, H4NotificationRecord, H4NotificationRecordListResponse,
    H4WechatSettings, H4WechatSettingsResponse, H4WechatSettingsTestResponse, PageMeta,
    SendH4NotificationRequest, UpsertH4NotificationConfigRequest, UpsertH4WechatSettingsRequest,
};

use crate::{
    auth::AuthContext,
    wechat_notify_idempotency::{
        finish_mutation, json_request_hash, lock_idempotency_key, replay_idempotency,
    },
};

use self::models::{ApprovalRow, ConfigRow, RecordRow, WechatSettingsRow};

pub const DEFAULT_H4_QUERY_LIMIT: u32 = 50;
pub const MAX_H4_QUERY_LIMIT: u32 = 100;
const PROVIDER_NOT_CONFIGURED_REASON: &str = "企业微信外部发送能力尚未启用";

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
    RecordNotFound,
    RecordNotResendable,
    WechatSettingsNotFound,
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
        validation::validate_config_request(&req)?;
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
        .bind(validation::normalize_channels(req.channels))
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
        validation::validate_wechat_settings_request(&req)?;
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

    pub async fn test_wechat_settings(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        now: DateTime<Utc>,
    ) -> Result<H4WechatSettingsTestResponse, WechatNotifyError> {
        let settings = self
            .get_wechat_settings(pool, ctx)
            .await?
            .data
            .ok_or(WechatNotifyError::WechatSettingsNotFound)?;
        validation::validate_wechat_settings_request(&UpsertH4WechatSettingsRequest {
            corp_id: settings.corp_id.clone(),
            agent_id: settings.agent_id.clone(),
            secret_alias: settings.secret_alias.clone(),
            callback_token_alias: settings.callback_token_alias.clone(),
            aes_key_alias: settings.aes_key_alias.clone(),
            callback_url: settings.callback_url.clone(),
            approval_callback_path: settings.approval_callback_path.clone(),
            enabled: settings.enabled,
            retry_max_attempts: settings.retry_max_attempts,
            retry_interval_seconds: settings.retry_interval_seconds,
        })?;
        let (status, message) = if settings.enabled {
            ("success", "企业微信参数校验通过")
        } else {
            ("warning", "企业微信参数已保存但未启用")
        };
        Ok(H4WechatSettingsTestResponse {
            status: status.to_string(),
            message: message.to_string(),
            checked_at: now,
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
        validation::validate_send_request(&req)?;
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
        let config =
            validation::load_enabled_config(&mut tx, ctx.owner_id, &req.event_type).await?;
        let content = validation::render_template(&config.template, &req.payload)?;
        let summary = validation::summarize(&content);
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
                        'failed', 0, $9, NULL, $10, $10)
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
            .bind(PROVIDER_NOT_CONFIGURED_REASON)
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
            "h4.notify.delivery_failed",
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
        mut req: CreateH4ApprovalRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<H4ApprovalRecord>, WechatNotifyError> {
        validation::validate_approval_request(&req)?;
        req.scenario = req.scenario.trim().to_string();
        req.business_ref = req.business_ref.trim().to_string();
        req.dedupe_key = req.dedupe_key.trim().to_string();
        req.approver_user = Uuid::parse_str(req.approver_user.trim())
            .map_err(|_| WechatNotifyError::InvalidRequest)?
            .to_string();
        req.process_id = req.process_id.trim().to_string();
        req.callback_path = req.callback_path.trim().to_string();
        req.summary = req.summary.trim().to_string();
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
        .bind(&req.scenario)
        .bind(&req.business_ref)
        .bind(&req.dedupe_key)
        .bind(&req.approver_user)
        .bind(&req.process_id)
        .bind(&req.callback_path)
        .bind(&req.summary)
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
        mut req: H4ApprovalCallbackRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<H4ApprovalRecord>, WechatNotifyError> {
        let status = match req.conclusion.trim() {
            "approved" | "同意" => "approved",
            "rejected" | "拒绝" => "rejected",
            _ => return Err(WechatNotifyError::InvalidStatus),
        };
        let approved_by_user_id = Uuid::parse_str(req.approved_by.trim())
            .map_err(|_| WechatNotifyError::InvalidRequest)?;
        if approved_by_user_id != ctx.user_id {
            return Err(WechatNotifyError::InvalidRequest);
        }
        req.conclusion = status.to_string();
        req.approved_by = approved_by_user_id.to_string();
        req.external_approval_id = req
            .external_approval_id
            .map(|value| value.trim().to_string());
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
        let current = sqlx::query_as::<_, ApprovalRow>(
            r#"
            SELECT id, owner_id, scenario, business_ref, dedupe_key, approver_user,
                   process_id, callback_path, summary, status, opinion,
                   external_approval_id, approved_by, approved_at, failure_reason,
                   created_at, updated_at
              FROM h4_approval_records
             WHERE owner_id = $1 AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(approval_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(WechatNotifyError::ApprovalNotFound)?;
        let approved_by = req.approved_by.clone();
        let external_approval_id = req
            .external_approval_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(WechatNotifyError::InvalidRequest)?;
        if approved_by != current.approver_user {
            return Err(WechatNotifyError::InvalidRequest);
        }
        if current.status != "pending" {
            if current.status != status {
                return Err(WechatNotifyError::IdempotencyConflict);
            }
            if current.approved_by.as_deref() != Some(approved_by.as_str())
                || current.external_approval_id.as_deref() != Some(external_approval_id)
            {
                return Err(WechatNotifyError::InvalidRequest);
            }
            tx.commit().await.map_err(map_db_error)?;
            return Ok(IdempotentMutation {
                value: current.into(),
                replayed: true,
            });
        }
        let row = sqlx::query_as::<_, ApprovalRow>(
            r#"
            UPDATE h4_approval_records
               SET status = $3,
                   opinion = $4,
                   external_approval_id = $5,
                   approved_by = $6,
                   approved_at = $7,
                   updated_at = $7
             WHERE owner_id = $1 AND id = $2 AND status = 'pending'
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
        .bind(external_approval_id)
        .bind(&approved_by)
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
        let can_read_all = is_system_admin(pool, ctx).await?;
        let user_id = ctx.user_id.to_string();
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
               AND ($7::BOOLEAN OR recipient = $8 OR recipient = $9)
             ORDER BY created_at DESC, id DESC
             LIMIT $10
            "#,
        )
        .bind(ctx.owner_id)
        .bind(query.event_type)
        .bind(query.recipient)
        .bind(query.status)
        .bind(query.from)
        .bind(query.to)
        .bind(can_read_all)
        .bind(&ctx.actor_name)
        .bind(user_id)
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
        let status: String = sqlx::query_scalar(
            "SELECT status FROM h4_notification_records WHERE owner_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(record_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(WechatNotifyError::RecordNotFound)?;
        if !matches!(status.as_str(), "failed" | "retrying") {
            return Err(WechatNotifyError::RecordNotResendable);
        }
        let row = sqlx::query_as::<_, RecordRow>(
            r#"
            UPDATE h4_notification_records
               SET status = 'failed',
                   retry_count = retry_count + 1,
                   failure_reason = $3,
                   sent_at = NULL,
                   updated_at = $4
             WHERE owner_id = $1 AND id = $2
             RETURNING id, owner_id, config_id, event_type, dedupe_key, recipient,
                       channel, content_summary, status, retry_count, failure_reason,
                       sent_at, created_at, updated_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(record_id)
        .bind(PROVIDER_NOT_CONFIGURED_REASON)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(WechatNotifyError::RecordNotFound)?;
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

async fn is_system_admin(pool: &PgPool, ctx: &AuthContext) -> Result<bool, WechatNotifyError> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM auth_user_roles user_role
              JOIN auth_roles role ON role.id = user_role.role_id
             WHERE user_role.user_id = $1
               AND user_role.owner_id = $2
               AND role.owner_id = $2
               AND lower(role.role_code) = 'system_admin'
        )
        "#,
    )
    .bind(ctx.user_id)
    .bind(ctx.owner_id)
    .fetch_one(pool)
    .await
    .map_err(map_db_error)
}

fn map_db_error(error: sqlx::Error) -> WechatNotifyError {
    WechatNotifyError::Database(error.to_string())
}
