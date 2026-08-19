use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn add_for_stock_surplus_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    batch_id: Uuid,
    warehouse_id: Uuid,
    quantity: wms_domain::Quantity,
    source_document_id: Uuid,
    approval_source: &str,
    approval_id: &str,
    now: DateTime<Utc>,
) -> Result<Option<wms_domain::Quantity>, sqlx::Error> {
    let required_volume = sqlx::query_scalar::<_, i64>(
        r#"
        WITH target AS (
            SELECT location.id,
                   CASE
                       WHEN product.volume_cm3 IS NOT NULL
                       THEN CEIL(product.volume_cm3)::NUMERIC * $4
                   END AS required_volume
              FROM inventory_batches batch
              JOIN products product
                ON product.owner_id = batch.owner_id
               AND product.id = batch.product_id
               AND product.status = 'active'
              JOIN warehouse_locations location
                ON location.owner_id = batch.owner_id
               AND location.id = batch.location_id
               AND location.warehouse_id = $3
              JOIN warehouse_zones zone
                ON zone.owner_id = location.owner_id
               AND zone.id = location.zone_id
               AND zone.warehouse_id = location.warehouse_id
             WHERE batch.owner_id = $1
               AND batch.id = $2
               AND $4 > 0
               AND location.status IN ('available', 'occupied')
               AND (location.current_owner_id IS NULL OR location.current_owner_id = $1)
               AND zone.status = 'active'
               AND zone.temperature_zone = product.storage_condition
               AND zone.quality_color = (
                    SELECT item.item_code
                      FROM system_dictionary_items item
                      JOIN system_dictionary_categories category
                        ON category.dict_code = item.dict_code
                       AND category.enabled = TRUE
                     WHERE item.dict_code = 'quality_color'
                       AND item.params->>'inventory_quality_status' = batch.status
                       AND (item.owner_id IS NULL OR item.owner_id = $1)
                       AND item.enabled = TRUE
                       AND (item.effective_from IS NULL OR item.effective_from <= $5)
                       AND (item.effective_to IS NULL OR item.effective_to > $5)
                     ORDER BY CASE WHEN item.owner_id = $1 THEN 0 ELSE 1 END,
                              item.updated_at DESC,
                              item.item_code
                     LIMIT 1
               )
             FOR UPDATE OF batch, location
        )
        UPDATE warehouse_locations location
           SET used_volume_cm3 = location.used_volume_cm3 + target.required_volume::BIGINT,
               status = 'occupied',
               updated_at = $5,
               version = location.version + 1
          FROM target
         WHERE location.id = target.id
           AND location.owner_id = $1
           AND target.required_volume IS NOT NULL
           AND target.required_volume <= location.max_volume_cm3 - location.used_volume_cm3
        RETURNING target.required_volume::BIGINT
        "#,
    )
    .bind(owner_id)
    .bind(batch_id)
    .bind(warehouse_id)
    .bind(quantity)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await?;
    if required_volume.is_none() {
        return Ok(None);
    }
    let total = sqlx::query_scalar::<_, wms_domain::Quantity>(
        r#"
        UPDATE inventory_batches
           SET qty_on_hand = qty_on_hand + $3,
               updated_at = $4,
               version = version + 1
         WHERE owner_id = $1
           AND id = $2
           AND $3 > 0
        RETURNING qty_on_hand
        "#,
    )
    .bind(owner_id)
    .bind(batch_id)
    .bind(quantity)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await?;
    if total.is_none() {
        return Ok(None);
    }
    sqlx::query(
        r#"
        INSERT INTO inventory_movements (
            id, owner_id, batch_id, movement_type, qty_delta,
            source_document_type, source_document_id, approval_source,
            approval_id, occurred_at
        ) VALUES ($1,$2,$3,'stock_surplus',$4,'stock_surplus_order',$5,$6,$7,$8)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(batch_id)
    .bind(quantity)
    .bind(source_document_id)
    .bind(approval_source)
    .bind(approval_id)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(total)
}
