use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::ReconciliationError;

pub(crate) fn request_hash<T: Serialize>(value: &T) -> Result<String, ReconciliationError> {
    let text =
        serde_json::to_string(value).map_err(|e| ReconciliationError::Serialize(e.to_string()))?;
    Ok(hex::encode(Sha256::digest(text.as_bytes())))
}

pub(crate) async fn lock_idempotency(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
) -> Result<(), ReconciliationError> {
    let digest = Sha256::digest(format!("{owner_id}\0{key}").as_bytes());
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&digest[..8]);
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(i64::from_be_bytes(bytes))
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
    Ok(())
}

pub(crate) async fn lock_reconciliation_window(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    window_key: &str,
) -> Result<(), ReconciliationError> {
    let digest =
        Sha256::digest(format!("reconciliation-window\0{owner_id}\0{window_key}").as_bytes());
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&digest[..8]);
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(i64::from_be_bytes(bytes))
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
    Ok(())
}

pub(crate) async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, ReconciliationError> {
    let row: Option<(String, serde_json::Value, DateTime<Utc>)> = sqlx::query_as(
        "SELECT request_hash, response_body, expires_at FROM idempotency_request
          WHERE owner_id = $1 AND idempotency_key = $2 FOR UPDATE",
    )
    .bind(owner_id)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    let Some((stored_hash, body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(key)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        return Ok(None);
    }
    if stored_hash != hash {
        return Err(ReconciliationError::IdempotencyConflict);
    }
    serde_json::from_value(body)
        .map(Some)
        .map_err(|e| ReconciliationError::Serialize(e.to_string()))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn store_idempotency<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    value: &T,
    now: DateTime<Utc>,
) -> Result<(), ReconciliationError> {
    sqlx::query(
        "INSERT INTO idempotency_request
         (id, owner_id, idempotency_key, request_hash, method, path, status_code,
          response_body, resource_type, resource_id, expires_at, created_at)
         VALUES ($1,$2,$3,$4,$5,$6,200,$7,$8,$9,$10,$11)",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(key)
    .bind(hash)
    .bind(method)
    .bind(path)
    .bind(serde_json::to_value(value).map_err(|e| ReconciliationError::Serialize(e.to_string()))?)
    .bind(resource_type)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}

pub(crate) fn db(error: sqlx::Error) -> ReconciliationError {
    ReconciliationError::Database(error.to_string())
}
