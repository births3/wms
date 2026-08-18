//! 补货策略、库位组与任务生成用例编排。

#[path = "replenishment_service_exception.rs"]
mod exception;
#[path = "replenishment_service_job.rs"]
mod job;
#[path = "replenishment_service_patrol.rs"]
mod patrol;
#[path = "replenishment_service_timeout.rs"]
mod timeout;
#[path = "replenishment_service_wave.rs"]
mod wave;

use chrono::Utc;
use uuid::Uuid;
use wms_domain::{
    task_qty, validate_replenish_route, zone_treats_as_qualified,
    BindReplenishmentLocationsRequest, BindReplenishmentLocationsResponse,
    CreateReplenishmentTaskRequest, PageMeta, Quantity, ReplenishmentLocationGroup,
    ReplenishmentLocationGroupListResponse, ReplenishmentPreviewResponse, ReplenishmentStrategy,
    ReplenishmentStrategyListResponse, ReplenishmentTask, UpsertReplenishmentLocationGroupRequest,
    UpsertReplenishmentStrategyRequest,
};

use crate::{
    auth::AuthContext,
    document_numbering::{
        DocumentNumberingError, GenerateDocumentNumberRequest, PgDocumentNumberingService,
    },
    idempotency::{self, IdempotencyError},
    inventory::reserve_replenish_in_tx,
    replenishment_repository::{
        PgReplenishmentRepository, ReplenishmentRepoError, SourceBatchLock,
    },
};

const MANAGE: &str = "m3.replenishment.manage";

#[derive(Debug)]
pub enum ReplenishmentError {
    PermissionDenied,
    StrategyInvalid,
    ScopeNotFound,
    LocationBound,
    TaskNotFound,
    SourceUnavailable,
    NumberingUnavailable,
    PutawayBlocked,
    ClaimConflict,
    QtyExceeded,
    SourceMismatch,
    TargetMismatch,
    StateInvalid,
    CancelBlocked,
    ReturnBlocked,
    IdempotencyRequired,
    IdempotencyConflict,
    Database(sqlx::Error),
}

#[derive(Clone, Copy)]
pub(crate) struct WaveLink {
    pub wave_id: Uuid,
    pub outbound_order_id: Uuid,
    pub outbound_line_no: i32,
}

pub struct CreateWaveGapTasksRequest {
    pub wave_id: Uuid,
    pub outbound_order_id: Uuid,
    pub outbound_line_no: i32,
    pub product_id: Uuid,
    pub demand_qty: Quantity,
    pub target_location_id: Uuid,
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

    pub async fn list_strategies(
        &self,
        ctx: &AuthContext,
        keyword: Option<&str>,
        enabled: Option<bool>,
        scope_type: Option<&str>,
    ) -> Result<ReplenishmentStrategyListResponse, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let data = self
            .repo
            .list_strategies(ctx.owner_id, keyword, enabled, scope_type)
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
            .ok_or(ReplenishmentError::TaskNotFound)
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

    pub async fn list_location_groups(
        &self,
        ctx: &AuthContext,
    ) -> Result<ReplenishmentLocationGroupListResponse, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let data = self.repo.list_location_groups(ctx.owner_id).await?;
        let count = u32::try_from(data.len()).unwrap_or(u32::MAX);
        Ok(ReplenishmentLocationGroupListResponse {
            data,
            page: PageMeta {
                next_cursor: None,
                count,
                total: Some(count),
            },
        })
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

