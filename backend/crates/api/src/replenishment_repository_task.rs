use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{Quantity, ReplenishmentTask};

use super::{PgReplenishmentRepository, SourceBatchLock, TaskRow};

#[derive(Clone, Debug, Default)]
pub struct ListReplenishmentTasksFilter {
    pub status: Option<String>,
    pub trigger_mode: Option<String>,
    pub priority: Option<String>,
    pub source_location_id: Option<Uuid>,
    pub target_location_id: Option<Uuid>,
    pub location_id: Option<Uuid>,
    pub operator_id: Option<Uuid>,
    pub wave_id: Option<Uuid>,
    pub keyword: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

pub struct ExecuteListFilter {
    pub user_id: Uuid,
    pub allowed_zone_ids: Vec<Uuid>,
    pub open_warehouse_ids: Vec<Uuid>,
}

pub struct OperatorZoneScope {
    pub allowed_zone_ids: Vec<Uuid>,
    pub open_warehouse_ids: Vec<Uuid>,
}

pub struct TargetLocationScope {
    pub zone_id: Uuid,
    pub warehouse_id: Uuid,
}

fn scope_from_rows(rows: Vec<(Vec<Uuid>, Uuid)>) -> OperatorZoneScope {
    let mut allowed_zone_ids = Vec::new();
    let mut open_warehouse_ids = Vec::new();
    for (zone_ids, warehouse_id) in rows {
        if zone_ids.is_empty() {
            open_warehouse_ids.push(warehouse_id);
        } else {
            allowed_zone_ids.extend(zone_ids);
        }
    }
    OperatorZoneScope {
        allowed_zone_ids,
        open_warehouse_ids,
    }
}

impl OperatorZoneScope {
    pub fn allows(&self, location: &TargetLocationScope) -> bool {
        self.allowed_zone_ids.contains(&location.zone_id)
            || self.open_warehouse_ids.contains(&location.warehouse_id)
    }

