use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    BindReplenishmentLocationsResponse, ReplenishmentLocationGroup, ReplenishmentPreviewItem,
    ReplenishmentStrategy, UpsertReplenishmentLocationGroupRequest,
    UpsertReplenishmentStrategyRequest,
};

use super::{GroupRow, PgReplenishmentRepository, PreviewRow, ReplenishmentRepoError, StrategyRow};

impl PgReplenishmentRepository {
    pub async fn list_strategies(
        &self,
        owner_id: Uuid,
        keyword: Option<&str>,
        enabled: Option<bool>,
        scope_type: Option<&str>,
    ) -> Result<Vec<ReplenishmentStrategy>, sqlx::Error> {
        sqlx::query_as::<_, StrategyRow>(
            r#"
            SELECT id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
                   location_type, source_type, target_type,
                   min_safety_threshold, max_replenish_target, trigger_modes, enabled
              FROM replenishment_strategies
             WHERE owner_id = $1
               AND ($2::text IS NULL OR strategy_code ILIKE '%' || $2 || '%'
                    OR strategy_name ILIKE '%' || $2 || '%')
               AND ($3::bool IS NULL OR enabled = $3)
               AND ($4::text IS NULL OR scope_type = $4)
             ORDER BY strategy_code
            "#,
        )
        .bind(owner_id)
        .bind(keyword)
        .bind(enabled)
        .bind(scope_type)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
    }

