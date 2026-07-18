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
        validate_document_type(&req.document_type)?;
        validate_receiving_order_lines(&req.document_type, &req.lines)?;
        validate_create_receiving_order_request(&req, now).map_err(map_request_validation_error)?;

        let mut tx = self.begin().await?;
        validate_receiving_order_references(&mut tx, ctx.owner_id, &req).await?;
        let order = insert_receiving_order_in_tx(&mut tx, ctx, req, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(order)
    }

    pub async fn create_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        req: CreateReceivingOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: AuditWriteRequest,
    ) -> Result<IdempotentMutation<ReceivingOrder>, Wave3RepositoryError> {
        validate_document_type(&req.document_type)?;
        validate_receiving_order_lines(&req.document_type, &req.lines)?;
        validate_create_receiving_order_request(&req, now).map_err(map_request_validation_error)?;
        let request_hash = request_hash(&serde_json::json!({
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
        validate_receiving_order_references(&mut tx, ctx.owner_id, &req).await?;
        let order = insert_receiving_order_in_tx(&mut tx, ctx, req, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders",
            "receiving_order",
            order.id.to_string(),
            &order,
            now,
        )
        .await?;
        let mut audit = audit;
        audit.resource_id = order.id.to_string();
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: order,
            replayed: false,
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

        validate_receiving_gsp_fields(&req)?;
        let cold_chain = order_requires_cold_chain(&mut tx, ctx.owner_id, id).await?;
        if cold_chain {
            if req.arrival_temperature_celsius.is_none() {
                return Err(Wave3RepositoryError::MissingRequiredField(
                    "arrival_temperature_celsius".to_string(),
                ));
            }
            let control = req
                .details
                .as_ref()
                .and_then(|details| details.temperature_control_method.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if control.is_none() {
                return Err(Wave3RepositoryError::MissingRequiredField(
                    "temperature_control_method".to_string(),
                ));
            }
            if let Some(temperature) = req.arrival_temperature_celsius {
                let (lo, hi) = receiving_temperature_band(&mut tx, ctx.owner_id, id).await?;
                // 超出商品温区必须填写异常备注（稳定性报告/处置说明占位，附件链路仍待 H-FILE）。
                if !(lo..=hi).contains(&temperature)
                    && req
                        .exception_note
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .is_none()
                {
                    return Err(Wave3RepositoryError::TemperatureExcursionRequiresDisposition);
                }
            }
        }

        let receipt = ReceivingOrderReceipt {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            actual_qty: req.actual_qty,
            shortage_qty: req.shortage_qty,
            rejected_qty: req.rejected_qty,
            arrival_temperature_celsius: req.arrival_temperature_celsius,
            exception_note: req.exception_note.clone(),
            details: req.details.clone(),
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_order_receipts (
                id, receiving_order_id, owner_id, actual_qty, shortage_qty,
                rejected_qty, arrival_temperature_celsius, exception_note, receiving_details, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
        .bind(req.details.as_ref().map(|details| sqlx::types::Json(details.clone())))
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
            arrival_temperature_celsius: None,
            exception_note: Some(reason.clone()),
            details: None,
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_order_receipts (
                id, receiving_order_id, owner_id, actual_qty, shortage_qty,
                rejected_qty, arrival_temperature_celsius, exception_note, receiving_details, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NULL, $7, NULL, $8)
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
        if let Some(mut audit) = audit {
            audit.diff = Some(AuditDiff::compute(
                serde_json::json!({ "status": &order.status }),
                serde_json::json!({
                    "status": "closed_rejected",
                    "reason": &reason,
                    "rejected_qty": expected_qty,
                }),
            ));
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

        let expected_quality_color =
            resolve_quality_color(&mut tx, ctx.owner_id, &req.quality_status, now).await?;

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if order.status != "putaway" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "putaway".to_string(),
                actual: order.status,
            });
        }

        let (product_storage_condition, product_attrs): (String, serde_json::Value) =
            sqlx::query_as(
                "SELECT storage_condition, attrs FROM products WHERE owner_id = $1 AND product_code = $2 AND status = 'active'",
            )
            .bind(ctx.owner_id)
            .bind(&req.product_code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .ok_or(Wave3RepositoryError::NotFound)?;
        let unit_volume_cm3 = putaway::product_unit_volume_cm3(&product_attrs)?;
        let required_volume_cm3 = unit_volume_cm3
            .checked_mul(req.qty)
            .ok_or(Wave3RepositoryError::InvalidQuantity)?;

        let (
            location_status,
            location_quality_color,
            location_temperature_zone,
            location_max_volume_cm3,
            location_used_volume_cm3,
            location_max_sku_count,
        ): (String, String, String, i64, i64, i32) = sqlx::query_as(
            r#"
            SELECT location.status,
                   zone.quality_color,
                   zone.temperature_zone,
                   location.max_volume_cm3,
                   location.used_volume_cm3,
                   location.max_sku_count
              FROM warehouse_locations location
              JOIN warehouse_zones zone
                ON zone.id = location.zone_id
               AND zone.owner_id = location.owner_id
             WHERE location.id = $1
               AND location.owner_id = $2
               AND location.warehouse_id = $3
               AND location.location_code = $4
               AND (location.bound_owner_id IS NULL OR location.bound_owner_id = $2)
               AND zone.status = 'active'
             FOR UPDATE
            "#,
        )
        .bind(req.location_id)
        .bind(ctx.owner_id)
        .bind(order.warehouse_id)
        .bind(&req.location_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        if !matches!(location_status.as_str(), "available" | "occupied") {
            return Err(Wave3RepositoryError::InvalidLocation);
        }
        if location_quality_color != expected_quality_color {
            return Err(Wave3RepositoryError::LocationQualityMismatch);
        }
        if location_temperature_zone != product_storage_condition {
            return Err(Wave3RepositoryError::LocationTemperatureMismatch);
        }
        let available_volume_cm3 = location_max_volume_cm3
            .checked_sub(location_used_volume_cm3)
            .ok_or(Wave3RepositoryError::LocationCapacityExceeded)?;
        if required_volume_cm3 > available_volume_cm3 {
            return Err(Wave3RepositoryError::LocationCapacityExceeded);
        }
        let line = sqlx::query_as::<_, ReceivingOrderLineRow>(
            r#"
            SELECT id, line_no, product_id, product_code, expected_qty, batch_no,
                   production_date, expiry_date
              FROM receiving_order_lines
             WHERE receiving_order_id = $1
               AND owner_id = $2
               AND product_code = $3
               AND batch_no = $4
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
        let accepted_qty: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(accepted_qty), 0)::BIGINT FROM receiving_inspections WHERE receiving_order_id = $1 AND owner_id = $2 AND batch_no = $3",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.batch_no)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if accepted_qty <= 0 {
            return Err(Wave3RepositoryError::NotFound);
        }
        let quality_status_matches: bool = sqlx::query_scalar(
            r#"
            SELECT NOT EXISTS (
                SELECT 1
                  FROM receiving_inspections
                 WHERE receiving_order_id = $1
                   AND owner_id = $2
                   AND batch_no = $3
                   AND accepted_qty > 0
                   AND quality_status <> $4
            )
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.batch_no)
        .bind(&req.quality_status)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if !quality_status_matches {
            return Err(Wave3RepositoryError::InvalidQualityStatus);
        }
        let existing_putaway_qty: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(qty), 0)::BIGINT FROM receiving_putaways WHERE receiving_order_id = $1 AND owner_id = $2 AND product_code = $3 AND batch_no = $4",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.product_code)
        .bind(&req.batch_no)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let remaining_qty = accepted_qty
            .checked_sub(existing_putaway_qty)
            .ok_or(Wave3RepositoryError::QuantityClosureMismatch)?;
        if req.qty > remaining_qty {
            return Err(Wave3RepositoryError::QuantityClosureMismatch);
        }

        let same_product_at_location: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM inventory_batches WHERE owner_id = $1 AND location_id = $2 AND product_code = $3)",
        )
        .bind(ctx.owner_id)
        .bind(req.location_id)
        .bind(&req.product_code)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if !same_product_at_location {
            let sku_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(DISTINCT product_code)::BIGINT FROM inventory_batches WHERE owner_id = $1 AND location_id = $2",
            )
            .bind(ctx.owner_id)
            .bind(req.location_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            if sku_count >= i64::from(location_max_sku_count) {
                return Err(Wave3RepositoryError::LocationSkuLimitExceeded);
            }
        }

        let lpn_code = req
            .lpn_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let putaway = PutawayRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            batch_no: req.batch_no.clone(),
            product_code: req.product_code.clone(),
            qty: req.qty,
            location_id: req.location_id,
            location_code: req.location_code.clone(),
            lpn_code: lpn_code.clone(),
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_putaways (
                id, receiving_order_id, owner_id, batch_no, product_code,
                qty, location_id, location_code, quality_status, lpn_code, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
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
        .bind(&putaway.lpn_code)
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
            location_code: Some(req.location_code.clone()),
            from_location_code: None,
            to_location_code: Some(req.location_code.clone()),
            lpn_code: lpn_code.clone(),
            operator_user_id: Some(ctx.user_id),
            operator_name: Some(ctx.actor_name.clone()),
            volume_delta_cm3: None,
            product_code: Some(req.product_code.clone()),
            product_name: None,
            batch_no: Some(req.batch_no.clone()),
            expiry_date: Some(inventory_batch.expiry_date.clone()),
        };
        sqlx::query(
            r#"
            INSERT INTO inventory_movements (
                id, owner_id, batch_id, movement_type, qty_delta,
                source_document_type, source_document_id, occurred_at,
                location_code, from_location_code, to_location_code,
                lpn_code, operator_user_id, operator_name, volume_delta_cm3
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
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
        .bind(&movement.location_code)
        .bind(&movement.from_location_code)
        .bind(&movement.to_location_code)
        .bind(&movement.lpn_code)
        .bind(movement.operator_user_id)
        .bind(&movement.operator_name)
        .bind(movement.volume_delta_cm3)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        // M2 上架完成 → ERP 反馈 outbox（本地闭环标记；外部投递仍待 S4）
        sqlx::query(
            r#"
            INSERT INTO receiving_putaway_erp_feedback_outbox (
                id, owner_id, putaway_id, receiving_order_id, batch_id,
                event_type, payload, status, attempt_count, next_attempt_at,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, 'inbound_putaway_completed', $6,
                'pending', 0, $7, $7, $7
            )
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(putaway.id)
        .bind(id)
        .bind(inventory_batch.id)
        .bind(serde_json::json!({
            "product_code": req.product_code,
            "batch_no": req.batch_no,
            "qty": req.qty,
            "location_code": req.location_code,
            "lpn_code": lpn_code,
            "quality_status": req.quality_status,
        }))
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let location_updated = sqlx::query(
            r#"
            UPDATE warehouse_locations
               SET used_volume_cm3 = used_volume_cm3 + $1,
                   status = 'occupied',
                   updated_at = $2,
                   version = version + 1
             WHERE id = $3
               AND owner_id = $4
               AND warehouse_id = $5
               AND used_volume_cm3 + $1 <= max_volume_cm3
            "#,
        )
        .bind(required_volume_cm3)
        .bind(now)
        .bind(req.location_id)
        .bind(ctx.owner_id)
        .bind(order.warehouse_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if location_updated.rows_affected() != 1 {
            return Err(Wave3RepositoryError::LocationCapacityExceeded);
        }

        let accepted_total: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(accepted_qty), 0)::BIGINT FROM receiving_inspections WHERE receiving_order_id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let putaway_total: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(qty), 0)::BIGINT FROM receiving_putaways WHERE receiving_order_id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let next_status = if accepted_total > 0 && putaway_total >= accepted_total {
            "completed"
        } else {
            "putaway"
        };
        sqlx::query(
            "UPDATE receiving_orders SET status = $3, updated_at = $4, version = version + 1 WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(next_status)
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

async fn validate_receiving_order_references(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    req: &CreateReceivingOrderRequest,
) -> Result<(), Wave3RepositoryError> {
    let supplier_id = req
        .supplier_id
        .ok_or(Wave3RepositoryError::MissingSupplier)?;
    let supplier_row: Option<(String, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT status, qualification_valid_until FROM suppliers WHERE id = $1 AND owner_id = $2",
    )
    .bind(supplier_id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((supplier_status, qualification_valid_until)) = supplier_row else {
        return Err(Wave3RepositoryError::NotFound);
    };
    if supplier_status != "active" {
        return Err(Wave3RepositoryError::NotFound);
    }
    if qualification_valid_until.is_some_and(|until| until < chrono::Utc::now()) {
        return Err(Wave3RepositoryError::SupplierQualificationExpired);
    }

    for line in &req.lines {
        let active_product_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM products WHERE owner_id = $1 AND product_code = $2 AND status = 'active'",
        )
        .bind(owner_id)
        .bind(&line.product_code)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
        let Some(active_product_id) = active_product_id else {
            return Err(Wave3RepositoryError::NotFound);
        };
        if line.product_id.is_some_and(|product_id| product_id != active_product_id) {
            return Err(Wave3RepositoryError::NotFound);
        }
    }
    Ok(())
}
