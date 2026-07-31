//! PostgreSQL-only 幂等锁、回放和结果保存。

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

const IDEMPOTENCY_TTL: Duration = Duration::hours(24);

#[derive(Debug)]
pub(crate) enum IdempotencyError {
    Conflict,
    Database(sqlx::Error),
    Serialize(String),
}

pub(crate) fn request_hash<T: Serialize>(value: &T) -> Result<String, IdempotencyError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| IdempotencyError::Serialize(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

pub(crate) async fn lock_key(
    tx: &mut Transaction<'_, Postgres>,
    namespace: &str,
    owner_id: Uuid,
    key: &str,
) -> Result<(), IdempotencyError> {
    let digest = Sha256::digest(format!("{namespace}\0{owner_id}\0{key}").as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(i64::from_be_bytes(bytes))
        .execute(&mut **tx)
        .await
        .map_err(IdempotencyError::Database)?;
    Ok(())
}

pub(crate) async fn replay<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, IdempotencyError> {
    let row: Option<(String, String, String, Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT request_hash, method, path, response_body, expires_at
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(IdempotencyError::Database)?;
    let Some((stored_hash, stored_method, stored_path, response_body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(key)
            .execute(&mut **tx)
            .await
            .map_err(IdempotencyError::Database)?;
        return Ok(None);
    }
    if stored_hash != request_hash || stored_method != method || stored_path != path {
        return Err(IdempotencyError::Conflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| IdempotencyError::Serialize(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn store_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: &str,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), IdempotencyError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| IdempotencyError::Serialize(error.to_string()))?;
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
    .bind(key)
    .bind(request_hash)
    .bind(method)
    .bind(path)
    .bind(response_body)
    .bind(resource_type)
    .bind(resource_id)
    .bind(now + IDEMPOTENCY_TTL)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(IdempotencyError::Database)?;
    Ok(())
}
