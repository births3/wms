use super::*;

impl PgWave3Repository {
    pub async fn get_receiving_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<ReceivingOrder, Wave3RepositoryError> {
        let row = sqlx::query_as::<_, ReceivingOrderRow>(
            r#"
            SELECT id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
                   external_ref, status, expected_arrival_at, created_at, updated_at
              FROM receiving_orders
             WHERE id = $1 AND owner_id = $2
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let lines = self.load_receiving_order_lines(ctx.owner_id, id).await?;
        Ok(map_receiving_order(row, lines))
    }

    pub async fn get_receiving_order_print_data(
        &self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<ReceivingOrderPrintData, Wave3RepositoryError> {
        let order = self.get_receiving_order(ctx, id).await?;
        let receipts = sqlx::query_as::<_, ReceivingOrderReceiptRow>(
            r#"
            SELECT id, receiving_order_id, owner_id, actual_qty, shortage_qty,
                   rejected_qty, arrival_temperature_celsius, exception_note, receiving_details, occurred_at
              FROM receiving_order_receipts
             WHERE receiving_order_id = $1 AND owner_id = $2
             ORDER BY occurred_at, id
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(map_receiving_order_receipt)
        .collect();
        let inspections = sqlx::query_as::<_, ReceivingInspectionRow>(
            r#"
            SELECT id, receiving_order_id, owner_id, batch_no, accepted_qty,
                   rejected_qty, quality_status, occurred_at
              FROM receiving_inspections
             WHERE receiving_order_id = $1 AND owner_id = $2
             ORDER BY occurred_at, id
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(map_receiving_inspection)
        .collect();
        let signatures = sqlx::query_as::<_, InspectionSignatureRow>(
            r#"
            SELECT id, receiving_order_id, owner_id, first_signer_id,
                   second_signer_id, strategy_rule_id, approval_record_id, signed_at
              FROM receiving_inspection_signatures
             WHERE receiving_order_id = $1 AND owner_id = $2
             -- append-only：完整双签排前，便于打印取最新有效记录
             ORDER BY (second_signer_id IS NULL), signed_at DESC, id DESC
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(map_inspection_signature)
        .collect();
        Ok(ReceivingOrderPrintData {
            order,
            receipts,
            inspections,
            signatures,
        })
    }

    pub async fn delete_receiving_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<ReceivingOrder, Wave3RepositoryError> {
        let mut tx = self.begin().await?;
        let locked = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if locked.status != "draft" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "draft".to_string(),
                actual: locked.status,
            });
        }
        let lines = load_receiving_order_lines_in_tx(&mut tx, ctx.owner_id, id).await?;
        let order = map_receiving_order(locked, lines);
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            "delete",
            "M2",
            "receiving_order",
            id.to_string(),
            None,
        );
        audit.occurred_at = now;
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        sqlx::query("DELETE FROM receiving_orders WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(ctx.owner_id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(order)
    }

    pub async fn release_receiving_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<ReceivingOrder, Wave3RepositoryError> {
        self.release_receiving_order_with_audit(ctx, id, now, None, None)
            .await
    }

    /// draft → released。要求有效供应商/仓库（active）后才可放行收货。
    pub async fn release_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: Option<&str>,
        audit: Option<AuditWriteRequest>,
    ) -> Result<ReceivingOrder, Wave3RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "action": "release",
            "receiving_order_id": id,
        }))?;
        let mut tx = self.begin().await?;
        if let Some(key) = idempotency_key {
            lock_idempotency_key(&mut tx, ctx.owner_id, key).await?;
            if let Some(replay) =
                replay_idempotency(&mut tx, ctx.owner_id, key, &request_hash, now).await?
            {
                return Ok(replay);
            }
        }

        let locked = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if locked.status != "draft" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "draft".to_string(),
                actual: locked.status,
            });
        }
        let supplier_id = locked
            .supplier_id
            .ok_or(Wave3RepositoryError::InvalidReason)?;
        ensure_owned_reference(&mut tx, "suppliers", ctx.owner_id, supplier_id).await?;
        ensure_owned_reference(&mut tx, "warehouses", ctx.owner_id, locked.warehouse_id).await?;

        let updated = sqlx::query_as::<_, ReceivingOrderRow>(
            r#"
            UPDATE receiving_orders
               SET status = 'released', updated_at = $3, version = version + 1
             WHERE id = $1 AND owner_id = $2 AND status = 'draft'
            RETURNING id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
                      external_ref, status, expected_arrival_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let lines = load_receiving_order_lines_in_tx(&mut tx, ctx.owner_id, id).await?;
        let order = map_receiving_order(updated, lines);

        if let Some(key) = idempotency_key {
            store_idempotency_success(
                &mut tx,
                ctx.owner_id,
                key,
                &request_hash,
                "POST",
                "/api/v1/inbound/receiving-orders/{id}/release",
                "receiving_order",
                id.to_string(),
                &order,
                now,
            )
            .await?;
        }
        if let Some(audit_req) = audit {
            append_event_in_tx(&mut tx, &audit_req)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(order)
    }

    /// draft / released → cancelled。待收货（released）必须携带审批单号。
    pub async fn cancel_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: CancelReceivingOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<ReceivingOrder>, Wave3RepositoryError> {
        let reason = req.reason.trim();
        if reason.is_empty() {
            return Err(Wave3RepositoryError::InvalidReason);
        }
        let request_hash = request_hash(&serde_json::json!({
            "action": "cancel",
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

        let locked = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if locked.status != "draft" && locked.status != "released" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "draft/released".to_string(),
                actual: locked.status,
            });
        }
        if locked.status == "released" {
            let approval = req
                .approval_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or(Wave3RepositoryError::MissingApprovalSource)?;
            let _ = approval;
        }

        let updated = sqlx::query_as::<_, ReceivingOrderRow>(
            r#"
            UPDATE receiving_orders
               SET status = 'cancelled', updated_at = $3, version = version + 1
             WHERE id = $1 AND owner_id = $2 AND status IN ('draft', 'released')
            RETURNING id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
                      external_ref, status, expected_arrival_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let lines = load_receiving_order_lines_in_tx(&mut tx, ctx.owner_id, id).await?;
        let order = map_receiving_order(updated, lines);

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/cancel",
            "receiving_order",
            id.to_string(),
            &order,
            now,
        )
        .await?;
        if let Some(mut audit_req) = audit {
            audit_req.diff = Some(AuditDiff::compute(
                serde_json::json!({ "status": locked.status }),
                serde_json::json!({
                    "status": "cancelled",
                    "reason": reason,
                    "approval_id": req.approval_id,
                }),
            ));
            append_event_in_tx(&mut tx, &audit_req)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: order,
            replayed: false,
        })
    }

    /// inspecting → closed_shortage。要求收货记录存在短少数量。
    pub async fn force_close_shortage_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: ForceCloseShortageRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<ReceivingOrder>, Wave3RepositoryError> {
        let reason = req.reason.trim();
        if reason.is_empty() {
            return Err(Wave3RepositoryError::InvalidReason);
        }
        let request_hash = request_hash(&serde_json::json!({
            "action": "force_close_shortage",
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

        let locked = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if locked.status != "inspecting" && locked.status != "receiving" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "inspecting/receiving".to_string(),
                actual: locked.status,
            });
        }
        let shortage_qty: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(shortage_qty), 0)::BIGINT FROM receiving_order_receipts WHERE receiving_order_id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if shortage_qty <= 0 {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }

        let updated = sqlx::query_as::<_, ReceivingOrderRow>(
            r#"
            UPDATE receiving_orders
               SET status = 'closed_shortage', updated_at = $3, version = version + 1
             WHERE id = $1 AND owner_id = $2 AND status IN ('inspecting', 'receiving')
            RETURNING id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
                      external_ref, status, expected_arrival_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let lines = load_receiving_order_lines_in_tx(&mut tx, ctx.owner_id, id).await?;
        let order = map_receiving_order(updated, lines);

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/force-close-shortage",
            "receiving_order",
            id.to_string(),
            &order,
            now,
        )
        .await?;
        if let Some(mut audit_req) = audit {
            audit_req.diff = Some(AuditDiff::compute(
                serde_json::json!({ "status": locked.status, "shortage_qty": shortage_qty }),
                serde_json::json!({
                    "status": "closed_shortage",
                    "reason": reason,
                    "shortage_qty": shortage_qty,
                }),
            ));
            append_event_in_tx(&mut tx, &audit_req)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: order,
            replayed: false,
        })
    }
}
