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

    /// offset 分页列表（契约见 docs/api/api-pagination-standards.md）：
    /// 主列表 SQL 加 LIMIT/OFFSET + 同过滤 count(*)，返回 (本页数据, 总条数)。
    /// 明细拉取保持按单（本页单数），分页后 N+1 范围缩小到本页，语义不变。
    pub async fn list_receiving_orders(
        &self,
        ctx: &AuthContext,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<ReceivingOrder>, i64), Wave3RepositoryError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 200);
        let offset = ((page - 1) as i64) * (page_size as i64);
        let total: i64 =
            sqlx::query_scalar("SELECT count(*) FROM receiving_orders WHERE owner_id = $1")
                .bind(ctx.owner_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_db_error)?;
        let rows = sqlx::query_as::<_, ReceivingOrderRow>(
            r#"
            SELECT id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
                   external_ref, status, expected_arrival_at, created_at, updated_at
              FROM receiving_orders
             WHERE owner_id = $1
             ORDER BY updated_at DESC, receipt_no
             LIMIT $2 OFFSET $3
            "#,
        )
        .bind(ctx.owner_id)
        .bind(page_size as i64)
        .bind(offset)
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
        Ok((orders, total))
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
        if req.actual_qty < wms_domain::Quantity::ZERO || req.shortage_qty < wms_domain::Quantity::ZERO || req.rejected_qty < wms_domain::Quantity::ZERO {
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
        let pending_cancel: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (
                   SELECT 1
                     FROM erp_order_cancel_commands command
                     JOIN receiving_orders receiving
                       ON receiving.owner_id=command.owner_id
                      AND receiving.erp_bill_code=command.erp_bill_code
                      AND receiving.erp_revision=command.revision
                    WHERE receiving.owner_id=$1 AND receiving.id=$2
                      AND command.order_type=1 AND command.status='pending'
               )"#,
        )
        .bind(ctx.owner_id)
        .bind(order.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if pending_cancel {
            return Err(Wave3RepositoryError::PendingErpCancel);
        }
        if order.status != "released" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "released".to_string(),
                actual: order.status,
            });
        }
        let lines = load_receiving_order_lines_in_tx(&mut tx, ctx.owner_id, id).await?;
        let expected_qty = lines.iter().map(|line| line.expected_qty).sum();
        if req.actual_qty > expected_qty {
            return Err(Wave3RepositoryError::OverReceiptNotAllowed);
        }
        if req.actual_qty + req.shortage_qty + req.rejected_qty != expected_qty {
            return Err(Wave3RepositoryError::QuantityClosureMismatch);
        }

        receiving_validation::validate_receiving_gsp_fields(
            &order.document_type,
            &lines,
            &req,
            ctx.user_id,
        )?;
        if let Some(second_receiver_id) = req
            .details
            .as_ref()
            .and_then(|details| details.second_receiver_id)
        {
            ensure_receiving_clerk_signer(&mut tx, ctx.owner_id, second_receiver_id).await?;
        }
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
        let expected_qty: wms_domain::Quantity = sqlx::query_scalar(
            "SELECT COALESCE(SUM(expected_qty), 0) FROM receiving_order_lines WHERE receiving_order_id = $1 AND owner_id = $2",
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
            actual_qty: wms_domain::Quantity::ZERO,
            shortage_qty: wms_domain::Quantity::ZERO,
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
}
