//! H8 消息 PostgreSQL 仓储。

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    can_claim_message, can_replay_message, can_transition_message_status, estimate_p95_latency_ms,
    may_auto_purge, sanitize_error_summary, H8ErpMessage, H8ErpMessageAttempt, H8ErpMessageStats,
    H8MessageError,
};

use super::error::H8ErpMessageRepoError;
use super::repository::H8ErpMessageRepository;

pub struct PgH8ErpMessageRepository {
    pool: PgPool,
}

impl PgH8ErpMessageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[axum::async_trait]
impl H8ErpMessageRepository for PgH8ErpMessageRepository {
    async fn list(
        &self,
        owner_id: Uuid,
        direction: Option<&str>,
        message_type: Option<&str>,
        status: Option<&str>,
        created_from: Option<DateTime<Utc>>,
        created_to: Option<DateTime<Utc>>,
    ) -> Result<Vec<H8ErpMessage>, H8ErpMessageRepoError> {
        // 默认要求时间范围：未传则限制最近 7 天，避免无界扫描（AC12）
        let from = created_from.unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
        let rows = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, owner_id, warehouse_id, connector_id, connector_code, config_version,
                   direction, message_type, channel, external_ref, wms_resource_id,
                   idempotency_key, correlation_id, sync_status, retry_count, next_retry_at,
                   last_error_summary, payload_digest, claimed_by, lease_expires_at,
                   created_at, updated_at, completed_at, acked_at
            FROM h8_erp_messages
            WHERE owner_id = $1
              AND created_at >= $2
              AND ($3::timestamptz IS NULL OR created_at <= $3)
              AND ($4::text IS NULL OR direction = $4)
              AND ($5::text IS NULL OR message_type = $5)
              AND ($6::text IS NULL OR sync_status = $6)
            ORDER BY created_at DESC
            LIMIT 200
            "#,
        )
        .bind(owner_id)
        .bind(from)
        .bind(created_to)
        .bind(direction)
        .bind(message_type)
        .bind(status)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get(&self, owner_id: Uuid, id: Uuid) -> Result<H8ErpMessage, H8ErpMessageRepoError> {
        let row = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, owner_id, warehouse_id, connector_id, connector_code, config_version,
                   direction, message_type, channel, external_ref, wms_resource_id,
                   idempotency_key, correlation_id, sync_status, retry_count, next_retry_at,
                   last_error_summary, payload_digest, claimed_by, lease_expires_at,
                   created_at, updated_at, completed_at, acked_at
            FROM h8_erp_messages
            WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?
        .ok_or(H8ErpMessageRepoError::NotFound)?;
        Ok(row.into())
    }

