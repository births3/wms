use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    operation_context::OperationContext as AuthContext,
};

use super::{QualityLiaisonError, QualityLiaisonOrderRow};

pub(super) fn order_columns() -> &'static str {
    "id, owner_id, liaison_no, type_code, related_document_type, related_document_no, problem_description, disposition_suggestion, trigger_source, business_payload, status, approval_record_id, approved_by, approval_opinion, approved_at, created_by, created_at, updated_at, version"
}

pub(super) async fn load_order_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
) -> Result<QualityLiaisonOrderRow, QualityLiaisonError> {
    sqlx::query_as::<_, QualityLiaisonOrderRow>(&format!(
        "SELECT {} FROM quality_liaison_orders WHERE owner_id = $1 AND id = $2 FOR UPDATE",
        order_columns()
    ))
    .bind(owner_id)
    .bind(order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?
    .ok_or(QualityLiaisonError::NotFound)
}

pub(super) async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
) -> Result<(), QualityLiaisonError> {
    let digest = Sha256::digest(format!("mql-quality-liaison:{owner_id}:{key}").as_bytes());
    let lock_id = i64::from_be_bytes(
        digest[..8]
            .try_into()
            .map_err(|error| QualityLiaisonError::Serialize(format!("{error:?}")))?,
    );
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(lock_id)
        .execute(&mut **tx)
        .await
        .map_err(map_database_error)?;
    Ok(())
}

pub(super) async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    expected_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, QualityLiaisonError> {
    let row: Option<(String, serde_json::Value, DateTime<Utc>)> = sqlx::query_as(
        "SELECT request_hash, response_body, expires_at FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2 FOR UPDATE",
    )
    .bind(owner_id)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let Some((stored_hash, body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(key)
            .execute(&mut **tx)
            .await
            .map_err(map_database_error)?;
        return Ok(None);
    }
    if stored_hash != expected_hash {
        return Err(QualityLiaisonError::IdempotencyConflict);
    }
    serde_json::from_value(body)
        .map(Some)
        .map_err(|error| QualityLiaisonError::Serialize(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn finish_mutation<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    key: &str,
    hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: Uuid,
    value: &T,
    action: &str,
    now: DateTime<Utc>,
) -> Result<(), QualityLiaisonError> {
    let body = serde_json::to_value(value)
        .map_err(|error| QualityLiaisonError::Serialize(error.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path, status_code,
            response_body, resource_type, resource_id, expires_at, created_at
        ) VALUES ($1,$2,$3,$4,$5,$6,200,$7,$8,$9,$10,$11)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(key)
    .bind(hash)
    .bind(method)
    .bind(path)
    .bind(&body)
    .bind(resource_type)
    .bind(resource_id.to_string())
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M-QL",
        resource_type,
        resource_id.to_string(),
        Some(AuditDiff::compute(serde_json::json!({}), body)),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| QualityLiaisonError::Audit(format!("{error:?}")))?;
    Ok(())
}

pub(super) fn request_hash<T: Serialize>(value: &T) -> Result<String, QualityLiaisonError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| QualityLiaisonError::Serialize(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

pub(super) fn map_database_error(error: sqlx::Error) -> QualityLiaisonError {
    QualityLiaisonError::Database(error.to_string())
}
