use std::collections::BTreeMap;

use super::*;

impl PgWave3Repository {
    pub async fn approve_inventory_count_with_audit(
        &self,
        ctx: &AuthContext,
        count_id: Uuid,
        req: ApproveInventoryCountRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<InventoryCount>, Wave3RepositoryError> {
        let request_hash = request_hash(&json!({
            "count_id": count_id,
            "request": &req,
        }))?;
        let mut tx = self.begin().await?;
        super::super::lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<InventoryCount>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let count = lock_inventory_count(&mut tx, ctx.owner_id, count_id).await?;
        if count.status != "pending_approval" {
            return Err(Wave3RepositoryError::InvalidInventoryCountState);
        }
        let lines = sqlx::query_as::<_, InventoryCountLineRow>(
            r#"
            SELECT id, count_id, owner_id, inventory_batch_id, location_id,
                   location_code, product_code, batch_no, book_qty, physical_qty,
                   variance_qty, variance_type
              FROM inventory_count_lines
             WHERE owner_id = $1 AND count_id = $2
             ORDER BY id
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(count_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if lines.is_empty() || lines.iter().any(|line| line.physical_qty.is_none()) {
            return Err(Wave3RepositoryError::InventoryCountNotReady);
        }
        let requires_elevated = count_requires_elevated_approval(
            lines
                .iter()
                .map(|line| (line.book_qty, line.variance_qty.unwrap_or_default())),
        );
        validate_approval_for_variance(&req, requires_elevated).map_err(|error| match error {
            wms_domain::InventoryCountValidationError::ElevatedApprovalRequired => {
                Wave3RepositoryError::MissingApprovalSource
            }
            _ => Wave3RepositoryError::MissingApprovalSource,
        })?;

        let mut adjustments = Vec::new();
        let mut snapshots = BTreeMap::<
            Uuid,
            BTreeMap<
                (String, String, chrono::NaiveDate, String),
                (wms_domain::Quantity, wms_domain::Quantity),
            >,
        >::new();
        for line in &lines {
            let batch = sqlx::query_as::<_, InventoryCountBatchRow>(
                r#"
                SELECT batch.id, location.warehouse_id, batch.product_code,
                       batch.batch_no, batch.expiry_date, batch.quality_status,
                       batch.qty_on_hand, batch.qty_locked
                  FROM inventory_batches batch
                  JOIN warehouse_locations location
                    ON location.owner_id = batch.owner_id
                   AND location.id = batch.location_id
                 WHERE batch.owner_id = $1 AND batch.id = $2
                 FOR UPDATE OF batch
                "#,
            )
            .bind(ctx.owner_id)
            .bind(line.inventory_batch_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .ok_or(Wave3RepositoryError::NotFound)?;
            let variance_qty = line.variance_qty.unwrap_or_default();
            let next_qty = batch
                .qty_on_hand
                .checked_add(variance_qty)
                .ok_or(Wave3RepositoryError::InvalidQuantity)?;
            if next_qty < batch.qty_locked {
                return Err(Wave3RepositoryError::InventoryCountQuantityConflict);
            }
            let updated = sqlx::query(
                r#"
                UPDATE inventory_batches
                   SET qty_on_hand = $3, updated_at = $4, version = version + 1
                 WHERE owner_id = $1 AND id = $2 AND qty_on_hand = $5
                "#,
            )
            .bind(ctx.owner_id)
            .bind(batch.id)
            .bind(next_qty)
            .bind(now)
            .bind(batch.qty_on_hand)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            if updated.rows_affected() != 1 {
                return Err(Wave3RepositoryError::InventoryCountQuantityConflict);
            }
            if variance_qty != wms_domain::Quantity::ZERO {
                sqlx::query(
                    r#"
                    INSERT INTO inventory_movements (
                        id, owner_id, batch_id, movement_type, qty_delta,
                        source_document_type, source_document_id, occurred_at
                    )
                    VALUES ($1, $2, $3, 'inventory_count_adjustment', $4, 'inventory_count', $5, $6)
                    "#,
                )
                .bind(Uuid::new_v4())
                .bind(ctx.owner_id)
                .bind(batch.id)
                .bind(variance_qty)
                .bind(count_id)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
            }
            adjustments.push(json!({
                "batch_id": batch.id,
                "product_code": batch.product_code,
                "batch_no": batch.batch_no,
                "book_qty": line.book_qty,
                "physical_qty": line.physical_qty,
                "variance_qty": variance_qty,
                "qty_on_hand_before": batch.qty_on_hand,
                "qty_on_hand_after": next_qty,
            }));
            snapshots
                .entry(batch.warehouse_id)
                .or_default()
                .entry((
                    batch.product_code,
                    batch.batch_no,
                    batch.expiry_date,
                    batch.quality_status,
                ))
                .and_modify(|(amount, allocated)| {
                    *amount += next_qty;
                    *allocated += batch.qty_locked;
                })
                .or_insert((next_qty, batch.qty_locked));
        }

        sqlx::query(
            r#"
            UPDATE inventory_counts
               SET status = 'approved', approved_by = $3, approved_at = $4,
                   approval_source = $5, approval_id = $6, updated_at = $4
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(count_id)
        .bind(ctx.user_id)
        .bind(now)
        .bind(&req.approval_source)
        .bind(&req.approval_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let approved = load_inventory_count_in_tx(&mut tx, ctx.owner_id, count_id).await?;
        for (warehouse_id, grouped) in snapshots {
            let outbox_id = Uuid::new_v4();
            let snapshot_no = outbox_id.simple().to_string();
            let lines = grouped
                .into_iter()
                .enumerate()
                .map(
                    |(
                        index,
                        ((product_code, batch_no, valid_date, goods_status), (amount, allocated)),
                    )| {
                        json!({
                            "row_no": index + 1,
                            "product_code": product_code,
                            "batch_no": batch_no,
                            "valid_date": valid_date,
                            "goods_status": goods_status,
                            "wms_amount": format!("{amount:.4}"),
                            "wms_pickable": format!("{:.4}", amount - allocated),
                            "wms_allocated": format!("{allocated:.4}"),
                            "wms_frozen": "0.0000",
                        })
                    },
                )
                .collect::<Vec<_>>();
            sqlx::query(
                r#"
                INSERT INTO inventory_snapshot_erp_feedback_outbox (
                    id, owner_id, snapshot_no, payload, created_at, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $5)
                "#,
            )
            .bind(outbox_id)
            .bind(ctx.owner_id)
            .bind(&snapshot_no)
            .bind(json!({
                "warehouse_id": warehouse_id,
                "snapshot_id": snapshot_no,
                "receive_time": now,
                "correlation_id": outbox_id,
                "count_id": count_id,
                "count_type": count.count_type,
                "zone_id": count.zone_id,
                "product_code": count.product_code,
                "approval_source": req.approval_source,
                "approval_id": req.approval_id,
                "approved_at": now,
                "lines": lines,
            }))
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        let mut audit_event = audit.unwrap_or_else(|| {
            AuditWriteRequest::from_auth_context(
                ctx,
                "approve_inventory_count",
                "M3",
                "inventory_count",
                count_id.to_string(),
                Some(AuditDiff::compute(
                    json!({ "status": "pending_approval" }),
                    json!({
                        "status": "approved",
                        "approval_source": &req.approval_source,
                        "approval_id": &req.approval_id,
                        "adjustments": &adjustments,
                    }),
                )),
            )
        });
        audit_event.occurred_at = now;
        audit_event.resource_id = count_id.to_string();
        append_event_in_tx(&mut tx, &audit_event)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &format!("/api/v1/inventory/counts/{count_id}/approve"),
            "inventory_count",
            count_id.to_string(),
            &approved,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: approved,
            replayed: false,
        })
    }
}
