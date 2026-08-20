// @governance: skip-page-size 该文件是 wave4_repository 的事务切片，保留完整出库事务以便审计原子边界。
#[derive(FromRow)]
struct OutboundErpFeedbackRow {
    erp_bill_code: String,
    erp_revision: i32,
    erp_correlation_id: String,
}

#[derive(FromRow)]
struct OutboundErpFeedbackLine {
    line_no: i32,
    goods_id: i64,
    product_code: String,
    batch_no: String,
    expected_amount: wms_domain::Quantity,
    picked_amount: wms_domain::Quantity,
    shipped_amount: wms_domain::Quantity,
}

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
        crate::outbound_state_rules::validate_outbound_transition(
            &order_row.status,
            OUTBOUND_STATUS_SHIPPED,
            "handover_confirmed",
        )
        .map_err(|_| Wave4RepositoryError::InvalidStateTransition {
            from: order_row.status.clone(),
            to: OUTBOUND_STATUS_SHIPPED.to_string(),
            approval_source: "handover_confirmed".to_string(),
        })?;
        let cold_chain =
            outbound_order_requires_cold_chain(&mut tx, ctx.owner_id, order_id).await?;
        validate_ship_outbound_request(&req, cold_chain)
            .map_err(Wave4RepositoryError::ShipmentValidation)?;
        let driver_name =
            outbound_driver_name(&mut tx, ctx.owner_id, &req).await?;
        let handover_to = driver_name
            .as_deref()
            .or(req.courier_name.as_deref())
            .unwrap_or_default()
            .to_string();
        ensure_handover_signature_attachment(
            &mut tx,
            ctx.owner_id,
            order_id,
            req.signature_attachment_id,
        )
        .await?;
        let cold_chain_packages = serde_json::to_value(&req.cold_chain_packages)
            .map_err(|error| Wave4RepositoryError::Serialize(error.to_string()))?;

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
        let shipment_id = Uuid::new_v4();
        let package_count =
            i32::try_from(req.package_count).map_err(|_| Wave4RepositoryError::InvalidQuantity)?;
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
                id, owner_id, outbound_order_id, delivery_provider_type,
                vehicle_no, plate_no, driver_user_id, driver_name,
                courier_name, courier_phone, signature_attachment_id,
                cold_chain, loading_temperature_celsius, cold_chain_packages,
                package_count, handover_by, shipped_at, created_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14, $15, $16, $17, $17
            )
            "#,
        )
        .bind(shipment_id)
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(&req.delivery_provider_type)
        .bind(req.vehicle_no.as_deref().map(str::trim))
        .bind(req.plate_no.trim())
        .bind(req.driver_user_id)
        .bind(driver_name)
        .bind(req.courier_name.as_deref().map(str::trim))
        .bind(req.courier_phone.as_deref().map(str::trim))
        .bind(req.signature_attachment_id)
        .bind(cold_chain)
        .bind(req.loading_temperature_celsius)
        .bind(cold_chain_packages)
        .bind(package_count)
        .bind(ctx.user_id)
        .bind(now)
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
        publish_customer_portal_order_snapshot(
            &mut tx,
            &shipped,
            now,
        )
        .await?;

        let erp = sqlx::query_as::<_, OutboundErpFeedbackRow>(
            r#"
            SELECT erp_bill_code, erp_revision, erp_correlation_id
              FROM outbound_orders
             WHERE owner_id = $1 AND id = $2
               AND erp_bill_code IS NOT NULL
               AND erp_revision IS NOT NULL
               AND erp_correlation_id IS NOT NULL
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if let Some(erp) = erp {
            let lines = sqlx::query_as::<_, OutboundErpFeedbackLine>(
                r#"
                SELECT line.line_no, product.erp_goods_id AS goods_id,
                       line.product_code, line.batch_no,
                       line.planned_qty AS expected_amount,
                       line.picked_qty AS picked_amount,
                       line.shipped_qty AS shipped_amount
                  FROM outbound_order_lines line
                  JOIN products product
                    ON product.owner_id = line.owner_id
                   AND product.product_code = line.product_code
                 WHERE line.owner_id = $1 AND line.outbound_order_id = $2
                   AND product.erp_goods_id IS NOT NULL
                 ORDER BY line.line_no
                "#,
            )
            .bind(ctx.owner_id)
            .bind(order_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_db_error)?;
            if lines.len() != shipped.lines.len() {
                return Err(Wave4RepositoryError::ErpGoodsMappingIncomplete);
            }
            let lines = lines
                .into_iter()
                .map(|line| {
                    serde_json::json!({
                        "line_no": line.line_no,
                        "goods_id": line.goods_id,
                        "product_code": line.product_code,
                        "batch_no": line.batch_no,
                        "expected_amount": format!("{:.4}", line.expected_amount),
                        "picked_amount": format!("{:.4}", line.picked_amount),
                        "shipped_amount": format!("{:.4}", line.shipped_amount),
                    })
                })
                .collect::<Vec<_>>();
            let carrier: Option<(String, String)> = if req.delivery_provider_type
                == "third_party_express"
            {
                sqlx::query_as(
                    r#"
                    SELECT waybill.waybill_no, carrier.carrier_name
                      FROM h5_express_waybills waybill
                      JOIN h5_express_carriers carrier
                        ON carrier.owner_id = waybill.owner_id
                       AND carrier.carrier_code = waybill.carrier_code
                     WHERE waybill.owner_id = $1
                       AND waybill.outbound_order_id = $2
                       AND waybill.status <> 'cancelled'
                     ORDER BY waybill.created_at DESC
                     LIMIT 1
                    "#,
                )
                .bind(ctx.owner_id)
                .bind(order_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?
            } else {
                None
            };
            let (waybill_no, express_company) = carrier
                .map(|(waybill, company)| (Some(waybill), Some(company)))
                .unwrap_or((None, None));
            sqlx::query(
                r#"
                INSERT INTO shipment_confirm_erp_feedback_outbox (
                    id, owner_id, shipment_id, outbound_order_id, payload
                )
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(shipment_id)
            .bind(order_id)
            .bind(serde_json::json!({
                "warehouse_id": shipped.warehouse_id,
                "shipment_id": shipment_id,
                "outbound_order_id": shipped.id,
                "wms_order_no": shipped.wms_order_no,
                "erp_bill_code": erp.erp_bill_code,
                "revision": erp.erp_revision,
                "correlation_id": erp.erp_correlation_id,
                "line_count": lines.len(),
                "waybill_no": waybill_no,
                "express_company": express_company,
                "ship_time": now,
                "operator_name": ctx.actor_name,
                "carrier_type": req.delivery_provider_type,
                "handover_to": handover_to,
                "package_count": package_count,
                "lines": lines,
            }))
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

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
        let mut audit = audit.unwrap_or_else(|| {
            AuditWriteRequest::from_auth_context(
                ctx,
                "ship_outbound_order",
                "M4",
                "outbound_order",
                shipped.id.to_string(),
                None,
            )
        });
        audit.diff = Some(AuditDiff::compute(
            serde_json::Value::Null,
            serde_json::to_value(&shipped.shipment)
                .map_err(|error| Wave4RepositoryError::Serialize(error.to_string()))?,
        ));
        append_outbound_audit(
            &mut tx,
            ctx,
            Some(audit),
            "ship_outbound_order",
            shipped.id,
            now,
        )
        .await?;
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
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<TemperatureExcursionEvent>, i64), Wave4RepositoryError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 200);
        let offset = ((page - 1) as i64) * (page_size as i64);
        let total: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM temperature_excursion_events WHERE owner_id = $1 AND status = 'pending_disposition'",
        )
        .bind(ctx.owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        let rows = sqlx::query_as::<_, TemperatureExcursionEventRow>(
            r#"
            SELECT id, owner_id, external_event_id, device_code, location_code,
                   started_at, ended_at, min_temperature_celsius,
                   max_temperature_celsius, affected_batch_ids, status, created_at
              FROM temperature_excursion_events
             WHERE owner_id = $1 AND status = 'pending_disposition'
             ORDER BY created_at DESC, external_event_id ASC
             LIMIT $2 OFFSET $3
            "#,
        )
        .bind(ctx.owner_id)
        .bind(page_size as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        Ok((
            rows.into_iter().map(map_temperature_excursion).collect(),
            total,
        ))
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
                       qty_on_hand, qty_frozen, status, location_id, location_code,
                       recall_flag, created_at, updated_at,
                       qty_allocated, qty_replenish_in_transit, qty_replenish_out_transit
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

            let from_status = batch_row.status.clone();
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
                       SET status = $3,
                           updated_at = $4,
                           version = version + 1
                     WHERE owner_id = $1 AND id = $2
                    RETURNING id, owner_id, product_code, batch_no, production_date, expiry_date,
                              qty_on_hand, qty_frozen, status, location_id, location_code,
                              recall_flag, created_at, updated_at,
                              qty_allocated, qty_replenish_in_transit, qty_replenish_out_transit
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

    pub async fn list_driver_today_tasks(
        &self,
        ctx: &AuthContext,
    ) -> Result<DriverTaskListResponse, Wave4RepositoryError> {
        #[derive(FromRow)]
        struct DriverTaskRow {
            order_no: String,
            customer_name: Option<String>,
            delivery_address: Option<String>,
            planned_arrival_at: Option<DateTime<Utc>>,
            cold_chain: bool,
            status: String,
            owner_id: Uuid,
        }

        let rows = sqlx::query_as::<_, DriverTaskRow>(
            r#"
            SELECT
                o.wms_order_no AS order_no,
                COALESCE(c.customer_name, '未知客户') AS customer_name,
                COALESCE(
                    o.delivery_address_snapshot->>'address',
                    o.delivery_address_snapshot->>'detail',
                    o.delivery_address_snapshot->>'address_line',
                    ''
                ) AS delivery_address,
                o.required_ship_at AS planned_arrival_at,
                s.cold_chain,
                o.status,
                s.owner_id
            FROM outbound_shipments s
            JOIN outbound_orders o ON o.id = s.outbound_order_id AND o.owner_id = s.owner_id
            LEFT JOIN customers c ON c.id = o.customer_id AND c.owner_id = o.owner_id
            WHERE s.owner_id = $1
              AND (s.driver_user_id = $2 OR s.driver_user_id IS NULL)
            ORDER BY s.shipped_at DESC
            LIMIT 100
            "#,
        )
        .bind(ctx.owner_id)
        .bind(ctx.user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        let count = rows.len() as u32;
        let data = rows
            .into_iter()
            .map(|row| DriverTask {
                order_no: row.order_no,
                customer_name: row.customer_name.unwrap_or_else(|| "未知客户".to_string()),
                delivery_address: row.delivery_address.unwrap_or_default(),
                planned_arrival_at: row.planned_arrival_at,
                cold_chain: row.cold_chain,
                status: row.status,
                owner_id: row.owner_id,
            })
            .collect();

        Ok(DriverTaskListResponse {
            data,
            page: PageMeta {
                next_cursor: None,
                count,
                total: Some(count),
            },
        })
    }

    pub async fn get_store_dashboard(
        &self,
        ctx: &AuthContext,
    ) -> Result<StoreDashboardResponse, Wave4RepositoryError> {
        #[derive(FromRow)]
        struct StoreDashboardRow {
            pending_receipt_orders: i64,
            in_transit_orders: i64,
            signed_orders_last_7_days: i64,
            inventory_alert_count: i64,
            returns_this_month: i64,
            exceptions_this_month: i64,
        }

        let store_id: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id
              FROM customers
             WHERE owner_id = $1 AND customer_type = 'store'
             LIMIT 1
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?;

        let row = sqlx::query_as::<_, StoreDashboardRow>(
            r#"
            SELECT
                (SELECT COUNT(*)::bigint FROM outbound_orders WHERE owner_id = $1 AND status IN ('shipped', 'in_transit', 'delivering', 'pending_receipt')) AS pending_receipt_orders,
                (SELECT COUNT(*)::bigint FROM outbound_orders WHERE owner_id = $1 AND status IN ('shipped', 'in_transit', 'delivering')) AS in_transit_orders,
                (SELECT COUNT(*)::bigint FROM outbound_orders WHERE owner_id = $1 AND status IN ('signed', 'completed', 'delivered') AND updated_at >= NOW() - INTERVAL '7 days') AS signed_orders_last_7_days,
                (SELECT COUNT(*)::bigint FROM inventory_batches WHERE owner_id = $1 AND (status != 'qualified' OR recall_flag = TRUE)) AS inventory_alert_count,
                (SELECT COUNT(*)::bigint FROM purchase_return_orders WHERE owner_id = $1 AND created_at >= date_trunc('month', CURRENT_TIMESTAMP)) AS returns_this_month,
                (SELECT COUNT(*)::bigint FROM temperature_excursion_events WHERE owner_id = $1 AND created_at >= date_trunc('month', CURRENT_TIMESTAMP)) AS exceptions_this_month
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;

        Ok(StoreDashboardResponse {
            store_id,
            pending_receipt_orders: row.pending_receipt_orders.clamp(0, u32::MAX as i64) as u32,
            in_transit_orders: row.in_transit_orders.clamp(0, u32::MAX as i64) as u32,
            signed_orders_last_7_days: row.signed_orders_last_7_days.clamp(0, u32::MAX as i64) as u32,
            inventory_alert_count: row.inventory_alert_count.clamp(0, u32::MAX as i64) as u32,
            returns_this_month: row.returns_this_month.clamp(0, u32::MAX as i64) as u32,
            exceptions_this_month: row.exceptions_this_month.clamp(0, u32::MAX as i64) as u32,
            generated_at: Utc::now(),
        })
    }
}

#[derive(FromRow)]
struct OutboundAllocationRow {
    id: Uuid,
    batch_id: Uuid,
    allocated_qty: wms_domain::Quantity,
}

async fn append_outbound_inventory_movement(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    batch_id: Uuid,
    qty_delta: wms_domain::Quantity,
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
    // 拣选位可用量双公式：qty_on_hand - qty_allocated - qty_frozen + qty_replenish_in_transit
    let candidates: Vec<(Uuid, wms_domain::Quantity)> = sqlx::query_as(
        r#"
        SELECT id,
               qty_on_hand - qty_allocated - qty_frozen + qty_replenish_in_transit AS available_qty
          FROM inventory_batches
         WHERE owner_id = $1
           AND product_code = $2
           AND batch_no = $3
           AND status = $4
           AND recall_flag = FALSE
           AND qty_on_hand - qty_allocated - qty_frozen + qty_replenish_in_transit > 0
           AND NOT EXISTS (
                SELECT 1
                  FROM inventory_count_lines line
                  JOIN inventory_counts count_sheet
                    ON count_sheet.id = line.count_id
                   AND count_sheet.owner_id = line.owner_id
                 WHERE line.owner_id = inventory_batches.owner_id
                   AND line.inventory_batch_id = inventory_batches.id
                   AND count_sheet.status = 'in_progress'
           )
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
    let available = candidates.iter().try_fold(wms_domain::Quantity::ZERO, |total, (_, qty)| {
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
        if remaining == wms_domain::Quantity::ZERO {
            break;
        }
        let allocated_qty = available_qty.min(remaining);
        let updated = sqlx::query(
            r#"
            UPDATE inventory_batches
               SET qty_frozen = qty_frozen + $3,
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
               AND status = $5 AND recall_flag = FALSE
               AND qty_on_hand - qty_allocated - qty_frozen + qty_replenish_in_transit >= $3
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
    let allocated_qty = allocations.iter().try_fold(wms_domain::Quantity::ZERO, |total, allocation| {
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
                   qty_frozen = qty_frozen - $3,
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
               AND status = $5 AND recall_flag = FALSE
               AND qty_on_hand >= $3 AND qty_frozen >= $3
               AND NOT EXISTS (
                    SELECT 1 FROM warehouse_locations wl
                     WHERE wl.id = inventory_batches.location_id
                       AND wl.owner_id = inventory_batches.owner_id
                       AND wl.agv_unreachable_at IS NOT NULL
               )
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
