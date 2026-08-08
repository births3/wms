use super::*;
use serde_json::json;

impl PgWave3Repository {
    pub async fn cancel_erp_receiving_order(
        &self,
        ctx: &AuthContext,
        erp_bill_code: &str,
        revision: i32,
        command_id: &str,
        correlation_id: &str,
        memo: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<IdempotentMutation<Uuid>, Wave3RepositoryError> {
        let mut tx = self.begin().await?;
        if let Some(id) = sqlx::query_scalar(
            "SELECT receiving_order_id FROM receiving_putaway_erp_feedback_outbox WHERE owner_id=$1 AND command_id=$2",
        )
        .bind(ctx.owner_id)
        .bind(command_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(IdempotentMutation { value: id, replayed: true });
        }
        sqlx::query(
            "INSERT INTO erp_order_cancel_commands (owner_id,command_id,erp_bill_code,revision,order_type,correlation_id,memo,created_at) VALUES ($1,$2,$3,$4,1,$5,$6,$7) ON CONFLICT (owner_id,command_id) DO NOTHING",
        )
        .bind(ctx.owner_id)
        .bind(command_id)
        .bind(erp_bill_code)
        .bind(revision)
        .bind(correlation_id)
        .bind(memo)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let orders: Vec<(Uuid, String, Uuid)> = sqlx::query_as(
            "SELECT id,status,warehouse_id FROM receiving_orders WHERE owner_id=$1 AND erp_bill_code=$2 AND erp_revision=$3 ORDER BY erp_line_no FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(erp_bill_code)
        .bind(revision)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let Some((first_id, _, warehouse_id)) = orders.first() else {
            tx.commit().await.map_err(map_db_error)?;
            return Err(Wave3RepositoryError::NotFound);
        };
        let (first_id, warehouse_id) = (*first_id, *warehouse_id);
        let cancellable = orders
            .iter()
            .all(|(_, status, _)| matches!(status.as_str(), "draft" | "released" | "cancelled"));
        if cancellable {
            sqlx::query("UPDATE receiving_orders SET status='cancelled',updated_at=$4,version=version+1 WHERE owner_id=$1 AND erp_bill_code=$2 AND erp_revision=$3 AND status IN ('draft','released')")
                .bind(ctx.owner_id).bind(erp_bill_code).bind(revision).bind(now)
                .execute(&mut *tx).await.map_err(map_db_error)?;
        }
        let (feedback_type, result_code, result_message) = if cancellable {
            (100, None, None)
        } else {
            (
                9,
                Some("INBOUND_RECEIPT_STARTED"),
                Some("任一 ASN 已开始收货，拒绝 ERP 整单取消"),
            )
        };
        sqlx::query(
            "INSERT INTO receiving_putaway_erp_feedback_outbox (id,owner_id,putaway_id,receiving_order_id,batch_id,command_id,event_type,payload,status,attempt_count,next_attempt_at,created_at,updated_at) VALUES ($1,$2,NULL,$3,NULL,$4,'order_status',$5,'pending',0,$6,$6,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(first_id)
        .bind(command_id)
        .bind(json!({
            "warehouse_id": warehouse_id,
            "erp_bill_code": erp_bill_code,
            "revision": revision,
            "order_type": 1,
            "feedback_type": feedback_type,
            "command_id": command_id,
            "result_code": result_code,
            "result_message": result_message,
            "correlation_id": correlation_id,
            "feedback_time": now,
        }))
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query("UPDATE erp_order_cancel_commands SET status=$3,resolved_at=$4 WHERE owner_id=$1 AND command_id=$2")
            .bind(ctx.owner_id)
            .bind(command_id)
            .bind(if cancellable { "completed" } else { "rejected" })
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        let audit = AuditWriteRequest::from_auth_context(
            ctx,
            "erp_order_cancel",
            "H8",
            "receiving_order",
            first_id.to_string(),
            Some(AuditDiff::compute(
                serde_json::Value::Null,
                json!({"status": if cancellable { "cancelled" } else { "rejected" }, "command_id": command_id, "memo": memo}),
            )),
        );
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: first_id,
            replayed: false,
        })
    }

    pub async fn enqueue_status_erp_feedback_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        warehouse_id: Option<Uuid>,
        batch_id: Uuid,
        status_change_id: Option<Uuid>,
        from_status: &str,
        to_status: &str,
        product_code: &str,
        batch_no: &str,
        qty: wms_domain::Quantity,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Result<(), Wave3RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO inventory_status_erp_feedback_outbox (
                id, owner_id, batch_id, status_change_id, event_type, payload,
                status, attempt_count, next_attempt_at, created_at, updated_at
            ) VALUES (
                $1,$2,$3,$4,'inventory_status_changed',$5,'pending',0,$6,$6,$6
            )
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(batch_id)
        .bind(status_change_id)
        .bind(json!({
            "warehouse_id": warehouse_id,
            "product_code": product_code,
            "batch_no": batch_no,
            "qty": qty,
            "from_status": from_status,
            "to_status": to_status,
            "reason": reason,
        }))
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        Ok(())
    }

    pub async fn process_status_erp_feedback_outbox(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<usize, Wave3RepositoryError> {
        let mut tx = self.begin().await?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id
              FROM inventory_status_erp_feedback_outbox
             WHERE status IN ('pending', 'failed')
               AND next_attempt_at <= $1
             ORDER BY next_attempt_at ASC
             LIMIT $2
             FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(now)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let mut processed = 0;
        for id in ids {
            // 本地闭环：无外部 ERP 时将待发送记录标记成功；失败路径由 attempt 重试。
            sqlx::query(
                r#"
                UPDATE inventory_status_erp_feedback_outbox
                   SET status = 'succeeded',
                       attempt_count = attempt_count + 1,
                       last_error = NULL,
                       updated_at = $2
                 WHERE id = $1
                "#,
            )
            .bind(id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            processed += 1;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(processed)
    }

    pub async fn mark_status_erp_feedback_failed(
        &self,
        id: Uuid,
        error: &str,
        now: DateTime<Utc>,
    ) -> Result<(), Wave3RepositoryError> {
        let next = now + chrono::Duration::minutes(5);
        sqlx::query(
            r#"
            UPDATE inventory_status_erp_feedback_outbox
               SET status = 'failed',
                   attempt_count = attempt_count + 1,
                   next_attempt_at = $3,
                   last_error = $2,
                   updated_at = $4
             WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(error)
        .bind(next)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(())
    }

    /// 处理上架 ERP 反馈 outbox（本地闭环：无外部 ERP 时标记成功）。
    pub async fn process_putaway_erp_feedback_outbox(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<usize, Wave3RepositoryError> {
        let mut tx = self.begin().await?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id
              FROM receiving_putaway_erp_feedback_outbox
             WHERE status IN ('pending', 'failed')
               AND next_attempt_at <= $1
             ORDER BY next_attempt_at ASC
             LIMIT $2
             FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(now)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let mut processed = 0;
        for id in ids {
            sqlx::query(
                r#"
                UPDATE receiving_putaway_erp_feedback_outbox
                   SET status = 'succeeded',
                       attempt_count = attempt_count + 1,
                       last_error = NULL,
                       updated_at = $2
                 WHERE id = $1
                "#,
            )
            .bind(id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            processed += 1;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(processed)
    }
}
