use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;
use wms_domain::{task_qty, Quantity, ReplenishmentStrategy, ReplenishmentTask};

use super::{resolve_source_lpn, ReplenishmentError, ReplenishmentService, MANAGE};
use crate::{auth::AuthContext, h2_lifecycle::publish_event_in_tx};

impl ReplenishmentService {
    pub async fn generate_task(
        &self,
        ctx: &AuthContext,
        strategy_id: Uuid,
        target_location_id: Uuid,
        product_id: Uuid,
    ) -> Result<Vec<ReplenishmentTask>, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let mut tx = self.repo.pool().begin().await?;
        let strategy = self
            .repo
            .get_strategy_in_tx(&mut tx, ctx.owner_id, strategy_id)
            .await?
            .ok_or(ReplenishmentError::StrategyNotFound)?;
        if !strategy.enabled || !strategy.trigger_modes.iter().any(|mode| mode == "min_max") {
            return Ok(Vec::new());
        }
        let target = self
            .repo
            .load_location_route(&mut tx, ctx.owner_id, target_location_id)
            .await?
            .ok_or(ReplenishmentError::ScopeNotFound)?;
        if target.replenish_strategy_id != Some(strategy.id) {
            return Ok(Vec::new());
        }
        let available = self
            .repo
            .pick_available_qty(&mut tx, ctx.owner_id, target_location_id, product_id)
            .await?;
        if available > strategy.min_safety_threshold {
            return Ok(Vec::new());
        }
        let pack = self
            .repo
            .default_pack_ratio(&mut tx, ctx.owner_id, product_id)
            .await?;
        let mut remaining = strategy.max_replenish_target - available;
        let mut created = Vec::new();
        let ratio_tenths = self
            .repo
            .runtime_setting_in_tx(&mut tx, Some(ctx.owner_id), "replenishment.full_lpn_ratio")
            .await?
            .as_deref()
            .and_then(super::ratio_to_tenths)
            .unwrap_or(8);
        while remaining >= Quantity::from(pack) {
            let Some(source) = self
                .repo
                .lock_fefo_source(
                    &mut tx,
                    ctx.owner_id,
                    product_id,
                    &strategy.source_type,
                    Quantity::from(pack),
                )
                .await?
            else {
                if created.is_empty() {
                    return Err(ReplenishmentError::SourceUnavailable);
                }
                break;
            };
            self.ensure_source_ok(&source, &strategy.source_type, None)?;
            let qty = task_qty(remaining, source.available_qty, pack);
            if qty <= Quantity::ZERO {
                break;
            }
            match self
                .ensure_target_putaway(
                    &mut tx,
                    ctx.owner_id,
                    &target,
                    &strategy.target_type,
                    product_id,
                    qty,
                )
                .await
            {
                Ok(()) => {}
                Err(ReplenishmentError::PutawayBlocked) if !created.is_empty() => break,
                Err(error) => return Err(error),
            }
            let source_lpn_id =
                resolve_source_lpn(&source, qty, &strategy.source_type, ratio_tenths);
            let task = self
                .persist_task(
                    &mut tx,
                    ctx,
                    &source,
                    target_location_id,
                    qty,
                    "min_max",
                    "normal",
                    Some(strategy.id),
                    source_lpn_id,
                    "system:min_max",
                    None,
                )
                .await?;
            remaining -= qty;
            created.push(task);
        }
        tx.commit().await?;
        Ok(created)
    }

    pub async fn run_min_max_patrol(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<ReplenishmentTask>, ReplenishmentError> {
        let patrol_run_id = Uuid::new_v4();
        let strategies = self.repo.list_enabled_min_max_strategies().await?;
        let mut created = Vec::new();
        for strategy in strategies {
            let locations = self
                .repo
                .list_bound_locations(strategy.owner_id, strategy.id, &strategy.target_type)
                .await?;
            let ctx = system_ctx(strategy.owner_id);
            for location_id in locations {
                for product_id in self.patrol_products(&strategy, location_id).await? {
                    match self
                        .generate_task(&ctx, strategy.id, location_id, product_id)
                        .await
                    {
                        Ok(tasks) => created.extend(tasks),
                        Err(ReplenishmentError::SourceUnavailable) => {
                            self.write_patrol_fail(
                                strategy.owner_id,
                                Some(strategy.id),
                                location_id,
                                product_id,
                                "source_unavailable",
                                patrol_run_id,
                                now,
                            )
                            .await?;
                            self.maybe_alert_patrol_fail_repeat(
                                strategy.owner_id,
                                strategy.id,
                                location_id,
                                product_id,
                                "source_unavailable",
                                now,
                            )
                            .await?;
                        }
                        Err(ReplenishmentError::PutawayBlocked) => {
                            self.write_patrol_fail(
                                strategy.owner_id,
                                Some(strategy.id),
                                location_id,
                                product_id,
                                "putaway_blocked",
                                patrol_run_id,
                                now,
                            )
                            .await?;
                            self.maybe_alert_patrol_fail_repeat(
                                strategy.owner_id,
                                strategy.id,
                                location_id,
                                product_id,
                                "putaway_blocked",
                                now,
                            )
                            .await?;
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
        }
        Ok(created)
    }

    pub(crate) async fn patrol_products(
        &self,
        strategy: &ReplenishmentStrategy,
        location_id: Uuid,
    ) -> Result<Vec<Uuid>, ReplenishmentError> {
        if strategy.scope_type == "product" {
            return Ok(vec![strategy.scope_ref]);
        }
        if strategy.scope_type == "category" {
            return Ok(self
                .repo
                .products_at_location_for_category(
                    strategy.owner_id,
                    location_id,
                    strategy.scope_ref,
                )
                .await?);
        }
        Ok(self
            .repo
            .products_at_location(strategy.owner_id, location_id)
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn write_patrol_fail(
        &self,
        owner_id: Uuid,
        strategy_id: Option<Uuid>,
        location_id: Uuid,
        product_id: Uuid,
        reason_code: &str,
        patrol_run_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), ReplenishmentError> {
        let mut tx = self.repo.pool().begin().await?;
        self.write_patrol_fail_in_tx(
            &mut tx,
            owner_id,
            strategy_id,
            location_id,
            product_id,
            reason_code,
            patrol_run_id,
            now,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn write_patrol_fail_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner_id: Uuid,
        strategy_id: Option<Uuid>,
        location_id: Uuid,
        product_id: Uuid,
        reason_code: &str,
        patrol_run_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), ReplenishmentError> {
        let strategy_key = strategy_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".into());
        publish_event_in_tx(
            tx,
            owner_id,
            &format!(
                "patrol_fail:{strategy_key}:{location_id}:{product_id}:{reason_code}:{patrol_run_id}"
            ),
            "replenishment.patrol_fail",
            "M3",
            "replenishment_task",
            &strategy_key,
            json!({
                "target_location_id": location_id,
                "product_id": product_id,
                "reason_code": reason_code,
                "strategy_id": strategy_id
            }),
            now,
        )
        .await
        .map_err(|error| {
            ReplenishmentError::Database(sqlx::Error::Protocol(format!("{error:?}")))
        })?;
        Ok(())
    }

    async fn maybe_alert_patrol_fail_repeat(
        &self,
        owner_id: Uuid,
        strategy_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        reason_code: &str,
        now: DateTime<Utc>,
    ) -> Result<(), ReplenishmentError> {
        let fail_times = self
            .repo
            .recent_patrol_fail_times(owner_id, location_id, product_id, reason_code)
            .await?;
        let Some(oldest) = fail_times.get(2).copied() else {
            return Ok(());
        };
        let generated_after = self
            .repo
            .has_generated_task_since(owner_id, location_id, product_id, oldest)
            .await?;
        if generated_after {
            return Ok(());
        }
        let mut tx = self.repo.pool().begin().await?;
        publish_event_in_tx(
            &mut tx,
            owner_id,
            &format!("replenishment.patrol_fail_repeat:{strategy_id}:{location_id}:{product_id}:{reason_code}"),
            "business.replenishment_patrol_fail_repeat",
            "M3",
            "replenishment_task",
            &strategy_id.to_string(),
            json!({
                "target_location_id": location_id,
                "product_id": product_id,
                "reason_code": reason_code,
                "strategy_id": strategy_id,
                "consecutive_fail_count": 3
            }),
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

fn system_ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::nil(),
        owner_id,
        actor_name: "system:min_max".into(),
        permissions: vec![MANAGE.into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}
