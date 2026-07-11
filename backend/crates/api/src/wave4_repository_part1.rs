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
            SELECT id, owner_id, wms_order_no, erp_order_no, customer_id,
                   warehouse_id, required_ship_at, status, short_pick,
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
            orders.push(map_outbound_order(row, lines));
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
            SELECT id, owner_id, wms_order_no, erp_order_no, customer_id,
                   warehouse_id, required_ship_at, status, short_pick,
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
        Ok(map_outbound_order(row, lines))
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
        sqlx::query(
            r#"
            INSERT INTO outbound_orders (
                id, owner_id, wms_order_no, erp_order_no, customer_id, warehouse_id,
                required_ship_at, status, short_pick, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, FALSE, $9, $9)
            "#,
        )
        .bind(order_id)
        .bind(ctx.owner_id)
        .bind(&req.wms_order_no)
        .bind(&req.erp_order_no)
        .bind(req.customer_id)
        .bind(req.warehouse_id)
        .bind(req.required_ship_at)
        .bind(OUTBOUND_STATUS_CONFIRMED)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_insert_error)?;

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

        for order_id in &req.order_ids {
            let order = lock_outbound_order(&mut tx, ctx.owner_id, *order_id).await?;
            if order.status != OUTBOUND_STATUS_CONFIRMED {
                return Err(Wave4RepositoryError::InvalidStatus {
                    expected: OUTBOUND_STATUS_CONFIRMED.to_string(),
                    actual: order.status,
                });
            }
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

        let order = lock_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        if !matches!(order.status.as_str(), "picked" | "picked_short") {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "picked|picked_short".to_string(),
                actual: order.status,
            });
        }
        sqlx::query(
            r#"
            UPDATE outbound_order_lines
               SET reviewed_qty = picked_qty
             WHERE owner_id = $1 AND outbound_order_id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

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