    pub fn to_list_filter(&self, user_id: Uuid) -> ExecuteListFilter {
        ExecuteListFilter {
            user_id,
            allowed_zone_ids: self.allowed_zone_ids.clone(),
            open_warehouse_ids: self.open_warehouse_ids.clone(),
        }
    }
}

impl PgReplenishmentRepository {
    pub async fn list_tasks(
        &self,
        owner_id: Uuid,
        filter: &ListReplenishmentTasksFilter,
        execute: Option<&ExecuteListFilter>,
    ) -> Result<Vec<ReplenishmentTask>, sqlx::Error> {
        let execute_user = execute.map(|item| item.user_id);
        let allowed_zones = execute
            .map(|item| item.allowed_zone_ids.as_slice())
            .unwrap_or(&[]);
        let open_warehouses = execute
            .map(|item| item.open_warehouse_ids.as_slice())
            .unwrap_or(&[]);
        let keyword = filter
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let limit = filter.limit.unwrap_or(100).clamp(1, 200);
        let offset = filter
            .cursor
            .as_deref()
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value >= 0)
            .unwrap_or(0);
        sqlx::query_as::<_, TaskRow>(
            r#"
            SELECT t.id, t.owner_id, t.task_no, t.trigger_mode, t.priority, t.strategy_id,
                   t.source_location_id, t.source_batch_id, t.source_lpn_id, t.target_location_id,
                   t.product_id, t.batch_no, t.qty, t.picked_qty, t.done_qty, t.status, t.operator_id,
                   t.created_by, t.version,
                   t.wave_id, t.outbound_order_id, t.outbound_line_no,
                   t.claimed_at, t.last_progress_at, t.return_reason, t.created_at,
                   t.confirmed_at, t.cancel_reason, t.updated_at
              FROM replenishment_tasks t
              JOIN warehouse_locations loc
                ON loc.id = t.target_location_id
               AND loc.owner_id = t.owner_id
             WHERE t.owner_id = $1
               AND ($2::text IS NULL OR t.status = $2)
               AND ($3::text IS NULL OR t.trigger_mode = $3)
               AND ($7::text IS NULL OR t.priority = $7)
               AND ($8::uuid IS NULL OR t.source_location_id = $8)
               AND ($9::uuid IS NULL OR t.target_location_id = $9)
               AND ($10::uuid IS NULL OR t.source_location_id = $10 OR t.target_location_id = $10)
               AND ($11::uuid IS NULL OR t.operator_id = $11)
               AND ($12::uuid IS NULL OR t.wave_id = $12)
               AND ($13::text IS NULL OR t.task_no ILIKE '%' || $13 || '%')
               AND ($14::timestamptz IS NULL OR t.created_at >= $14)
               AND ($15::timestamptz IS NULL OR t.created_at <= $15)
               AND (
                    $4::uuid IS NULL
                    OR t.operator_id = $4
                    OR (
                        t.status = 'pending'
                        AND (
                            t.priority = 'urgent'
                            OR loc.zone_id = ANY($5)
                            OR loc.warehouse_id = ANY($6)
                        )
                    )
               )
             ORDER BY
               CASE WHEN $4::uuid IS NULL THEN t.created_at END ASC,
               CASE WHEN $4::uuid IS NOT NULL AND t.priority = 'urgent' THEN 0 ELSE 1 END,
               loc.pick_sequence_no ASC NULLS LAST,
               t.task_no
             LIMIT $16 OFFSET $17
            "#,
        )
        .bind(owner_id)
        .bind(filter.status.as_deref())
        .bind(filter.trigger_mode.as_deref())
        .bind(execute_user)
        .bind(allowed_zones)
        .bind(open_warehouses)
        .bind(filter.priority.as_deref())
        .bind(filter.source_location_id)
        .bind(filter.target_location_id)
        .bind(filter.location_id)
        .bind(filter.operator_id)
        .bind(filter.wave_id)
        .bind(keyword)
        .bind(filter.created_from)
        .bind(filter.created_to)
        .bind(i64::from(limit) + 1)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_task(
        &self,
        owner_id: Uuid,
        task_id: Uuid,
    ) -> Result<Option<ReplenishmentTask>, sqlx::Error> {
        sqlx::query_as::<_, TaskRow>(
            r#"
            SELECT id, owner_id, task_no, trigger_mode, priority, strategy_id,
                   source_location_id, source_batch_id, source_lpn_id, target_location_id,
                   product_id, batch_no, qty, picked_qty, done_qty, status, operator_id,
                   created_by, version,
                   wave_id, outbound_order_id, outbound_line_no,
                   claimed_at, last_progress_at, return_reason, created_at,
                   confirmed_at, cancel_reason, updated_at
              FROM replenishment_tasks
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
    }

    pub async fn operator_replenish_zone_scope(
        &self,
        owner_id: Uuid,
        user_id: Uuid,
    ) -> Result<OperatorZoneScope, sqlx::Error> {
        let rows: Vec<(Vec<Uuid>, Uuid)> = sqlx::query_as(
            r#"
            SELECT task_group.zone_ids, task_group.warehouse_id
              FROM task_groups task_group
             WHERE task_group.owner_id = $1
               AND task_group.enabled
               AND 'replenish' = ANY(task_group.task_type_codes)
               AND EXISTS (
                    SELECT 1
                      FROM task_group_memberships membership
                     WHERE membership.task_group_id = task_group.id
                       AND membership.owner_id = $1
                       AND membership.user_id = $2
                       AND (
                            membership.qualification_valid_until IS NULL
                            OR membership.qualification_valid_until > now()
                       )
               )
            "#,
        )
        .bind(owner_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(scope_from_rows(rows))
    }

    pub async fn operator_replenish_zone_scope_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        user_id: Uuid,
    ) -> Result<OperatorZoneScope, sqlx::Error> {
        let rows: Vec<(Vec<Uuid>, Uuid)> = sqlx::query_as(
            r#"
            SELECT task_group.zone_ids, task_group.warehouse_id
              FROM task_groups task_group
             WHERE task_group.owner_id = $1
               AND task_group.enabled
               AND 'replenish' = ANY(task_group.task_type_codes)
               AND EXISTS (
                    SELECT 1
                      FROM task_group_memberships membership
                     WHERE membership.task_group_id = task_group.id
                       AND membership.owner_id = $1
                       AND membership.user_id = $2
                       AND (
                            membership.qualification_valid_until IS NULL
                            OR membership.qualification_valid_until > now()
                       )
               )
            "#,
        )
        .bind(owner_id)
        .bind(user_id)
        .fetch_all(&mut **tx)
        .await?;
        Ok(scope_from_rows(rows))
    }

    pub async fn target_location_scope(
        &self,
        owner_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<TargetLocationScope>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT zone_id, warehouse_id
              FROM warehouse_locations
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| {
            row.map(|(zone_id, warehouse_id)| TargetLocationScope {
                zone_id,
                warehouse_id,
            })
        })
    }