    pub async fn update_strategy(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        id: Uuid,
        req: &UpsertReplenishmentStrategyRequest,
    ) -> Result<Option<ReplenishmentStrategy>, sqlx::Error> {
        sqlx::query_as::<_, StrategyRow>(
            r#"
            UPDATE replenishment_strategies
               SET strategy_name = $3,
                   scope_type = $4,
                   scope_ref = $5,
                   location_type = $6,
                   source_type = $7,
                   target_type = $8,
                   min_safety_threshold = $9,
                   max_replenish_target = $10,
                   trigger_modes = $11,
                   enabled = $12,
                   updated_at = now()
             WHERE owner_id = $1 AND id = $2
         RETURNING id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
                   location_type, source_type, target_type,
                   min_safety_threshold, max_replenish_target, trigger_modes, enabled
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .bind(&req.strategy_name)
        .bind(&req.scope_type)
        .bind(req.scope_ref)
        .bind(&req.target_type)
        .bind(&req.source_type)
        .bind(&req.target_type)
        .bind(req.min_safety_threshold)
        .bind(req.max_replenish_target)
        .bind(&req.trigger_modes)
        .bind(req.enabled)
        .fetch_optional(&mut **tx)
        .await
        .map(|row| row.map(Into::into))
    }

    pub async fn disable_strategy(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<Option<ReplenishmentStrategy>, sqlx::Error> {
        sqlx::query_as::<_, StrategyRow>(
            r#"
            UPDATE replenishment_strategies
               SET enabled = FALSE,
                   updated_at = now()
             WHERE owner_id = $1 AND id = $2
         RETURNING id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
                   location_type, source_type, target_type,
                   min_safety_threshold, max_replenish_target, trigger_modes, enabled
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map(|row| row.map(Into::into))
    }

    pub async fn list_location_groups(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<ReplenishmentLocationGroup>, sqlx::Error> {
        let groups = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT id, owner_id, group_code, group_name, enabled
              FROM replenishment_location_groups
             WHERE owner_id = $1
             ORDER BY group_code
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;
        let mut result = Vec::with_capacity(groups.len());
        for group in groups {
            let location_ids = sqlx::query_scalar::<_, Uuid>(
                r#"
                SELECT location_id
                  FROM replenishment_location_group_members
                 WHERE group_id = $1
                "#,
            )
            .bind(group.id)
            .fetch_all(&self.pool)
            .await?;
            result.push(ReplenishmentLocationGroup {
                id: group.id,
                owner_id: group.owner_id,
                group_code: group.group_code,
                group_name: group.group_name,
                enabled: group.enabled,
                location_ids,
            });
        }
        Ok(result)
    }

    pub async fn get_location_group(
        &self,
        owner_id: Uuid,
        group_id: Uuid,
    ) -> Result<Option<ReplenishmentLocationGroup>, sqlx::Error> {
        let group = sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT id, owner_id, group_code, group_name, enabled
              FROM replenishment_location_groups
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(group_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(group) = group else {
            return Ok(None);
        };
        let location_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT location_id
              FROM replenishment_location_group_members
             WHERE group_id = $1
            "#,
        )
        .bind(group.id)
        .fetch_all(&self.pool)
        .await?;
        Ok(Some(ReplenishmentLocationGroup {
            id: group.id,
            owner_id: group.owner_id,
            group_code: group.group_code,
            group_name: group.group_name,
            enabled: group.enabled,
            location_ids,
        }))
    }

    pub async fn update_location_group(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        group_id: Uuid,
        req: &UpsertReplenishmentLocationGroupRequest,
    ) -> Result<Option<ReplenishmentLocationGroup>, sqlx::Error> {
        let updated = sqlx::query_as::<_, GroupRow>(
            r#"
            UPDATE replenishment_location_groups
               SET group_code = $3,
                   group_name = $4,
                   enabled = $5,
                   updated_at = now()
             WHERE owner_id = $1 AND id = $2
         RETURNING id, owner_id, group_code, group_name, enabled
            "#,
        )
        .bind(owner_id)
        .bind(group_id)
        .bind(&req.group_code)
        .bind(&req.group_name)
        .bind(req.enabled)
        .fetch_optional(&mut **tx)
        .await?;
        let Some(group) = updated else {
            return Ok(None);
        };
        sqlx::query("DELETE FROM replenishment_location_group_members WHERE group_id = $1")
            .bind(group.id)
            .execute(&mut **tx)
            .await?;
        for location_id in &req.location_ids {
            sqlx::query(
                r#"
                INSERT INTO replenishment_location_group_members (group_id, location_id)
                VALUES ($1, $2)
                "#,
            )
            .bind(group.id)
            .bind(location_id)
            .execute(&mut **tx)
            .await?;
        }
        Ok(Some(ReplenishmentLocationGroup {
            id: group.id,
            owner_id: group.owner_id,
            group_code: group.group_code,
            group_name: group.group_name,
            enabled: group.enabled,
            location_ids: req.location_ids.clone(),
        }))
    }

    pub async fn disable_location_group(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        group_id: Uuid,
    ) -> Result<Option<ReplenishmentLocationGroup>, sqlx::Error> {
        let updated = sqlx::query_as::<_, GroupRow>(
            r#"
            UPDATE replenishment_location_groups
               SET enabled = FALSE,
                   updated_at = now()
             WHERE owner_id = $1 AND id = $2
         RETURNING id, owner_id, group_code, group_name, enabled
            "#,
        )
        .bind(owner_id)
        .bind(group_id)
        .fetch_optional(&mut **tx)
        .await?;
        let Some(group) = updated else {
            return Ok(None);
        };
        sqlx::query(
            r#"
            UPDATE warehouse_locations loc
               SET replenish_strategy_id = NULL,
                   updated_at = now()
             WHERE loc.owner_id = $1
               AND loc.id IN (
                    SELECT member.location_id
                      FROM replenishment_location_group_members member
                     WHERE member.group_id = $2
               )
               AND loc.replenish_strategy_id IN (
                    SELECT strategy.id
                      FROM replenishment_strategies strategy
                     WHERE strategy.owner_id = $1
                       AND strategy.scope_type = 'location_group'
                       AND strategy.scope_ref = $2
               )
            "#,
        )
        .bind(owner_id)
        .bind(group_id)
        .execute(&mut **tx)
        .await?;
        let location_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT location_id
              FROM replenishment_location_group_members
             WHERE group_id = $1
            "#,
        )
        .bind(group.id)
        .fetch_all(&mut **tx)
        .await?;
        Ok(Some(ReplenishmentLocationGroup {
            id: group.id,
            owner_id: group.owner_id,
            group_code: group.group_code,
            group_name: group.group_name,
            enabled: group.enabled,
            location_ids,
        }))
    }

    pub async fn replace_strategy_locations(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        strategy_id: Uuid,
        target_type: &str,
        location_ids: &[Uuid],
    ) -> Result<BindReplenishmentLocationsResponse, ReplenishmentRepoError> {
        let conflicting: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id
              FROM warehouse_locations
             WHERE owner_id = $1
               AND id = ANY($2)
               AND replenish_strategy_id IS NOT NULL
               AND replenish_strategy_id <> $3
             LIMIT 1
            "#,
        )
        .bind(owner_id)
        .bind(location_ids)
        .bind(strategy_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(ReplenishmentRepoError::Database)?;
        if conflicting.is_some() {
            return Err(ReplenishmentRepoError::LocationBound);
        }

        let typed: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
              FROM warehouse_locations
             WHERE owner_id = $1
               AND id = ANY($2)
               AND location_type = $3
            "#,
        )
        .bind(owner_id)
        .bind(location_ids)
        .bind(target_type)
        .fetch_one(&mut **tx)
        .await
        .map_err(ReplenishmentRepoError::Database)?;
        if typed as usize != location_ids.len() {
            return Err(ReplenishmentRepoError::LocationTypeMismatch);
        }

        sqlx::query(
            r#"
            UPDATE warehouse_locations
               SET replenish_strategy_id = NULL,
                   updated_at = now()
             WHERE owner_id = $1
               AND replenish_strategy_id = $2
               AND NOT (id = ANY($3))
            "#,
        )
        .bind(owner_id)
        .bind(strategy_id)
        .bind(location_ids)
        .execute(&mut **tx)
        .await
        .map_err(ReplenishmentRepoError::Database)?;

        sqlx::query(
            r#"
            UPDATE warehouse_locations
               SET replenish_strategy_id = $2,
                   updated_at = now()
             WHERE owner_id = $1
               AND id = ANY($3)
            "#,
        )
        .bind(owner_id)
        .bind(strategy_id)
        .bind(location_ids)
        .execute(&mut **tx)
        .await
        .map_err(ReplenishmentRepoError::Database)?;

        Ok(BindReplenishmentLocationsResponse {
            strategy_id,
            location_ids: location_ids.to_vec(),
        })
    }

    pub async fn preview_strategy(
        &self,
        owner_id: Uuid,
        strategy: &ReplenishmentStrategy,
    ) -> Result<Vec<ReplenishmentPreviewItem>, sqlx::Error> {
        sqlx::query_as::<_, PreviewRow>(
            r#"
            SELECT location.id,
                   location.location_code,
                   $4::uuid AS product_id,
                   COALESCE((
                       SELECT SUM(
                           qty_on_hand - qty_allocated - qty_frozen + qty_replenish_in_transit
                       )
                         FROM inventory_batches
                        WHERE owner_id = location.owner_id
                          AND location_id = location.id
                          AND status = 'qualified'
                          AND ($4::uuid IS NULL OR product_id = $4)
                   ), 0) AS available_qty
              FROM warehouse_locations location
             WHERE location.owner_id = $1
               AND location.replenish_strategy_id = $2
               AND location.location_type = $3
               AND location.status <> 'disabled'
            "#,
        )
        .bind(owner_id)
        .bind(strategy.id)
        .bind(&strategy.target_type)
        .bind(if strategy.scope_type == "product" {
            Some(strategy.scope_ref)
        } else {
            None
        })
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| {
                    let would_trigger = row.available_qty <= strategy.min_safety_threshold;
                    ReplenishmentPreviewItem {
                        location_id: row.id,
                        location_code: row.location_code,
                        product_id: row.product_id,
                        available_qty: row.available_qty,
                        min_safety_threshold: strategy.min_safety_threshold,
                        max_replenish_target: strategy.max_replenish_target,
                        would_trigger,
                    }
                })
                .collect()
        })
    }
}
