use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{H4NotificationRecord, SendH4NotificationRequest};

use crate::{
    operation_context::OperationContext as AuthContext,
    wechat_notify_idempotency::{
        append_mutation_audit, finish_mutation, json_request_hash, lock_idempotency_key,
        replay_idempotency, update_idempotency_response,
    },
};

use super::{
    models::{RecordRow, WechatSettingsRow},
    validation, IdempotentMutation, PgWechatNotifyService, UnconfiguredWechatProvider,
    WechatNotifyError, WechatProvider, WechatProviderError, WechatProviderRequest,
    PROVIDER_NOT_CONFIGURED_REASON,
};

impl PgWechatNotifyService {
    pub async fn send_notification(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: SendH4NotificationRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<Vec<H4NotificationRecord>>, WechatNotifyError> {
        self.send_notification_with_provider(
            pool,
            ctx,
            req,
            now,
            idempotency_key,
            &UnconfiguredWechatProvider,
        )
        .await
    }

    pub async fn send_notification_with_provider(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: SendH4NotificationRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        provider: &dyn WechatProvider,
    ) -> Result<IdempotentMutation<Vec<H4NotificationRecord>>, WechatNotifyError> {
        validation::validate_send_request(&req)?;
        let request_hash = json_request_hash(&req)?;
        let mut tx = pool.begin().await.map_err(super::map_db_error)?;
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
        let settings = load_wechat_settings(&mut tx, ctx.owner_id).await?;
        let retry_max_attempts = settings
            .as_ref()
            .map(|value| value.retry_max_attempts)
            .unwrap_or_default();
        let mut records = Vec::new();
        let mut pending = Vec::new();
        for recipient in req
            .recipients
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            let row = sqlx::query_as::<_, RecordRow>(
                r#"
                INSERT INTO h4_notification_records (
                    id, owner_id, config_id, event_type, dedupe_key, recipient, channel,
                    content, content_summary, status, retry_count, failure_reason,
                    sent_at, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, 'wechat', $7, $8,
                        'retrying', 0, NULL, NULL, $9, $9)
                ON CONFLICT (owner_id, event_type, recipient, dedupe_key)
                DO NOTHING
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
            .fetch_optional(&mut *tx)
            .await
            .map_err(super::map_db_error)?;
            let (row, should_send) = match row {
                Some(row) => (row, true),
                None => {
                    let existing = sqlx::query_as::<_, RecordRow>(
                        r#"
                        SELECT id, owner_id, config_id, event_type, dedupe_key, recipient,
                               channel, content_summary, status, retry_count, failure_reason,
                               sent_at, created_at, updated_at
                          FROM h4_notification_records
                         WHERE owner_id = $1 AND event_type = $2
                           AND recipient = $3 AND dedupe_key = $4
                        "#,
                    )
                    .bind(ctx.owner_id)
                    .bind(&req.event_type)
                    .bind(recipient)
                    .bind(&req.dedupe_key)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(super::map_db_error)?;
                    (existing, false)
                }
            };
            let record: H4NotificationRecord = row.into();
            if should_send {
                pending.push((record.id, recipient.to_string()));
            }
            records.push(record);
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
            "h4.notify.delivery_queued",
            now,
        )
        .await?;
        tx.commit().await.map_err(super::map_db_error)?;

        let mut final_records = records;
        for (record_id, recipient) in pending {
            let outcome = if settings.as_ref().is_some_and(|value| value.enabled) {
                provider
                    .send(WechatProviderRequest {
                        corp_id: settings.as_ref().map(|value| value.corp_id.clone()),
                        agent_id: settings.as_ref().map(|value| value.agent_id.clone()),
                        secret_alias: settings.as_ref().map(|value| value.secret_alias.clone()),
                        recipient,
                        content: content.clone(),
                    })
                    .await
            } else {
                Err(WechatProviderError::NotConfigured(
                    PROVIDER_NOT_CONFIGURED_REASON.to_string(),
                ))
            };
            let record =
                finalize_delivery(pool, ctx, record_id, outcome, retry_max_attempts, now).await?;
            if let Some(existing) = final_records.iter_mut().find(|value| value.id == record.id) {
                *existing = record;
            }
        }

        let mut tx = pool.begin().await.map_err(super::map_db_error)?;
        update_idempotency_response(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &final_records,
        )
        .await?;
        tx.commit().await.map_err(super::map_db_error)?;
        Ok(IdempotentMutation {
            value: final_records,
            replayed: false,
        })
    }
}

async fn load_wechat_settings(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    owner_id: Uuid,
) -> Result<Option<WechatSettingsRow>, WechatNotifyError> {
    sqlx::query_as::<_, WechatSettingsRow>(
        r#"
        SELECT id, owner_id, corp_id, agent_id, secret_alias, callback_token_alias,
               aes_key_alias, callback_url, approval_callback_path, enabled,
               retry_max_attempts, retry_interval_seconds, created_at, updated_at, version
          FROM h4_wechat_settings
         WHERE owner_id = $1
        "#,
    )
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(super::map_db_error)
}

async fn finalize_delivery(
    pool: &PgPool,
    ctx: &AuthContext,
    record_id: Uuid,
    outcome: Result<(), WechatProviderError>,
    retry_max_attempts: i32,
    now: DateTime<Utc>,
) -> Result<H4NotificationRecord, WechatNotifyError> {
    let (status, failure_reason, sent_at) = match outcome {
        Ok(()) => ("success", None, Some(now)),
        Err(WechatProviderError::Retryable(reason)) if retry_max_attempts > 0 => {
            ("retrying", Some(safe_failure_reason(reason)), None)
        }
        Err(WechatProviderError::NotConfigured(reason)) => {
            ("failed", Some(safe_failure_reason(reason)), None)
        }
        Err(WechatProviderError::Permanent(reason)) => {
            ("failed", Some(safe_failure_reason(reason)), None)
        }
        Err(WechatProviderError::Retryable(reason)) => {
            ("failed", Some(safe_failure_reason(reason)), None)
        }
    };
    let action = match status {
        "success" => "h4.notify.delivery_succeeded",
        "retrying" => "h4.notify.delivery_retrying",
        _ => "h4.notify.delivery_failed",
    };
    let mut tx = pool.begin().await.map_err(super::map_db_error)?;
    let row = sqlx::query_as::<_, RecordRow>(
        r#"
        UPDATE h4_notification_records
           SET status = $3,
               failure_reason = $4,
               sent_at = $5,
               updated_at = $6
         WHERE owner_id = $1 AND id = $2
         RETURNING id, owner_id, config_id, event_type, dedupe_key, recipient,
                   channel, content_summary, status, retry_count, failure_reason,
                   sent_at, created_at, updated_at
        "#,
    )
    .bind(ctx.owner_id)
    .bind(record_id)
    .bind(status)
    .bind(failure_reason)
    .bind(sent_at)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await
    .map_err(super::map_db_error)?
    .ok_or(WechatNotifyError::RecordNotFound)?;
    let value: H4NotificationRecord = row.into();
    append_mutation_audit(
        &mut tx,
        ctx,
        action,
        "h4_notification_record",
        value.id.to_string(),
        &value,
    )
    .await?;
    tx.commit().await.map_err(super::map_db_error)?;
    Ok(value)
}

fn safe_failure_reason(reason: String) -> String {
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        return PROVIDER_NOT_CONFIGURED_REASON.to_string();
    }
    trimmed.chars().take(240).collect()
}
