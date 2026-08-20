use super::*;
use wms_domain::{InventoryRelocation, RelocateInventoryRequest};

use crate::inventory::STATUS_QUALIFIED;

#[derive(FromRow)]
struct RelocationRow {
    id: Uuid,
    owner_id: Uuid,
    batch_id: Uuid,
    product_code: String,
    batch_no: String,
    qty: wms_domain::Quantity,
    from_location_id: Uuid,
    from_location_code: String,
    to_location_id: Uuid,
    to_location_code: String,
    relocation_mode: String,
    lpn_code: Option<String>,
    quality_status: String,
    status: String,
    reason: Option<String>,
    created_by: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PgWave3Repository {
    pub async fn relocate_inventory_with_audit(
        &self,
        ctx: &AuthContext,
        req: RelocateInventoryRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<InventoryRelocation>, Wave3RepositoryError> {
        if req.qty <= wms_domain::Quantity::ZERO {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let mode = req
            .relocation_mode
            .as_deref()
            .unwrap_or("direct")
            .trim()
            .to_string();
        if !matches!(mode.as_str(), "direct" | "lpn_full" | "partial" | "piece") {
            return Err(Wave3RepositoryError::InvalidLocation);
        }
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
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

        let batch = sqlx::query_as::<_, InventoryBatchRow>(
            r#"
            SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                   qty_on_hand, qty_frozen, status, location_id, location_code,
                   recall_flag, created_at, updated_at
              FROM inventory_batches
             WHERE id = $1 AND owner_id = $2
             FOR UPDATE
            "#,
        )
        .bind(req.batch_id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;

        if batch.status != STATUS_QUALIFIED || batch.recall_flag {
            return Err(Wave3RepositoryError::InvalidStateTransition {
                from: batch.status,
                to: "relocate".to_string(),
                approval_source: "M3-006".to_string(),
            });
        }
        let available = batch.qty_on_hand - batch.qty_frozen;
        if req.qty > available {
            return Err(Wave3RepositoryError::InsufficientQuantity);
        }
        if batch.location_id == req.to_location_id {
            return Err(Wave3RepositoryError::InvalidLocation);
        }
        if crate::inventory::location_is_unreachable_in_tx(&mut tx, ctx.owner_id, batch.location_id)
            .await
            .map_err(map_db_error)?
            || crate::inventory::location_is_unreachable_in_tx(
                &mut tx,
                ctx.owner_id,
                req.to_location_id,
            )
            .await
            .map_err(map_db_error)?
        {
            return Err(Wave3RepositoryError::LocationUnreachable);
        }

        let (to_zone_temp, to_quality_color, to_max_volume, to_used_volume, to_max_sku): (
            String,
            String,
            i64,
            i64,
            i32,
        ) = sqlx::query_as(
            r#"
            SELECT zones.temperature_zone, zones.quality_color,
                   locations.max_volume_cm3, locations.used_volume_cm3, locations.max_sku_count
              FROM warehouse_locations locations
              JOIN warehouse_zones zones
                ON zones.id = locations.zone_id AND zones.owner_id = locations.owner_id
             WHERE locations.id = $1 AND locations.owner_id = $2
            "#,
        )
        .bind(req.to_location_id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::InvalidLocation)?;

        let from_zone_temp: Option<String> = sqlx::query_scalar(
            r#"
            SELECT zones.temperature_zone
              FROM warehouse_locations locations
              JOIN warehouse_zones zones
                ON zones.id = locations.zone_id AND zones.owner_id = locations.owner_id
             WHERE locations.id = $1 AND locations.owner_id = $2
            "#,
        )
        .bind(batch.location_id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;

        if let Some(from_temp) = from_zone_temp.as_deref() {
            if !from_temp.eq_ignore_ascii_case(&to_zone_temp) {
                return Err(Wave3RepositoryError::LocationTemperatureMismatch);
            }
        }
        if to_quality_color.contains("unqualified") || to_quality_color.eq_ignore_ascii_case("red")
        {
            return Err(Wave3RepositoryError::LocationQualityMismatch);
        }
        if to_used_volume >= to_max_volume && to_max_volume > 0 {
            return Err(Wave3RepositoryError::LocationCapacityExceeded);
        }
        let current_sku: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT product_code)::BIGINT
              FROM inventory_batches
             WHERE owner_id = $1 AND location_id = $2 AND qty_on_hand > 0
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.to_location_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let already_same_product: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM inventory_batches
                 WHERE owner_id = $1 AND location_id = $2 AND product_code = $3 AND qty_on_hand > 0
            )
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.to_location_id)
        .bind(&batch.product_code)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if !already_same_product && current_sku >= i64::from(to_max_sku) {
            return Err(Wave3RepositoryError::LocationSkuLimitExceeded);
        }

        let target_batch_id = if req.qty == batch.qty_on_hand
            && batch.qty_frozen == wms_domain::Quantity::ZERO
        {
            sqlx::query(
                r#"
                UPDATE inventory_batches
                   SET location_id = $3,
                       location_code = $4,
                       updated_at = $5,
                       version = version + 1
                 WHERE id = $1 AND owner_id = $2
                "#,
            )
            .bind(batch.id)
            .bind(ctx.owner_id)
            .bind(req.to_location_id)
            .bind(&req.to_location_code)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            batch.id
        } else {
            sqlx::query(
                r#"
                UPDATE inventory_batches
                   SET qty_on_hand = qty_on_hand - $3,
                       updated_at = $4,
                       version = version + 1
                 WHERE id = $1 AND owner_id = $2
                "#,
            )
            .bind(batch.id)
            .bind(ctx.owner_id)
            .bind(req.qty)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;

            let existing_target: Option<Uuid> = sqlx::query_scalar(
                r#"
                SELECT id FROM inventory_batches
                 WHERE owner_id = $1 AND product_code = $2 AND batch_no = $3
                   AND location_id = $4 AND status = $5 AND recall_flag = FALSE
                 FOR UPDATE
                "#,
            )
            .bind(ctx.owner_id)
            .bind(&batch.product_code)
            .bind(&batch.batch_no)
            .bind(req.to_location_id)
            .bind(&batch.status)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?;

            if let Some(target_id) = existing_target {
                sqlx::query(
                    r#"
                    UPDATE inventory_batches
                       SET qty_on_hand = qty_on_hand + $3,
                           updated_at = $4,
                           version = version + 1
                     WHERE id = $1 AND owner_id = $2
                    "#,
                )
                .bind(target_id)
                .bind(ctx.owner_id)
                .bind(req.qty)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
                target_id
            } else {
                let new_id = Uuid::new_v4();
                sqlx::query(
                        r#"
                    INSERT INTO inventory_batches (
                        id, owner_id, product_id, product_code, batch_no, production_date, expiry_date,
                        qty_on_hand, qty_frozen, status, location_id, location_code,
                        recall_flag, created_at, updated_at
                    ) VALUES (
                        $1,$2,
                        (SELECT product.id FROM products product WHERE product.owner_id = $2 AND product.product_code = $3 AND product.status = 'active' LIMIT 1),
                        $3,$4,$5,$6,$7,0,$8,$9,$10,FALSE,$11,$11
                    )
                    "#,
                    )
                    .bind(new_id)
                    .bind(ctx.owner_id)
                    .bind(&batch.product_code)
                    .bind(&batch.batch_no)
                    .bind(batch.production_date)
                    .bind(batch.expiry_date)
                    .bind(req.qty)
                    .bind(&batch.status)
                    .bind(req.to_location_id)
                    .bind(&req.to_location_code)
                    .bind(now)
                    .execute(&mut *tx)
                    .await
                    .map_err(map_db_error)?;
                new_id
            }
        };

        let relocation_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO inventory_movements (
                id, owner_id, batch_id, movement_type, qty_delta,
                source_document_type, source_document_id, occurred_at,
                location_code, from_location_code, to_location_code,
                lpn_code, operator_user_id, operator_name
            ) VALUES
              ($1,$2,$3,'relocation_out',-$4,'inventory_relocation',$5,$6,$7,$7,$8,$9,$10,$11),
              ($12,$2,$13,'relocation_in',$4,'inventory_relocation',$5,$6,$8,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(batch.id)
        .bind(req.qty)
        .bind(relocation_id)
        .bind(now)
        .bind(&batch.location_code)
        .bind(&req.to_location_code)
        .bind(&req.lpn_code)
        .bind(ctx.user_id)
        .bind(&ctx.actor_name)
        .bind(Uuid::new_v4())
        .bind(target_batch_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let row = sqlx::query_as::<_, RelocationRow>(
            r#"
            INSERT INTO inventory_relocations (
                id, owner_id, batch_id, product_code, batch_no, qty,
                from_location_id, from_location_code, to_location_id, to_location_code,
                relocation_mode, lpn_code, quality_status, status, reason,
                created_by, created_at, updated_at
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,'completed',$14,$15,$16,$16
            )
            RETURNING id, owner_id, batch_id, product_code, batch_no, qty,
                      from_location_id, from_location_code, to_location_id, to_location_code,
                      relocation_mode, lpn_code, quality_status, status, reason,
                      created_by, created_at, updated_at
            "#,
        )
        .bind(relocation_id)
        .bind(ctx.owner_id)
        .bind(batch.id)
        .bind(&batch.product_code)
        .bind(&batch.batch_no)
        .bind(req.qty)
        .bind(batch.location_id)
        .bind(&batch.location_code)
        .bind(req.to_location_id)
        .bind(&req.to_location_code)
        .bind(&mode)
        .bind(&req.lpn_code)
        .bind(&batch.status)
        .bind(&req.reason)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let relocation = map_relocation(row);
        if let Some(audit) = audit {
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inventory/relocations",
            "inventory_relocation",
            relocation.id.to_string(),
            &relocation,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: relocation,
            replayed: false,
        })
    }

    pub async fn list_inventory_relocations(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<InventoryRelocation>, Wave3RepositoryError> {
        let rows = sqlx::query_as::<_, RelocationRow>(
            r#"
            SELECT id, owner_id, batch_id, product_code, batch_no, qty,
                   from_location_id, from_location_code, to_location_id, to_location_code,
                   relocation_mode, lpn_code, quality_status, status, reason,
                   created_by, created_at, updated_at
              FROM inventory_relocations
             WHERE owner_id = $1
             ORDER BY created_at DESC, id DESC
             LIMIT 200
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(map_relocation).collect())
    }
}

fn map_relocation(row: RelocationRow) -> InventoryRelocation {
    InventoryRelocation {
        id: row.id,
        owner_id: row.owner_id,
        batch_id: row.batch_id,
        product_code: row.product_code,
        batch_no: row.batch_no,
        qty: row.qty,
        from_location_id: row.from_location_id,
        from_location_code: row.from_location_code,
        to_location_id: row.to_location_id,
        to_location_code: row.to_location_code,
        relocation_mode: row.relocation_mode,
        lpn_code: row.lpn_code,
        quality_status: row.quality_status,
        status: row.status,
        reason: row.reason,
        created_by: row.created_by,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}
