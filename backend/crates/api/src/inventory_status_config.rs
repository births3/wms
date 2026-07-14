use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    InventoryStatusTransition, InventoryStatusTransitionListResponse,
    UpsertInventoryStatusTransitionRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

const STATUS_DICTIONARY: &str = "inventory_quality_status";
const STATUS_TRANSITION_PATH: &str = "/api/v1/inventory/status-transitions";

#[derive(Clone, Debug)]
pub struct PgInventoryStatusConfigRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InventoryStatusConfigError {
    CrossOwnerAccess,
    InvalidStatus,
    InvalidTransition,
    InvalidApprovalSources,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, FromRow)]
struct InventoryStatusTransitionRow {
    id: Uuid,
    owner_id: Option<Uuid>,
    from_status: String,
    to_status: String,
    approval_sources: Vec<String>,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<InventoryStatusTransitionRow> for InventoryStatusTransition {
    fn from(row: InventoryStatusTransitionRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            from_status: row.from_status,
            to_status: row.to_status,
            approval_sources: row.approval_sources,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl PgInventoryStatusConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_effective(
        &self,
        ctx: &AuthContext,
    ) -> Result<InventoryStatusTransitionListResponse, InventoryStatusConfigError> {
        let rows = sqlx::query_as::<_, InventoryStatusTransitionRow>(
            r#"
            WITH scoped_transitions AS (
                SELECT id, owner_id, from_status, to_status, approval_sources,
                       enabled, created_at, updated_at,
                       ROW_NUMBER() OVER (
                           PARTITION BY from_status, to_status
                           ORDER BY CASE WHEN owner_id = $1 THEN 1 ELSE 0 END DESC,
                                    updated_at DESC,
                                    id DESC
                       ) AS scope_rank
                  FROM inventory_status_transitions
                 WHERE owner_id IS NULL OR owner_id = $1
            )
            SELECT id, owner_id, from_status, to_status, approval_sources,
                   enabled, created_at, updated_at
              FROM scoped_transitions
             WHERE scope_rank = 1
             ORDER BY from_status, to_status
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let data = rows.into_iter().map(Into::into).collect::<Vec<_>>();
        Ok(InventoryStatusTransitionListResponse {
            page: wms_domain::PageMeta {
                next_cursor: None,
                count: data.len() as u32,
            },
            data,
        })
    }

    pub async fn upsert(
        &self,
        ctx: &AuthContext,
        from_status: &str,
        to_status: &str,
        req: UpsertInventoryStatusTransitionRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<InventoryStatusTransition>, InventoryStatusConfigError> {
        let from_status = from_status.trim();
        let to_status = to_status.trim();
        if from_status.is_empty() || to_status.is_empty() || from_status == to_status {
            return Err(InventoryStatusConfigError::InvalidTransition);
        }
        if req
            .owner_id
            .is_some_and(|owner_id| owner_id != ctx.owner_id)
        {
            return Err(InventoryStatusConfigError::CrossOwnerAccess);
        }
        let approval_sources = normalize_approval_sources(req.approval_sources.clone())?;
        let request_hash = request_hash(&json!({
            "from_status": from_status,
            "to_status": to_status,
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }

        let statuses_are_enabled =
            effective_status_enabled(&mut tx, ctx.owner_id, from_status, now).await?
                && effective_status_enabled(&mut tx, ctx.owner_id, to_status, now).await?;
        if !statuses_are_enabled {
            return Err(InventoryStatusConfigError::InvalidStatus);
        }

        let existing = sqlx::query_as::<_, InventoryStatusTransitionRow>(
            r#"
            SELECT id, owner_id, from_status, to_status, approval_sources,
                   enabled, created_at, updated_at
              FROM inventory_status_transitions
             WHERE owner_id IS NOT DISTINCT FROM $1
               AND from_status = $2
               AND to_status = $3
             FOR UPDATE
            "#,
        )
        .bind(req.owner_id)
        .bind(from_status)
        .bind(to_status)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let before = existing.clone().map(InventoryStatusTransition::from);
        let row = if let Some(existing) = existing {
            sqlx::query_as::<_, InventoryStatusTransitionRow>(
                r#"
                UPDATE inventory_status_transitions
                   SET approval_sources = $1,
                       enabled = $2,
                       updated_at = $3
                 WHERE id = $4
                RETURNING id, owner_id, from_status, to_status, approval_sources,
                          enabled, created_at, updated_at
                "#,
            )
            .bind(&approval_sources)
            .bind(req.enabled)
            .bind(now)
            .bind(existing.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?
        } else {
            sqlx::query_as::<_, InventoryStatusTransitionRow>(
                r#"
                INSERT INTO inventory_status_transitions (
                    id, owner_id, from_status, to_status, approval_sources,
                    enabled, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
                RETURNING id, owner_id, from_status, to_status, approval_sources,
                          enabled, created_at, updated_at
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(req.owner_id)
            .bind(from_status)
            .bind(to_status)
            .bind(&approval_sources)
            .bind(req.enabled)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?
        };
        let transition = InventoryStatusTransition::from(row);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PUT",
            &format!("{STATUS_TRANSITION_PATH}/{from_status}/{to_status}"),
            "inventory_status_transition",
            transition.id.to_string(),
            &transition,
            now,
        )
        .await?;

        let before = before
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| InventoryStatusConfigError::Serialize(error.to_string()))?
            .unwrap_or(Value::Null);
        let after = serde_json::to_value(&transition)
            .map_err(|error| InventoryStatusConfigError::Serialize(error.to_string()))?;
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            "upsert_inventory_status_transition",
            "M3",
            "inventory_status_transition",
            transition.id.to_string(),
            Some(AuditDiff::compute(before, after)),
        );
        audit.occurred_at = now;
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| InventoryStatusConfigError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: transition,
            replayed: false,
        })
    }
}

pub(crate) async fn is_transition_allowed_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    from_status: &str,
    to_status: &str,
    approval_source: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT enabled AND $4 = ANY(approval_sources)
          FROM inventory_status_transitions
         WHERE (owner_id IS NULL OR owner_id = $1)
           AND from_status = $2
           AND to_status = $3
         ORDER BY CASE WHEN owner_id = $1 THEN 1 ELSE 0 END DESC,
                  updated_at DESC,
                  id DESC
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .bind(from_status)
    .bind(to_status)
    .bind(approval_source.trim())
    .fetch_optional(&mut **tx)
    .await
    .map(|value| value.unwrap_or(false))
}

async fn effective_status_enabled(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    status: &str,
    now: DateTime<Utc>,
) -> Result<bool, InventoryStatusConfigError> {
    crate::system_dictionary::effective_item_enabled_in_tx(
        tx,
        owner_id,
        STATUS_DICTIONARY,
        status,
        now,
    )
    .await
    .map_err(map_db_error)
}

fn normalize_approval_sources(
    sources: Vec<String>,
) -> Result<Vec<String>, InventoryStatusConfigError> {
    let mut normalized = sources
        .into_iter()
        .map(|source| source.trim().to_string())
        .filter(|source| !source.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    if normalized.is_empty() {
        return Err(InventoryStatusConfigError::InvalidApprovalSources);
    }
    Ok(normalized)
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, InventoryStatusConfigError> {
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
        return Err(InventoryStatusConfigError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| InventoryStatusConfigError::Serialize(error.to_string()))
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), InventoryStatusConfigError> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(idempotency_lock_id(owner_id, idempotency_key))
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
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
) -> Result<(), InventoryStatusConfigError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| InventoryStatusConfigError::Serialize(error.to_string()))?;
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

fn request_hash(value: &Value) -> Result<String, InventoryStatusConfigError> {
    let text = serde_json::to_string(value)
        .map_err(|error| InventoryStatusConfigError::Serialize(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn idempotency_lock_id(owner_id: Uuid, idempotency_key: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(owner_id.as_bytes());
    hasher.update([0]);
    hasher.update(idempotency_key.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

fn map_db_error(error: sqlx::Error) -> InventoryStatusConfigError {
    InventoryStatusConfigError::Database(error.to_string())
}
