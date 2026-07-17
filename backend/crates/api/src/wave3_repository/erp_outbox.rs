use super::*;
use serde_json::json;

impl PgWave3Repository {
    pub async fn enqueue_status_erp_feedback_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        batch_id: Uuid,
        status_change_id: Option<Uuid>,
        from_status: &str,
        to_status: &str,
        product_code: &str,
        batch_no: &str,
        qty: i64,
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
}
