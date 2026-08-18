use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;
use wms_domain::{ReplenishmentStrategy, ReplenishmentTask};

use super::{ReplenishmentError, ReplenishmentService, MANAGE};
use crate::{auth::AuthContext, h2_lifecycle::publish_event_in_tx};

impl ReplenishmentService {
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
                .list_bound_locations(strategy.owner_id, strategy.id)
                .await?;
            let ctx = system_ctx(strategy.owner_id);
            for location_id in locations {
                for product_id in self.patrol_products(&strategy, location_id).await? {
                    match self
                        .generate_task(&ctx, strategy.id, location_id, product_id)
                        .await
                    {
                        Ok(Some(task)) => created.push(task),
                        Ok(None) => {}
                        Err(ReplenishmentError::SourceUnavailable)
                        | Err(ReplenishmentError::PutawayBlocked) => {
                            self.write_patrol_fail(
                                strategy.owner_id,
                                strategy.id,
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
                        Err(error) => return Err(error),
                    }
                }
            }
        }
        let _ = now;
        Ok(created)
    }

    async fn patrol_products(
        &self,
        strategy: &ReplenishmentStrategy,
        location_id: Uuid,
    ) -> Result<Vec<Uuid>, ReplenishmentError> {
        if strategy.scope_type == "product" {
            return Ok(vec![strategy.scope_ref]);
        }
        Ok(self
            .repo
            .products_at_location(strategy.owner_id, location_id)
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    async fn write_patrol_fail(
        &self,
        owner_id: Uuid,
        strategy_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        reason_code: &str,
        patrol_run_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), ReplenishmentError> {
        let mut tx = self.repo.pool().begin().await?;
        publish_event_in_tx(
            &mut tx,
            owner_id,
            &format!(
                "patrol_fail:{strategy_id}:{location_id}:{product_id}:{reason_code}:{patrol_run_id}"
            ),
            "replenishment.patrol_fail",
            "M3",
            "replenishment_task",
            &strategy_id.to_string(),
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
        tx.commit().await?;
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
        let fail_times: Vec<DateTime<Utc>> = sqlx::query_scalar(
            r#"
            SELECT created_at
              FROM event_bus_event
             WHERE owner_id = $1
               AND event_type = 'replenishment.patrol_fail'
               AND payload ->> 'target_location_id' = $2
               AND payload ->> 'product_id' = $3
               AND payload ->> 'reason_code' = $4
             ORDER BY created_at DESC
             LIMIT 3
            "#,
        )
        .bind(owner_id)
        .bind(location_id.to_string())
        .bind(product_id.to_string())
        .bind(reason_code)
        .fetch_all(self.repo.pool())
        .await?;
        let Some(oldest) = fail_times.get(2).copied() else {
            return Ok(());
        };
        let generated_after: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                  FROM replenishment_tasks
                 WHERE owner_id = $1
                   AND target_location_id = $2
                   AND product_id = $3
                   AND created_at > $4
            )
            "#,
        )
        .bind(owner_id)
        .bind(location_id)
        .bind(product_id)
        .bind(oldest)
        .fetch_one(self.repo.pool())
        .await?;
        if generated_after {
            return Ok(());
        }
        let mut tx = self.repo.pool().begin().await?;
        publish_event_in_tx(
            &mut tx,
            owner_id,
            &format!("replenishment.patrol_fail_repeat:{strategy_id}:{location_id}:{product_id}:{reason_code}"),
            "replenishment_patrol_fail_repeat",
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
