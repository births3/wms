use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use wms_domain::{
    can_cancel, CancelReplenishmentTaskRequest, Quantity, ReassignReplenishmentTaskRequest,
    ReplenishmentTask, ReturnReplenishmentTaskRequest, REPLENISH_STATUS_CANCELLED,
    REPLENISH_STATUS_PENDING, REPLENISH_STATUS_SUSPENDED,
};

use super::{ReplenishmentError, ReplenishmentService};
use crate::{
    auth::AuthContext, h2_lifecycle::publish_event_in_tx, idempotency,
    inventory::release_replenish_in_tx,
};

const MANAGE: &str = "m3.replenishment.manage";
const EXECUTE: &str = "m3.replenishment.execute";

impl ReplenishmentService {
    pub async fn cancel_task(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        req: CancelReplenishmentTaskRequest,
        idempotency_key: &str,
    ) -> Result<ReplenishmentTask, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        if req.reason.trim().is_empty() {
            return Err(ReplenishmentError::StateInvalid);
        }
        let path = format!("/api/v1/replenishment/tasks/{task_id}/cancel");
        let hash = idempotency::request_hash(&req)?;
        let mut tx = self.repo.pool().begin().await?;
        idempotency::lock_key(&mut tx, "replenishment_job", ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &path,
            Utc::now(),
        )
        .await?
        {
            return Ok(replay);
        }
        let mut task = self
            .repo
            .lock_task(&mut tx, ctx.owner_id, task_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        if task.version != req.version {
            return Err(ReplenishmentError::StateInvalid);
        }
        if !can_cancel(task.picked_qty, task.done_qty) {
            return Err(ReplenishmentError::CancelBlocked);
        }
        let remaining = task.qty - task.done_qty;
        if remaining > Quantity::ZERO {
            let target_batch_id = self
                .repo
                .target_batch_id(
                    &mut tx,
                    ctx.owner_id,
                    task.target_location_id,
                    task.product_id,
                    &task.batch_no,
                )
                .await?
                .ok_or(ReplenishmentError::PutawayBlocked)?;
            release_replenish_in_tx(
                &mut tx,
                ctx.owner_id,
                task.source_batch_id,
                target_batch_id,
                remaining,
                Utc::now(),
            )
            .await
            .map_err(|_| ReplenishmentError::SourceUnavailable)?;
        }
        task.status = REPLENISH_STATUS_CANCELLED.to_string();
        task.operator_id = None;
        let saved = self
            .repo
            .save_exception(
                &mut tx,
                &task,
                req.version,
                Some(req.reason.trim()),
                None,
                true,
            )
            .await?
            .ok_or(ReplenishmentError::StateInvalid)?;
        publish_bus(
            &mut tx,
            ctx.owner_id,
            &saved,
            "replenishment.cancelled",
            json!({ "task_id": saved.id, "reason": req.reason }),
        )
        .await?;
        store_job(&mut tx, ctx, idempotency_key, &hash, &path, &saved).await?;
        tx.commit().await?;
        Ok(saved)
    }

    pub async fn reassign_task(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        req: ReassignReplenishmentTaskRequest,
        idempotency_key: &str,
    ) -> Result<ReplenishmentTask, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let path = format!("/api/v1/replenishment/tasks/{task_id}/reassign");
        let hash = idempotency::request_hash(&req)?;
        let mut tx = self.repo.pool().begin().await?;
        idempotency::lock_key(&mut tx, "replenishment_job", ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &path,
            Utc::now(),
        )
        .await?
        {
            return Ok(replay);
        }
        let mut task = self
            .repo
            .lock_task(&mut tx, ctx.owner_id, task_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        if task.version != req.version {
            return Err(ReplenishmentError::StateInvalid);
        }
        if task.status != "in_progress" && task.status != REPLENISH_STATUS_SUSPENDED {
            return Err(ReplenishmentError::StateInvalid);
        }
        task.status = REPLENISH_STATUS_PENDING.to_string();
        task.operator_id = None;
        let saved = self
            .repo
            .save_exception(&mut tx, &task, req.version, None, None, true)
            .await?
            .ok_or(ReplenishmentError::StateInvalid)?;
        store_job(&mut tx, ctx, idempotency_key, &hash, &path, &saved).await?;
        tx.commit().await?;
        Ok(saved)
    }

