use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::{db, ReconciliationError};

pub(super) async fn acquire_item_locks(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    item_id: Uuid,
    product_code: &str,
    batch_no: &str,
    now: DateTime<Utc>,
) -> Result<i64, ReconciliationError> {
    let batches: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT batch.id,
                CASE
                    WHEN batch.quality_status = 'qualified' THEN batch.quality_status
                    ELSE source_lock.previous_status
                END AS previous_status,
                batch.quality_status
           FROM inventory_batches batch
           LEFT JOIN LATERAL (
                SELECT item_lock.previous_status
                  FROM reconciliation_item_locks item_lock
                 WHERE item_lock.owner_id = batch.owner_id
                   AND item_lock.inventory_batch_id = batch.id
                   AND item_lock.released_at IS NULL
                 ORDER BY item_lock.locked_at, item_lock.item_id
                 LIMIT 1
           ) source_lock ON TRUE
          WHERE batch.owner_id = $1
            AND batch.product_code = $2
            AND batch.batch_no = $3
            AND NOT EXISTS (
                SELECT 1
                  FROM reconciliation_item_locks current_lock
                 WHERE current_lock.owner_id = batch.owner_id
                   AND current_lock.inventory_batch_id = batch.id
                   AND current_lock.item_id = $4
                   AND current_lock.released_at IS NULL
            )
            AND (
                batch.quality_status = 'qualified'
                OR (
                    batch.quality_status = 'quarantined'
                    AND source_lock.previous_status IS NOT NULL
                )
            )
          ORDER BY batch.id
          FOR UPDATE OF batch",
    )
    .bind(owner_id)
    .bind(product_code)
    .bind(batch_no)
    .bind(item_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;

    for (batch_id, previous_status, current_status) in &batches {
        if current_status == "qualified" {
            sqlx::query(
                "UPDATE inventory_batches
                    SET quality_status = 'quarantined', updated_at = $3, version = version + 1
                  WHERE owner_id = $1 AND id = $2 AND quality_status = 'qualified'",
            )
            .bind(owner_id)
            .bind(batch_id)
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "INSERT INTO inventory_status_changes
                 (id, owner_id, batch_id, from_status, to_status, reason,
                  approval_source, approval_id, occurred_at)
                 VALUES ($1,$2,$3,$4,'quarantined','对账差异主管选择隔离',
                         'reconciliation',$5,$6)",
            )
            .bind(Uuid::new_v4())
            .bind(owner_id)
            .bind(batch_id)
            .bind(previous_status)
            .bind(item_id.to_string())
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        }
        sqlx::query(
            "INSERT INTO reconciliation_item_locks
             (item_id, inventory_batch_id, owner_id, previous_status, locked_at)
             VALUES ($1,$2,$3,$4,$5)
             ON CONFLICT (item_id, inventory_batch_id) DO UPDATE
                 SET previous_status = EXCLUDED.previous_status,
                     locked_at = EXCLUDED.locked_at,
                     released_at = NULL",
        )
        .bind(item_id)
        .bind(batch_id)
        .bind(owner_id)
        .bind(previous_status)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    }
    Ok(batches.len() as i64)
}

pub(super) async fn release_item_locks(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    item_id: Uuid,
    now: DateTime<Utc>,
) -> Result<i64, ReconciliationError> {
    let locks: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT inventory_batch_id, previous_status
           FROM reconciliation_item_locks
          WHERE owner_id = $1 AND item_id = $2 AND released_at IS NULL
          ORDER BY inventory_batch_id
          FOR UPDATE",
    )
    .bind(owner_id)
    .bind(item_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    for (batch_id, previous_status) in &locks {
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT quality_status
               FROM inventory_batches
              WHERE owner_id = $1 AND id = $2
              FOR UPDATE",
        )
        .bind(owner_id)
        .bind(batch_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        if current_status.as_deref() != Some("quarantined") {
            return Err(ReconciliationError::InvalidRequest);
        }
        sqlx::query(
            "UPDATE reconciliation_item_locks
                SET released_at = $3
              WHERE owner_id = $1 AND item_id = $2
                AND inventory_batch_id = $4 AND released_at IS NULL",
        )
        .bind(owner_id)
        .bind(item_id)
        .bind(now)
        .bind(batch_id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;

        let has_other_active_lock: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1
                  FROM reconciliation_item_locks
                 WHERE owner_id = $1
                   AND inventory_batch_id = $2
                   AND released_at IS NULL
            )",
        )
        .bind(owner_id)
        .bind(batch_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        if !has_other_active_lock {
            let result = sqlx::query(
                "UPDATE inventory_batches
                    SET quality_status = $3, updated_at = $4, version = version + 1
                  WHERE owner_id = $1 AND id = $2 AND quality_status = 'quarantined'",
            )
            .bind(owner_id)
            .bind(batch_id)
            .bind(previous_status)
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
            if result.rows_affected() != 1 {
                return Err(ReconciliationError::InvalidRequest);
            }
            sqlx::query(
                "INSERT INTO inventory_status_changes
                 (id, owner_id, batch_id, from_status, to_status, reason,
                  approval_source, approval_id, occurred_at)
                 VALUES ($1,$2,$3,'quarantined',$4,'对账差异处理完成释放隔离',
                         'reconciliation',$5,$6)",
            )
            .bind(Uuid::new_v4())
            .bind(owner_id)
            .bind(batch_id)
            .bind(previous_status)
            .bind(item_id.to_string())
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        }
    }
    Ok(locks.len() as i64)
}
