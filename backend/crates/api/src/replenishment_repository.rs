//! 补货策略与库位组存取。

#[path = "replenishment_repository_query.rs"]
mod query;
#[path = "replenishment_repository_strategy.rs"]
mod strategy;
#[path = "replenishment_repository_task.rs"]
mod task;

pub use task::{
    ExecuteListFilter, ListReplenishmentTasksFilter, OperatorZoneScope, TargetLocationScope,
};

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
}

pub(crate) async fn set_lock_timeout(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query("SET LOCAL lock_timeout = '3s'")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

impl PgReplenishmentRepository {
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
        target_type: &str,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT id
              FROM warehouse_locations
             WHERE owner_id = $1
               AND replenish_strategy_id = $2
               AND location_type = $3
               AND status <> 'disabled'
            "#,
        )
        .bind(owner_id)
        .bind(strategy_id)
        .bind(target_type)
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

    pub async fn strategy_has_open_tasks(
        &self,
        owner_id: Uuid,
        strategy_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                  FROM replenishment_tasks
                 WHERE owner_id = $1
                   AND strategy_id = $2
                   AND status IN ('pending', 'in_progress', 'suspended')
            )
            "#,
        )
        .bind(owner_id)
        .bind(strategy_id)
        .fetch_one(&self.pool)
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
    ) -> Result<ReplenishmentLocationGroup, ReplenishmentRepoError> {
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
        self.replace_group_members(tx, owner_id, group_id, &req.location_ids)
            .await?;
        self.sync_group_strategy_locations(tx, owner_id, group_id, &req.location_ids)
            .await?;
        Ok(ReplenishmentLocationGroup {
            id: group_id,
            owner_id,
            group_code: req.group_code.clone(),
            group_name: req.group_name.clone(),
            enabled: req.enabled,
            location_ids: req.location_ids.clone(),
        })
    }

    pub async fn sync_group_strategy_locations(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        group_id: Uuid,
        location_ids: &[Uuid],
    ) -> Result<(), ReplenishmentRepoError> {
        let strategies: Vec<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT id, target_type
              FROM replenishment_strategies
             WHERE owner_id = $1
               AND scope_type = 'location_group'
               AND scope_ref = $2
               AND enabled
            "#,
        )
        .bind(owner_id)
        .bind(group_id)
        .fetch_all(&mut **tx)
        .await?;
        for (strategy_id, target_type) in strategies {
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
            .await?;
            if conflicting.is_some() {
                return Err(ReplenishmentRepoError::LocationBound);
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
            .await?;
            sqlx::query(
                r#"
                UPDATE warehouse_locations
                   SET replenish_strategy_id = $2,
                       updated_at = now()
                 WHERE owner_id = $1
                   AND id = ANY($3)
                   AND location_type = $4
                "#,
            )
            .bind(owner_id)
            .bind(strategy_id)
            .bind(location_ids)
            .bind(target_type)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
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
                   location.max_volume_cm3,
                   location.used_volume_cm3,
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
        set_lock_timeout(tx).await?;
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
                   container.current_lock_category,
                   container.id AS container_id,
                   COALESCE((
                        SELECT SUM(peer.qty_on_hand)
                          FROM inventory_batches peer
                         WHERE peer.owner_id = batch.owner_id
                           AND peer.location_id = batch.location_id
                           AND peer.container_lpn IS NOT NULL
                           AND peer.container_lpn = batch.container_lpn
                   ), 0) AS lpn_on_hand
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
        set_lock_timeout(tx).await?;
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
                   container.current_lock_category,
                   container.id AS container_id,
                   COALESCE((
                        SELECT SUM(peer.qty_on_hand)
                          FROM inventory_batches peer
                         WHERE peer.owner_id = batch.owner_id
                           AND peer.location_id = batch.location_id
                           AND peer.container_lpn IS NOT NULL
                           AND peer.container_lpn = batch.container_lpn
                   ), 0) AS lpn_on_hand
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
               AND strategy.target_type = (
                    SELECT location_type
                      FROM warehouse_locations
                     WHERE owner_id = $1 AND id = $3
               )
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
    ScopeNotFound,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for ReplenishmentRepoError {
    fn from(value: sqlx::Error) -> Self {
        if matches!(&value, sqlx::Error::Protocol(code) if code == "M3_REPLENISH_LOCATION_BOUND") {
            return Self::LocationBound;
        }
        Self::Database(value)
    }
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
    pub max_volume_cm3: i64,
    pub used_volume_cm3: i64,
    pub quality_color: String,
    pub temperature_zone: String,
    pub is_external_use_zone: bool,
    pub is_fragrant_zone: bool,
}

#[derive(sqlx::FromRow)]
pub struct ProductPutawayAttrs {
    pub storage_condition: Option<String>,
    pub is_external_use: bool,
    pub is_fragrant: bool,
    pub volume_cm3: Option<f64>,
}

pub struct WaveGapLine {
    pub wave_id: Uuid,
    pub outbound_order_id: Uuid,
    pub outbound_line_no: i32,
    pub product_id: Uuid,
    pub demand_qty: Quantity,
    pub warehouse_id: Uuid,
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
    pub container_id: Option<Uuid>,
    pub lpn_on_hand: Quantity,
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
    wave_id: Option<Uuid>,
    outbound_order_id: Option<Uuid>,
    outbound_line_no: Option<i32>,
    claimed_at: Option<chrono::DateTime<chrono::Utc>>,
    last_progress_at: Option<chrono::DateTime<chrono::Utc>>,
    return_reason: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
    cancel_reason: Option<String>,
    updated_at: chrono::DateTime<chrono::Utc>,
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
            wave_id: row.wave_id,
            outbound_order_id: row.outbound_order_id,
            outbound_line_no: row.outbound_line_no,
            claimed_at: row.claimed_at,
            last_progress_at: row.last_progress_at,
            return_reason: row.return_reason,
            created_at: row.created_at,
            confirmed_at: row.confirmed_at,
            cancel_reason: row.cancel_reason,
            updated_at: row.updated_at,
        }
    }
}
