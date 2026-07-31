//! Admin menu idempotency and audit helpers.

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    admin_menu_model::{map_db_error, AdminMenuError},
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    idempotency::{self, IdempotencyError},
    operation_context::OperationContext as AuthContext,
};

const IDEMPOTENCY_NAMESPACE: &str = "admin-menu";

impl From<IdempotencyError> for AdminMenuError {
    fn from(error: IdempotencyError) -> Self {
        match error {
            IdempotencyError::Conflict => Self::IdempotencyConflict,
            IdempotencyError::Database(error) => Self::Database(format!("{error:?}")),
            IdempotencyError::Serialize(error) => Self::Serialize(error),
        }
    }
}

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
    let response_body = serde_json::to_value(value)
        .map_err(|error| AdminMenuError::Serialize(error.to_string()))?;
    let resource_id = response_body
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(idempotency_key);
    idempotency::store_success(
        &mut tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        "admin_menu",
        resource_id,
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
    method: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, AdminMenuError> {
    idempotency::replay(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        now,
    )
    .await
    .map_err(Into::into)
}

pub(crate) async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), AdminMenuError> {
    idempotency::lock_key(tx, IDEMPOTENCY_NAMESPACE, owner_id, idempotency_key)
        .await
        .map_err(Into::into)
}

pub(crate) fn json_request_hash<T: Serialize>(value: &T) -> Result<String, AdminMenuError> {
    idempotency::request_hash(value).map_err(Into::into)
}