    pub async fn target_location_scope_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<TargetLocationScope>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT zone_id, warehouse_id
              FROM warehouse_locations
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(location_id)
        .fetch_optional(&mut **tx)
        .await
        .map(|row| {
            row.map(|(zone_id, warehouse_id)| TargetLocationScope {
                zone_id,
                warehouse_id,
            })
        })
    }

    pub async fn list_task_owner_ids(&self) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT DISTINCT owner_id
              FROM replenishment_tasks
             WHERE owner_id IS NOT NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_urgent_pending_since(
        &self,
        owner_id: Uuid,
        created_before: DateTime<Utc>,
    ) -> Result<Vec<(Uuid, i64)>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, version
              FROM replenishment_tasks
             WHERE owner_id = $1
               AND priority = 'urgent'
               AND status = 'pending'
               AND created_at <= $2
            "#,
        )
        .bind(owner_id)
        .bind(created_before)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_stale_in_progress(
        &self,
        owner_id: Uuid,
        progress_before: DateTime<Utc>,
    ) -> Result<Vec<(Uuid, i64, DateTime<Utc>)>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, version, last_progress_at
              FROM replenishment_tasks
             WHERE owner_id = $1
               AND status = 'in_progress'
               AND last_progress_at IS NOT NULL
               AND last_progress_at <= $2
            "#,
        )
        .bind(owner_id)
        .bind(progress_before)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn lock_task(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        task_id: Uuid,
    ) -> Result<Option<ReplenishmentTask>, sqlx::Error> {
        super::set_lock_timeout(tx).await?;
        sqlx::query_as::<_, TaskRow>(
            r#"
            SELECT id, owner_id, task_no, trigger_mode, priority, strategy_id,
                   source_location_id, source_batch_id, source_lpn_id, target_location_id,
                   product_id, batch_no, qty, picked_qty, done_qty, status,
                   operator_id, created_by, version,
                   wave_id, outbound_order_id, outbound_line_no,
                   claimed_at, last_progress_at, return_reason, created_at,
                   confirmed_at, cancel_reason, updated_at
              FROM replenishment_tasks
             WHERE owner_id = $1 AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(owner_id)
        .bind(task_id)
        .fetch_optional(&mut **tx)
        .await
        .map(|row| row.map(Into::into))
    }

    pub async fn operator_has_in_progress(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        operator_id: Uuid,
        except_task_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1
                  FROM replenishment_tasks
                 WHERE owner_id = $1
                   AND operator_id = $2
                   AND status = 'in_progress'
                   AND id <> $3
            )
            "#,
        )
        .bind(owner_id)
        .bind(operator_id)
        .bind(except_task_id)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn location_code(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        location_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT location_code
              FROM warehouse_locations
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(location_id)
        .fetch_optional(&mut **tx)
        .await
    }

    pub async fn lpn_code(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        lpn_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT lpn_code
              FROM lpn_containers
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(lpn_id)
        .fetch_optional(&mut **tx)
        .await
    }

    pub async fn target_batch_id(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        batch_no: &str,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        super::set_lock_timeout(tx).await?;
        sqlx::query_scalar(
            r#"
            SELECT id
              FROM inventory_batches
             WHERE owner_id = $1
               AND location_id = $2
               AND product_id = $3
               AND batch_no = $4
               AND (container_lpn IS NULL OR container_lpn = '')
             FOR UPDATE
            "#,
        )
        .bind(owner_id)
        .bind(location_id)
        .bind(product_id)
        .bind(batch_no)
        .fetch_optional(&mut **tx)
        .await
    }

    pub async fn save_task(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        task: &ReplenishmentTask,
        expected_version: i64,
    ) -> Result<Option<ReplenishmentTask>, sqlx::Error> {
        sqlx::query_as::<_, TaskRow>(
            r#"
            UPDATE replenishment_tasks
               SET status = $4,
                   operator_id = $5,
                   picked_qty = $6,
                   done_qty = $7,
                   claimed_at = COALESCE(claimed_at, CASE WHEN $4 = 'in_progress' THEN now() END),
                   confirmed_at = CASE WHEN $4 = 'done' THEN now() ELSE confirmed_at END,
                   last_progress_at = now(),
                   updated_at = now(),
                   version = version + 1
             WHERE owner_id = $1
               AND id = $2
               AND version = $3
            RETURNING id, owner_id, task_no, trigger_mode, priority, strategy_id,
                      source_location_id, source_batch_id, source_lpn_id, target_location_id,
                      product_id, batch_no, qty, picked_qty, done_qty, status,
                      operator_id, created_by, version,
                      wave_id, outbound_order_id, outbound_line_no,
                      claimed_at, last_progress_at, return_reason, created_at,
                      confirmed_at, cancel_reason, updated_at
            "#,
        )
        .bind(task.owner_id)
        .bind(task.id)
        .bind(expected_version)
        .bind(&task.status)
        .bind(task.operator_id)
        .bind(task.picked_qty)
        .bind(task.done_qty)
        .fetch_optional(&mut **tx)
        .await
        .map(|row| row.map(Into::into))
    }

    pub async fn source_available(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        batch_id: Uuid,
    ) -> Result<Option<Quantity>, sqlx::Error> {
        super::set_lock_timeout(tx).await?;
        sqlx::query_scalar(
            r#"
            SELECT qty_on_hand - qty_allocated - qty_frozen - qty_replenish_out_transit
              FROM inventory_batches
             WHERE owner_id = $1 AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(owner_id)
        .bind(batch_id)
        .fetch_optional(&mut **tx)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn save_exception(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        task: &ReplenishmentTask,
        expected_version: i64,
        cancel_reason: Option<&str>,
        return_reason: Option<&str>,
        clear_claimed: bool,
    ) -> Result<Option<ReplenishmentTask>, sqlx::Error> {
        sqlx::query_as::<_, TaskRow>(
            r#"
            UPDATE replenishment_tasks
               SET status = $4,
                   operator_id = $5,
                   picked_qty = $6,
                   done_qty = $7,
                   cancel_reason = COALESCE($8, cancel_reason),
                   return_reason = COALESCE($9, return_reason),
                   claimed_at = CASE WHEN $10 THEN NULL ELSE claimed_at END,
                   last_progress_at = now(),
                   updated_at = now(),
                   version = version + 1
             WHERE owner_id = $1
               AND id = $2
               AND version = $3
            RETURNING id, owner_id, task_no, trigger_mode, priority, strategy_id,
                      source_location_id, source_batch_id, source_lpn_id, target_location_id,
                      product_id, batch_no, qty, picked_qty, done_qty, status,
                      operator_id, created_by, version,
                      wave_id, outbound_order_id, outbound_line_no,
                      claimed_at, last_progress_at, return_reason, created_at,
                      confirmed_at, cancel_reason, updated_at
            "#,
        )
        .bind(task.owner_id)
        .bind(task.id)
        .bind(expected_version)
        .bind(&task.status)
        .bind(task.operator_id)
        .bind(task.picked_qty)
        .bind(task.done_qty)
        .bind(cancel_reason)
        .bind(return_reason)
        .bind(clear_claimed)
        .fetch_optional(&mut **tx)
        .await
        .map(|row| row.map(Into::into))
    }

    pub async fn release_idle_container(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        lpn_id: Uuid,
        source_location_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE lpn_containers container
               SET status = 'idle',
                   location_id = NULL,
                   updated_at = now()
             WHERE container.owner_id = $1
               AND container.id = $2
               AND NOT EXISTS (
                    SELECT 1
                      FROM inventory_batches batch
                     WHERE batch.owner_id = container.owner_id
                       AND batch.container_lpn = container.lpn_code
                       AND batch.location_id = $3
                       AND batch.qty_on_hand > 0
               )
            "#,
        )
        .bind(owner_id)
        .bind(lpn_id)
        .bind(source_location_id)
        .execute(&mut **tx)
        .await?;
        Ok(())
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
                      operator_id, created_by, version,
                      wave_id, outbound_order_id, outbound_line_no,
                      claimed_at, last_progress_at, return_reason, created_at,
                      confirmed_at, cancel_reason, updated_at
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
}