    async fn list_attempts(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
    ) -> Result<Vec<H8ErpMessageAttempt>, H8ErpMessageRepoError> {
        let _ = self.get(owner_id, message_id).await?;
        let rows = sqlx::query_as::<_, AttemptRow>(
            r#"
            SELECT id, message_id, attempt_no, channel, started_at, finished_at,
                   result, error_summary, actor
            FROM h8_erp_message_attempts
            WHERE owner_id = $1 AND message_id = $2
            ORDER BY attempt_no ASC
            "#,
        )
        .bind(owner_id)
        .bind(message_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn stats(&self, owner_id: Uuid) -> Result<H8ErpMessageStats, H8ErpMessageRepoError> {
        let from = Utc::now() - chrono::Duration::days(30);
        let row = sqlx::query_as::<_, StatsRow>(
            r#"
            SELECT
              COUNT(*)::bigint AS total,
              COUNT(*) FILTER (WHERE sync_status IN ('succeeded','acked'))::bigint AS succeeded,
              COUNT(*) FILTER (WHERE sync_status = 'failed')::bigint AS failed,
              COUNT(*) FILTER (WHERE sync_status = 'dead')::bigint AS dead,
              COUNT(*) FILTER (WHERE sync_status = 'processing')::bigint AS processing,
              COUNT(*) FILTER (WHERE sync_status = 'pending')::bigint AS pending,
              COALESCE(SUM(retry_count),0)::bigint AS retry_total
            FROM h8_erp_messages
            WHERE owner_id = $1 AND created_at >= $2
            "#,
        )
        .bind(owner_id)
        .bind(from)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        let latency_rows: Vec<(Option<DateTime<Utc>>, DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT finished_at, started_at
            FROM h8_erp_message_attempts
            WHERE owner_id = $1 AND finished_at IS NOT NULL AND started_at >= $2
            "#,
        )
        .bind(owner_id)
        .bind(from)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        let samples: Vec<i64> = latency_rows
            .into_iter()
            .filter_map(|(finished, started)| finished.map(|f| (f - started).num_milliseconds()))
            .filter(|ms| *ms >= 0)
            .collect();
        Ok(H8ErpMessageStats {
            owner_id,
            total: row.total,
            succeeded: row.succeeded,
            failed: row.failed,
            dead: row.dead,
            processing: row.processing,
            pending: row.pending,
            retry_total: row.retry_total,
            p95_latency_ms: estimate_p95_latency_ms(&samples),
        })
    }

    async fn replay(
        &self,
        owner_id: Uuid,
        id: Uuid,
        reason: &str,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8ErpMessage, H8ErpMessageRepoError> {
        if reason.trim().is_empty() {
            return Err(H8ErpMessageRepoError::Domain(
                H8MessageError::FieldRequired("reason"),
            ));
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        let row = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, owner_id, warehouse_id, connector_id, connector_code, config_version,
                   direction, message_type, channel, external_ref, wms_resource_id,
                   idempotency_key, correlation_id, sync_status, retry_count, next_retry_at,
                   last_error_summary, payload_digest, claimed_by, lease_expires_at,
                   created_at, updated_at, completed_at, acked_at
            FROM h8_erp_messages
            WHERE owner_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?
        .ok_or(H8ErpMessageRepoError::NotFound)?;
        can_replay_message(&row.sync_status).map_err(H8ErpMessageRepoError::Domain)?;
        can_transition_message_status(&row.sync_status, "processing")
            .map_err(H8ErpMessageRepoError::Domain)?;
        let prev = row.sync_status.clone();
        let attempt_no: i32 = sqlx::query_scalar(
            r#"SELECT COALESCE(MAX(attempt_no), 0) + 1 FROM h8_erp_message_attempts WHERE message_id = $1"#,
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE h8_erp_messages
               SET sync_status = 'processing',
                   claimed_by = $3,
                   lease_expires_at = $4,
                   last_error_summary = $5,
                   updated_at = $4
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .bind(format!("replay:{actor}"))
        .bind(now + chrono::Duration::minutes(5))
        .bind(format!("replay: {reason}"))
        .execute(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO h8_erp_message_attempts (
              id, message_id, owner_id, attempt_no, channel, started_at, finished_at,
              result, error_summary, actor
            ) VALUES ($1,$2,$3,$4,$5,$6,$6,'replayed',$7,$8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(owner_id)
        .bind(attempt_no)
        .bind(&row.channel)
        .bind(now)
        .bind(format!("from {prev}; reason={reason}"))
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        self.get(owner_id, id).await
    }

    async fn claim(
        &self,
        owner_id: Uuid,
        id: Uuid,
        worker_id: &str,
        lease_seconds: i64,
        now: DateTime<Utc>,
    ) -> Result<H8ErpMessage, H8ErpMessageRepoError> {
        if worker_id.trim().is_empty() {
            return Err(H8ErpMessageRepoError::Domain(
                H8MessageError::FieldRequired("worker_id"),
            ));
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        let row = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, owner_id, warehouse_id, connector_id, connector_code, config_version,
                   direction, message_type, channel, external_ref, wms_resource_id,
                   idempotency_key, correlation_id, sync_status, retry_count, next_retry_at,
                   last_error_summary, payload_digest, claimed_by, lease_expires_at,
                   created_at, updated_at, completed_at, acked_at
            FROM h8_erp_messages
            WHERE owner_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?
        .ok_or(H8ErpMessageRepoError::NotFound)?;
        can_claim_message(&row.sync_status, row.lease_expires_at, now)
            .map_err(H8ErpMessageRepoError::Domain)?;
        let lease_until = now + chrono::Duration::seconds(lease_seconds.max(1));
        let attempt_no: i32 = sqlx::query_scalar(
            r#"SELECT COALESCE(MAX(attempt_no), 0) + 1 FROM h8_erp_message_attempts WHERE message_id = $1"#,
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        sqlx::query(
            r#"
            UPDATE h8_erp_messages
               SET sync_status = 'processing',
                   claimed_by = $3,
                   lease_expires_at = $4,
                   updated_at = $5
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .bind(worker_id)
        .bind(lease_until)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO h8_erp_message_attempts (
              id, message_id, owner_id, attempt_no, channel, started_at, finished_at,
              result, error_summary, actor
            ) VALUES ($1,$2,$3,$4,$5,$6,$6,'claimed',NULL,$7)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(owner_id)
        .bind(attempt_no)
        .bind(&row.channel)
        .bind(now)
        .bind(worker_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        self.get(owner_id, id).await
    }

    async fn mark_dead(
        &self,
        owner_id: Uuid,
        id: Uuid,
        error_summary: &str,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8ErpMessage, H8ErpMessageRepoError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        let row = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, owner_id, warehouse_id, connector_id, connector_code, config_version,
                   direction, message_type, channel, external_ref, wms_resource_id,
                   idempotency_key, correlation_id, sync_status, retry_count, next_retry_at,
                   last_error_summary, payload_digest, claimed_by, lease_expires_at,
                   created_at, updated_at, completed_at, acked_at
            FROM h8_erp_messages
            WHERE owner_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?
        .ok_or(H8ErpMessageRepoError::NotFound)?;
        can_transition_message_status(&row.sync_status, "dead")
            .map_err(H8ErpMessageRepoError::Domain)?;
        let summary = sanitize_error_summary(error_summary);
        let attempt_no: i32 = sqlx::query_scalar(
            r#"SELECT COALESCE(MAX(attempt_no), 0) + 1 FROM h8_erp_message_attempts WHERE message_id = $1"#,
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        sqlx::query(
            r#"
            UPDATE h8_erp_messages
               SET sync_status = 'dead',
                   last_error_summary = $3,
                   claimed_by = NULL,
                   lease_expires_at = NULL,
                   completed_at = $4,
                   updated_at = $4
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .bind(&summary)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO h8_erp_message_attempts (
              id, message_id, owner_id, attempt_no, channel, started_at, finished_at,
              result, error_summary, actor
            ) VALUES ($1,$2,$3,$4,$5,$6,$6,'dead',$7,$8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(owner_id)
        .bind(attempt_no)
        .bind(&row.channel)
        .bind(now)
        .bind(format!("from {}; {summary}", row.sync_status))
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        self.get(owner_id, id).await
    }

    async fn mark_archived(
        &self,
        owner_id: Uuid,
        id: Uuid,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8ErpMessage, H8ErpMessageRepoError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        let row = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, owner_id, warehouse_id, connector_id, connector_code, config_version,
                   direction, message_type, channel, external_ref, wms_resource_id,
                   idempotency_key, correlation_id, sync_status, retry_count, next_retry_at,
                   last_error_summary, payload_digest, claimed_by, lease_expires_at,
                   created_at, updated_at, completed_at, acked_at
            FROM h8_erp_messages
            WHERE owner_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?
        .ok_or(H8ErpMessageRepoError::NotFound)?;
        if !matches!(row.sync_status.as_str(), "succeeded" | "acked" | "dead") {
            return Err(H8ErpMessageRepoError::Domain(
                H8MessageError::IllegalTransition,
            ));
        }
        let attempt_no: i32 = sqlx::query_scalar(
            r#"SELECT COALESCE(MAX(attempt_no), 0) + 1 FROM h8_erp_message_attempts WHERE message_id = $1"#,
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO h8_erp_message_attempts (
              id, message_id, owner_id, attempt_no, channel, started_at, finished_at,
              result, error_summary, actor
            ) VALUES ($1,$2,$3,$4,$5,$6,$6,'archived',NULL,$7)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(owner_id)
        .bind(attempt_no)
        .bind(&row.channel)
        .bind(now)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        sqlx::query(
            r#"UPDATE h8_erp_messages SET updated_at = $3 WHERE owner_id = $1 AND id = $2"#,
        )
        .bind(owner_id)
        .bind(id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        self.get(owner_id, id).await
    }

    async fn purge_terminal(
        &self,
        owner_id: Uuid,
        retention_days: Option<i32>,
        now: DateTime<Utc>,
    ) -> Result<(i64, i32), H8ErpMessageRepoError> {
        let days_from_db: Option<i32> = sqlx::query_scalar(
            r#"SELECT retention_days FROM h8_erp_message_retention_policy WHERE owner_id = $1"#,
        )
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        let days = retention_days.or(days_from_db).filter(|d| *d > 0);
        if !may_auto_purge(days) {
            return Err(H8ErpMessageRepoError::Domain(
                H8MessageError::FieldRequired("retention_days"),
            ));
        }
        let days = days.expect("checked");
        let cutoff = now - chrono::Duration::days(i64::from(days));
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id FROM h8_erp_messages
             WHERE owner_id = $1
               AND sync_status IN ('succeeded','acked','dead')
               AND updated_at < $2
               AND sync_status NOT IN ('pending','processing','failed')
            "#,
        )
        .bind(owner_id)
        .bind(cutoff)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        if !ids.is_empty() {
            sqlx::query(
                r#"DELETE FROM h8_erp_message_attempts WHERE owner_id = $1 AND message_id = ANY($2)"#,
            )
            .bind(owner_id)
            .bind(&ids)
            .execute(&mut *tx)
            .await
            .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
            sqlx::query(r#"DELETE FROM h8_erp_messages WHERE owner_id = $1 AND id = ANY($2)"#)
                .bind(owner_id)
                .bind(&ids)
                .execute(&mut *tx)
                .await
                .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        }
        tx.commit()
            .await
            .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        Ok((ids.len() as i64, days))
    }

    async fn find_by_idempotency(
        &self,
        owner_id: Uuid,
        message_type: &str,
        external_ref: &str,
        idempotency_key: &str,
    ) -> Result<Option<H8ErpMessage>, H8ErpMessageRepoError> {
        let row = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT id, owner_id, warehouse_id, connector_id, connector_code, config_version,
                   direction, message_type, channel, external_ref, wms_resource_id,
                   idempotency_key, correlation_id, sync_status, retry_count, next_retry_at,
                   last_error_summary, payload_digest, claimed_by, lease_expires_at,
                   created_at, updated_at, completed_at, acked_at
            FROM h8_erp_messages
            WHERE owner_id = $1 AND message_type = $2 AND external_ref = $3 AND idempotency_key = $4
            LIMIT 1
            "#,
        )
        .bind(owner_id)
        .bind(message_type)
        .bind(external_ref)
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        Ok(row.map(Into::into))
    }

    async fn upsert_for_test(&self, message: &H8ErpMessage) -> Result<(), H8ErpMessageRepoError> {
        sqlx::query(
            r#"
            INSERT INTO h8_erp_messages (
              id, owner_id, warehouse_id, connector_id, connector_code, config_version,
              direction, message_type, channel, external_ref, wms_resource_id,
              idempotency_key, correlation_id, sync_status, retry_count, next_retry_at,
              last_error_summary, payload_digest, claimed_by, lease_expires_at,
              created_at, updated_at, completed_at, acked_at
            ) VALUES (
              $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24
            )
            ON CONFLICT (id) DO UPDATE SET
              sync_status = EXCLUDED.sync_status,
              retry_count = EXCLUDED.retry_count,
              last_error_summary = EXCLUDED.last_error_summary,
              updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(message.id)
        .bind(message.owner_id)
        .bind(message.warehouse_id)
        .bind(message.connector_id)
        .bind(&message.connector_code)
        .bind(message.config_version)
        .bind(&message.direction)
        .bind(&message.message_type)
        .bind(&message.channel)
        .bind(&message.external_ref)
        .bind(&message.wms_resource_id)
        .bind(&message.idempotency_key)
        .bind(&message.correlation_id)
        .bind(&message.sync_status)
        .bind(message.retry_count)
        .bind(message.next_retry_at)
        .bind(&message.last_error_summary)
        .bind(&message.payload_digest)
        .bind(&message.claimed_by)
        .bind(message.lease_expires_at)
        .bind(message.created_at)
        .bind(message.updated_at)
        .bind(message.completed_at)
        .bind(message.acked_at)
        .execute(&self.pool)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn append_attempt_for_test(
        &self,
        attempt: &H8ErpMessageAttempt,
    ) -> Result<(), H8ErpMessageRepoError> {
        sqlx::query(
            r#"
            INSERT INTO h8_erp_message_attempts (
              id, message_id, owner_id, attempt_no, channel, started_at, finished_at,
              result, error_summary, actor
            ) VALUES ($1,$2,(SELECT owner_id FROM h8_erp_messages WHERE id = $2),$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(attempt.id)
        .bind(attempt.message_id)
        .bind(attempt.attempt_no)
        .bind(&attempt.channel)
        .bind(attempt.started_at)
        .bind(attempt.finished_at)
        .bind(&attempt.result)
        .bind(&attempt.error_summary)
        .bind(&attempt.actor)
        .execute(&self.pool)
        .await
        .map_err(|e| H8ErpMessageRepoError::Db(e.to_string()))?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: Uuid,
    owner_id: Uuid,
    warehouse_id: Option<Uuid>,
    connector_id: Option<Uuid>,
    connector_code: Option<String>,
    config_version: Option<i64>,
    direction: String,
    message_type: String,
    channel: String,
    external_ref: String,
    wms_resource_id: Option<String>,
    idempotency_key: String,
    correlation_id: String,
    sync_status: String,
    retry_count: i32,
    next_retry_at: Option<DateTime<Utc>>,
    last_error_summary: Option<String>,
    payload_digest: String,
    claimed_by: Option<String>,
    lease_expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    acked_at: Option<DateTime<Utc>>,
}

impl From<MessageRow> for H8ErpMessage {
    fn from(r: MessageRow) -> Self {
        Self {
            id: r.id,
            owner_id: r.owner_id,
            warehouse_id: r.warehouse_id,
            connector_id: r.connector_id,
            connector_code: r.connector_code,
            config_version: r.config_version,
            direction: r.direction,
            message_type: r.message_type,
            channel: r.channel,
            external_ref: r.external_ref,
            wms_resource_id: r.wms_resource_id,
            idempotency_key: r.idempotency_key,
            correlation_id: r.correlation_id,
            sync_status: r.sync_status,
            retry_count: r.retry_count,
            next_retry_at: r.next_retry_at,
            last_error_summary: r.last_error_summary,
            payload_digest: r.payload_digest,
            claimed_by: r.claimed_by,
            lease_expires_at: r.lease_expires_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            completed_at: r.completed_at,
            acked_at: r.acked_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct AttemptRow {
    id: Uuid,
    message_id: Uuid,
    attempt_no: i32,
    channel: String,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    result: String,
    error_summary: Option<String>,
    actor: String,
}

impl From<AttemptRow> for H8ErpMessageAttempt {
    fn from(r: AttemptRow) -> Self {
        Self {
            id: r.id,
            message_id: r.message_id,
            attempt_no: r.attempt_no,
            channel: r.channel,
            started_at: r.started_at,
            finished_at: r.finished_at,
            result: r.result,
            error_summary: r.error_summary,
            actor: r.actor,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StatsRow {
    total: i64,
    succeeded: i64,
    failed: i64,
    dead: i64,
    processing: i64,
    pending: i64,
    retry_total: i64,
}
