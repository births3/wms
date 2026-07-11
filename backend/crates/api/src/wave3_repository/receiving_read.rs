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
}
