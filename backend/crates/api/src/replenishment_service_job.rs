use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use wms_domain::{
    can_confirm, can_pick, ClaimReplenishmentTaskRequest, ConfirmReplenishmentTaskRequest,
    PickReplenishmentTaskRequest, Quantity, ReplenishmentTask, REPLENISH_STATUS_DONE,
    REPLENISH_STATUS_IN_PROGRESS, REPLENISH_STATUS_PENDING,
};

use super::{ReplenishmentError, ReplenishmentService};
use crate::{
    auth::AuthContext, h2_lifecycle::publish_event_in_tx, idempotency,
    inventory::confirm_replenish_in_tx,
};

const EXECUTE: &str = "m3.replenishment.execute";

impl ReplenishmentService {
    pub async fn claim_task(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        req: ClaimReplenishmentTaskRequest,
        idempotency_key: &str,
    ) -> Result<ReplenishmentTask, ReplenishmentError> {
        ctx.require_permission(EXECUTE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let path = format!("/api/v1/replenishment/tasks/{task_id}/claim");
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
        if task.version != req.version || task.status != REPLENISH_STATUS_PENDING {
            return Err(ReplenishmentError::ClaimConflict);
        }
        if task.priority != "urgent" {
            let location = self
                .repo
                .target_location_scope(ctx.owner_id, task.target_location_id)
                .await?
                .ok_or(ReplenishmentError::TaskNotFound)?;
            let scope = self
                .repo
                .operator_replenish_zone_scope(ctx.owner_id, ctx.user_id)
                .await?;
            if !scope.allows(&location) {
                return Err(ReplenishmentError::ZoneDenied);
            }
        }
        if self
            .repo
            .operator_has_in_progress(&mut tx, ctx.owner_id, ctx.user_id, task.id)
            .await?
        {
            return Err(ReplenishmentError::ClaimConflict);
        }
        task.status = REPLENISH_STATUS_IN_PROGRESS.to_string();
        task.operator_id = Some(ctx.user_id);
        let saved = self
            .repo
            .save_task(&mut tx, &task, req.version)
            .await?
            .ok_or(ReplenishmentError::ClaimConflict)?;
        super::write_audit(
            &mut tx,
            ctx,
            "claim_replenishment_task",
            "replenishment_task",
            &saved.id.to_string(),
        )
        .await?;
        store_job(&mut tx, ctx, idempotency_key, &hash, &path, &saved).await?;
        tx.commit().await?;
        Ok(saved)
    }

    pub async fn pick_task(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        req: PickReplenishmentTaskRequest,
        idempotency_key: &str,
    ) -> Result<ReplenishmentTask, ReplenishmentError> {
        ctx.require_permission(EXECUTE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        if req.qty <= Quantity::ZERO {
            return Err(ReplenishmentError::QtyExceeded);
        }
        let path = format!("/api/v1/replenishment/tasks/{task_id}/pick");
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
        if !can_pick(&task.status) || task.operator_id != Some(ctx.user_id) {
            return Err(ReplenishmentError::StateInvalid);
        }
        let source_route = self
            .repo
            .load_location_route(&mut tx, ctx.owner_id, task.source_location_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        if source_route.lock_status == "lock_in" || source_route.lock_status == "lock_all" {
            return Err(ReplenishmentError::SourceUnavailable);
        }
        if self
            .suspend_if_source_short(&mut tx, ctx, &mut task, req.version)
            .await?
            .is_some()
        {
            tx.commit().await?;
            return Err(ReplenishmentError::SourceUnavailable);
        }
        let source_code = self
            .repo
            .location_code(&mut tx, ctx.owner_id, task.source_location_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        if req.scanned_location_code != source_code {
            return Err(ReplenishmentError::SourceMismatch);
        }
        if let Some(lpn_id) = task.source_lpn_id {
            let expected = self
                .repo
                .lpn_code(&mut tx, ctx.owner_id, lpn_id)
                .await?
                .ok_or(ReplenishmentError::SourceMismatch)?;
            if req.scanned_lpn_code.as_deref() != Some(expected.as_str()) {
                return Err(ReplenishmentError::SourceMismatch);
            }
        }
        if task.picked_qty + req.qty > task.qty {
            return Err(ReplenishmentError::QtyExceeded);
        }
        task.picked_qty += req.qty;
        let saved = self
            .repo
            .save_task(&mut tx, &task, req.version)
            .await?
            .ok_or(ReplenishmentError::StateInvalid)?;
        super::write_audit(
            &mut tx,
            ctx,
            "pick_replenishment_task",
            "replenishment_task",
            &saved.id.to_string(),
        )
        .await?;
        store_job(&mut tx, ctx, idempotency_key, &hash, &path, &saved).await?;
        tx.commit().await?;
        Ok(saved)
    }

    pub async fn confirm_task(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        req: ConfirmReplenishmentTaskRequest,
        idempotency_key: &str,
    ) -> Result<ReplenishmentTask, ReplenishmentError> {
        ctx.require_permission(EXECUTE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        if req.qty <= Quantity::ZERO {
            return Err(ReplenishmentError::QtyExceeded);
        }
        let path = format!("/api/v1/replenishment/tasks/{task_id}/confirm");
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
        if task.picked_qty <= Quantity::ZERO
            && self
                .suspend_if_source_short(&mut tx, ctx, &mut task, req.version)
                .await?
                .is_some()
        {
            tx.commit().await?;
            return Err(ReplenishmentError::SourceUnavailable);
        }
        if !can_confirm(&task.status, task.picked_qty) || task.operator_id != Some(ctx.user_id) {
            return Err(ReplenishmentError::StateInvalid);
        }
        let target_code = self
            .repo
            .location_code(&mut tx, ctx.owner_id, task.target_location_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        if req.scanned_location_code != target_code {
            return Err(ReplenishmentError::TargetMismatch);
        }
        if req.qty > task.picked_qty {
            return Err(ReplenishmentError::QtyExceeded);
        }
        let target = self
            .repo
            .load_location_route(&mut tx, ctx.owner_id, task.target_location_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        self.ensure_target_putaway(
            &mut tx,
            ctx.owner_id,
            &target,
            &target.location_type,
            task.product_id,
            req.qty,
        )
        .await?;
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
        let now = Utc::now();
        confirm_replenish_in_tx(
            &mut tx,
            ctx.owner_id,
            task.source_batch_id,
            target_batch_id,
            req.qty,
            task.id,
            "MANUAL",
            &task.id.to_string(),
            now,
        )
        .await
        .map_err(|_| ReplenishmentError::SourceUnavailable)?;
        task.picked_qty -= req.qty;
        task.done_qty += req.qty;
        if task.done_qty == task.qty {
            task.status = REPLENISH_STATUS_DONE.to_string();
        }
        let saved = self
            .repo
            .save_task(&mut tx, &task, req.version)
            .await?
            .ok_or(ReplenishmentError::StateInvalid)?;
        super::write_audit(
            &mut tx,
            ctx,
            "confirm_replenishment_task",
            "replenishment_task",
            &saved.id.to_string(),
        )
        .await?;
        if saved.status == REPLENISH_STATUS_DONE {
            publish_event_in_tx(
                &mut tx,
                ctx.owner_id,
                &format!("replenishment.done:{}", saved.id),
                "replenishment.done",
                "M3",
                "replenishment_task",
                &saved.id.to_string(),
                json!({
                    "task_id": saved.id,
                    "done_qty": saved.done_qty,
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
            if let Some(lpn_id) = saved.source_lpn_id {
                self.repo
                    .release_idle_container(&mut tx, ctx.owner_id, lpn_id)
                    .await?;
            }
        }
        store_job(&mut tx, ctx, idempotency_key, &hash, &path, &saved).await?;
        tx.commit().await?;
        Ok(saved)
    }
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