    pub async fn generate_task(
        &self,
        ctx: &AuthContext,
        strategy_id: Uuid,
        target_location_id: Uuid,
        product_id: Uuid,
    ) -> Result<Option<ReplenishmentTask>, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let strategy = self
            .repo
            .get_strategy(ctx.owner_id, strategy_id)
            .await?
            .ok_or(ReplenishmentError::TaskNotFound)?;
        if !strategy.enabled || !strategy.trigger_modes.iter().any(|mode| mode == "min_max") {
            return Ok(None);
        }
        let mut tx = self.repo.pool().begin().await?;
        let target = self
            .repo
            .load_location_route(&mut tx, ctx.owner_id, target_location_id)
            .await?
            .ok_or(ReplenishmentError::ScopeNotFound)?;
        if target.replenish_strategy_id != Some(strategy.id) {
            return Ok(None);
        }
        self.ensure_target_putaway(&target, &strategy.target_type)?;
        let available = self
            .repo
            .pick_available_qty(&mut tx, ctx.owner_id, target_location_id, product_id)
            .await?;
        if available > strategy.min_safety_threshold {
            return Ok(None);
        }
        let pack = self
            .repo
            .default_pack_ratio(&mut tx, ctx.owner_id, product_id)
            .await?;
        let need = strategy.max_replenish_target - available;
        let source = self
            .repo
            .lock_fefo_source(
                &mut tx,
                ctx.owner_id,
                product_id,
                &strategy.source_type,
                Quantity::from(pack),
            )
            .await?
            .ok_or(ReplenishmentError::SourceUnavailable)?;
        self.ensure_source_ok(&source, &strategy.source_type, None)?;
        let qty = task_qty(need, source.available_qty, pack);
        if qty <= Quantity::ZERO {
            return Ok(None);
        }
        let created = self
            .persist_task(
                &mut tx,
                ctx,
                &source,
                target_location_id,
                qty,
                "min_max",
                "normal",
                Some(strategy.id),
                None,
                "system:min_max",
                None,
            )
            .await?;
        tx.commit().await?;
        Ok(Some(created))
    }

    pub async fn create_task(
        &self,
        ctx: &AuthContext,
        req: CreateReplenishmentTaskRequest,
        idempotency_key: &str,
    ) -> Result<ReplenishmentTask, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let hash = idempotency::request_hash(&req)?;
        let mut tx = self.repo.pool().begin().await?;
        idempotency::lock_key(&mut tx, "replenishment_task", ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/replenishment/tasks",
            Utc::now(),
        )
        .await?
        {
            return Ok(replay);
        }
        let source = self
            .repo
            .lock_source_batch(&mut tx, ctx.owner_id, req.source_batch_id)
            .await?
            .ok_or(ReplenishmentError::SourceUnavailable)?;
        if source.location_id != req.source_location_id {
            return Err(ReplenishmentError::SourceUnavailable);
        }
        let product_id = source.product_id.ok_or(ReplenishmentError::ScopeNotFound)?;
        let pack = self
            .repo
            .default_pack_ratio(&mut tx, ctx.owner_id, product_id)
            .await?;
        if task_qty(req.qty, req.qty, pack) != req.qty || req.qty <= Quantity::ZERO {
            return Err(ReplenishmentError::StrategyInvalid);
        }
        let target = self
            .repo
            .load_location_route(&mut tx, ctx.owner_id, req.target_location_id)
            .await?
            .ok_or(ReplenishmentError::ScopeNotFound)?;
        if validate_replenish_route(&source.location_type, &target.location_type).is_err() {
            return Err(ReplenishmentError::StrategyInvalid);
        }
        self.ensure_target_putaway(&target, &target.location_type)?;
        self.ensure_source_ok(&source, &source.location_type, req.source_lpn_id)?;
        if source.available_qty < req.qty {
            return Err(ReplenishmentError::SourceUnavailable);
        }
        let created = self
            .persist_task(
                &mut tx,
                ctx,
                &source,
                req.target_location_id,
                req.qty,
                "manual",
                "normal",
                None,
                req.source_lpn_id,
                &ctx.actor_name,
                None,
            )
            .await?;
        idempotency::store_success_with_status(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/replenishment/tasks",
            200,
            "replenishment_task",
            &created.id.to_string(),
            &created,
            Utc::now(),
        )
        .await?;
        tx.commit().await?;
        Ok(created)
    }

    fn ensure_target_putaway(
        &self,
        target: &crate::replenishment_repository::LocationRouteRow,
        expected_type: &str,
    ) -> Result<(), ReplenishmentError> {
        if target.location_type != expected_type {
            return Err(ReplenishmentError::StrategyInvalid);
        }
        if !zone_treats_as_qualified(&target.quality_color) {
            return Err(ReplenishmentError::PutawayBlocked);
        }
        if target.lock_status == "lock_in" || target.lock_status == "lock_all" {
            return Err(ReplenishmentError::PutawayBlocked);
        }
        Ok(())
    }

    fn ensure_source_ok(
        &self,
        source: &SourceBatchLock,
        expected_type: &str,
        source_lpn_id: Option<Uuid>,
    ) -> Result<(), ReplenishmentError> {
        if source.location_type != expected_type {
            return Err(ReplenishmentError::StrategyInvalid);
        }
        if source.status != "qualified" {
            return Err(ReplenishmentError::SourceUnavailable);
        }
        if source.lock_status == "lock_out" || source.lock_status == "lock_all" {
            return Err(ReplenishmentError::SourceUnavailable);
        }
        if matches!(
            source.current_lock_category.as_deref(),
            Some("quarantine") | Some("rejected")
        ) {
            return Err(ReplenishmentError::SourceUnavailable);
        }
        if expected_type == "case_pick" && source_lpn_id.is_some() {
            return Err(ReplenishmentError::StrategyInvalid);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn persist_task(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        ctx: &AuthContext,
        source: &SourceBatchLock,
        target_location_id: Uuid,
        qty: Quantity,
        trigger_mode: &str,
        priority: &str,
        strategy_id: Option<Uuid>,
        source_lpn_id: Option<Uuid>,
        created_by: &str,
        wave: Option<WaveLink>,
    ) -> Result<ReplenishmentTask, ReplenishmentError> {
        let task_id = Uuid::new_v4();
        let now = Utc::now();
        let numbered = PgDocumentNumberingService::new()
            .generate_in_tx(
                tx,
                ctx,
                GenerateDocumentNumberRequest {
                    document_type: "replenishment_task".into(),
                    idempotency_key: format!("m3-replenish-task:{task_id}"),
                    source_module: "M3".into(),
                    source_document_id: Some(task_id),
                },
                now,
            )
            .await?;
        let product_id = source.product_id.ok_or(ReplenishmentError::ScopeNotFound)?;
        let created = self
            .repo
            .insert_task(
                tx,
                ctx.owner_id,
                task_id,
                &numbered.value.generated_no,
                trigger_mode,
                priority,
                strategy_id,
                source,
                source_lpn_id,
                target_location_id,
                product_id,
                qty,
                created_by,
                wave.map(|link| link.wave_id),
                wave.map(|link| link.outbound_order_id),
                wave.map(|link| link.outbound_line_no),
            )
            .await?;
        reserve_replenish_in_tx(tx, ctx.owner_id, source.id, target_location_id, qty, now)
            .await
            .map_err(|error| match error {
                crate::inventory::InventoryReplenishError::Insufficient => {
                    ReplenishmentError::SourceUnavailable
                }
                _ => ReplenishmentError::PutawayBlocked,
            })?;
        Ok(created)
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

impl From<DocumentNumberingError> for ReplenishmentError {
    fn from(value: DocumentNumberingError) -> Self {
        match value {
            DocumentNumberingError::RuleNotFound
            | DocumentNumberingError::DocumentTypeInvalid
            | DocumentNumberingError::InvalidRule => Self::NumberingUnavailable,
            DocumentNumberingError::IdempotencyConflict => Self::IdempotencyConflict,
            other => Self::Database(sqlx::Error::Protocol(format!("{other:?}"))),
        }
    }
}
