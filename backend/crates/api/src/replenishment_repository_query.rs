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
        sqlx::query_as::<_, (Uuid, Uuid, i32, Uuid, Quantity, Uuid)>(
            r#"
            SELECT wave_order.wave_id,
                   outbound.id,
                   line.line_no,
                   product.id,
                   line.planned_qty,
                   COALESCE(
                        (
                            SELECT location.id
                              FROM warehouse_locations location
                             WHERE location.owner_id = outbound.owner_id
                               AND location.warehouse_id = outbound.warehouse_id
                               AND location.location_type IN ('piece_pick', 'case_pick')
                               AND location.replenish_strategy_id IS NOT NULL
                             ORDER BY location.pick_sequence_no ASC NULLS LAST, location.location_code
                             LIMIT 1
                        ),
                        (
                            SELECT location.id
                              FROM warehouse_locations location
                             WHERE location.owner_id = outbound.owner_id
                               AND location.warehouse_id = outbound.warehouse_id
                               AND location.location_type IN ('piece_pick', 'case_pick')
                             ORDER BY location.pick_sequence_no ASC NULLS LAST, location.location_code
                             LIMIT 1
                        )
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
                        target_location_id,
                    )| WaveGapLine {
                        wave_id,
                        outbound_order_id,
                        outbound_line_no,
                        product_id,
                        demand_qty,
                        target_location_id,
                    },
                )
                .collect()
        })
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
