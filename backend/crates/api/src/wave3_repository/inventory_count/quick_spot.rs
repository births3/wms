use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;
use wms_domain::{
    calculate_variance, variance_kind_to_classification, Quantity, QuickSpotCountRequest,
    QuickSpotCountResponse, COUNT_STATUS_APPROVED, COUNT_STATUS_PENDING_APPROVAL,
    INVENTORY_COUNT_TYPE_SPOT, VARIANCE_TYPE_MATCH,
};

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    operation_context::OperationContext as AuthContext,
};

use super::super::{
    lock_idempotency_key, map_db_error, replay_idempotency, request_hash,
    store_idempotency_success, IdempotentMutation, PgWave3Repository, Wave3RepositoryError,
};

impl PgWave3Repository {
    pub async fn quick_spot_count(
        &self,
        ctx: &AuthContext,
        req: QuickSpotCountRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<QuickSpotCountResponse>, Wave3RepositoryError> {
        let request_hash = request_hash(&json!({
            "action": "quick_spot_count",
            "request": &req,
        }))?;

        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        // 1. Verify location exists
        let location = sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>)>(
            r#"
            SELECT id, warehouse_id, zone_id
              FROM warehouse_locations
             WHERE owner_id = $1 AND lower(location_code) = lower($2)
             LIMIT 1
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.location_code.trim())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::InvalidLocation)?;

        let (location_id, warehouse_id, zone_id) = location;

        // 2. Query book qty
        let book_batch = sqlx::query_as::<_, (Uuid, Quantity)>(
            r#"
            SELECT id, COALESCE(qty_on_hand, 0)
              FROM inventory_batches
             WHERE owner_id = $1
               AND location_id = $2
               AND lower(product_code) = lower($3)
               AND batch_no = $4
             LIMIT 1
            "#,
        )
        .bind(ctx.owner_id)
        .bind(location_id)
        .bind(req.product_code.trim())
        .bind(req.batch_no.trim())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let (batch_id, book_qty) = match book_batch {
            Some((id, qty)) => (Some(id), qty),
            None => (None, Quantity::ZERO),
        };

        let physical_qty = req.physical_qty;
        let (variance_qty, variance_kind) = calculate_variance(book_qty, physical_qty);
        let variance_type = variance_kind_to_classification(variance_kind).to_string();

        let count_id = Uuid::new_v4();
        let count_status = if variance_type == VARIANCE_TYPE_MATCH {
            COUNT_STATUS_APPROVED
        } else {
            COUNT_STATUS_PENDING_APPROVAL
        };

        sqlx::query(
            r#"
            INSERT INTO inventory_counts (
                id, owner_id, count_type, warehouse_id, zone_id, product_code,
                status, started_at, created_by, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $8, $8
            )
            "#,
        )
        .bind(count_id)
        .bind(ctx.owner_id)
        .bind(INVENTORY_COUNT_TYPE_SPOT)
        .bind(warehouse_id)
        .bind(zone_id)
        .bind(req.product_code.trim())
        .bind(count_status)
        .bind(now)
        .bind(ctx.user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let line_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO inventory_count_lines (
                id, count_id, owner_id, inventory_batch_id, location_id,
                location_code, product_code, batch_no, book_qty, physical_qty,
                variance_qty, variance_type
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10,
                $11, $12
            )
            "#,
        )
        .bind(line_id)
        .bind(count_id)
        .bind(ctx.owner_id)
        .bind(batch_id)
        .bind(location_id)
        .bind(req.location_code.trim())
        .bind(req.product_code.trim())
        .bind(req.batch_no.trim())
        .bind(book_qty)
        .bind(physical_qty)
        .bind(variance_qty)
        .bind(&variance_type)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let response = QuickSpotCountResponse {
            count_id,
            book_qty,
            physical_qty,
            variance_qty,
            variance_type,
        };

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inventory/counts/quick-spot-count",
            "inventory_count",
            count_id.to_string(),
            &response,
            now,
        )
        .await?;

        if let Some(audit) = audit {
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }

        tx.commit().await.map_err(map_db_error)?;

        Ok(IdempotentMutation {
            value: response,
            replayed: false,
        })
    }
}
