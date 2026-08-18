//! 补货策略与库位组用例编排。

use chrono::Utc;
use uuid::Uuid;
use wms_domain::{
    validate_replenish_route, BindReplenishmentLocationsRequest,
    BindReplenishmentLocationsResponse, ReplenishmentLocationGroup, ReplenishmentPreviewResponse,
    ReplenishmentStrategy, UpsertReplenishmentLocationGroupRequest,
    UpsertReplenishmentStrategyRequest,
};

use crate::{
    auth::AuthContext,
    idempotency::{self, IdempotencyError},
    replenishment_repository::{PgReplenishmentRepository, ReplenishmentRepoError},
};

const MANAGE: &str = "m3.replenishment.manage";

#[derive(Debug)]
pub enum ReplenishmentError {
    PermissionDenied,
    StrategyInvalid,
    ScopeNotFound,
    LocationBound,
    TaskNotFound,
    IdempotencyRequired,
    IdempotencyConflict,
    Database(sqlx::Error),
}

pub struct ReplenishmentService {
    repo: PgReplenishmentRepository,
}

impl ReplenishmentService {
    pub fn new(repo: PgReplenishmentRepository) -> Self {
        Self { repo }
    }

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
        tx.commit().await?;
        Ok(created)
    }

    pub async fn bind_locations(
        &self,
        ctx: &AuthContext,
        strategy_id: Uuid,
        req: BindReplenishmentLocationsRequest,
        idempotency_key: &str,
    ) -> Result<BindReplenishmentLocationsResponse, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let strategy = self
            .repo
            .get_strategy(ctx.owner_id, strategy_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        let hash = idempotency::request_hash(&req)?;
        let path = format!("/api/v1/replenishment/strategies/{strategy_id}/locations");
        let mut tx = self.repo.pool().begin().await?;
        idempotency::lock_key(&mut tx, "replenishment_bind", ctx.owner_id, idempotency_key).await?;
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
        let bound = self
            .repo
            .replace_strategy_locations(
                &mut tx,
                ctx.owner_id,
                strategy.id,
                &strategy.target_type,
                &req.location_ids,
            )
            .await?;
        idempotency::store_success_with_status(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "PUT",
            &path,
            200,
            "replenishment_strategy",
            &strategy.id.to_string(),
            &bound,
            Utc::now(),
        )
        .await?;
        tx.commit().await?;
        Ok(bound)
    }

    pub async fn preview(
        &self,
        ctx: &AuthContext,
        strategy_id: Uuid,
    ) -> Result<ReplenishmentPreviewResponse, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let strategy = self
            .repo
            .get_strategy(ctx.owner_id, strategy_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        let data = self.repo.preview_strategy(ctx.owner_id, &strategy).await?;
        Ok(ReplenishmentPreviewResponse { data })
    }

    pub async fn upsert_location_group(
        &self,
        ctx: &AuthContext,
        req: UpsertReplenishmentLocationGroupRequest,
        idempotency_key: &str,
    ) -> Result<ReplenishmentLocationGroup, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let hash = idempotency::request_hash(&req)?;
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
            "/api/v1/replenishment/location-groups",
            Utc::now(),
        )
        .await?
        {
            return Ok(replay);
        }
        let group = self
            .repo
            .upsert_location_group(&mut tx, ctx.owner_id, Uuid::new_v4(), &req)
            .await?;
        idempotency::store_success_with_status(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/replenishment/location-groups",
            200,
            "replenishment_location_group",
            &group.id.to_string(),
            &group,
            Utc::now(),
        )
        .await?;
        tx.commit().await?;
        Ok(group)
    }

    async fn validate_strategy(
        &self,
        owner_id: Uuid,
        req: &UpsertReplenishmentStrategyRequest,
    ) -> Result<(), ReplenishmentError> {
        if validate_replenish_route(&req.source_type, &req.target_type).is_err() {
            return Err(ReplenishmentError::StrategyInvalid);
        }
        if req.min_safety_threshold < wms_domain::Quantity::ZERO
            || req.max_replenish_target <= req.min_safety_threshold
        {
            return Err(ReplenishmentError::StrategyInvalid);
        }
        if req.trigger_modes.is_empty()
            || req
                .trigger_modes
                .iter()
                .any(|mode| mode != "min_max" && mode != "wave_gap")
        {
            return Err(ReplenishmentError::StrategyInvalid);
        }
        match req.scope_type.as_str() {
            "product" => {
                if !self.repo.product_exists(owner_id, req.scope_ref).await? {
                    return Err(ReplenishmentError::ScopeNotFound);
                }
            }
            "category" => {
                if !self
                    .repo
                    .special_drug_category_exists(owner_id, req.scope_ref)
                    .await?
                {
                    return Err(ReplenishmentError::ScopeNotFound);
                }
            }
            "location_group" => {
                if !self
                    .repo
                    .location_group_exists(owner_id, req.scope_ref)
                    .await?
                {
                    return Err(ReplenishmentError::ScopeNotFound);
                }
            }
            _ => return Err(ReplenishmentError::StrategyInvalid),
        }
        Ok(())
    }
}

impl From<sqlx::Error> for ReplenishmentError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<IdempotencyError> for ReplenishmentError {
    fn from(value: IdempotencyError) -> Self {
        match value {
            IdempotencyError::Conflict => Self::IdempotencyConflict,
            IdempotencyError::Database(error) => Self::Database(error),
            IdempotencyError::Serialize(_) => Self::StrategyInvalid,
        }
    }
}

impl From<ReplenishmentRepoError> for ReplenishmentError {
    fn from(value: ReplenishmentRepoError) -> Self {
        match value {
            ReplenishmentRepoError::LocationBound => Self::LocationBound,
            ReplenishmentRepoError::LocationTypeMismatch => Self::StrategyInvalid,
            ReplenishmentRepoError::Database(error) => Self::Database(error),
        }
    }
}
