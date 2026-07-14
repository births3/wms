impl PgWave4Repository {
    pub async fn list_outbound_waves(
        &self,
        ctx: &AuthContext,
        status: Option<&str>,
        q: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<OutboundWave>, Wave4RepositoryError> {
        let status = non_empty_filter(status);
        let q = non_empty_filter(q);
        let limit = i64::from(limit.unwrap_or(50).clamp(1, 200));
        let rows = sqlx::query_as::<_, OutboundWaveRow>(
            r#"
            SELECT id, owner_id, wave_no, status, created_at, updated_at
              FROM outbound_waves
             WHERE owner_id = $1
               AND ($2::TEXT IS NULL OR status = $2)
               AND ($3::TEXT IS NULL OR wave_no ILIKE '%' || $3 || '%')
             ORDER BY updated_at DESC, wave_no ASC
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

        let mut waves = Vec::with_capacity(rows.len());
        for row in rows {
            waves.push(self.load_outbound_wave(row, ctx.owner_id).await?);
        }
        Ok(waves)
    }

    pub async fn get_outbound_wave(
        &self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<OutboundWave, Wave4RepositoryError> {
        let row = sqlx::query_as::<_, OutboundWaveRow>(
            r#"
            SELECT id, owner_id, wave_no, status, created_at, updated_at
              FROM outbound_waves
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;
        self.load_outbound_wave(row, ctx.owner_id).await
    }

    pub async fn cancel_outbound_wave(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundWave>, Wave4RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({ "wave_id": id, "action": "cancel" }))?;
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

        let mut wave_row = sqlx::query_as::<_, OutboundWaveRow>(
            r#"
            SELECT id, owner_id, wave_no, status, created_at, updated_at
              FROM outbound_waves
             WHERE owner_id = $1 AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;
        if wave_row.status != "draft" && wave_row.status != "released" {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "draft or released".to_string(),
                actual: wave_row.status,
            });
        }

        if let Some(status) = sqlx::query_scalar::<_, String>(
            r#"
            SELECT order_row.status
              FROM outbound_wave_orders link
              JOIN outbound_orders order_row
                ON order_row.owner_id = link.owner_id
               AND order_row.id = link.outbound_order_id
             WHERE link.owner_id = $1 AND link.wave_id = $2
               AND order_row.status <> $3
             LIMIT 1
            "#,
        )
        .bind(ctx.owner_id)
        .bind(id)
        .bind(OUTBOUND_STATUS_IN_WAVE)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: OUTBOUND_STATUS_IN_WAVE.to_string(),
                actual: status,
            });
        }

        sqlx::query(
            r#"
            UPDATE inventory_batches AS batch
               SET qty_locked = batch.qty_locked - released.qty,
                   updated_at = $3
              FROM (
                    SELECT allocation.batch_id, SUM(allocation.allocated_qty)::BIGINT AS qty
                      FROM inventory_allocations allocation
                      JOIN outbound_wave_orders link
                        ON link.owner_id = allocation.owner_id
                       AND link.outbound_order_id = allocation.outbound_order_id
                     WHERE allocation.owner_id = $1
                       AND link.wave_id = $2
                       AND allocation.status = 'locked'
                     GROUP BY allocation.batch_id
                   ) released
             WHERE batch.owner_id = $1 AND batch.id = released.batch_id
            "#,
        )
        .bind(ctx.owner_id)
        .bind(id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query(
            r#"
            DELETE FROM inventory_allocations allocation
             USING outbound_wave_orders link
             WHERE allocation.owner_id = $1
               AND link.owner_id = $1
               AND link.wave_id = $2
               AND link.outbound_order_id = allocation.outbound_order_id
               AND allocation.status = 'locked'
            "#,
        )
        .bind(ctx.owner_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query(
            r#"
            UPDATE outbound_orders order_row
               SET status = $3, updated_at = $4, version = version + 1
              FROM outbound_wave_orders link
             WHERE link.owner_id = $1
               AND link.wave_id = $2
               AND order_row.owner_id = link.owner_id
               AND order_row.id = link.outbound_order_id
            "#,
        )
        .bind(ctx.owner_id)
        .bind(id)
        .bind(OUTBOUND_STATUS_CONFIRMED)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query(
            "UPDATE outbound_waves SET status = 'cancelled', updated_at = $3, version = version + 1 WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        wave_row.status = "cancelled".to_string();
        wave_row.updated_at = now;
        let cancelled = self.load_outbound_wave(wave_row, ctx.owner_id).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/waves/{wave_id}/cancel",
            "outbound_wave",
            cancelled.id.to_string(),
            &cancelled,
            now,
        )
        .await?;
        append_outbound_audit(&mut tx, ctx, audit, "cancel_outbound_wave", id, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: cancelled,
            replayed: false,
        })
    }

    async fn load_outbound_wave(
        &self,
        row: OutboundWaveRow,
        owner_id: Uuid,
    ) -> Result<OutboundWave, Wave4RepositoryError> {
        let order_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT outbound_order_id
              FROM outbound_wave_orders
             WHERE owner_id = $1 AND wave_id = $2
             ORDER BY created_at ASC, outbound_order_id ASC
            "#,
        )
        .bind(owner_id)
        .bind(row.id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(map_outbound_wave(row, order_ids))
    }
}
