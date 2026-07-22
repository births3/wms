//! H8 消息存储（内存 + PostgreSQL）。

//! H8 消息仓储 trait 与内存实现。

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;
use wms_domain::{
    can_claim_message, can_replay_message, can_transition_message_status, estimate_p95_latency_ms,
    may_auto_purge, sanitize_error_summary, H8ErpMessage, H8ErpMessageAttempt, H8ErpMessageStats,
    H8MessageError,
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
        connector_code: Option<&str>,
        channel: Option<&str>,
        warehouse_id: Option<Uuid>,
        external_ref: Option<&str>,
        idempotency_key: Option<&str>,
        correlation_id: Option<&str>,
        created_from: Option<DateTime<Utc>>,
        created_to: Option<DateTime<Utc>>,
    ) -> Result<Vec<H8ErpMessage>, H8ErpMessageRepoError>;

    async fn get(&self, owner_id: Uuid, id: Uuid) -> Result<H8ErpMessage, H8ErpMessageRepoError>;

    async fn list_attempts(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
    ) -> Result<Vec<H8ErpMessageAttempt>, H8ErpMessageRepoError>;

    async fn stats(
        &self,
        owner_id: Uuid,
        connector_code: Option<&str>,
        channel: Option<&str>,
        message_type: Option<&str>,
    ) -> Result<H8ErpMessageStats, H8ErpMessageRepoError>;

    async fn replay(
        &self,
        owner_id: Uuid,
        id: Uuid,
        reason: &str,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8ErpMessage, H8ErpMessageRepoError>;

    /// Worker 并发认领（租约）。
    async fn claim(
        &self,
        owner_id: Uuid,
        id: Uuid,
        worker_id: &str,
        lease_seconds: i64,
        now: DateTime<Utc>,
    ) -> Result<H8ErpMessage, H8ErpMessageRepoError>;

    /// 进入 dead（AC6）；调用方须写 H2 审计。
    async fn mark_dead(
        &self,
        owner_id: Uuid,
        id: Uuid,
        error_summary: &str,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8ErpMessage, H8ErpMessageRepoError>;

    /// 归档终态消息（不删除，保留策略清理另走 purge）。
    async fn mark_archived(
        &self,
        owner_id: Uuid,
        id: Uuid,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8ErpMessage, H8ErpMessageRepoError>;

    /// 按保留策略清理终态消息（succeeded/acked/dead 且超期）；未配置策略拒绝。
    async fn purge_terminal(
        &self,
        owner_id: Uuid,
        retention_days: Option<i32>,
        now: DateTime<Utc>,
    ) -> Result<(i64, i32), H8ErpMessageRepoError>;

    /// 按幂等键查找消息（交换生命周期审计 upsert）。
    async fn find_by_idempotency(
        &self,
        owner_id: Uuid,
        message_type: &str,
        external_ref: &str,
        idempotency_key: &str,
    ) -> Result<Option<H8ErpMessage>, H8ErpMessageRepoError>;

    /// 测试/Worker 写入入口。
    async fn upsert_for_test(&self, message: &H8ErpMessage) -> Result<(), H8ErpMessageRepoError>;

    /// 测试用：写入尝试样本以计算 P95。
    async fn append_attempt_for_test(
        &self,
        attempt: &H8ErpMessageAttempt,
    ) -> Result<(), H8ErpMessageRepoError>;
}

#[derive(Default)]
pub struct MemoryH8ErpMessageRepository {
    inner: Mutex<MemoryInner>,
}

#[derive(Default)]
struct MemoryInner {
    messages: HashMap<Uuid, H8ErpMessage>,
    attempts: HashMap<Uuid, Vec<H8ErpMessageAttempt>>,
    /// owner -> retention_days
    retention: HashMap<Uuid, i32>,
}

#[axum::async_trait]
impl H8ErpMessageRepository for MemoryH8ErpMessageRepository {
    async fn list(
        &self,
        owner_id: Uuid,
        direction: Option<&str>,
        message_type: Option<&str>,
        status: Option<&str>,
        connector_code: Option<&str>,
        channel: Option<&str>,
        warehouse_id: Option<Uuid>,
        external_ref: Option<&str>,
        idempotency_key: Option<&str>,
        correlation_id: Option<&str>,
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
            .filter(|m| connector_code.is_none_or(|code| m.connector_code.as_deref() == Some(code)))
            .filter(|m| channel.is_none_or(|value| m.channel == value))
            .filter(|m| warehouse_id.is_none_or(|value| m.warehouse_id == Some(value)))
            .filter(|m| external_ref.is_none_or(|value| m.external_ref == value))
            .filter(|m| idempotency_key.is_none_or(|value| m.idempotency_key == value))
            .filter(|m| correlation_id.is_none_or(|value| m.correlation_id == value))
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

    async fn stats(
        &self,
        owner_id: Uuid,
        connector_code: Option<&str>,
        channel: Option<&str>,
        message_type: Option<&str>,
    ) -> Result<H8ErpMessageStats, H8ErpMessageRepoError> {
        let rows = self
            .list(
                owner_id,
                None,
                message_type,
                None,
                connector_code,
                channel,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await?;
        let mut stats = H8ErpMessageStats {
            owner_id,
            total: rows.len() as i64,
            succeeded: 0,
            failed: 0,
            dead: 0,
            processing: 0,
            pending: 0,
            retry_total: 0,
            p95_latency_ms: 0,
        };
        let mut latencies = Vec::new();
        let guard = self.inner.lock().expect("lock");
        for m in &rows {
            stats.retry_total += i64::from(m.retry_count);
            match m.sync_status.as_str() {
                "succeeded" | "acked" => stats.succeeded += 1,
                "failed" => stats.failed += 1,
                "dead" => stats.dead += 1,
                "processing" => stats.processing += 1,
                "pending" => stats.pending += 1,
                _ => {}
            }
            if let Some(attempts) = guard.attempts.get(&m.id) {
                for a in attempts {
                    if let Some(finished) = a.finished_at {
                        let ms = (finished - a.started_at).num_milliseconds();
                        if ms >= 0 {
                            latencies.push(ms);
                        }
                    }
                }
            }
        }
        stats.p95_latency_ms = estimate_p95_latency_ms(&latencies);
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
        let mut guard = self.inner.lock().expect("lock");
        let Some(msg) = guard
            .messages
            .get(&id)
            .filter(|m| m.owner_id == owner_id)
            .cloned()
        else {
            return Err(H8ErpMessageRepoError::NotFound);
        };
        can_claim_message(&msg.sync_status, msg.lease_expires_at, now)
            .map_err(H8ErpMessageRepoError::Domain)?;
        can_transition_message_status(&msg.sync_status, "processing")
            .or_else(|_| {
                // processing + expired lease 可重新认领
                if msg.sync_status == "processing" {
                    Ok(())
                } else {
                    Err(H8MessageError::IllegalTransition)
                }
            })
            .map_err(H8ErpMessageRepoError::Domain)?;
        let mut next = msg;
        next.sync_status = "processing".into();
        next.claimed_by = Some(worker_id.to_string());
        next.lease_expires_at = Some(now + chrono::Duration::seconds(lease_seconds.max(1)));
        next.updated_at = now;
        let attempt_no = guard
            .attempts
            .get(&id)
            .map(|a| a.len() as i32 + 1)
            .unwrap_or(1);
        guard
            .attempts
            .entry(id)
            .or_default()
            .push(H8ErpMessageAttempt {
                id: Uuid::new_v4(),
                message_id: id,
                attempt_no,
                channel: next.channel.clone(),
                started_at: now,
                finished_at: Some(now),
                result: "claimed".into(),
                error_summary: None,
                actor: worker_id.into(),
            });
        guard.messages.insert(id, next.clone());
        Ok(next)
    }

    async fn mark_dead(
        &self,
        owner_id: Uuid,
        id: Uuid,
        error_summary: &str,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8ErpMessage, H8ErpMessageRepoError> {
        let mut guard = self.inner.lock().expect("lock");
        let Some(msg) = guard
            .messages
            .get(&id)
            .filter(|m| m.owner_id == owner_id)
            .cloned()
        else {
            return Err(H8ErpMessageRepoError::NotFound);
        };
        can_transition_message_status(&msg.sync_status, "dead")
            .map_err(H8ErpMessageRepoError::Domain)?;
        let mut next = msg;
        let prev = next.sync_status.clone();
        next.sync_status = "dead".into();
        next.last_error_summary = Some(sanitize_error_summary(error_summary));
        next.updated_at = now;
        next.completed_at = Some(now);
        next.claimed_by = None;
        next.lease_expires_at = None;
        let attempt_no = guard
            .attempts
            .get(&id)
            .map(|a| a.len() as i32 + 1)
            .unwrap_or(1);
        guard
            .attempts
            .entry(id)
            .or_default()
            .push(H8ErpMessageAttempt {
                id: Uuid::new_v4(),
                message_id: id,
                attempt_no,
                channel: next.channel.clone(),
                started_at: now,
                finished_at: Some(now),
                result: "dead".into(),
                error_summary: Some(format!(
                    "from {prev}; {}",
                    sanitize_error_summary(error_summary)
                )),
                actor: actor.into(),
            });
        guard.messages.insert(id, next.clone());
        Ok(next)
    }

    async fn mark_archived(
        &self,
        owner_id: Uuid,
        id: Uuid,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8ErpMessage, H8ErpMessageRepoError> {
        let mut guard = self.inner.lock().expect("lock");
        let Some(msg) = guard
            .messages
            .get(&id)
            .filter(|m| m.owner_id == owner_id)
            .cloned()
        else {
            return Err(H8ErpMessageRepoError::NotFound);
        };
        if !matches!(msg.sync_status.as_str(), "succeeded" | "acked" | "dead") {
            return Err(H8ErpMessageRepoError::Domain(
                H8MessageError::IllegalTransition,
            ));
        }
        let mut next = msg;
        let attempt_no = guard
            .attempts
            .get(&id)
            .map(|a| a.len() as i32 + 1)
            .unwrap_or(1);
        guard
            .attempts
            .entry(id)
            .or_default()
            .push(H8ErpMessageAttempt {
                id: Uuid::new_v4(),
                message_id: id,
                attempt_no,
                channel: next.channel.clone(),
                started_at: now,
                finished_at: Some(now),
                result: "archived".into(),
                error_summary: None,
                actor: actor.into(),
            });
        next.updated_at = now;
        guard.messages.insert(id, next.clone());
        Ok(next)
    }

    async fn purge_terminal(
        &self,
        owner_id: Uuid,
        retention_days: Option<i32>,
        now: DateTime<Utc>,
    ) -> Result<(i64, i32), H8ErpMessageRepoError> {
        let mut guard = self.inner.lock().expect("lock");
        let days = retention_days
            .or_else(|| guard.retention.get(&owner_id).copied())
            .filter(|d| *d > 0);
        if !may_auto_purge(days) {
            return Err(H8ErpMessageRepoError::Domain(
                H8MessageError::FieldRequired("retention_days"),
            ));
        }
        let days = days.expect("checked");
        let cutoff = now - chrono::Duration::days(i64::from(days));
        let to_delete: Vec<Uuid> = guard
            .messages
            .values()
            .filter(|m| m.owner_id == owner_id)
            .filter(|m| matches!(m.sync_status.as_str(), "succeeded" | "acked" | "dead"))
            .filter(|m| m.updated_at < cutoff)
            .map(|m| m.id)
            .collect();
        for id in &to_delete {
            guard.messages.remove(id);
            guard.attempts.remove(id);
        }
        Ok((to_delete.len() as i64, days))
    }

    async fn find_by_idempotency(
        &self,
        owner_id: Uuid,
        message_type: &str,
        external_ref: &str,
        idempotency_key: &str,
    ) -> Result<Option<H8ErpMessage>, H8ErpMessageRepoError> {
        let guard = self.inner.lock().expect("lock");
        Ok(guard
            .messages
            .values()
            .find(|m| {
                m.owner_id == owner_id
                    && m.message_type == message_type
                    && m.external_ref == external_ref
                    && m.idempotency_key == idempotency_key
            })
            .cloned())
    }

    async fn upsert_for_test(&self, message: &H8ErpMessage) -> Result<(), H8ErpMessageRepoError> {
        let mut guard = self.inner.lock().expect("lock");
        guard.messages.insert(message.id, message.clone());
        Ok(())
    }

    async fn append_attempt_for_test(
        &self,
        attempt: &H8ErpMessageAttempt,
    ) -> Result<(), H8ErpMessageRepoError> {
        let mut guard = self.inner.lock().expect("lock");
        guard
            .attempts
            .entry(attempt.message_id)
            .or_default()
            .push(attempt.clone());
        Ok(())
    }
}

impl MemoryH8ErpMessageRepository {
    #[cfg(test)]
    pub fn set_retention_for_test(&self, owner_id: Uuid, days: i32) {
        let mut guard = self.inner.lock().expect("lock");
        guard.retention.insert(owner_id, days);
    }
}
