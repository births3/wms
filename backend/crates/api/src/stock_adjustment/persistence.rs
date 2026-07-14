use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    DualPersonPolicy, StockAdjustmentSource, StockAdjustmentStatus, StockLossOrder, StockLossReason,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

use super::{StockAdjustmentError, StockLossOrderRow};

pub(super) async fn update_status_and_liaison(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
    status: StockAdjustmentStatus,
    quality_liaison_id: &str,
    now: DateTime<Utc>,
) -> Result<StockLossOrder, StockAdjustmentError> {
    let row = sqlx::query_as::<_, StockLossOrderRow>(
        &format!(
            "UPDATE stock_adjustment_orders SET status = $3, quality_liaison_id = $4, updated_at = $5, version = version + 1 WHERE owner_id = $1 AND id = $2 RETURNING {}",
            order_columns()
        ),
    )
    .bind(owner_id)
    .bind(order_id)
    .bind(status.as_str())
    .bind(quality_liaison_id)
    .bind(now)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)?;
    row_to_domain(row)
}

pub(super) async fn load_order_from_pool(
    pool: &PgPool,
    owner_id: Uuid,
    order_id: Uuid,
) -> Result<StockLossOrder, StockAdjustmentError> {
    let row = sqlx::query_as::<_, StockLossOrderRow>(&format!(
        "SELECT {} FROM stock_adjustment_orders WHERE owner_id = $1 AND id = $2",
        order_columns()
    ))
    .bind(owner_id)
    .bind(order_id)
    .fetch_optional(pool)
    .await
    .map_err(map_database_error)?;
    let row = match row {
        Some(row) => row,
        None => {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM stock_adjustment_orders WHERE id = $1)",
            )
            .bind(order_id)
            .fetch_one(pool)
            .await
            .map_err(map_database_error)?;
            return Err(if exists {
                StockAdjustmentError::CrossOwner
            } else {
                StockAdjustmentError::NotFound
            });
        }
    };
    row_to_domain(row)
}

pub(super) async fn load_order_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
) -> Result<StockLossOrder, StockAdjustmentError> {
    let row = sqlx::query_as::<_, StockLossOrderRow>(&format!(
        "SELECT {} FROM stock_adjustment_orders WHERE owner_id = $1 AND id = $2 FOR UPDATE",
        order_columns()
    ))
    .bind(owner_id)
    .bind(order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let row = match row {
        Some(row) => row,
        None => {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM stock_adjustment_orders WHERE id = $1)",
            )
            .bind(order_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_database_error)?;
            return Err(if exists {
                StockAdjustmentError::CrossOwner
            } else {
                StockAdjustmentError::NotFound
            });
        }
    };
    row_to_domain(row)
}

pub(super) fn order_columns() -> &'static str {
    "id, owner_id, warehouse_id, order_no, batch_id, product_code, batch_no, quantity, reason_code, recall_id, source, external_ref, status, requires_quality_approval, quality_liaison_id, policy, source_rule_id, first_operator_id, second_operator_id, approval_record_id, started_at, completed_at, created_at, updated_at"
}

pub(super) fn row_to_domain(
    row: StockLossOrderRow,
) -> Result<StockLossOrder, StockAdjustmentError> {
    Ok(StockLossOrder {
        id: row.id,
        owner_id: row.owner_id,
        warehouse_id: row.warehouse_id,
        order_no: row.order_no,
        batch_id: row.batch_id,
        product_code: row.product_code,
        batch_no: row.batch_no,
        quantity: row.quantity,
        reason: StockLossReason::try_from(row.reason_code.as_str())
            .map_err(|_| StockAdjustmentError::Database("非法报损原因".to_string()))?,
        recall_id: row.recall_id,
        source: StockAdjustmentSource::try_from(row.source.as_str())
            .map_err(|_| StockAdjustmentError::Database("非法报损来源".to_string()))?,
        external_ref: row.external_ref,
        status: StockAdjustmentStatus::try_from(row.status.as_str())
            .map_err(|_| StockAdjustmentError::Database("非法报损状态".to_string()))?,
        requires_quality_approval: row.requires_quality_approval,
        quality_liaison_id: row.quality_liaison_id,
        policy: row
            .policy
            .as_deref()
            .map(DualPersonPolicy::try_from)
            .transpose()
            .map_err(|_| StockAdjustmentError::Database("非法双人策略".to_string()))?,
        source_rule_id: row.source_rule_id,
        first_operator_id: row.first_operator_id,
        second_operator_id: row.second_operator_id,
        approval_record_id: row.approval_record_id,
        started_at: row.started_at,
        completed_at: row.completed_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub(super) async fn append_order_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    before: Option<&StockLossOrder>,
    after: &StockLossOrder,
    now: DateTime<Utc>,
) -> Result<(), StockAdjustmentError> {
    let before_value = before
        .map(json_value)
        .transpose()?
        .unwrap_or_else(|| serde_json::json!({}));
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M-SA",
        "stock_adjustment_order",
        after.id.to_string(),
        Some(AuditDiff::compute(before_value, json_value(after)?)),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| StockAdjustmentError::Audit(format!("{error:?}")))?;
    Ok(())
}

pub(super) async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
) -> Result<(), StockAdjustmentError> {
    let digest = Sha256::digest(format!("msa-stock-adjustment:{owner_id}:{key}").as_bytes());
    let lock_id = i64::from_be_bytes(
        digest[..8]
            .try_into()
            .map_err(|error| StockAdjustmentError::Serialize(format!("{error:?}")))?,
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
) -> Result<Option<T>, StockAdjustmentError> {
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
        return Err(StockAdjustmentError::IdempotencyConflict);
    }
    serde_json::from_value(body)
        .map(Some)
        .map_err(|error| StockAdjustmentError::Serialize(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    key: &str,
    hash: &str,
    method: &str,
    path: &str,
    resource_id: &str,
    order: &T,
    now: DateTime<Utc>,
) -> Result<(), StockAdjustmentError> {
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path, status_code,
            response_body, resource_type, resource_id, expires_at, created_at
        ) VALUES ($1,$2,$3,$4,$5,$6,200,$7,'stock_adjustment_order',$8,$9,$10)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(key)
    .bind(hash)
    .bind(method)
    .bind(path)
    .bind(json_value(order)?)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_database_error)?;
    Ok(())
}

pub(super) fn request_hash<T: Serialize>(value: &T) -> Result<String, StockAdjustmentError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| StockAdjustmentError::Serialize(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

pub(super) fn json_value<T: Serialize>(
    value: &T,
) -> Result<serde_json::Value, StockAdjustmentError> {
    serde_json::to_value(value).map_err(|error| StockAdjustmentError::Serialize(error.to_string()))
}

pub(super) fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let normalized = value.trim();
        (!normalized.is_empty()).then(|| normalized.to_string())
    })
}

pub(super) fn map_database_error(error: sqlx::Error) -> StockAdjustmentError {
    StockAdjustmentError::Database(error.to_string())
}
