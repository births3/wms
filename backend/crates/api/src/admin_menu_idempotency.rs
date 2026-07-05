//! Admin menu idempotency and audit helpers.

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    admin_menu_model::{map_db_error, AdminMenuError},
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

pub(crate) async fn finish_mutation<T: Serialize>(
    mut tx: Transaction<'_, Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    value: &T,
    action: &str,
    now: DateTime<Utc>,
) -> Result<(), AdminMenuError> {
    store_idempotency_success(
        &mut tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        value,
        now,
    )
    .await?;
    append_event_in_tx(
        &mut tx,
        &AuditWriteRequest {
            occurred_at: now,
            actor_id: ctx.user_id,
            actor_name: ctx.actor_name.clone(),
            owner_id: ctx.owner_id,
            jti: ctx.jti.clone(),
            action: action.to_string(),
            module: "H1".to_string(),
            resource_type: "admin_menu".to_string(),
            resource_id: idempotency_key.to_string(),
            diff: Some(AuditDiff::compute(
                serde_json::json!({}),
                serde_json::to_value(value)
                    .map_err(|e| AdminMenuError::Serialize(e.to_string()))?,
            )),
            request_id: None,
            ip: None,
            user_agent: None,
        },
    )
    .await
    .map_err(|error| AdminMenuError::Audit(format!("{error:?}")))?;
    tx.commit().await.map_err(map_db_error)
}

pub(crate) async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
) -> Result<Option<T>, AdminMenuError> {
    let existing: Option<(String, serde_json::Value)> = sqlx::query_as(
        r#"
        SELECT request_hash, response_body
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((existing_hash, response_body)) = existing else {
        return Ok(None);
    };
    if existing_hash != request_hash {
        return Err(AdminMenuError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|e| AdminMenuError::Serialize(e.to_string()))
}

pub(crate) async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), AdminMenuError> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(idempotency_lock_id(owner_id, idempotency_key))
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

pub(crate) fn json_request_hash<T: Serialize>(value: &T) -> Result<String, AdminMenuError> {
    let bytes = serde_json::to_vec(value).map_err(|e| AdminMenuError::Serialize(e.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    value: &T,
    now: DateTime<Utc>,
) -> Result<(), AdminMenuError> {
    let response_body =
        serde_json::to_value(value).map_err(|e| AdminMenuError::Serialize(e.to_string()))?;
    let resource_id = response_body
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(idempotency_key)
        .to_string();
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 200, $7, $8, $9, $10, $11)
        ON CONFLICT (owner_id, idempotency_key)
        DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(method)
    .bind(path)
    .bind(response_body)
    .bind("admin_menu")
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

fn idempotency_lock_id(owner_id: Uuid, idempotency_key: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(owner_id.as_bytes());
    hasher.update(idempotency_key.as_bytes());
    let digest = hasher.finalize();
    i64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}
