impl PgWave3Repository {
pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_receiving_order(
        &self,
        ctx: &AuthContext,
        req: CreateReceivingOrderRequest,
        now: DateTime<Utc>,
    ) -> Result<ReceivingOrder, Wave3RepositoryError> {
        if req.lines.is_empty() {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        validate_document_type(&req.document_type)?;

        let mut tx = self.begin().await?;
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO receiving_orders (
                id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
                external_ref, status, expected_arrival_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'draft', $8, $9, $9)
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.receipt_no)
        .bind(&req.document_type)
        .bind(req.supplier_id)
        .bind(req.warehouse_id)
        .bind(&req.external_ref)
        .bind(req.expected_arrival_at)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        insert_receiving_order_lines(&mut tx, ctx.owner_id, id, &req.lines).await?;

        tx.commit().await.map_err(map_db_error)?;
        Ok(ReceivingOrder {
            id,
            owner_id: ctx.owner_id,
            receipt_no: req.receipt_no,
            document_type: req.document_type,
            supplier_id: req.supplier_id,
            warehouse_id: req.warehouse_id,
            external_ref: req.external_ref,
            status: "draft".to_string(),
            expected_arrival_at: req.expected_arrival_at,
            lines: req.lines,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn list_receiving_orders(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<ReceivingOrder>, Wave3RepositoryError> {
        let rows = sqlx::query_as::<_, ReceivingOrderRow>(
            r#"
            SELECT id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
                   external_ref, status, expected_arrival_at, created_at, updated_at
              FROM receiving_orders
             WHERE owner_id = $1
             ORDER BY updated_at DESC, receipt_no
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        let mut orders = Vec::with_capacity(rows.len());
        for row in rows {
            let lines = self
                .load_receiving_order_lines(ctx.owner_id, row.id)
                .await?;
            orders.push(map_receiving_order(row, lines));
        }
        Ok(orders)
    }

    pub async fn receive_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: ReceiveReceivingOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<ReceivingOrderReceipt>, Wave3RepositoryError> {
        if req.actual_qty < 0 || req.shortage_qty < 0 || req.rejected_qty < 0 {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let request_hash = request_hash(&serde_json::json!({
            "receiving_order_id": id,
            "request": req,
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

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if order.status != "released" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "released".to_string(),
                actual: order.status,
            });
        }
        let expected_qty: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(expected_qty), 0)::BIGINT FROM receiving_order_lines WHERE receiving_order_id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if req.actual_qty > expected_qty {
            return Err(Wave3RepositoryError::OverReceiptNotAllowed);
        }
        if req.actual_qty + req.shortage_qty + req.rejected_qty != expected_qty {
            return Err(Wave3RepositoryError::QuantityClosureMismatch);
        }

        let receipt = ReceivingOrderReceipt {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            actual_qty: req.actual_qty,
            shortage_qty: req.shortage_qty,
            rejected_qty: req.rejected_qty,
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_order_receipts (
                id, receiving_order_id, owner_id, actual_qty, shortage_qty,
                rejected_qty, arrival_temperature_celsius, exception_note, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(receipt.id)
        .bind(receipt.receiving_order_id)
        .bind(receipt.owner_id)
        .bind(receipt.actual_qty)
        .bind(receipt.shortage_qty)
        .bind(receipt.rejected_qty)
        .bind(req.arrival_temperature_celsius)
        .bind(&req.exception_note)
        .bind(receipt.occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_receipt_insert_error)?;

        sqlx::query(
            "UPDATE receiving_orders SET status = 'inspecting', updated_at = $3, version = version + 1 WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/receive",
            "receiving_order_receipt",
            receipt.id.to_string(),
            &receipt,
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
            value: receipt,
            replayed: false,
        })
    }

    pub async fn reject_receiving_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: RejectReceivingOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<ReceivingOrderReceipt, Wave3RepositoryError> {
        Ok(self
            .reject_receiving_order_with_audit(ctx, id, req, now, idempotency_key, None)
            .await?
            .value)
    }

    pub async fn reject_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: RejectReceivingOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<ReceivingOrderReceipt>, Wave3RepositoryError> {
        let reason = req.reason.trim().to_string();
        if reason.is_empty() {
            return Err(Wave3RepositoryError::InvalidReason);
        }
        let request_hash = request_hash(&serde_json::json!({
            "receiving_order_id": id,
            "request": req,
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

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if order.status != "released" && order.status != "receiving" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "released/receiving".to_string(),
                actual: order.status,
            });
        }
        let expected_qty: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(expected_qty), 0)::BIGINT FROM receiving_order_lines WHERE receiving_order_id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let receipt = ReceivingOrderReceipt {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            actual_qty: 0,
            shortage_qty: 0,
            rejected_qty: expected_qty,
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_order_receipts (
                id, receiving_order_id, owner_id, actual_qty, shortage_qty,
                rejected_qty, arrival_temperature_celsius, exception_note, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NULL, $7, $8)
            "#,
        )
        .bind(receipt.id)
        .bind(receipt.receiving_order_id)
        .bind(receipt.owner_id)
        .bind(receipt.actual_qty)
        .bind(receipt.shortage_qty)
        .bind(receipt.rejected_qty)
        .bind(&reason)
        .bind(receipt.occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_receipt_insert_error)?;

        sqlx::query(
            "UPDATE receiving_orders SET status = 'closed_rejected', updated_at = $3, version = version + 1 WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/reject",
            "receiving_order_receipt",
            receipt.id.to_string(),
            &receipt,
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
            value: receipt,
            replayed: false,
        })
    }

    pub async fn putaway_receiving_order_and_inventory(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: PutawayRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<PutawayInventoryCommit, Wave3RepositoryError> {
        Ok(self
            .putaway_receiving_order_and_inventory_with_audit(
                ctx,
                id,
                req,
                now,
                idempotency_key,
                None,
            )
            .await?
            .value)
    }

    pub async fn putaway_receiving_order_and_inventory_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: PutawayRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PutawayInventoryCommit>, Wave3RepositoryError> {
        if req.qty <= 0 {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let request_hash = request_hash(&serde_json::json!({
            "receiving_order_id": id,
            "request": req,
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

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if order.status != "putaway" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "putaway".to_string(),
                actual: order.status,
            });
        }
        let line = sqlx::query_as::<_, ReceivingOrderLineRow>(
            r#"
            SELECT line_no, product_id, product_code, expected_qty, batch_no,
                   production_date, expiry_date
              FROM receiving_order_lines
             WHERE receiving_order_id = $1
               AND owner_id = $2
               AND product_code = $3
               AND (batch_no = $4 OR batch_no IS NULL)
             ORDER BY line_no
             LIMIT 1
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.product_code)
        .bind(&req.batch_no)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let production_date = line
            .production_date
            .ok_or_else(|| Wave3RepositoryError::InvalidDate("production_date".to_string()))?;
        let expiry_date = line
            .expiry_date
            .ok_or_else(|| Wave3RepositoryError::InvalidDate("expiry_date".to_string()))?;

        let putaway = PutawayRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            batch_no: req.batch_no.clone(),
            product_code: req.product_code.clone(),
            qty: req.qty,
            location_id: req.location_id,
            location_code: req.location_code.clone(),
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_putaways (
                id, receiving_order_id, owner_id, batch_no, product_code,
                qty, location_id, location_code, quality_status, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(putaway.id)
        .bind(putaway.receiving_order_id)
        .bind(putaway.owner_id)
        .bind(&putaway.batch_no)
        .bind(&putaway.product_code)
        .bind(putaway.qty)
        .bind(putaway.location_id)
        .bind(&putaway.location_code)
        .bind(&req.quality_status)
        .bind(putaway.occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let batch_row = sqlx::query_as::<_, InventoryBatchRow>(
            r#"
            INSERT INTO inventory_batches (
                id, owner_id, product_code, batch_no, production_date, expiry_date,
                qty_on_hand, qty_locked, quality_status, location_id, location_code,
                recall_flag, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, $9, $10, FALSE, $11, $11)
            ON CONFLICT (owner_id, product_code, batch_no, location_id, quality_status)
            DO UPDATE SET
                qty_on_hand = inventory_batches.qty_on_hand + EXCLUDED.qty_on_hand,
                updated_at = EXCLUDED.updated_at,
                version = inventory_batches.version + 1
            RETURNING id, owner_id, product_code, batch_no, production_date, expiry_date,
                      qty_on_hand, qty_locked, quality_status, location_id, location_code,
                      recall_flag, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(&req.product_code)
        .bind(&req.batch_no)
        .bind(production_date)
        .bind(expiry_date)
        .bind(req.qty)
        .bind(if req.quality_status.is_empty() {
            STATUS_QUALIFIED
        } else {
            &req.quality_status
        })
        .bind(req.location_id)
        .bind(&req.location_code)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let inventory_batch = map_inventory_batch(batch_row);

        let movement = InventoryMovement {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            batch_id: inventory_batch.id,
            movement_type: "inbound_putaway".to_string(),
            qty_delta: req.qty,
            source_document_type: "receiving_order".to_string(),
            source_document_id: id,
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO inventory_movements (
                id, owner_id, batch_id, movement_type, qty_delta,
                source_document_type, source_document_id, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(movement.id)
        .bind(movement.owner_id)
        .bind(movement.batch_id)
        .bind(&movement.movement_type)
        .bind(movement.qty_delta)
        .bind(&movement.source_document_type)
        .bind(movement.source_document_id)
        .bind(movement.occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        sqlx::query(
            "UPDATE receiving_orders SET status = 'completed', updated_at = $3, version = version + 1 WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let result = PutawayInventoryCommit {
            putaway,
            inventory_batch,
            inventory_movement: movement,
        };
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/putaway",
            "receiving_putaway",
            result.putaway.id.to_string(),
            &result,
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
            value: result,
            replayed: false,
        })
    }
}
