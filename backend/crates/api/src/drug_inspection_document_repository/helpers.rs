use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::DrugInspectionDocumentRepositoryError;
use crate::idempotency;

impl From<crate::idempotency::IdempotencyError> for DrugInspectionDocumentRepositoryError {
    fn from(error: crate::idempotency::IdempotencyError) -> Self {
        match error {
            crate::idempotency::IdempotencyError::Conflict => Self::IdempotencyConflict,
            crate::idempotency::IdempotencyError::Database(error) => {
                Self::Database(error.to_string())
            }
            crate::idempotency::IdempotencyError::Serialize(error) => Self::Serialize(error),
        }
    }
}

pub(crate) async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), DrugInspectionDocumentRepositoryError> {
    idempotency::lock_key(tx, "drug-inspection-document", owner_id, idempotency_key)
        .await
        .map_err(Into::into)
}

pub(crate) async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, DrugInspectionDocumentRepositoryError> {
    idempotency::replay_hash_only(tx, owner_id, idempotency_key, request_hash, now)
        .await
        .map_err(Into::into)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn store_idempotency<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: Uuid,
    value: &T,
    now: DateTime<Utc>,
) -> Result<(), DrugInspectionDocumentRepositoryError> {
    let resource_id = resource_id.to_string();
    idempotency::store_success(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
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

pub(crate) fn request_hash<T: Serialize>(
    value: &T,
) -> Result<String, DrugInspectionDocumentRepositoryError> {
    idempotency::request_hash(value).map_err(Into::into)
}
