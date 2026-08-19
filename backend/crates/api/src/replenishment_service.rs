//! 补货策略、库位组与任务生成用例编排。

#[path = "replenishment_service_exception.rs"]
mod exception;
#[path = "replenishment_service_group.rs"]
mod group;
#[path = "replenishment_service_job.rs"]
mod job;
#[path = "replenishment_service_patrol.rs"]
mod patrol;
#[path = "replenishment_service_strategy.rs"]
mod strategy;
#[path = "replenishment_service_timeout.rs"]
mod timeout;
#[path = "replenishment_service_wave.rs"]
mod wave;

use chrono::Utc;
use uuid::Uuid;
use wms_domain::{
    is_temperature_zone_subset, task_qty, validate_external_fragrant, validate_replenish_route,
    zone_treats_as_qualified, BindReplenishmentLocationsRequest,
    BindReplenishmentLocationsResponse, CreateReplenishmentTaskRequest, PageMeta, Quantity,
    ReplenishmentLocationGroup, ReplenishmentLocationGroupListResponse, ReplenishmentPreviewItem,
    ReplenishmentPreviewResponse, ReplenishmentTask, ReplenishmentTaskListResponse,
    UpsertReplenishmentLocationGroupRequest, UpsertReplenishmentStrategyRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
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
const EXECUTE: &str = concat!("m3.replenishment", ".", "execute");

#[derive(Debug)]
pub enum ReplenishmentError {
    PermissionDenied,
    StrategyInvalid,
    ScopeNotFound,
    LocationBound,
    TaskNotFound,
    StrategyNotFound,
    GroupNotFound,
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
    ZoneDenied,
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

pub use crate::replenishment_repository::ListReplenishmentTasksFilter;

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
            .ok_or(ReplenishmentError::StrategyNotFound)?;
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
        write_audit(
            &mut tx,
            ctx,
            "bind_replenishment_locations",
            "replenishment_strategy",
            &strategy.id.to_string(),
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
            .ok_or(ReplenishmentError::StrategyNotFound)?;
        let locations = self.repo.preview_strategy(ctx.owner_id, &strategy).await?;
        let mut tx = self.repo.pool().begin().await?;
        let mut data = Vec::new();
        for location in locations {
            for product_id in self
                .patrol_products(&strategy, location.location_id)
                .await?
            {
                let available_qty = self
                    .repo
                    .pick_available_qty(&mut tx, ctx.owner_id, location.location_id, product_id)
                    .await?;
                data.push(ReplenishmentPreviewItem {
                    location_id: location.location_id,
                    location_code: location.location_code.clone(),
                    product_id: Some(product_id),
                    available_qty,
                    min_safety_threshold: strategy.min_safety_threshold,
                    max_replenish_target: strategy.max_replenish_target,
                    would_trigger: available_qty <= strategy.min_safety_threshold,
                });
            }
        }
        tx.commit().await?;
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
        write_audit(
            &mut tx,
            ctx,
            "upsert_replenishment_location_group",
            "replenishment_location_group",
            &group.id.to_string(),
        )
        .await?;
        tx.commit().await?;
        Ok(group)
    }

    pub async fn list_tasks(
        &self,
        ctx: &AuthContext,
        filter: &ListReplenishmentTasksFilter,
    ) -> Result<ReplenishmentTaskListResponse, ReplenishmentError> {
        let execute = if ctx.require_permission(MANAGE).is_ok() {
            None
        } else {
            ctx.require_permission(EXECUTE)
                .map_err(|_| ReplenishmentError::PermissionDenied)?;
            let scope = self
                .repo
                .operator_replenish_zone_scope(ctx.owner_id, ctx.user_id)
                .await?;
            Some(scope.to_list_filter(ctx.user_id))
        };
        let limit = filter.limit.unwrap_or(100).clamp(1, 200);
        let offset = filter
            .cursor
            .as_deref()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0);
        let mut data = self
            .repo
            .list_tasks(ctx.owner_id, filter, execute.as_ref())
            .await?;
        let next_cursor = if u32::try_from(data.len()).unwrap_or(u32::MAX) > limit {
            data.pop();
            Some(
                offset
                    .saturating_add(u32::try_from(data.len()).unwrap_or(0))
                    .to_string(),
            )
        } else {
            None
        };
        let count = u32::try_from(data.len()).unwrap_or(u32::MAX);
        Ok(ReplenishmentTaskListResponse {
            data,
            page: PageMeta {
                next_cursor,
                count,
                total: None,
            },
        })
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
        self.ensure_target_putaway(
            &mut tx,
            ctx.owner_id,
            &target,
            &target.location_type,
            product_id,
            req.qty,
        )
        .await?;
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

    pub(crate) async fn ensure_target_putaway(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner_id: Uuid,
        target: &crate::replenishment_repository::LocationRouteRow,
        expected_type: &str,
        product_id: Uuid,
        qty: Quantity,
    ) -> Result<(), ReplenishmentError> {
        if target.location_type != expected_type {
            return Err(ReplenishmentError::StrategyInvalid);
        }
        if !zone_treats_as_qualified(&target.quality_color) {
            return Err(ReplenishmentError::PutawayBlocked);
        }
        if !wms_domain::location_allows_inbound(&target.lock_status) {
            return Err(ReplenishmentError::PutawayBlocked);
        }
        if self
            .repo
            .location_has_work_lock(tx, owner_id, target.id)
            .await?
        {
            return Err(ReplenishmentError::PutawayBlocked);
        }
        let Some(product) = self
            .repo
            .load_product_putaway_attrs(tx, owner_id, product_id)
            .await?
        else {
            return Err(ReplenishmentError::ScopeNotFound);
        };
        if let Some(product_temp) = product.storage_condition.as_deref() {
            if !is_temperature_zone_subset(&target.temperature_zone, product_temp) {
                return Err(ReplenishmentError::PutawayBlocked);
            }
        }
        if !validate_external_fragrant(
            product.is_external_use,
            target.is_external_use_zone,
            product.is_fragrant,
            target.is_fragrant_zone,
        ) {
            return Err(ReplenishmentError::PutawayBlocked);
        }
        if let Some(unit_volume) = product.volume_cm3 {
            let units = i64::try_from(qty).unwrap_or(i64::MAX);
            let required = unit_volume * (units as f64);
            let available = target.max_volume_cm3.saturating_sub(target.used_volume_cm3);
            if required > available as f64 {
                return Err(ReplenishmentError::PutawayBlocked);
            }
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
        if !wms_domain::location_allows_outbound(&source.lock_status) {
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
            .map_err(map_inventory_replenish_error)?;
        write_audit(
            tx,
            ctx,
            "create_replenishment_task",
            "replenishment_task",
            &created.id.to_string(),
        )
        .await?;
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

pub(crate) fn map_inventory_replenish_error(
    error: crate::inventory::InventoryReplenishError,
) -> ReplenishmentError {
    match error {
        crate::inventory::InventoryReplenishError::Insufficient => {
            ReplenishmentError::SourceUnavailable
        }
        crate::inventory::InventoryReplenishError::Database(message) => {
            ReplenishmentError::Database(sqlx::Error::Protocol(message))
        }
        crate::inventory::InventoryReplenishError::InvalidQuantity => {
            ReplenishmentError::QtyExceeded
        }
        crate::inventory::InventoryReplenishError::NotFound => ReplenishmentError::PutawayBlocked,
    }
}

pub(crate) fn ratio_to_tenths(raw: &str) -> Option<i64> {
    let value: f64 = raw.parse().ok()?;
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    Some((value * 10.0).round() as i64)
}

pub(crate) fn resolve_source_lpn(
    source: &SourceBatchLock,
    qty: Quantity,
    source_type: &str,
    full_lpn_ratio_tenths: i64,
) -> Option<Uuid> {
    if source_type == "case_pick" {
        return None;
    }
    let lpn_id = source.container_id?;
    if source.lpn_on_hand <= Quantity::ZERO {
        return None;
    }
    let tenths = if full_lpn_ratio_tenths <= 0 {
        8
    } else {
        full_lpn_ratio_tenths
    };
    if qty * Quantity::from(10) >= source.lpn_on_hand * Quantity::from(tenths) {
        Some(lpn_id)
    } else {
        None
    }
}

pub(crate) async fn write_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_type: &str,
    resource_id: &str,
) -> Result<(), ReplenishmentError> {
    append_event_in_tx(
        tx,
        &AuditWriteRequest::from_auth_context(ctx, action, "M3", resource_type, resource_id, None),
    )
    .await
    .map_err(|error| ReplenishmentError::Database(sqlx::Error::Protocol(format!("{error:?}"))))?;
    Ok(())
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
            IdempotencyError::Serialize(_) => {
                Self::Database(sqlx::Error::Protocol("idempotency serialize failed".into()))
            }
        }
    }
}

impl From<ReplenishmentRepoError> for ReplenishmentError {
    fn from(value: ReplenishmentRepoError) -> Self {
        match value {
            ReplenishmentRepoError::LocationBound => Self::LocationBound,
            ReplenishmentRepoError::LocationTypeMismatch => Self::StrategyInvalid,
            ReplenishmentRepoError::ScopeNotFound => Self::ScopeNotFound,
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