    pub async fn return_task(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        req: ReturnReplenishmentTaskRequest,
        idempotency_key: &str,
    ) -> Result<ReplenishmentTask, ReplenishmentError> {
        ctx.require_permission(EXECUTE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        if !matches!(
            req.return_reason.as_str(),
            "source_mismatch" | "target_blocked" | "other"
        ) {
            return Err(ReplenishmentError::StateInvalid);
        }
        let path = format!("/api/v1/replenishment/tasks/{task_id}/return");
        let hash = idempotency::request_hash(&req)?;
        let mut tx = self.repo.pool().begin().await?;
        idempotency::lock_key(&mut tx, "replenishment_job", ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &path,
            Utc::now(),
        )
        .await?
        {
            return Ok(replay);
        }
        let mut task = self
            .repo
            .lock_task(&mut tx, ctx.owner_id, task_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        if task.version != req.version {
            return Err(ReplenishmentError::StateInvalid);
        }
        if task.operator_id != Some(ctx.user_id) {
            return Err(ReplenishmentError::StateInvalid);
        }
        if task.picked_qty > Quantity::ZERO || task.done_qty > Quantity::ZERO {
            return Err(ReplenishmentError::ReturnBlocked);
        }
        task.status = REPLENISH_STATUS_PENDING.to_string();
        task.operator_id = None;
        let saved = self
            .repo
            .save_exception(
                &mut tx,
                &task,
                req.version,
                None,
                Some(&req.return_reason),
                true,
            )
            .await?
            .ok_or(ReplenishmentError::StateInvalid)?;
        if req.return_reason == "source_mismatch" {
            publish_bus(
                &mut tx,
                ctx.owner_id,
                &saved,
                "replenishment.source_mismatch",
                json!({
                    "task_id": saved.id,
                    "source_location_id": saved.source_location_id,
                    "source_batch_id": saved.source_batch_id,
                    "operator_id": ctx.user_id
                }),
            )
            .await?;
            publish_bus(
                &mut tx,
                ctx.owner_id,
                &saved,
                "replenishment_source_mismatch",
                json!({ "task_id": saved.id }),
            )
            .await?;
        }
        store_job(&mut tx, ctx, idempotency_key, &hash, &path, &saved).await?;
        tx.commit().await?;
        Ok(saved)
    }

    pub(super) async fn suspend_if_source_short(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        ctx: &AuthContext,
        task: &mut ReplenishmentTask,
        expected_version: i64,
    ) -> Result<Option<ReplenishmentTask>, ReplenishmentError> {
        let remaining_unpicked = task.qty - task.done_qty - task.picked_qty;
        if remaining_unpicked <= Quantity::ZERO {
            return Ok(None);
        }
        let raw = self
            .repo
            .source_available(tx, ctx.owner_id, task.source_batch_id)
            .await?
            .ok_or(ReplenishmentError::SourceUnavailable)?;
        let reserved = task.qty - task.done_qty;
        let available_for_task = raw + reserved;
        if available_for_task >= remaining_unpicked {
            return Ok(None);
        }
        task.status = REPLENISH_STATUS_SUSPENDED.to_string();
        let saved = self
            .repo
            .save_exception(tx, task, expected_version, None, None, false)
            .await?
            .ok_or(ReplenishmentError::StateInvalid)?;
        publish_bus(
            tx,
            ctx.owner_id,
            &saved,
            "replenishment_source_frozen",
            json!({ "task_id": saved.id, "source_batch_id": saved.source_batch_id }),
        )
        .await?;
        Ok(Some(saved))
    }
}

async fn publish_bus(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    owner_id: Uuid,
    task: &ReplenishmentTask,
    event_type: &str,
    payload: serde_json::Value,
) -> Result<(), ReplenishmentError> {
    publish_event_in_tx(
        tx,
        owner_id,
        &format!("{event_type}:{}", task.id),
        event_type,
        "M3",
        "replenishment_task",
        &task.id.to_string(),
        payload,
        Utc::now(),
    )
    .await
    .map_err(|error| ReplenishmentError::Database(sqlx::Error::Protocol(format!("{error:?}"))))?;
    Ok(())
}

async fn store_job(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    hash: &str,
    path: &str,
    task: &ReplenishmentTask,
) -> Result<(), ReplenishmentError> {
    idempotency::store_success_with_status(
        tx,
        ctx.owner_id,
        idempotency_key,
        hash,
        "POST",
        path,
        200,
        "replenishment_task",
        &task.id.to_string(),
        task,
        Utc::now(),
    )
    .await?;
    Ok(())
}
