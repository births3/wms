use uuid::Uuid;
use wms_domain::Quantity;

use super::{PgReplenishmentRepository, ProductPutawayAttrs, WaveGapLine};

impl PgReplenishmentRepository {
    pub async fn products_at_location_for_category(
        &self,
        owner_id: Uuid,
        location_id: Uuid,
        category_item_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT DISTINCT batch.product_id
              FROM inventory_batches batch
              JOIN products product
                ON product.id = batch.product_id
               AND product.owner_id = batch.owner_id
              JOIN system_dictionary_items item
                ON item.id = $3
               AND item.dict_code = 'special_drug_category'
               AND (item.owner_id IS NULL OR item.owner_id = $1)
             WHERE batch.owner_id = $1
               AND batch.location_id = $2
               AND batch.product_id IS NOT NULL
               AND product.special_drug_category = item.item_code
            "#,
        )
        .bind(owner_id)
        .bind(location_id)
        .bind(category_item_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn load_product_putaway_attrs(
        &self,
        owner_id: Uuid,
        product_id: Uuid,
    ) -> Result<Option<ProductPutawayAttrs>, sqlx::Error> {
        sqlx::query_as::<_, ProductPutawayAttrs>(
            r#"
            SELECT storage_condition, is_external_use, is_fragrant, volume_cm3
              FROM products
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_wave_gap_lines(
        &self,
        owner_id: Uuid,
        wave_id: Uuid,
    ) -> Result<Vec<WaveGapLine>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, Uuid, i32, Uuid, Quantity, Uuid, Option<Uuid>)>(
            r#"
            SELECT wave_order.wave_id,
                   outbound.id,
                   line.line_no,
                   product.id,
                   line.planned_qty,
                   outbound.warehouse_id,
                   (
                        SELECT pick.location_id
                          FROM outbound_pick_tasks pick
                         WHERE pick.owner_id = wave_order.owner_id
                           AND pick.wave_id = wave_order.wave_id
                           AND pick.outbound_order_id = outbound.id
                           AND pick.line_no = line.line_no
                         ORDER BY pick.route_sequence ASC, pick.id
                         LIMIT 1
                   )
              FROM outbound_wave_orders wave_order
              JOIN outbound_orders outbound
                ON outbound.id = wave_order.outbound_order_id
               AND outbound.owner_id = wave_order.owner_id
              JOIN outbound_order_lines line
                ON line.outbound_order_id = outbound.id
               AND line.owner_id = outbound.owner_id
              JOIN products product
                ON product.owner_id = outbound.owner_id
               AND product.product_code = line.product_code
             WHERE wave_order.owner_id = $1
               AND wave_order.wave_id = $2
            "#,
        )
        .bind(owner_id)
        .bind(wave_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(
                    |(
                        wave_id,
                        outbound_order_id,
                        outbound_line_no,
                        product_id,
                        demand_qty,
                        warehouse_id,
                        pick_location_id,
                    )| WaveGapLine {
                        wave_id,
                        outbound_order_id,
                        outbound_line_no,
                        product_id,
                        demand_qty,
                        warehouse_id,
                        pick_location_id,
                    },
                )
                .collect()
        })
    }

    pub async fn list_pick_target_candidates(
        &self,
        owner_id: Uuid,
        warehouse_id: Uuid,
        product_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT location.id
              FROM warehouse_locations location
             WHERE location.owner_id = $1
               AND location.warehouse_id = $2
               AND location.location_type IN ('piece_pick', 'case_pick')
               AND location.status <> 'disabled'
               AND (
                    EXISTS (
                        SELECT 1
                          FROM inventory_batches batch
                         WHERE batch.owner_id = location.owner_id
                           AND batch.location_id = location.id
                           AND batch.product_id = $3
                    )
                    OR location.replenish_strategy_id IS NOT NULL
               )
             ORDER BY
               EXISTS (
                    SELECT 1
                      FROM inventory_batches batch
                     WHERE batch.owner_id = location.owner_id
                       AND batch.location_id = location.id
                       AND batch.product_id = $3
               ) DESC,
               (location.replenish_strategy_id IS NOT NULL) DESC,
               location.pick_sequence_no ASC NULLS LAST,
               location.location_code
            "#,
        )
        .bind(owner_id)
        .bind(warehouse_id)
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn location_has_work_lock(
        &self,
        owner_id: Uuid,
        location_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                  FROM inventory_count_lines line
                  JOIN inventory_counts count_sheet
                    ON count_sheet.id = line.count_id
                   AND count_sheet.owner_id = line.owner_id
                 WHERE line.owner_id = $1
                   AND line.location_id = $2
                   AND count_sheet.status IN ('in_progress', 'pending_approval')
                UNION ALL
                SELECT 1
                  FROM inventory_maintenance_tasks task
                  JOIN inventory_batches batch
                    ON batch.id = task.inventory_batch_id
                   AND batch.owner_id = task.owner_id
                 WHERE task.owner_id = $1
                   AND batch.location_id = $2
                   AND task.status = 'pending'
            )
            "#,
        )
        .bind(owner_id)
        .bind(location_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn recent_patrol_fail_times(
        &self,
        owner_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        reason_code: &str,
    ) -> Result<Vec<chrono::DateTime<chrono::Utc>>, sqlx::Error> {
        sqlx::query_scalar(
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
        .fetch_all(&self.pool)
        .await
    }

    pub async fn has_generated_task_since(
        &self,
        owner_id: Uuid,
        location_id: Uuid,
        product_id: Uuid,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
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
        .bind(since)
        .fetch_one(&self.pool)
        .await
    }
}
