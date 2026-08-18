use chrono::Utc;
use uuid::Uuid;
use wms_domain::{ReplenishmentStrategy, UpsertReplenishmentStrategyRequest};

use super::{ReplenishmentError, ReplenishmentService, MANAGE};
use crate::{auth::AuthContext, idempotency};

impl ReplenishmentService {
    pub async fn update_strategy(
        &self,
        ctx: &AuthContext,
        strategy_id: Uuid,
        req: UpsertReplenishmentStrategyRequest,
        idempotency_key: &str,
    ) -> Result<ReplenishmentStrategy, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        self.validate_strategy(ctx.owner_id, &req).await?;
        let current = self
            .repo
            .get_strategy(ctx.owner_id, strategy_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        let route_changed = current.scope_type != req.scope_type
            || current.scope_ref != req.scope_ref
            || current.source_type != req.source_type
            || current.target_type != req.target_type;
        if route_changed
            && self
                .repo
                .strategy_has_open_tasks(ctx.owner_id, strategy_id)
                .await?
        {
            return Err(ReplenishmentError::StrategyInvalid);
        }
        let hash = idempotency::request_hash(&req)?;
        let path = format!("/api/v1/replenishment/strategies/{strategy_id}");
        let mut tx = self.repo.pool().begin().await?;
        idempotency::lock_key(
            &mut tx,
            "replenishment_strategy",
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
            .update_strategy(&mut tx, ctx.owner_id, strategy_id, &req)
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
            "replenishment_strategy",
            &updated.id.to_string(),
            &updated,
            Utc::now(),
        )
        .await?;
        tx.commit().await?;
        Ok(updated)
    }

    pub async fn disable_strategy(
        &self,
        ctx: &AuthContext,
        strategy_id: Uuid,
        idempotency_key: &str,
    ) -> Result<ReplenishmentStrategy, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let path = format!("/api/v1/replenishment/strategies/{strategy_id}/disable");
        let hash = idempotency::request_hash(&strategy_id)?;
        let mut tx = self.repo.pool().begin().await?;
        idempotency::lock_key(
            &mut tx,
            "replenishment_strategy",
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
            .disable_strategy(&mut tx, ctx.owner_id, strategy_id)
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
            "replenishment_strategy",
            &updated.id.to_string(),
            &updated,
            Utc::now(),
        )
        .await?;
        tx.commit().await?;
        Ok(updated)
    }
}
