use chrono::{DateTime, Utc};
use sqlx::{query_as, FromRow};
use uuid::Uuid;
use wms_domain::{InventoryBatchTrace, InventoryMovement, InventoryStatusChange};

use super::{map_inventory_batch, InventoryBatchRow, PgWave3Repository, Wave3RepositoryError};
use crate::auth::AuthContext;

#[derive(Clone, FromRow)]
struct InventoryMovementRow {
    id: Uuid,
    owner_id: Uuid,
    batch_id: Uuid,
    movement_type: String,
    qty_delta: i64,
    source_document_type: String,
    source_document_id: Uuid,
    occurred_at: DateTime<Utc>,
}

#[derive(Clone, FromRow)]
struct InventoryStatusChangeRow {
    id: Uuid,
    owner_id: Uuid,
    batch_id: Uuid,
    from_status: String,
    to_status: String,
    reason: String,
    approval_source: String,
    approval_id: String,
    occurred_at: DateTime<Utc>,
}

impl PgWave3Repository {
    pub async fn get_inventory_batch_trace(
        &self,
        ctx: &AuthContext,
        batch_id: Uuid,
    ) -> Result<InventoryBatchTrace, Wave3RepositoryError> {
        let batch = query_as::<_, InventoryBatchRow>(
            r#"
            SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                   qty_on_hand, qty_locked, quality_status, location_id, location_code,
                   recall_flag, created_at, updated_at
              FROM inventory_batches
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(batch_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(super::map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;

        let movements = query_as::<_, InventoryMovementRow>(
            r#"
            SELECT id, owner_id, batch_id, movement_type, qty_delta,
                   source_document_type, source_document_id, occurred_at
              FROM inventory_movements
             WHERE owner_id = $1 AND batch_id = $2
             ORDER BY occurred_at ASC, id ASC
            "#,
        )
        .bind(ctx.owner_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .map_err(super::map_db_error)?
        .into_iter()
        .map(|row| InventoryMovement {
            id: row.id,
            owner_id: row.owner_id,
            batch_id: row.batch_id,
            movement_type: row.movement_type,
            qty_delta: row.qty_delta,
            source_document_type: row.source_document_type,
            source_document_id: row.source_document_id,
            occurred_at: row.occurred_at,
        })
        .collect();

        let status_changes = query_as::<_, InventoryStatusChangeRow>(
            r#"
            SELECT id, owner_id, batch_id, from_status, to_status,
                   reason, approval_source, approval_id, occurred_at
              FROM inventory_status_changes
             WHERE owner_id = $1 AND batch_id = $2
             ORDER BY occurred_at ASC, id ASC
            "#,
        )
        .bind(ctx.owner_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .map_err(super::map_db_error)?
        .into_iter()
        .map(|row| InventoryStatusChange {
            id: row.id,
            owner_id: row.owner_id,
            batch_id: row.batch_id,
            from_status: row.from_status,
            to_status: row.to_status,
            reason: row.reason,
            approval_source: row.approval_source,
            approval_id: row.approval_id,
            occurred_at: row.occurred_at,
        })
        .collect();

        Ok(InventoryBatchTrace {
            batch: map_inventory_batch(batch),
            movements,
            status_changes,
        })
    }
}
