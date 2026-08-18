use serde_json::json;
use uuid::Uuid;
use wms_domain::{task_qty, Quantity, ReplenishmentTask};

use super::{
    resolve_source_lpn, CreateWaveGapTasksRequest, ReplenishmentError, ReplenishmentService,
    WaveLink, MANAGE,
};
use crate::{auth::AuthContext, h2_lifecycle::publish_event_in_tx};

impl ReplenishmentService {
    pub async fn create_wave_gap_tasks(
        &self,
        ctx: &AuthContext,
        req: CreateWaveGapTasksRequest,
    ) -> Result<Vec<ReplenishmentTask>, ReplenishmentError> {
        ctx.require_permission(MANAGE)
            .map_err(|_| ReplenishmentError::PermissionDenied)?;
        let mut tx = self.repo.pool().begin().await?;
        let target = self
            .repo
            .load_location_route(&mut tx, ctx.owner_id, req.target_location_id)
            .await?
            .ok_or(ReplenishmentError::ScopeNotFound)?;
        if target.location_type != "case_pick" && target.location_type != "piece_pick" {
            return Err(ReplenishmentError::StrategyInvalid);
        }
        let strategy = self
            .repo
            .find_wave_gap_strategy(
                &mut tx,
                ctx.owner_id,
                req.product_id,
                req.target_location_id,
            )
            .await?;
        let (source_type, strategy_id) = match &strategy {
            Some(found) => (found.source_type.as_str(), Some(found.id)),
            None => ("storage", None),
        };
        let available = self
            .repo
            .pick_available_qty(
                &mut tx,
                ctx.owner_id,
                req.target_location_id,
                req.product_id,
            )
            .await?;
        let urgent = req.demand_qty - available;
        if urgent <= Quantity::ZERO {
            return Ok(Vec::new());
        }
        let pack = self
            .repo
            .default_pack_ratio(&mut tx, ctx.owner_id, req.product_id)
            .await?;
        let mut remaining = urgent;
        let mut created = Vec::new();
        let created_by = format!("system:wave:{}", req.wave_id);
        let wave = WaveLink {
            wave_id: req.wave_id,
            outbound_order_id: req.outbound_order_id,
            outbound_line_no: req.outbound_line_no,
        };
        while remaining >= Quantity::from(pack) {
            let Some(source) = self
                .repo
                .lock_fefo_source(
                    &mut tx,
                    ctx.owner_id,
                    req.product_id,
                    source_type,
                    Quantity::from(pack),
                )
                .await?
            else {
                break;
            };
            self.ensure_source_ok(&source, source_type, None)?;
            let qty = task_qty(remaining, source.available_qty, pack);
            if qty <= Quantity::ZERO {
                break;
            }
            match self
                .ensure_target_putaway(
                    ctx.owner_id,
                    &target,
                    &target.location_type,
                    req.product_id,
                    qty,
                )
                .await
            {
                Ok(()) => {}
                Err(ReplenishmentError::PutawayBlocked) if !created.is_empty() => break,
                Err(error) => return Err(error),
            }
            let source_lpn_id = resolve_source_lpn(&source, qty, source_type);
            let task = self
                .persist_task(
                    &mut tx,
                    ctx,
                    &source,
                    req.target_location_id,
                    qty,
                    "wave_gap",
                    "urgent",
                    strategy_id,
                    source_lpn_id,
                    &created_by,
                    Some(wave),
                )
                .await?;
            publish_event_in_tx(
                &mut tx,
                ctx.owner_id,
                &format!("replenishment.waiting:{}", task.id),
                "replenishment.waiting",
                "M3",
                "replenishment_task",
                &task.id.to_string(),
                json!({
                    "wave_id": req.wave_id,
                    "outbound_order_id": req.outbound_order_id,
                    "outbound_line_no": req.outbound_line_no,
                    "task_id": task.id,
                    "qty": task.qty
                }),
                chrono::Utc::now(),
            )
            .await
            .map_err(|error| {
                ReplenishmentError::Database(sqlx::Error::Protocol(format!("{error:?}")))
            })?;
            remaining -= qty;
            created.push(task);
        }
        tx.commit().await?;
        Ok(created)
    }

    pub async fn fill_wave_pick_gaps(
        &self,
        owner_id: Uuid,
        wave_id: Uuid,
    ) -> Result<Vec<ReplenishmentTask>, ReplenishmentError> {
        let ctx = AuthContext {
            user_id: Uuid::nil(),
            owner_id,
            actor_name: format!("system:wave:{wave_id}"),
            permissions: vec![MANAGE.into()],
            jti: Uuid::new_v4().to_string(),
            warehouse_scope: None,
        };
        let lines = self.repo.list_wave_gap_lines(owner_id, wave_id).await?;
        let mut created = Vec::new();
        for line in lines {
            let candidates = self
                .repo
                .list_pick_target_candidates(owner_id, line.warehouse_id, line.product_id)
                .await?;
            let Some(target_location_id) = candidates.into_iter().next() else {
                continue;
            };
            match self
                .create_wave_gap_tasks(
                    &ctx,
                    CreateWaveGapTasksRequest {
                        wave_id: line.wave_id,
                        outbound_order_id: line.outbound_order_id,
                        outbound_line_no: line.outbound_line_no,
                        product_id: line.product_id,
                        demand_qty: line.demand_qty,
                        target_location_id,
                    },
                )
                .await
            {
                Ok(tasks) => created.extend(tasks),
                Err(ReplenishmentError::PutawayBlocked)
                | Err(ReplenishmentError::SourceUnavailable)
                | Err(ReplenishmentError::StrategyInvalid)
                | Err(ReplenishmentError::ScopeNotFound) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(created)
    }
}
