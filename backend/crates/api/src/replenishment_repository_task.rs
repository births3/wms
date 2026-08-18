use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{Quantity, ReplenishmentTask};

use super::{PgReplenishmentRepository, TaskRow};

impl PgReplenishmentRepository {
    pub async fn list_tasks(
        &self,
        owner_id: Uuid,
        status: Option<&str>,
        trigger_mode: Option<&str>,
    ) -> Result<Vec<ReplenishmentTask>, sqlx::Error> {
        sqlx::query_as::<_, TaskRow>(
            r#"
            SELECT id, owner_id, task_no, trigger_mode, priority, strategy_id,
                   source_location_id, source_batch_id, source_lpn_id, target_location_id,
                   product_id, batch_no, qty, picked_qty, done_qty, status, operator_id,
                   created_by, version
              FROM replenishment_tasks
             WHERE owner_id = $1
               AND ($2::text IS NULL OR status = $2)
               AND ($3::text IS NULL OR trigger_mode = $3)
             ORDER BY CASE WHEN priority = 'urgent' THEN 0 ELSE 1 END, created_at
            "#,
        )
        .bind(owner_id)
        .bind(status)
        .bind(trigger_mode)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
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
        sqlx::query_as::<_, TaskRow>(
            r#"
            SELECT id, owner_id, task_no, trigger_mode, priority, strategy_id,
                   source_location_id, source_batch_id, source_lpn_id, target_location_id,
                   product_id, batch_no, qty, picked_qty, done_qty, status,
                   operator_id, created_by, version
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
                      operator_id, created_by, version
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
                      operator_id, created_by, version
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
                       AND batch.qty_on_hand > 0
               )
            "#,
        )
        .bind(owner_id)
        .bind(lpn_id)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
