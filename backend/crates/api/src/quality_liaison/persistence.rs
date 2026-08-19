use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    idempotency,
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
    idempotency::lock_key(tx, "quality-liaison", owner_id, key)
        .await
        .map_err(Into::into)
}

pub(super) async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    expected_hash: &str,
    method: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, QualityLiaisonError> {
    idempotency::replay(tx, owner_id, key, expected_hash, method, path, now)
        .await
        .map_err(Into::into)
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
    let resource_id_text = resource_id.to_string();
    idempotency::store_success(
        tx,
        ctx.owner_id,
        key,
        hash,
        method,
        path,
        resource_type,
        &resource_id_text,
        value,
        now,
    )
    .await
    .map_err(QualityLiaisonError::from)?;
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
    idempotency::request_hash(value).map_err(Into::into)
}

pub(super) fn map_database_error(error: sqlx::Error) -> QualityLiaisonError {
    QualityLiaisonError::Database(error.to_string())
}
