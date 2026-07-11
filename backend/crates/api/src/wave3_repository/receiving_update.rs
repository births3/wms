use super::*;

impl PgWave3Repository {
    pub async fn receive_receiving_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: ReceiveReceivingOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<ReceivingOrderReceipt, Wave3RepositoryError> {
        Ok(self
            .receive_receiving_order_with_audit(ctx, id, req, now, idempotency_key, None)
            .await?
            .value)
    }

    pub async fn update_receiving_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateReceivingOrderRequest,
        now: DateTime<Utc>,
        audit: AuditWriteRequest,
    ) -> Result<ReceivingOrder, Wave3RepositoryError> {
        if req.lines.as_ref().is_some_and(Vec::is_empty) {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        if req.status.is_some() {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "workflow action".to_string(),
                actual: req.status.unwrap_or_default(),
            });
        }

        let mut tx = self.begin().await?;
        let locked = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if locked.status != "draft" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "draft".to_string(),
                actual: locked.status,
            });
        }
        if let Some(supplier_id) = req.supplier_id {
            ensure_owned_reference(&mut tx, "suppliers", ctx.owner_id, supplier_id).await?;
        }
        if let Some(warehouse_id) = req.warehouse_id {
            ensure_owned_reference(&mut tx, "warehouses", ctx.owner_id, warehouse_id).await?;
        }
        for product_id in req
            .lines
            .iter()
            .flatten()
            .filter_map(|line| line.product_id)
        {
            ensure_owned_reference(&mut tx, "products", ctx.owner_id, product_id).await?;
        }
        let before_lines = load_receiving_order_lines_in_tx(&mut tx, ctx.owner_id, id).await?;
        let before = map_receiving_order(locked, before_lines);
        let external_ref_is_set = req.external_ref.is_some();
        let external_ref = req.external_ref.flatten();
        let row = sqlx::query_as::<_, ReceivingOrderRow>(
            r#"
            UPDATE receiving_orders
               SET supplier_id = COALESCE($3, supplier_id),
                   warehouse_id = COALESCE($4, warehouse_id),
                   external_ref = CASE WHEN $5 THEN $6 ELSE external_ref END,
                   expected_arrival_at = COALESCE($7, expected_arrival_at),
                   updated_at = $8,
                   version = version + 1
             WHERE id = $1 AND owner_id = $2
            RETURNING id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
                      external_ref, status, expected_arrival_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(req.supplier_id)
        .bind(req.warehouse_id)
        .bind(external_ref_is_set)
        .bind(external_ref)
        .bind(req.expected_arrival_at)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;

        if let Some(lines) = &req.lines {
            sqlx::query(
                "DELETE FROM receiving_order_lines WHERE receiving_order_id = $1 AND owner_id = $2",
            )
            .bind(id)
            .bind(ctx.owner_id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            insert_receiving_order_lines(&mut tx, ctx.owner_id, id, lines).await?;
        }
        let after_lines = load_receiving_order_lines_in_tx(&mut tx, ctx.owner_id, id).await?;
        let after = map_receiving_order(row, after_lines);
        let mut audit = audit;
        audit.diff = Some(AuditDiff::compute(
            serde_json::to_value(&before)
                .map_err(|error| Wave3RepositoryError::Serialize(error.to_string()))?,
            serde_json::to_value(&after)
                .map_err(|error| Wave3RepositoryError::Serialize(error.to_string()))?,
        ));
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(after)
    }
}
