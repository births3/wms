use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use uuid::Uuid;
use wms_domain::{can_cancel, Quantity, REPLENISH_STATUS_CANCELLED};

use super::{write_audit, ReplenishmentError, ReplenishmentService};
use crate::{h2_lifecycle::publish_event_in_tx, inventory::release_replenish_in_tx};

impl ReplenishmentService {
    pub async fn run_timeout_scan(&self, now: DateTime<Utc>) -> Result<usize, ReplenishmentError> {
        let owners = self.repo.list_task_owner_ids().await?;
        let mut acted = 0;
        for owner_id in owners {
            acted += self.scan_owner_timeouts(owner_id, now).await?;
        }
        Ok(acted)
    }

    async fn scan_owner_timeouts(
        &self,
        owner_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<usize, ReplenishmentError> {
        let mut acted = 0;
        let urgent_timeout = self
            .repo
            .list_urgent_pending_since(owner_id, now - Duration::minutes(20))
            .await?;
        for (task_id, version) in urgent_timeout {
            if self.timeout_cancel(owner_id, task_id, version, now).await? {
                acted += 1;
            }
        }
        let unclaimed = self
            .repo
            .list_urgent_pending_since(owner_id, now - Duration::minutes(10))
            .await?;
        for (task_id, _) in unclaimed {
            self.publish_timeout_event(
                owner_id,
                task_id,
                &format!("replenishment.urgent_unclaimed:{task_id}"),
                "business.replenishment_urgent_unclaimed",
                json!({
                    "task_id": task_id,
                    "unclaimed_minutes": 10,
                    "task_status": "pending"
                }),
                now,
            )
            .await?;
            acted += 1;
        }
        let stale = self
            .repo
            .list_stale_in_progress(owner_id, now - Duration::hours(1))
            .await?;
        for (task_id, _, last_progress_at) in stale {
            self.publish_timeout_event(
                owner_id,
                task_id,
                &format!(
                    "replenishment.no_progress:{task_id}:{}",
                    last_progress_at.timestamp_millis()
                ),
                "business.replenishment_no_progress",
                json!({
                    "task_id": task_id,
                    "last_progress_at": last_progress_at,
                    "stale_minutes": 60,
                    "task_status": "in_progress"
                }),
                now,
            )
            .await?;
            acted += 1;
        }
        Ok(acted)
    }

    async fn timeout_cancel(
        &self,
        owner_id: Uuid,
        task_id: Uuid,
        version: i64,
        now: DateTime<Utc>,
    ) -> Result<bool, ReplenishmentError> {
        let mut tx = self.repo.pool().begin().await?;
        let Some(mut task) = self.repo.lock_task(&mut tx, owner_id, task_id).await? else {
            return Ok(false);
        };
        if task.version != version || !can_cancel(&task.status, task.picked_qty, task.done_qty) {
            return Ok(false);
        }
        let remaining = task.qty - task.done_qty;
        if remaining > Quantity::ZERO {
            let target_batch_id = self
                .repo
                .target_batch_id(
                    &mut tx,
                    owner_id,
                    task.target_location_id,
                    task.product_id,
                    &task.batch_no,
                )
                .await?
                .ok_or(ReplenishmentError::PutawayBlocked)?;
            release_replenish_in_tx(
                &mut tx,
                owner_id,
                task.source_batch_id,
                target_batch_id,
                remaining,
                now,
            )
            .await
            .map_err(|_| ReplenishmentError::SourceUnavailable)?;
        }
        task.status = REPLENISH_STATUS_CANCELLED.to_string();
        task.operator_id = None;
        let saved = self
            .repo
            .save_exception(&mut tx, &task, version, Some("urgent_timeout"), None, true)
            .await?
            .ok_or(ReplenishmentError::StateInvalid)?;
        publish_event_in_tx(
            &mut tx,
            owner_id,
            &format!("replenishment.cancelled:{}", saved.id),
            "replenishment.cancelled",
            "M3",
            "replenishment_task",
            &saved.id.to_string(),
            json!({
                "task_id": saved.id,
                "reason": "urgent_timeout",
                "wave_id": saved.wave_id,
                "outbound_order_id": saved.outbound_order_id,
                "outbound_line_no": saved.outbound_line_no
            }),
            now,
        )
        .await
        .map_err(|error| {
            ReplenishmentError::Database(sqlx::Error::Protocol(format!("{error:?}")))
        })?;
        publish_event_in_tx(
            &mut tx,
            owner_id,
            &format!("replenishment.urgent_timeout:{}", saved.id),
            "business.replenishment_urgent_timeout",
            "M3",
            "replenishment_task",
            &saved.id.to_string(),
            json!({
                "task_id": saved.id,
                "unclaimed_minutes": 20,
                "task_status": "cancelled",
                "task_no": saved.task_no
            }),
            now,
        )
        .await
        .map_err(|error| {
            ReplenishmentError::Database(sqlx::Error::Protocol(format!("{error:?}")))
        })?;
        let system = crate::auth::AuthContext {
            user_id: Uuid::nil(),
            owner_id,
            actor_name: "system:timeout".into(),
            permissions: vec!["m3.replenishment.manage".into()],
            jti: Uuid::new_v4().to_string(),
            warehouse_scope: None,
        };
        write_audit(
            &mut tx,
            &system,
            "timeout_cancel_replenishment_task",
            "replenishment_task",
            &saved.id.to_string(),
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn publish_timeout_event(
        &self,
        owner_id: Uuid,
        task_id: Uuid,
        idempotency_key: &str,
        event_type: &str,
        payload: serde_json::Value,
        now: DateTime<Utc>,
    ) -> Result<(), ReplenishmentError> {
        let mut tx = self.repo.pool().begin().await?;
        publish_event_in_tx(
            &mut tx,
            owner_id,
            idempotency_key,
            event_type,
            "M3",
            "replenishment_task",
            &task_id.to_string(),
            payload,
            now,
        )
        .await
        .map_err(|error| {
            ReplenishmentError::Database(sqlx::Error::Protocol(format!("{error:?}")))
        })?;
        tx.commit().await?;
        Ok(())
    }
}
