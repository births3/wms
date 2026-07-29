// @governance: skip-page-size outbound create, wave, pick, and review transactions stay auditable together.
impl PgWave4Repository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_outbound_orders(
        &self,
        ctx: &AuthContext,
        status: Option<&str>,
        q: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<OutboundOrder>, Wave4RepositoryError> {
        let status = non_empty_filter(status);
        let q = non_empty_filter(q);
        let limit = i64::from(limit.unwrap_or(50).clamp(1, 200));
        let rows = sqlx::query_as::<_, OutboundOrderRow>(
            r#"
            SELECT id, owner_id, document_type, wms_order_no, erp_order_no,
                   invoice_no, transport_mode_code, department_code, sales_group_code,
                   order_group_no, business_type_code, customer_id,
                   delivery_address_id, delivery_address_snapshot, warehouse_id,
                   required_ship_at, status, short_pick,
                   created_at, updated_at
              FROM outbound_orders
             WHERE owner_id = $1
               AND ($2::TEXT IS NULL OR status = $2)
               AND (
                    $3::TEXT IS NULL
                    OR wms_order_no ILIKE '%' || $3 || '%'
                    OR erp_order_no ILIKE '%' || $3 || '%'
               )
             ORDER BY updated_at DESC, wms_order_no ASC
             LIMIT $4
            "#,
        )
        .bind(ctx.owner_id)
        .bind(status)
        .bind(q)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        let mut orders = Vec::with_capacity(rows.len());
        for row in rows {
            let lines =
                load_outbound_order_lines_from_pool(&self.pool, ctx.owner_id, row.id).await?;
            let shipment =
                load_outbound_shipment_from_pool(&self.pool, ctx.owner_id, row.id).await?;
            orders.push(map_outbound_order(row, lines, shipment));
        }
        Ok(orders)
    }

    pub async fn get_outbound_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<OutboundOrder, Wave4RepositoryError> {
        let row = sqlx::query_as::<_, OutboundOrderRow>(
            r#"
            SELECT id, owner_id, document_type, wms_order_no, erp_order_no,
                   invoice_no, transport_mode_code, department_code, sales_group_code,
                   order_group_no, business_type_code, customer_id,
                   delivery_address_id, delivery_address_snapshot, warehouse_id,
                   required_ship_at, status, short_pick,
                   created_at, updated_at
              FROM outbound_orders
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;
        let lines = load_outbound_order_lines_from_pool(&self.pool, ctx.owner_id, id).await?;
        let shipment = load_outbound_shipment_from_pool(&self.pool, ctx.owner_id, id).await?;
        Ok(map_outbound_order(row, lines, shipment))
    }

    pub async fn create_outbound_order(
        &self,
        ctx: &AuthContext,
        req: CreateOutboundOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundOrder>, Wave4RepositoryError> {
        if req.lines.is_empty() || req.lines.iter().any(|line| line.planned_qty <= 0) {
            return Err(Wave4RepositoryError::InvalidQuantity);
        }
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;

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

        let order_id = Uuid::new_v4();
        ensure_outbound_document_type(&mut tx, ctx.owner_id, &req.document_type).await?;
        let delivery_address_snapshot: serde_json::Value = sqlx::query_scalar(
            r#"
            SELECT jsonb_build_object(
                'province', province,
                'city', city,
                'district', district,
                'detail_address', detail_address,
                'contact_name', contact_name,
                'contact_phone', contact_phone
            )
              FROM customer_addresses
             WHERE owner_id = $1 AND customer_id = $2 AND id = $3
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.customer_id)
        .bind(req.delivery_address_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::InvalidDeliveryAddress)?;
        let wms_order_no = if req.wms_order_no.trim().is_empty() {
            PgDocumentNumberingService::new()
                .generate_in_tx(
                    &mut tx,
                    ctx,
                    GenerateDocumentNumberRequest {
                        document_type: req.document_type.clone(),
                        idempotency_key: format!("m4-outbound-create:{order_id}"),
                        source_module: "M4".to_string(),
                        source_document_id: Some(order_id),
                    },
                    now,
                )
                .await
                .map_err(|error| Wave4RepositoryError::DocumentNumbering(format!("{error:?}")))?
                .value
                .generated_no
        } else {
            req.wms_order_no.clone()
        };
        sqlx::query(
            r#"
            INSERT INTO outbound_orders (
                id, owner_id, document_type, wms_order_no, erp_order_no,
                invoice_no, transport_mode_code, department_code, sales_group_code,
                order_group_no, business_type_code, customer_id,
                delivery_address_id, delivery_address_snapshot, warehouse_id,
                required_ship_at, status, short_pick, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                $12, $13, $14, $15, $16, $17, FALSE, $18, $18
            )
            "#,
        )
        .bind(order_id)
        .bind(ctx.owner_id)
        .bind(&req.document_type)
        .bind(&wms_order_no)
        .bind(&req.erp_order_no)
        .bind(&req.invoice_no)
        .bind(&req.transport_mode_code)
        .bind(&req.department_code)
        .bind(&req.sales_group_code)
        .bind(&req.order_group_no)
        .bind(&req.business_type_code)
        .bind(req.customer_id)
        .bind(req.delivery_address_id)
        .bind(delivery_address_snapshot)
        .bind(req.warehouse_id)
        .bind(req.required_ship_at)
        .bind(OUTBOUND_STATUS_CONFIRMED)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_insert_error)?;

        freeze_outbound_route_in_tx(
            &mut tx,
            ctx.owner_id,
            order_id,
            req.warehouse_id,
            req.customer_id,
            req.delivery_address_id,
            now,
        )
        .await
        .map_err(map_route_freeze_error)?;

        for line in &req.lines {
            sqlx::query(
                r#"
                INSERT INTO outbound_order_lines (
                    id, outbound_order_id, owner_id, line_no, product_code,
                    batch_no, planned_qty
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(order_id)
            .bind(ctx.owner_id)
            .bind(i32::try_from(line.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?)
            .bind(&line.product_code)
            .bind(&line.batch_no)
            .bind(line.planned_qty)
            .execute(&mut *tx)
            .await
            .map_err(map_insert_error)?;
        }

        let order = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/orders",
            "outbound_order",
            order.id.to_string(),
            &order,
            now,
        )
        .await?;
        append_outbound_audit(&mut tx, ctx, audit, "create_outbound_order", order.id, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: order,
            replayed: false,
        })
    }

    pub async fn create_outbound_wave(
        &self,
        ctx: &AuthContext,
        req: CreateOutboundWaveRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundWave>, Wave4RepositoryError> {
        if req.order_ids.is_empty() {
            return Err(Wave4RepositoryError::EmptySelection);
        }
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;

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

        let wave_row = sqlx::query_as::<_, OutboundWaveRow>(
            r#"
            INSERT INTO outbound_waves (id, owner_id, wave_no, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $5)
            RETURNING id, owner_id, wave_no, status, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(&req.wave_no)
        .bind("released")
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_insert_error)?;

        let mut selected_order_ids = HashSet::with_capacity(req.order_ids.len());
        let mut task_drafts = Vec::new();
        for order_id in &req.order_ids {
            if !selected_order_ids.insert(*order_id) {
                return Err(Wave4RepositoryError::OrderAlreadyInWave);
            }
            let order = lock_outbound_order(&mut tx, ctx.owner_id, *order_id).await?;
            let already_in_wave: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM outbound_wave_orders WHERE owner_id = $1 AND outbound_order_id = $2)",
            )
            .bind(ctx.owner_id)
            .bind(order.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            if already_in_wave {
                return Err(Wave4RepositoryError::OrderAlreadyInWave);
            }
            if order.status != OUTBOUND_STATUS_CONFIRMED {
                return Err(Wave4RepositoryError::InvalidStatus {
                    expected: OUTBOUND_STATUS_CONFIRMED.to_string(),
                    actual: order.status,
                });
            }
            let order_with_lines = load_outbound_order(&mut tx, ctx.owner_id, order.id).await?;
            for line in &order_with_lines.lines {
                allocate_inventory_for_outbound(&mut tx, ctx.owner_id, order.id, line, now).await?;
            }
            task_drafts.extend(
                collect_locked_allocation_task_drafts(
                    &mut tx,
                    ctx.owner_id,
                    order.id,
                    &order.wms_order_no,
                )
                .await?,
            );
            sqlx::query(
                r#"
                UPDATE outbound_orders
                   SET status = $3, updated_at = $4, version = version + 1
                 WHERE owner_id = $1 AND id = $2
                "#,
            )
            .bind(ctx.owner_id)
            .bind(order.id)
            .bind(OUTBOUND_STATUS_IN_WAVE)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            sqlx::query(
                r#"
                INSERT INTO outbound_wave_orders (id, owner_id, wave_id, outbound_order_id, created_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(wave_row.id)
            .bind(order.id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_insert_error)?;
        }

        insert_wave_pick_tasks(&mut tx, ctx, wave_row.id, task_drafts, now).await?;

        let wave = map_outbound_wave(wave_row, req.order_ids);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/waves",
            "outbound_wave",
            wave.id.to_string(),
            &wave,
            now,
        )
        .await?;
        append_outbound_audit(&mut tx, ctx, audit, "create_outbound_wave", wave.id, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: wave,
            replayed: false,
        })
    }

    pub async fn complete_pick_task(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        req: CompletePickTaskRequest,
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

        let order = lock_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        if !matches!(
            order.status.as_str(),
            OUTBOUND_STATUS_IN_WAVE | "picked" | "picked_short" | OUTBOUND_STATUS_REVIEWED_SHORT
        ) {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "in_wave|picked|picked_short|reviewed_short".to_string(),
                actual: order.status,
            });
        }

        let planned_qty: i64 = sqlx::query_scalar(
            r#"
            SELECT planned_qty
              FROM outbound_order_lines
             WHERE owner_id = $1 AND outbound_order_id = $2 AND line_no = $3
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(i32::try_from(req.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;
        let short_qty = short_pick_qty(planned_qty, req.picked_qty)
            .map_err(|_| Wave4RepositoryError::InvalidQuantity)?;

        sqlx::query(
            r#"
            UPDATE outbound_order_lines
               SET picked_qty = $4,
                   short_pick_qty = $5
             WHERE owner_id = $1 AND outbound_order_id = $2 AND line_no = $3
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(i32::try_from(req.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?)
        .bind(req.picked_qty)
        .bind(short_qty)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let mut updated = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        let next_status = status_after_pick(&updated.lines);
        let short_pick = updated.lines.iter().any(|line| line.short_pick_qty > 0);
        sqlx::query(
            r#"
            UPDATE outbound_orders
               SET status = $3,
                   short_pick = $4,
                   updated_at = $5,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(next_status)
        .bind(short_pick)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        updated = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/pick-tasks/{id}/complete",
            "outbound_order",
            updated.id.to_string(),
            &updated,
            now,
        )
        .await?;
        append_outbound_audit(&mut tx, ctx, audit, "complete_pick_task", updated.id, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: updated,
            replayed: false,
        })
    }

    pub async fn review_outbound_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        req: ReviewOutboundOrderRequest,
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
        if !matches!(order_row.status.as_str(), "picked" | "picked_short") {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "picked|picked_short".to_string(),
                actual: order_row.status,
            });
        }

        let order = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        let pick_operator_ids =
            load_outbound_pick_operator_ids(&mut tx, ctx.owner_id, order_id).await?;
        validate_review_submission(&order.lines, &req, ctx.user_id, &pick_operator_ids)
            .map_err(Wave4RepositoryError::ReviewValidation)?;

        let (strategy, approval_record_id) = integrations::resolve_outbound_review_policy(
            &mut tx,
            ctx,
            order_id,
            &order,
            req.second_reviewer_id,
        )
        .await?;

        for line in &req.lines {
            let affected = sqlx::query(
                r#"
                UPDATE outbound_order_lines
                   SET reviewed_qty = $4
                 WHERE owner_id = $1 AND outbound_order_id = $2 AND line_no = $3
                "#,
            )
            .bind(ctx.owner_id)
            .bind(order_id)
            .bind(i32::try_from(line.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?)
            .bind(line.reviewed_qty)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            if affected.rows_affected() != 1 {
                return Err(Wave4RepositoryError::NotFound);
            }
        }

        let mut updated = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        let next_status = status_after_review(&updated.lines);
        sqlx::query(
            r#"
            UPDATE outbound_orders
               SET status = $3,
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(next_status)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        updated = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;

        let audit = audit.map(|mut audit| {
            audit.diff = Some(AuditDiff::compute(
                serde_json::json!({
                    "status": &order.status,
                    "lines": order.lines.iter().map(|line| {
                        serde_json::json!({
                            "line_no": line.line_no,
                            "reviewed_qty": line.reviewed_qty,
                        })
                    }).collect::<Vec<_>>(),
                }),
                serde_json::json!({
                    "status": &updated.status,
                    "reviewer_id": req.reviewer_id,
                    "review_mode": &req.review_mode,
                    "second_reviewer_id": &req.second_reviewer_id,
                    "strategy_rule_id": strategy.source_rule_id,
                    "approval_record_id": approval_record_id,
                    "lines": req.lines.iter().map(|line| {
                        serde_json::json!({
                            "line_no": line.line_no,
                            "product_code": &line.product_code,
                            "reviewed_qty": line.reviewed_qty,
                        })
                    }).collect::<Vec<_>>(),
                }),
            ));
            audit
        });

        sqlx::query(
            r#"
            INSERT INTO outbound_review_records (
                id, owner_id, outbound_order_id, review_mode, first_reviewer_id,
                second_reviewer_id, strategy_rule_id, approval_record_id,
                reviewed_at, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(&req.review_mode)
        .bind(req.reviewer_id)
        .bind(req.second_reviewer_id)
        .bind(strategy.source_rule_id)
        .bind(approval_record_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        if updated.status == OUTBOUND_STATUS_REVIEWED {
            let task_warehouse_id: Uuid = sqlx::query_scalar(
                r#"
                SELECT warehouse_id
                  FROM (
                        SELECT task.warehouse_id, 1 AS source_rank
                          FROM warehouse_tasks task
                         WHERE task.owner_id = $1
                           AND task.source_doc_type = 'outbound_order'
                           AND task.source_doc_id = $2
                           AND task.task_type_code = 'pick'
                        UNION ALL
                        SELECT location.warehouse_id, 2 AS source_rank
                          FROM inventory_allocations allocation
                          JOIN inventory_batches batch
                            ON batch.owner_id = allocation.owner_id
                           AND batch.id = allocation.batch_id
                          JOIN warehouse_locations location
                            ON location.owner_id = batch.owner_id
                           AND location.id = batch.location_id
                         WHERE allocation.owner_id = $1
                           AND allocation.outbound_order_id = $2
                       ) candidate
                 ORDER BY source_rank, warehouse_id
                 LIMIT 1
                "#,
            )
            .bind(ctx.owner_id)
            .bind(order_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .unwrap_or(order.warehouse_id);
            for line in updated.lines.iter().filter(|line| line.reviewed_qty > 0) {
                crate::task_engine::create_task_in_tx(
                    &mut tx,
                    ctx,
                    &wms_domain::CreateWarehouseTaskRequest {
                        task_type_code: wms_domain::TASK_TYPE_LOADING.to_string(),
                        source_module: "M4".to_string(),
                        source_doc_type: "outbound_order".to_string(),
                        source_doc_id: Some(order_id),
                        source_doc_no: updated.wms_order_no.clone(),
                        source_line_no: Some(
                            i32::try_from(line.line_no)
                                .map_err(|_| Wave4RepositoryError::InvalidQuantity)?,
                        ),
                        source_task_key: format!("M4:loading:{order_id}:{}", line.line_no),
                        warehouse_id: task_warehouse_id,
                        task_group_code: crate::task_engine::default_task_group_code(
                            task_warehouse_id,
                        ),
                        product_id: None,
                        product_code: line.product_code.clone(),
                        batch_id: None,
                        batch_no: Some(line.batch_no.clone()),
                        planned_qty: line.reviewed_qty,
                        source_location_id: None,
                        source_location_code: None,
                        target_location_id: None,
                        target_location_code: None,
                        priority: None,
                        urgent_order: false,
                        predecessor_task_id: None,
                    },
                    now,
                )
                .await
                .map_err(|error| {
                    Wave4RepositoryError::Database(format!("M-TE 装车任务创建失败: {error:?}"))
                })?;
            }
        }

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/orders/{id}/review",
            "outbound_order",
            updated.id.to_string(),
            &updated,
            now,
        )
        .await?;
        append_outbound_audit(
            &mut tx,
            ctx,
            audit,
            "review_outbound_order",
            updated.id,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: updated,
            replayed: false,
        })
    }
}
