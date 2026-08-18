//! 补货策略与库位组存取。

#[path = "replenishment_repository_strategy.rs"]
mod strategy;
#[path = "replenishment_repository_task.rs"]
mod task;

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    Quantity, ReplenishmentLocationGroup, ReplenishmentStrategy, ReplenishmentTask,
    UpsertReplenishmentLocationGroupRequest, UpsertReplenishmentStrategyRequest,
};

pub struct PgReplenishmentRepository {
    pool: PgPool,
}

impl PgReplenishmentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn insert_strategy(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        id: Uuid,
        req: &UpsertReplenishmentStrategyRequest,
    ) -> Result<ReplenishmentStrategy, sqlx::Error> {
        sqlx::query_as::<_, StrategyRow>(
            r#"
            INSERT INTO replenishment_strategies (
                id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
                location_type, source_type, target_type,
                min_safety_threshold, max_replenish_target, trigger_modes, enabled
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            RETURNING id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
                      location_type, source_type, target_type,
                      min_safety_threshold, max_replenish_target, trigger_modes, enabled
            "#,
        )
        .bind(id)
        .bind(owner_id)
        .bind(&req.strategy_code)
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
        .fetch_one(&mut **tx)
        .await
        .map(Into::into)
    }

    pub async fn list_enabled_min_max_strategies(
        &self,
    ) -> Result<Vec<ReplenishmentStrategy>, sqlx::Error> {
        sqlx::query_as::<_, StrategyRow>(
            r#"
            SELECT id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
                   location_type, source_type, target_type,
                   min_safety_threshold, max_replenish_target, trigger_modes, enabled
              FROM replenishment_strategies
             WHERE owner_id IS NOT NULL
               AND enabled
               AND 'min_max' = ANY(trigger_modes)
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_bound_locations(
        &self,
        owner_id: Uuid,
        strategy_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT id
              FROM warehouse_locations
             WHERE owner_id = $1
               AND replenish_strategy_id = $2
               AND status <> 'disabled'
            "#,
        )
        .bind(owner_id)
        .bind(strategy_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn products_at_location(
        &self,
        owner_id: Uuid,
        location_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT DISTINCT product_id
              FROM inventory_batches
             WHERE owner_id = $1
               AND location_id = $2
               AND product_id IS NOT NULL
            "#,
        )
        .bind(owner_id)
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_strategy(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<Option<ReplenishmentStrategy>, sqlx::Error> {
        sqlx::query_as::<_, StrategyRow>(
            r#"
            SELECT id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
                   location_type, source_type, target_type,
                   min_safety_threshold, max_replenish_target, trigger_modes, enabled
              FROM replenishment_strategies
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
    }

    pub async fn special_drug_category_exists(
        &self,
        owner_id: Uuid,
        item_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                  FROM system_dictionary_items
                 WHERE id = $2
                   AND dict_code = 'special_drug_category'
                   AND enabled
                   AND (owner_id IS NULL OR owner_id = $1)
            )
            "#,
        )
        .bind(owner_id)
        .bind(item_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn product_exists(
        &self,
        owner_id: Uuid,
        product_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM products WHERE owner_id = $1 AND id = $2
            )
            "#,
        )
        .bind(owner_id)
        .bind(product_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn location_group_exists(
        &self,
        owner_id: Uuid,
        group_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                  FROM replenishment_location_groups
                 WHERE owner_id = $1 AND id = $2
            )
            "#,
        )
        .bind(owner_id)
        .bind(group_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn upsert_location_group(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        id: Uuid,
        req: &UpsertReplenishmentLocationGroupRequest,
    ) -> Result<ReplenishmentLocationGroup, sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO replenishment_location_groups (
                id, owner_id, group_code, group_name, enabled
            ) VALUES ($1,$2,$3,$4,$5)
            ON CONFLICT (owner_id, group_code) DO UPDATE
               SET group_name = EXCLUDED.group_name,
                   enabled = EXCLUDED.enabled
            "#,
        )
        .bind(id)
        .bind(owner_id)
        .bind(&req.group_code)
        .bind(&req.group_name)
        .bind(req.enabled)
        .execute(&mut **tx)
        .await?;
        let group_id: Uuid = sqlx::query_scalar(
            r#"
            SELECT id FROM replenishment_location_groups
             WHERE owner_id = $1 AND group_code = $2
            "#,
        )
        .bind(owner_id)
        .bind(&req.group_code)
        .fetch_one(&mut **tx)
        .await?;
        sqlx::query("DELETE FROM replenishment_location_group_members WHERE group_id = $1")
            .bind(group_id)
            .execute(&mut **tx)
            .await?;
        for location_id in &req.location_ids {
            sqlx::query(
                r#"
                INSERT INTO replenishment_location_group_members (group_id, location_id)
                VALUES ($1, $2)
                "#,
            )
            .bind(group_id)
            .bind(location_id)
            .execute(&mut **tx)
            .await?;
        }
        Ok(ReplenishmentLocationGroup {
            id: group_id,
            owner_id,
            group_code: req.group_code.clone(),
            group_name: req.group_name.clone(),
            enabled: req.enabled,
            location_ids: req.location_ids.clone(),
        })
    }

    pub async fn pick_available_qty(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
    ) -> Result<Quantity, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(
                qty_on_hand - qty_allocated - qty_frozen + qty_replenish_in_transit
            ), 0)
              FROM inventory_batches
             WHERE owner_id = $1
               AND location_id = $2
               AND product_id = $3
               AND status = 'qualified'
            "#,
        )
        .bind(owner_id)
        .bind(location_id)
        .bind(product_id)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn default_pack_ratio(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        product_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        let ratio: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT ratio_to_base
              FROM product_packaging_levels
             WHERE owner_id = $1
               AND product_id = $2
               AND is_default
            "#,
        )
        .bind(owner_id)
        .bind(product_id)
        .fetch_optional(&mut **tx)
        .await?;
        Ok(ratio.unwrap_or(1))
    }

    pub async fn load_location_route(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<LocationRouteRow>, sqlx::Error> {
        sqlx::query_as::<_, LocationRouteRow>(
            r#"
            SELECT location.id,
                   location.location_type,
                   location.lock_status,
                   location.replenish_strategy_id,
                   zone.quality_color,
                   zone.temperature_zone,
                   zone.is_external_use_zone,
                   zone.is_fragrant_zone
              FROM warehouse_locations location
              JOIN warehouse_zones zone
                ON zone.id = location.zone_id
               AND zone.owner_id = location.owner_id
             WHERE location.owner_id = $1
               AND location.id = $2
            "#,
        )
        .bind(owner_id)
        .bind(location_id)
        .fetch_optional(&mut **tx)
        .await
    }

    pub async fn lock_source_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        batch_id: Uuid,
    ) -> Result<Option<SourceBatchLock>, sqlx::Error> {
        sqlx::query_as::<_, SourceBatchLock>(
            r#"
            SELECT batch.id,
                   batch.location_id,
                   batch.product_id,
                   batch.batch_no,
                   batch.status,
                   batch.qty_on_hand - batch.qty_allocated - batch.qty_frozen
                       - batch.qty_replenish_out_transit AS available_qty,
                   location.location_type,
                   location.lock_status,
                   container.current_lock_category
              FROM inventory_batches batch
              JOIN warehouse_locations location
                ON location.id = batch.location_id
               AND location.owner_id = batch.owner_id
              LEFT JOIN lpn_containers container
                ON container.owner_id = batch.owner_id
               AND container.lpn_code = batch.container_lpn
             WHERE batch.owner_id = $1
               AND batch.id = $2
             FOR UPDATE OF batch
            "#,
        )
        .bind(owner_id)
        .bind(batch_id)
        .fetch_optional(&mut **tx)
        .await
    }

    pub async fn lock_fefo_source(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        product_id: Uuid,
        source_type: &str,
        min_qty: Quantity,
    ) -> Result<Option<SourceBatchLock>, sqlx::Error> {
        sqlx::query_as::<_, SourceBatchLock>(
            r#"
            SELECT batch.id,
                   batch.location_id,
                   batch.product_id,
                   batch.batch_no,
                   batch.status,
                   batch.qty_on_hand - batch.qty_allocated - batch.qty_frozen
                       - batch.qty_replenish_out_transit AS available_qty,
                   location.location_type,
                   location.lock_status,
                   container.current_lock_category
              FROM inventory_batches batch
              JOIN warehouse_locations location
                ON location.id = batch.location_id
               AND location.owner_id = batch.owner_id
              LEFT JOIN lpn_containers container
                ON container.owner_id = batch.owner_id
               AND container.lpn_code = batch.container_lpn
             WHERE batch.owner_id = $1
               AND batch.product_id = $2
               AND batch.status = 'qualified'
               AND location.location_type = $3
               AND location.lock_status IN ('normal', 'lock_in')
               AND COALESCE(container.current_lock_category, 'qualified')
                   NOT IN ('quarantine', 'rejected')
               AND batch.qty_on_hand - batch.qty_allocated - batch.qty_frozen
                   - batch.qty_replenish_out_transit >= $4
             ORDER BY batch.expiry_date ASC NULLS LAST, batch.id
             FOR UPDATE OF batch
             LIMIT 1
            "#,
        )
        .bind(owner_id)
        .bind(product_id)
        .bind(source_type)
        .bind(min_qty)
        .fetch_optional(&mut **tx)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_task(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        id: Uuid,
        task_no: &str,
        trigger_mode: &str,
        priority: &str,
        strategy_id: Option<Uuid>,
        source: &SourceBatchLock,
        source_lpn_id: Option<Uuid>,
        target_location_id: Uuid,
        product_id: Uuid,
        qty: Quantity,
        created_by: &str,
        wave_id: Option<Uuid>,
        outbound_order_id: Option<Uuid>,
        outbound_line_no: Option<i32>,
    ) -> Result<ReplenishmentTask, sqlx::Error> {
        sqlx::query_as::<_, TaskRow>(
            r#"
            INSERT INTO replenishment_tasks (
                id, owner_id, task_no, trigger_mode, priority, strategy_id,
                source_location_id, source_batch_id, source_lpn_id, target_location_id,
                product_id, batch_no, qty, status, created_by,
                wave_id, outbound_order_id, outbound_line_no
            ) VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10,
                $11, $12, $13, 'pending', $14,
                $15, $16, $17
            )
            RETURNING id, owner_id, task_no, trigger_mode, priority, strategy_id,
                      source_location_id, source_batch_id, source_lpn_id, target_location_id,
                      product_id, batch_no, qty, picked_qty, done_qty, status,
                      operator_id, created_by, version
            "#,
        )
        .bind(id)
        .bind(owner_id)
        .bind(task_no)
        .bind(trigger_mode)
        .bind(priority)
        .bind(strategy_id)
        .bind(source.location_id)
        .bind(source.id)
        .bind(source_lpn_id)
        .bind(target_location_id)
        .bind(product_id)
        .bind(&source.batch_no)
        .bind(qty)
        .bind(created_by)
        .bind(wave_id)
        .bind(outbound_order_id)
        .bind(outbound_line_no)
        .fetch_one(&mut **tx)
        .await
        .map(Into::into)
    }

    pub async fn find_wave_gap_strategy(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        product_id: Uuid,
        target_location_id: Uuid,
    ) -> Result<Option<ReplenishmentStrategy>, sqlx::Error> {
        sqlx::query_as::<_, StrategyRow>(
            r#"
            SELECT id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
                   location_type, source_type, target_type,
                   min_safety_threshold, max_replenish_target, trigger_modes, enabled
              FROM replenishment_strategies strategy
             WHERE strategy.owner_id = $1
               AND strategy.enabled
               AND 'wave_gap' = ANY(strategy.trigger_modes)
               AND (
                    (strategy.scope_type = 'product' AND strategy.scope_ref = $2)
                    OR (
                        strategy.scope_type = 'category'
                        AND EXISTS (
                            SELECT 1
                              FROM products product
                              JOIN system_dictionary_items item
                                ON item.id = strategy.scope_ref
                               AND item.dict_code = 'special_drug_category'
                               AND (item.owner_id IS NULL OR item.owner_id = $1)
                             WHERE product.owner_id = $1
                               AND product.id = $2
                               AND product.special_drug_category = item.item_code
                        )
                    )
                    OR (
                        strategy.scope_type = 'location_group'
                        AND EXISTS (
                            SELECT 1
                              FROM replenishment_location_groups grp
                              JOIN replenishment_location_group_members member
                                ON member.group_id = grp.id
                             WHERE grp.owner_id = $1
                               AND grp.id = strategy.scope_ref
                               AND member.location_id = $3
                        )
                    )
               )
             ORDER BY CASE strategy.scope_type
                        WHEN 'product' THEN 0
                        WHEN 'category' THEN 1
                        ELSE 2
                      END
             LIMIT 1
            "#,
        )
        .bind(owner_id)
        .bind(product_id)
        .bind(target_location_id)
        .fetch_optional(&mut **tx)
        .await
        .map(|row| row.map(Into::into))
    }
}

