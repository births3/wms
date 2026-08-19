use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::idempotency;

use super::ReconciliationError;

pub(crate) fn request_hash<T: Serialize>(value: &T) -> Result<String, ReconciliationError> {
    idempotency::request_hash(value).map_err(Into::into)
}

pub(crate) async fn lock_idempotency(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
) -> Result<(), ReconciliationError> {
    idempotency::lock_key(tx, "reconciliation-idempotency", owner_id, key)
        .await
        .map_err(Into::into)
}

pub(crate) async fn lock_reconciliation_window(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    window_key: &str,
) -> Result<(), ReconciliationError> {
    idempotency::lock_key(tx, "reconciliation-window", owner_id, window_key)
        .await
        .map_err(Into::into)
}

pub(crate) async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    hash: &str,
    method: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, ReconciliationError> {
    idempotency::replay(tx, owner_id, key, hash, method, path, now)
        .await
        .map_err(Into::into)
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
    idempotency::store_success(
        tx,
        owner_id,
        key,
        hash,
        method,
        path,
        resource_type,
        &resource_id,
        value,
        now,
    )
    .await
    .map_err(Into::into)
}

pub(crate) fn db(error: sqlx::Error) -> ReconciliationError {
    ReconciliationError::Database(error.to_string())
}
