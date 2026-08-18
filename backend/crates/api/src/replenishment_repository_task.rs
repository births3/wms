use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::ReplenishmentTask;

use super::{PgReplenishmentRepository, TaskRow};

impl PgReplenishmentRepository {
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
