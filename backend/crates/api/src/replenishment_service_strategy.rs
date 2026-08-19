use chrono::Utc;
use uuid::Uuid;
use wms_domain::{
    PageMeta, ReplenishmentStrategy, ReplenishmentStrategyListResponse,
    UpsertReplenishmentStrategyRequest,
};

use super::{write_audit, ReplenishmentError, ReplenishmentService, MANAGE};
use crate::{auth::AuthContext, idempotency};

impl ReplenishmentService {
    pub async fn create_strategy(
        &self,
        ctx: &AuthContext,
        req: UpsertReplenishmentStrategyRequest,
        idempotency_key: &str,
    ) -> Result<ReplenishmentStrategy, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        self.validate_strategy(ctx.owner_id, &req).await?;
        let hash = idempotency::request_hash(&req)?;
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
            "/api/v1/replenishment/strategies",
            Utc::now(),
        )
        .await?
        {
            return Ok(replay);
        }
        let created = self
            .repo
            .insert_strategy(&mut tx, ctx.owner_id, Uuid::new_v4(), &req)
            .await?;
        idempotency::store_success_with_status(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/replenishment/strategies",
            200,
            "replenishment_strategy",
            &created.id.to_string(),
            &created,
            Utc::now(),
        )
        .await?;
        write_audit(
            &mut tx,
            ctx,
            "create_replenishment_strategy",
            "replenishment_strategy",
            &created.id.to_string(),
        )
        .await?;
        tx.commit().await?;
        Ok(created)
    }

    pub async fn list_strategies(
        &self,
        ctx: &AuthContext,
        keyword: Option<&str>,
        enabled: Option<bool>,
        scope_type: Option<&str>,
        target_type: Option<&str>,
    ) -> Result<ReplenishmentStrategyListResponse, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let data = self
            .repo
            .list_strategies(ctx.owner_id, keyword, enabled, scope_type, target_type)
            .await?;
        let count = u32::try_from(data.len()).unwrap_or(u32::MAX);
        Ok(ReplenishmentStrategyListResponse {
            data,
            page: PageMeta {
                next_cursor: None,
                count,
                total: Some(count),
            },
        })
    }

    pub async fn get_strategy(
        &self,
        ctx: &AuthContext,
        strategy_id: Uuid,
    ) -> Result<ReplenishmentStrategy, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        self.repo
            .get_strategy(ctx.owner_id, strategy_id)
            .await?
            .ok_or(ReplenishmentError::StrategyNotFound)
    }

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
            .ok_or(ReplenishmentError::StrategyNotFound)?;
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
            .ok_or(ReplenishmentError::StrategyNotFound)?;
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
        super::write_audit(
            &mut tx,
            ctx,
            "update_replenishment_strategy",
            "replenishment_strategy",
            &updated.id.to_string(),
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
            .ok_or(ReplenishmentError::StrategyNotFound)?;
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
        super::write_audit(
            &mut tx,
            ctx,
            "disable_replenishment_strategy",
            "replenishment_strategy",
            &updated.id.to_string(),
        )
        .await?;
        tx.commit().await?;
        Ok(updated)
    }
}
