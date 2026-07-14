//! H1 API Key 到期提醒任务，提醒通过 H4 统一落库和发送。

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use wms_domain::SendH4NotificationRequest;

use crate::{
    auth::AuthContext,
    wechat_notify_service::{PgWechatNotifyService, WechatNotifyError},
};

const EXPIRY_EVENT: &str = "auth.api_key.expiring";
const EXPIRY_TEMPLATE: &str =
    "API Key {{caller_name}} 将于 {{expires_at}} 到期（Key ID {{key_id}}），请及时轮换。";

#[derive(Clone, Debug, FromRow)]
struct ExpiringApiKey {
    key_id: Uuid,
    owner_id: Uuid,
    responsible_user_id: Uuid,
    caller_name: String,
    expires_at: DateTime<Utc>,
}

pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            if let Err(error) = notify_expiring_api_keys(&pool, Utc::now()).await {
                tracing::error!(?error, "API Key 到期通知任务失败");
            }
        }
    });
}

pub async fn notify_expiring_api_keys(
    pool: &PgPool,
    now: DateTime<Utc>,
) -> Result<usize, ApiKeyExpiryError> {
    let keys = sqlx::query_as::<_, ExpiringApiKey>(
        r#"
        SELECT id AS key_id, owner_id, responsible_user_id, caller_name, expires_at
          FROM auth_api_keys
         WHERE status <> 'revoked'
           AND expires_at > $1
           AND expires_at <= $1 + INTERVAL '30 days'
         ORDER BY expires_at, id
        "#,
    )
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(|error| ApiKeyExpiryError::Database(error.to_string()))?;

    let service = PgWechatNotifyService::new();
    let mut notified = 0;
    for key in keys {
        ensure_default_config(pool, &key, now).await?;
        let context = AuthContext {
            user_id: key.responsible_user_id,
            owner_id: key.owner_id,
            actor_name: "H1 API Key 到期任务".to_string(),
            permissions: vec!["h4.notify.send".to_string()],
            jti: format!("api-key-expiry:{}", key.key_id),
        };
        let request = SendH4NotificationRequest {
            event_type: EXPIRY_EVENT.to_string(),
            dedupe_key: format!(
                "api-key-expiry:{}:{}",
                key.key_id,
                key.expires_at.date_naive()
            ),
            recipients: vec![key.responsible_user_id.to_string()],
            payload: serde_json::json!({
                "caller_name": key.caller_name,
                "expires_at": key.expires_at.to_rfc3339(),
                "key_id": key.key_id.to_string(),
            }),
        };
        service
            .send_notification(
                pool,
                &context,
                request,
                now,
                &format!(
                    "api-key-expiry:{}:{}",
                    key.key_id,
                    key.expires_at.date_naive()
                ),
            )
            .await
            .map_err(ApiKeyExpiryError::Notification)?;
        notified += 1;
    }
    Ok(notified)
}

async fn ensure_default_config(
    pool: &PgPool,
    key: &ExpiringApiKey,
    now: DateTime<Utc>,
) -> Result<(), ApiKeyExpiryError> {
    sqlx::query(
        r#"
        INSERT INTO h4_notification_configs (
            id, owner_id, event_type, enabled, template, recipient_rule,
            channels, created_by, updated_by, created_at, updated_at, version
        )
        VALUES ($1, $2, $3, TRUE, $4, jsonb_build_object('users', jsonb_build_array($5::TEXT)),
                ARRAY['wechat']::TEXT[], $5, $5, $6, $6, 1)
        ON CONFLICT (owner_id, event_type) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(key.owner_id)
    .bind(EXPIRY_EVENT)
    .bind(EXPIRY_TEMPLATE)
    .bind(key.responsible_user_id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|error| ApiKeyExpiryError::Database(error.to_string()))?;
    Ok(())
}

#[derive(Debug)]
pub enum ApiKeyExpiryError {
    Database(String),
    Notification(WechatNotifyError),
}
