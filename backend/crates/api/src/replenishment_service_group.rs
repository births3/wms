use chrono::Utc;
use uuid::Uuid;
use wms_domain::{
    ReplenishmentLocationGroup, ReplenishmentTask, UpsertReplenishmentLocationGroupRequest,
};

use super::{ReplenishmentError, ReplenishmentService, EXECUTE, MANAGE};
use crate::{auth::AuthContext, idempotency};

impl ReplenishmentService {
    pub async fn get_task(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
    ) -> Result<ReplenishmentTask, ReplenishmentError> {
        let task = self
            .repo
            .get_task(ctx.owner_id, task_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        if ctx.require_permission(MANAGE).is_ok() {
            return Ok(task);
        }
        ctx.require_permission(EXECUTE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        if task.operator_id == Some(ctx.user_id) {
            return Ok(task);
        }
        if task.status == "pending" && task.priority == "urgent" {
            return Ok(task);
        }
        if task.status == "pending" {
            let location = self
                .repo
                .target_location_scope(ctx.owner_id, task.target_location_id)
                .await?
                .ok_or(ReplenishmentError::TaskNotFound)?;
            let scope = self
                .repo
                .operator_replenish_zone_scope(ctx.owner_id, ctx.user_id)
                .await?;
            if scope.allows(&location) {
                return Ok(task);
            }
        }
        Err(ReplenishmentError::TaskNotFound)
    }

    pub async fn get_location_group(
        &self,
        ctx: &AuthContext,
        group_id: Uuid,
    ) -> Result<ReplenishmentLocationGroup, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        self.repo
            .get_location_group(ctx.owner_id, group_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)
    }

    pub async fn update_location_group(
        &self,
        ctx: &AuthContext,
        group_id: Uuid,
        req: UpsertReplenishmentLocationGroupRequest,
        idempotency_key: &str,
    ) -> Result<ReplenishmentLocationGroup, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let hash = idempotency::request_hash(&req)?;
        let path = format!("/api/v1/replenishment/location-groups/{group_id}");
        let mut tx = self.repo.pool().begin().await?;
        idempotency::lock_key(
            &mut tx,
            "replenishment_group",
            ctx.owner_id,
            idempotency_key,
        )
        .await?;
        if let Some(replay) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "PUT",
            &path,
            Utc::now(),
        )
        .await?
        {
            return Ok(replay);
        }
        let updated = self
            .repo
            .update_location_group(&mut tx, ctx.owner_id, group_id, &req)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        idempotency::store_success_with_status(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "PUT",
            &path,
            200,
            "replenishment_location_group",
            &updated.id.to_string(),
            &updated,
            Utc::now(),
        )
        .await?;
        super::write_audit(
            &mut tx,
            ctx,
            "update_replenishment_location_group",
            "replenishment_location_group",
            &updated.id.to_string(),
        )
        .await?;
        tx.commit().await?;
        Ok(updated)
    }

    pub async fn disable_location_group(
        &self,
        ctx: &AuthContext,
        group_id: Uuid,
        idempotency_key: &str,
    ) -> Result<ReplenishmentLocationGroup, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let path = format!("/api/v1/replenishment/location-groups/{group_id}/disable");
        let hash = idempotency::request_hash(&group_id)?;
        let mut tx = self.repo.pool().begin().await?;
        idempotency::lock_key(
            &mut tx,
            "replenishment_group",
            ctx.owner_id,
            idempotency_key,
        )
        .await?;
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
        let updated = self
            .repo
            .disable_location_group(&mut tx, ctx.owner_id, group_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        idempotency::store_success_with_status(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &path,
            200,
            "replenishment_location_group",
            &updated.id.to_string(),
            &updated,
            Utc::now(),
        )
        .await?;
        super::write_audit(
            &mut tx,
            ctx,
            "disable_replenishment_location_group",
            "replenishment_location_group",
            &updated.id.to_string(),
        )
        .await?;
        tx.commit().await?;
        Ok(updated)
    }
}
