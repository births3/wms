//! H8 消息存储（内存 + PostgreSQL）。

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;
use wms_domain::{
    can_replay_message, can_transition_message_status, H8ErpMessage, H8ErpMessageAttempt,
    H8ErpMessageStats, H8MessageError,
};

use super::error::H8ErpMessageRepoError;

#[axum::async_trait]
pub trait H8ErpMessageRepository: Send + Sync {
    async fn list(
        &self,
        owner_id: Uuid,
        direction: Option<&str>,
        message_type: Option<&str>,
        status: Option<&str>,
        created_from: Option<DateTime<Utc>>,
        created_to: Option<DateTime<Utc>>,
    ) -> Result<Vec<H8ErpMessage>, H8ErpMessageRepoError>;

    async fn get(&self, owner_id: Uuid, id: Uuid) -> Result<H8ErpMessage, H8ErpMessageRepoError>;

    async fn list_attempts(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
    ) -> Result<Vec<H8ErpMessageAttempt>, H8ErpMessageRepoError>;

    async fn stats(&self, owner_id: Uuid) -> Result<H8ErpMessageStats, H8ErpMessageRepoError>;

    async fn replay(
        &self,
        owner_id: Uuid,
        id: Uuid,
        reason: &str,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8ErpMessage, H8ErpMessageRepoError>;

    /// 测试/Worker 写入入口。
    async fn upsert_for_test(&self, message: &H8ErpMessage) -> Result<(), H8ErpMessageRepoError>;
}

#[derive(Default)]
pub struct MemoryH8ErpMessageRepository {
    inner: Mutex<MemoryInner>,
}

#[derive(Default)]
struct MemoryInner {
    messages: HashMap<Uuid, H8ErpMessage>,
    attempts: HashMap<Uuid, Vec<H8ErpMessageAttempt>>,
}

#[axum::async_trait]
impl H8ErpMessageRepository for MemoryH8ErpMessageRepository {
    async fn list(
        &self,
        owner_id: Uuid,
        direction: Option<&str>,
        message_type: Option<&str>,
        status: Option<&str>,
        created_from: Option<DateTime<Utc>>,
        created_to: Option<DateTime<Utc>>,
    ) -> Result<Vec<H8ErpMessage>, H8ErpMessageRepoError> {
        let guard = self.inner.lock().expect("lock");
        let mut rows: Vec<_> = guard
            .messages
            .values()
            .filter(|m| m.owner_id == owner_id)
            .filter(|m| direction.is_none_or(|d| m.direction == d))
            .filter(|m| message_type.is_none_or(|t| m.message_type == t))
            .filter(|m| status.is_none_or(|s| m.sync_status == s))
            .filter(|m| created_from.is_none_or(|f| m.created_at >= f))
            .filter(|m| created_to.is_none_or(|t| m.created_at <= t))
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(rows)
    }

    async fn get(&self, owner_id: Uuid, id: Uuid) -> Result<H8ErpMessage, H8ErpMessageRepoError> {
        let guard = self.inner.lock().expect("lock");
        guard
            .messages
            .get(&id)
            .filter(|m| m.owner_id == owner_id)
            .cloned()
            .ok_or(H8ErpMessageRepoError::NotFound)
    }

    async fn list_attempts(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
    ) -> Result<Vec<H8ErpMessageAttempt>, H8ErpMessageRepoError> {
        let msg = self.get(owner_id, message_id).await?;
        let _ = msg;
        let guard = self.inner.lock().expect("lock");
        Ok(guard.attempts.get(&message_id).cloned().unwrap_or_default())
    }

    async fn stats(&self, owner_id: Uuid) -> Result<H8ErpMessageStats, H8ErpMessageRepoError> {
        let rows = self.list(owner_id, None, None, None, None, None).await?;
        let mut stats = H8ErpMessageStats {
            owner_id,
            total: rows.len() as i64,
            succeeded: 0,
            failed: 0,
            dead: 0,
            processing: 0,
            pending: 0,
            retry_total: 0,
        };
        for m in rows {
            stats.retry_total += i64::from(m.retry_count);
            match m.sync_status.as_str() {
                "succeeded" | "acked" => stats.succeeded += 1,
                "failed" => stats.failed += 1,
                "dead" => stats.dead += 1,
                "processing" => stats.processing += 1,
                "pending" => stats.pending += 1,
                _ => {}
            }
        }
        Ok(stats)
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
        let mut guard = self.inner.lock().expect("lock");
        let Some(msg) = guard
            .messages
            .get(&id)
            .filter(|m| m.owner_id == owner_id)
            .cloned()
        else {
            return Err(H8ErpMessageRepoError::NotFound);
        };
        can_replay_message(&msg.sync_status).map_err(H8ErpMessageRepoError::Domain)?;
        can_transition_message_status(&msg.sync_status, "processing")
            .map_err(H8ErpMessageRepoError::Domain)?;
        let prev = msg.sync_status.clone();
        let channel = msg.channel.clone();
        let attempt_no = guard
            .attempts
            .get(&id)
            .map(|a| a.len() as i32 + 1)
            .unwrap_or(1);
        let mut next = msg;
        next.sync_status = "processing".into();
        next.claimed_by = Some(format!("replay:{actor}"));
        next.lease_expires_at = Some(now + chrono::Duration::minutes(5));
        next.updated_at = now;
        next.last_error_summary = Some(format!("replay: {reason}"));
        let attempt = H8ErpMessageAttempt {
            id: Uuid::new_v4(),
            message_id: id,
            attempt_no,
            channel,
            started_at: now,
            finished_at: Some(now),
            result: "replayed".into(),
            error_summary: Some(format!("from {prev}; reason={reason}")),
            actor: actor.into(),
        };
        guard.attempts.entry(id).or_default().push(attempt);
        guard.messages.insert(id, next.clone());
        Ok(next)
    }

    async fn upsert_for_test(&self, message: &H8ErpMessage) -> Result<(), H8ErpMessageRepoError> {
        let mut guard = self.inner.lock().expect("lock");
        guard.messages.insert(message.id, message.clone());
        Ok(())
    }
}

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
        Ok(H8ErpMessageStats {
            owner_id,
            total: row.total,
            succeeded: row.succeeded,
            failed: row.failed,
            dead: row.dead,
            processing: row.processing,
            pending: row.pending,
            retry_total: row.retry_total,
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
