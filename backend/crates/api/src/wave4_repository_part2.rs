impl PgWave4Repository {
pub async fn ship_outbound_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        req: ShipOutboundOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundOrder>, Wave4RepositoryError> {
        if req.package_count == 0 {
            return Err(Wave4RepositoryError::InvalidQuantity);
        }
        let request_hash = request_hash(&serde_json::json!({
            "outbound_order_id": order_id,
            "request": req,
        }))?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let order_row = lock_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        if !matches!(
            order_row.status.as_str(),
            OUTBOUND_STATUS_REVIEWED | OUTBOUND_STATUS_REVIEWED_SHORT
        ) {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "reviewed|reviewed_short".to_string(),
                actual: order_row.status,
            });
        }
        let order = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        all_lines_reviewed_for_ship(&order.lines)
            .map_err(|_| Wave4RepositoryError::ShortPickNotReplenished)?;

        for line in &order.lines {
            if !consume_inventory_allocation_for_outbound(
                &mut tx,
                ctx.owner_id,
                order_id,
                line,
                now,
            )
            .await?
            {
                deduct_inventory_for_outbound(&mut tx, ctx.owner_id, order_id, line, now).await?;
            }
        }
        sqlx::query(
            r#"
            UPDATE outbound_order_lines
               SET shipped_qty = planned_qty
             WHERE owner_id = $1 AND outbound_order_id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query(
            r#"
            INSERT INTO outbound_shipments (
                id, owner_id, outbound_order_id, carrier_type, handover_to,
                package_count, shipped_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(&req.carrier_type)
        .bind(&req.handover_to)
        .bind(i32::try_from(req.package_count).map_err(|_| Wave4RepositoryError::InvalidQuantity)?)
        .bind(req.shipped_at.unwrap_or(now))
        .execute(&mut *tx)
        .await
        .map_err(map_insert_error)?;
        sqlx::query(
            r#"
            UPDATE outbound_orders
               SET status = $3,
                   short_pick = FALSE,
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(OUTBOUND_STATUS_SHIPPED)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let shipped = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/orders/{id}/ship",
            "outbound_order",
            shipped.id.to_string(),
            &shipped,
            now,
        )
        .await?;
        append_outbound_audit(&mut tx, ctx, audit, "ship_outbound_order", shipped.id, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: shipped,
            replayed: false,
        })
    }

    pub async fn create_traceability_outbound_report(
        &self,
        ctx: &AuthContext,
        req: TraceabilityOutboundReportRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<TraceabilityOutboundReport>, Wave4RepositoryError> {
        let report = TraceabilityCodeService
            .traceability_report_at(req, now)
            .map_err(|_| Wave4RepositoryError::InvalidTraceabilityEvent)?;
        let request_hash = request_hash(&serde_json::json!({ "request": report.events }))?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        sqlx::query(
            r#"
            INSERT INTO traceability_outbound_reports (
                id, owner_id, platform, status, queued_count, generated_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $6, $6)
            "#,
        )
        .bind(report.report_id)
        .bind(ctx.owner_id)
        .bind(&report.platform)
        .bind(&report.status)
        .bind(
            i32::try_from(report.queued_count)
                .map_err(|_| Wave4RepositoryError::InvalidQuantity)?,
        )
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_insert_error)?;

        for event in &report.events {
            sqlx::query(
                r#"
                INSERT INTO traceability_outbound_report_events (
                    event_id, owner_id, report_id, trace_code, status_change_type,
                    occurred_at, report_status, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
                "#,
            )
            .bind(event.event_id)
            .bind(ctx.owner_id)
            .bind(report.report_id)
            .bind(&event.trace_code)
            .bind(&event.status_change_type)
            .bind(event.occurred_at)
            .bind(&report.status)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_insert_error)?;
        }

        let persisted =
            load_traceability_outbound_report(&mut tx, ctx.owner_id, report.report_id).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/traceability/outbound-reports",
            "traceability_outbound_report",
            persisted.report_id.to_string(),
            &persisted,
            now,
        )
        .await?;
        append_traceability_audit(
            &mut tx,
            ctx,
            audit,
            "create_traceability_outbound_report",
            persisted.report_id,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: persisted,
            replayed: false,
        })
    }

    pub async fn apply_traceability_platform_response(
        &self,
        ctx: &AuthContext,
        event_id: Uuid,
        response: TraceabilityPlatformResponse,
        now: DateTime<Utc>,
        audit: Option<AuditWriteRequest>,
    ) -> Result<TraceabilityReplayDecision, Wave4RepositoryError> {
        let decision = TraceabilityCodeService
            .classify_platform_response(response)
            .map_err(|_| Wave4RepositoryError::InvalidTraceabilityEvent)?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let report_id: Uuid = sqlx::query_scalar(
            r#"
            SELECT report_id
              FROM traceability_outbound_report_events
             WHERE owner_id = $1 AND event_id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(event_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;

        sqlx::query(
            r#"
            UPDATE traceability_outbound_report_events
               SET report_status = $3,
                   retry_count = retry_count + CASE WHEN $4 THEN 1 ELSE 0 END,
                   last_error_code = $5,
                   platform_receipt_id = $6,
                   updated_at = $7
             WHERE owner_id = $1 AND event_id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(event_id)
        .bind(&decision.status)
        .bind(decision.should_retry)
        .bind(&decision.error_code)
        .bind(&decision.platform_receipt_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        refresh_traceability_report_status(&mut tx, ctx.owner_id, report_id, now).await?;
        append_traceability_event_audit(&mut tx, ctx, audit, &decision.audit_action, event_id, now)
            .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(decision)
    }

    pub async fn list_pending_temperature_excursions(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<TemperatureExcursionEvent>, Wave4RepositoryError> {
        let rows = sqlx::query_as::<_, TemperatureExcursionEventRow>(
            r#"
            SELECT id, owner_id, external_event_id, device_code, location_code,
                   started_at, ended_at, min_temperature_celsius,
                   max_temperature_celsius, affected_batch_ids, status, created_at
              FROM temperature_excursion_events
             WHERE owner_id = $1 AND status = 'pending_disposition'
             ORDER BY created_at DESC, external_event_id ASC
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        Ok(rows.into_iter().map(map_temperature_excursion).collect())
    }

    pub async fn dispose_temperature_excursion_and_quarantine_batches(
        &self,
        ctx: &AuthContext,
        external_event_id: &str,
        selected_batch_ids: Vec<Uuid>,
        now: DateTime<Utc>,
        audit: Option<AuditWriteRequest>,
    ) -> Result<TemperatureExcursionDisposition, Wave4RepositoryError> {
        if selected_batch_ids.is_empty() {
            return Err(Wave4RepositoryError::EmptySelection);
        }

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let event_row = sqlx::query_as::<_, TemperatureExcursionEventRow>(
            r#"
            SELECT id, owner_id, external_event_id, device_code, location_code,
                   started_at, ended_at, min_temperature_celsius,
                   max_temperature_celsius, affected_batch_ids, status, created_at
              FROM temperature_excursion_events
             WHERE owner_id = $1 AND external_event_id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(external_event_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;

        if event_row.status != "pending_disposition" {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "pending_disposition".to_string(),
                actual: event_row.status,
            });
        }

        for batch_id in &selected_batch_ids {
            if !event_row.affected_batch_ids.contains(batch_id) {
                return Err(Wave4RepositoryError::BatchNotAffected(*batch_id));
            }
        }

        let mut quarantined_batches = Vec::new();
        for batch_id in selected_batch_ids {
            let batch_row = sqlx::query_as::<_, InventoryBatchRow>(
                r#"
                SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                       qty_on_hand, qty_locked, quality_status, location_id, location_code,
                       recall_flag, created_at, updated_at
                  FROM inventory_batches
                 WHERE owner_id = $1 AND id = $2
                 FOR UPDATE
                "#,
            )
            .bind(ctx.owner_id)
            .bind(batch_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .ok_or(Wave4RepositoryError::NotFound)?;

            let from_status = batch_row.quality_status.clone();
            let batch = if from_status == STATUS_QUARANTINED {
                map_inventory_batch(batch_row)
            } else {
                if !crate::inventory_status_config::is_transition_allowed_in_tx(
                    &mut tx,
                    ctx.owner_id,
                    &from_status,
                    STATUS_QUARANTINED,
                    APPROVAL_SOURCE_TEMPERATURE_EXCURSION,
                )
                .await
                .map_err(map_db_error)?
                {
                    return Err(Wave4RepositoryError::InvalidStateTransition {
                        from: from_status,
                        to: STATUS_QUARANTINED.to_string(),
                        approval_source: APPROVAL_SOURCE_TEMPERATURE_EXCURSION.to_string(),
                    });
                }

                sqlx::query(
                    r#"
                    INSERT INTO inventory_status_changes (
                        id, owner_id, batch_id, from_status, to_status,
                        reason, approval_source, approval_id, occurred_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    "#,
                )
                .bind(Uuid::new_v4())
                .bind(ctx.owner_id)
                .bind(batch_id)
                .bind(&from_status)
                .bind(STATUS_QUARANTINED)
                .bind("temperature excursion disposition")
                .bind(APPROVAL_SOURCE_TEMPERATURE_EXCURSION)
                .bind(external_event_id)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;

                let updated = sqlx::query_as::<_, InventoryBatchRow>(
                    r#"
                    UPDATE inventory_batches
                       SET quality_status = $3,
                           updated_at = $4,
                           version = version + 1
                     WHERE owner_id = $1 AND id = $2
                    RETURNING id, owner_id, product_code, batch_no, production_date, expiry_date,
                              qty_on_hand, qty_locked, quality_status, location_id, location_code,
                              recall_flag, created_at, updated_at
                    "#,
                )
                .bind(ctx.owner_id)
                .bind(batch_id)
                .bind(STATUS_QUARANTINED)
                .bind(now)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_db_error)?;
                map_inventory_batch(updated)
            };
            quarantined_batches.push(batch);
        }

        let event_row = sqlx::query_as::<_, TemperatureExcursionEventRow>(
            r#"
            UPDATE temperature_excursion_events
               SET status = 'disposed'
             WHERE owner_id = $1 AND external_event_id = $2
            RETURNING id, owner_id, external_event_id, device_code, location_code,
                      started_at, ended_at, min_temperature_celsius,
                      max_temperature_celsius, affected_batch_ids, status, created_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(external_event_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let event = map_temperature_excursion(event_row);

        let mut audit = audit.unwrap_or_else(|| {
            AuditWriteRequest::from_auth_context(
                ctx,
                "dispose_temperature_excursion",
                "M5",
                "temperature_excursion",
                event.id.to_string(),
                None,
            )
        });
        audit.resource_id = event.id.to_string();
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave4RepositoryError::Audit(format!("{error:?}")))?;

        tx.commit().await.map_err(map_db_error)?;
        Ok(TemperatureExcursionDisposition {
            event,
            quarantined_batches,
        })
    }
}

#[derive(FromRow)]
struct OutboundAllocationRow {
    id: Uuid,
    batch_id: Uuid,
    allocated_qty: i64,
}

async fn append_outbound_inventory_movement(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    batch_id: Uuid,
    qty_delta: i64,
    order_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    sqlx::query(
        r#"
        INSERT INTO inventory_movements (
            id, owner_id, batch_id, movement_type, qty_delta,
            source_document_type, source_document_id, occurred_at
        )
        VALUES ($1, $2, $3, 'outbound_ship', $4, 'outbound_order', $5, $6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(batch_id)
    .bind(qty_delta)
    .bind(order_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

async fn allocate_inventory_for_outbound(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
    line: &OutboundOrderLine,
    now: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    let candidates: Vec<(Uuid, i64)> = sqlx::query_as(
        r#"
        SELECT id, qty_on_hand - qty_locked AS available_qty
          FROM inventory_batches
         WHERE owner_id = $1
           AND product_code = $2
           AND batch_no = $3
           AND quality_status = $4
           AND recall_flag = FALSE
           AND qty_on_hand - qty_locked > 0
         ORDER BY location_code ASC, id ASC
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(&line.product_code)
    .bind(&line.batch_no)
    .bind(STATUS_QUALIFIED)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let available = candidates.iter().try_fold(0_i64, |total, (_, qty)| {
        total
            .checked_add(*qty)
            .ok_or(Wave4RepositoryError::InvalidQuantity)
    })?;
    if available < line.planned_qty {
        return Err(Wave4RepositoryError::InvalidQuantity);
    }

    let line_no = i32::try_from(line.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?;
    let mut remaining = line.planned_qty;
    for (batch_id, available_qty) in candidates {
        if remaining == 0 {
            break;
        }
        let allocated_qty = available_qty.min(remaining);
        let updated = sqlx::query(
            r#"
            UPDATE inventory_batches
               SET qty_locked = qty_locked + $3,
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
               AND quality_status = $5 AND recall_flag = FALSE
               AND qty_on_hand - qty_locked >= $3
            "#,
        )
        .bind(owner_id)
        .bind(batch_id)
        .bind(allocated_qty)
        .bind(now)
        .bind(STATUS_QUALIFIED)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if updated.rows_affected() != 1 {
            return Err(Wave4RepositoryError::InvalidQuantity);
        }
        sqlx::query(
            r#"
            INSERT INTO inventory_allocations (
                id, owner_id, outbound_order_id, line_no, batch_id,
                allocated_qty, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'locked', $7, $7)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(order_id)
        .bind(line_no)
        .bind(batch_id)
        .bind(allocated_qty)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_insert_error)?;
        remaining -= allocated_qty;
    }
    Ok(())
}

async fn consume_inventory_allocation_for_outbound(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
    line: &OutboundOrderLine,
    now: DateTime<Utc>,
) -> Result<bool, Wave4RepositoryError> {
    let line_no = i32::try_from(line.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?;
    let allocations = sqlx::query_as::<_, OutboundAllocationRow>(
        r#"
        SELECT id, batch_id, allocated_qty
          FROM inventory_allocations
         WHERE owner_id = $1 AND outbound_order_id = $2
           AND line_no = $3 AND status = 'locked'
         ORDER BY id
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(order_id)
    .bind(line_no)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if allocations.is_empty() {
        return Ok(false);
    }
    let allocated_qty = allocations.iter().try_fold(0_i64, |total, allocation| {
        total
            .checked_add(allocation.allocated_qty)
            .ok_or(Wave4RepositoryError::InvalidQuantity)
    })?;
    if allocated_qty != line.planned_qty {
        return Err(Wave4RepositoryError::InvalidQuantity);
    }

    for allocation in allocations {
        let updated = sqlx::query(
            r#"
            UPDATE inventory_batches
               SET qty_on_hand = qty_on_hand - $3,
                   qty_locked = qty_locked - $3,
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
               AND quality_status = $5 AND recall_flag = FALSE
               AND qty_on_hand >= $3 AND qty_locked >= $3
            "#,
        )
        .bind(owner_id)
        .bind(allocation.batch_id)
        .bind(allocation.allocated_qty)
        .bind(now)
        .bind(STATUS_QUALIFIED)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if updated.rows_affected() != 1 {
            return Err(Wave4RepositoryError::InvalidQuantity);
        }
        append_outbound_inventory_movement(
            tx,
            owner_id,
            allocation.batch_id,
            -allocation.allocated_qty,
            order_id,
            now,
        )
        .await?;
        let marked = sqlx::query(
            r#"
            UPDATE inventory_allocations
               SET status = 'consumed', consumed_at = $3, updated_at = $3
             WHERE owner_id = $1 AND id = $2 AND status = 'locked'
            "#,
        )
        .bind(owner_id)
        .bind(allocation.id)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if marked.rows_affected() != 1 {
            return Err(Wave4RepositoryError::InvalidQuantity);
        }
    }
    Ok(true)
}