#[derive(Debug)]
pub enum ReplenishmentRepoError {
    LocationBound,
    LocationTypeMismatch,
    Database(sqlx::Error),
}

#[derive(sqlx::FromRow)]
pub(crate) struct StrategyRow {
    id: Uuid,
    owner_id: Uuid,
    strategy_code: String,
    strategy_name: String,
    scope_type: String,
    scope_ref: Uuid,
    location_type: String,
    source_type: String,
    target_type: String,
    min_safety_threshold: Quantity,
    max_replenish_target: Quantity,
    trigger_modes: Vec<String>,
    enabled: bool,
}

#[derive(sqlx::FromRow)]
pub(crate) struct GroupRow {
    id: Uuid,
    owner_id: Uuid,
    group_code: String,
    group_name: String,
    enabled: bool,
}

impl From<StrategyRow> for ReplenishmentStrategy {
    fn from(row: StrategyRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            strategy_code: row.strategy_code,
            strategy_name: row.strategy_name,
            scope_type: row.scope_type,
            scope_ref: row.scope_ref,
            location_type: row.location_type,
            source_type: row.source_type,
            target_type: row.target_type,
            min_safety_threshold: row.min_safety_threshold,
            max_replenish_target: row.max_replenish_target,
            trigger_modes: row.trigger_modes,
            enabled: row.enabled,
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct PreviewRow {
    id: Uuid,
    location_code: String,
    product_id: Option<Uuid>,
    available_qty: Quantity,
}

#[derive(sqlx::FromRow)]
pub struct LocationRouteRow {
    pub id: Uuid,
    pub location_type: String,
    pub lock_status: String,
    pub replenish_strategy_id: Option<Uuid>,
    pub quality_color: String,
    pub temperature_zone: String,
    pub is_external_use_zone: bool,
    pub is_fragrant_zone: bool,
}

#[derive(sqlx::FromRow)]
pub struct SourceBatchLock {
    pub id: Uuid,
    pub location_id: Uuid,
    pub product_id: Option<Uuid>,
    pub batch_no: String,
    pub status: String,
    pub available_qty: Quantity,
    pub location_type: String,
    pub lock_status: String,
    pub current_lock_category: Option<String>,
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: Uuid,
    owner_id: Uuid,
    task_no: String,
    trigger_mode: String,
    priority: String,
    strategy_id: Option<Uuid>,
    source_location_id: Uuid,
    source_batch_id: Uuid,
    source_lpn_id: Option<Uuid>,
    target_location_id: Uuid,
    product_id: Uuid,
    batch_no: String,
    qty: Quantity,
    picked_qty: Quantity,
    done_qty: Quantity,
    status: String,
    operator_id: Option<Uuid>,
    created_by: String,
    version: i64,
}

impl From<TaskRow> for ReplenishmentTask {
    fn from(row: TaskRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            task_no: row.task_no,
            trigger_mode: row.trigger_mode,
            priority: row.priority,
            strategy_id: row.strategy_id,
            source_location_id: row.source_location_id,
            source_batch_id: row.source_batch_id,
            source_lpn_id: row.source_lpn_id,
            target_location_id: row.target_location_id,
            product_id: row.product_id,
            batch_no: row.batch_no,
            qty: row.qty,
            picked_qty: row.picked_qty,
            done_qty: row.done_qty,
            status: row.status,
            operator_id: row.operator_id,
            created_by: row.created_by,
            version: row.version,
        }
    }
}
