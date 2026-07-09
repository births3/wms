//! H4 企业微信通知幂等与审计收尾工具。

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    wechat_notify_service::WechatNotifyError,
};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn finish_mutation<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
    action: &str,
    now: DateTime<Utc>,
) -> Result<(), WechatNotifyError> {
    store_idempotency_success(
        tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        resource_type,
        resource_id.clone(),
        response,
        now,
    )
    .await?;
    append_event_in_tx(
        tx,
        &AuditWriteRequest::from_auth_context(
            ctx,
            action,
            "H4",
            resource_type,
            resource_id,
            Some(AuditDiff::compute(
                serde_json::json!({}),
                serde_json::to_value(response)
                    .map_err(|error| WechatNotifyError::Serialize(error.to_string()))?,
            )),
        ),
    )
    .await
    .map(|_| ())
    .map_err(|error| WechatNotifyError::Audit(format!("{error:?}")))
}

pub(crate) async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), WechatNotifyError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), hashtext($2))")
        .bind(owner_id.to_string())
        .bind(idempotency_key)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

pub(crate) async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, WechatNotifyError> {
    let row: Option<(String, Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT request_hash, response_body, expires_at
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((stored_hash, response_body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        return Ok(None);
    }
    if stored_hash != request_hash {
        return Err(WechatNotifyError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| WechatNotifyError::Serialize(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), WechatNotifyError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| WechatNotifyError::Serialize(error.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 200, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(method)
    .bind(path)
    .bind(response_body)
    .bind(resource_type)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

pub(crate) fn json_request_hash<T: Serialize>(value: &T) -> Result<String, WechatNotifyError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| WechatNotifyError::Serialize(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn map_db_error(error: sqlx::Error) -> WechatNotifyError {
    WechatNotifyError::Database(error.to_string())
}
