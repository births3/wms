use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;
use wms_domain::{
    calculate_variance, variance_kind_to_classification, Quantity, QuickSpotCountRequest,
    QuickSpotCountResponse, COUNT_STATUS_APPROVED, COUNT_STATUS_PENDING_APPROVAL,
    INVENTORY_COUNT_TYPE_SPOT, VARIANCE_TYPE_MATCH,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    operation_context::OperationContext as AuthContext,
};

use super::super::{
    lock_idempotency_key, map_db_error, replay_idempotency, request_hash,
    store_idempotency_success, validated_pda_operated_at, IdempotentMutation, PgWave3Repository,
    Wave3RepositoryError,
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
        if req.physical_qty < Quantity::ZERO {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
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
        let operated_at = validated_pda_operated_at(req.operated_at, now)?;

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

        let book_rows = sqlx::query_as::<_, (Uuid, Quantity, String)>(
            r#"
            SELECT id, COALESCE(qty_on_hand, 0), status
              FROM inventory_batches
             WHERE owner_id = $1
               AND location_id = $2
               AND lower(product_code) = lower($3)
               AND batch_no = $4
             ORDER BY status, id
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(location_id)
        .bind(req.product_code.trim())
        .bind(req.batch_no.trim())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;

        if book_rows.len() > 1 {
            return Err(Wave3RepositoryError::InvalidInventoryState);
        }
        let (batch_id, book_qty) = match book_rows.into_iter().next() {
            Some((id, qty, _status)) => (Some(id), qty),
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
        let auto_approved = variance_type == VARIANCE_TYPE_MATCH;
        let approved_by = auto_approved.then_some(ctx.user_id);
        let approved_at = auto_approved.then_some(now);
        let approval_source = auto_approved.then(|| "system_auto_match".to_string());
        let approval_id = auto_approved.then(|| format!("quick-spot:{count_id}"));
        let reason = req
            .reason
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        sqlx::query(
            r#"
            INSERT INTO inventory_counts (
                id, owner_id, count_type, warehouse_id, zone_id, product_code,
                status, started_at, created_by, approved_by, approved_at,
                approval_source, approval_id, reason, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11,
                $12, $13, $14, $15, $15
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
        .bind(operated_at)
        .bind(ctx.user_id)
        .bind(approved_by)
        .bind(approved_at)
        .bind(&approval_source)
        .bind(&approval_id)
        .bind(&reason)
        .bind(now)
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

        if let Some(mut audit) = audit {
            audit.diff = Some(AuditDiff::compute(
                json!({}),
                json!({
                    "location_code": req.location_code.trim(),
                    "product_code": req.product_code.trim(),
                    "batch_no": req.batch_no.trim(),
                    "book_qty": book_qty,
                    "physical_qty": physical_qty,
                    "variance_qty": variance_qty,
                    "variance_type": response.variance_type,
                    "count_status": count_status,
                    "reason": reason,
                    "operated_at": operated_at,
                }),
            ));
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
