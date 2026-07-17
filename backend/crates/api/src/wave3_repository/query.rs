use chrono::{DateTime, NaiveDate, Utc};
use sqlx::query_as;
use wms_domain::{InventoryBatch, InventoryBatchQuery};

use super::{
    map_db_error, map_inventory_batch, parse_date, InventoryBatchRow, PgWave3Repository,
    Wave3RepositoryError,
};

pub(super) fn parse_optional_date(
    value: Option<&str>,
) -> Result<Option<NaiveDate>, Wave3RepositoryError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(parse_date)
        .transpose()
}

pub(super) fn parse_optional_datetime(
    value: Option<&str>,
) -> Result<Option<DateTime<Utc>>, Wave3RepositoryError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|_| Wave3RepositoryError::InvalidDate(value.to_string()))
        })
        .transpose()
}

impl PgWave3Repository {
    pub async fn list_inventory_batches(
        &self,
        ctx: &super::AuthContext,
    ) -> Result<Vec<InventoryBatch>, Wave3RepositoryError> {
        self.list_inventory_batches_with_query(ctx, InventoryBatchQuery::default())
            .await
    }

    pub async fn list_inventory_batches_with_query(
        &self,
        ctx: &super::AuthContext,
        query: InventoryBatchQuery,
    ) -> Result<Vec<InventoryBatch>, Wave3RepositoryError> {
        let q = query.q.filter(|value| !value.trim().is_empty());
        let product_code = query.product_code.filter(|value| !value.trim().is_empty());
        let batch_no = query.batch_no.filter(|value| !value.trim().is_empty());
        let location_code = query.location_code.filter(|value| !value.trim().is_empty());
        let location_type = query.location_type.filter(|value| !value.trim().is_empty());
        let zone_code = query.zone_code.filter(|value| !value.trim().is_empty());
        let temperature_zone = query
            .temperature_zone
            .filter(|value| !value.trim().is_empty());
        let quality_status = query
            .quality_status
            .filter(|value| !value.trim().is_empty());
        let production_from = parse_optional_date(query.production_from.as_deref())?;
        let production_to = parse_optional_date(query.production_to.as_deref())?;
        let expiry_from = parse_optional_date(query.expiry_from.as_deref())?;
        let expiry_to = parse_optional_date(query.expiry_to.as_deref())?;
        let created_from = parse_optional_datetime(query.created_from.as_deref())?;
        let created_to = parse_optional_datetime(query.created_to.as_deref())?;
        if production_from
            .zip(production_to)
            .is_some_and(|(from, to)| from > to)
        {
            return Err(Wave3RepositoryError::InvalidDate(
                "production_from_after_production_to".to_string(),
            ));
        }
        if expiry_from
            .zip(expiry_to)
            .is_some_and(|(from, to)| from > to)
        {
            return Err(Wave3RepositoryError::InvalidDate(
                "expiry_from_after_expiry_to".to_string(),
            ));
        }
        if created_from
            .zip(created_to)
            .is_some_and(|(from, to)| from > to)
        {
            return Err(Wave3RepositoryError::InvalidDate(
                "created_from_after_created_to".to_string(),
            ));
        }
        let order_by = if expiry_from.is_some() || expiry_to.is_some() {
            "inventory_batches.expiry_date ASC, inventory_batches.product_code ASC, inventory_batches.batch_no ASC, inventory_batches.id"
        } else {
            "inventory_batches.updated_at DESC, inventory_batches.id"
        };
        let sql = format!(
            r#"
            SELECT inventory_batches.id, inventory_batches.owner_id, inventory_batches.product_code,
                   products.product_name, products.specification, products.manufacturer,
                   inventory_batches.batch_no, inventory_batches.production_date,
                   inventory_batches.expiry_date, inventory_batches.qty_on_hand,
                   inventory_batches.qty_locked, inventory_batches.quality_status,
                   inventory_batches.location_id, inventory_batches.location_code,
                   locations.row_no, locations.column_no, locations.layer_no,
                   zones.zone_code, zones.temperature_zone, zones.quality_color,
                   locations.max_volume_cm3, locations.used_volume_cm3,
                   locations.max_volume_cm3 - locations.used_volume_cm3 AS remaining_volume_cm3,
                   locations.max_sku_count,
                   (SELECT COUNT(DISTINCT sibling.product_code)
                      FROM inventory_batches AS sibling
                     WHERE sibling.owner_id = inventory_batches.owner_id
                       AND sibling.location_id = inventory_batches.location_id
                       AND sibling.qty_on_hand > 0) AS current_sku_count,
                   inventory_batches.container_lpn,
                   inventory_batches.recall_flag, inventory_batches.created_at,
                   inventory_batches.updated_at
              FROM inventory_batches
              LEFT JOIN products
                ON products.owner_id = inventory_batches.owner_id
               AND products.product_code = inventory_batches.product_code
              LEFT JOIN warehouse_locations AS locations
                ON locations.id = inventory_batches.location_id
               AND locations.owner_id = inventory_batches.owner_id
              LEFT JOIN warehouse_zones AS zones
                ON zones.id = locations.zone_id
               AND zones.owner_id = locations.owner_id
             WHERE inventory_batches.owner_id = $1
               AND ($2::TEXT IS NULL OR inventory_batches.product_code ILIKE '%' || $2 || '%'
                    OR products.product_name ILIKE '%' || $2 || '%'
                    OR inventory_batches.batch_no ILIKE '%' || $2 || '%'
                    OR inventory_batches.location_code ILIKE '%' || $2 || '%'
                    OR inventory_batches.container_lpn ILIKE '%' || $2 || '%')
               AND ($3::TEXT IS NULL OR inventory_batches.product_code ILIKE '%' || $3 || '%')
               AND ($4::TEXT IS NULL OR inventory_batches.batch_no ILIKE '%' || $4 || '%')
               AND ($5::TEXT IS NULL OR inventory_batches.location_code ILIKE '%' || $5 || '%')
               AND ($6::TEXT IS NULL OR locations.location_type = $6)
               AND ($7::TEXT IS NULL OR zones.zone_code = $7)
               AND ($8::TEXT IS NULL OR zones.temperature_zone = $8)
               AND ($9::TEXT IS NULL OR inventory_batches.quality_status = $9)
               AND ($10::DATE IS NULL OR inventory_batches.production_date >= $10)
               AND ($11::DATE IS NULL OR inventory_batches.production_date <= $11)
               AND ($12::DATE IS NULL OR inventory_batches.expiry_date >= $12)
               AND ($13::DATE IS NULL OR inventory_batches.expiry_date <= $13)
               AND ($14::TIMESTAMPTZ IS NULL OR inventory_batches.created_at >= $14)
               AND ($15::TIMESTAMPTZ IS NULL OR inventory_batches.created_at <= $15)
            ORDER BY {order_by}
            "#
        );
        let rows = query_as::<_, InventoryBatchRow>(&sql)
            .bind(ctx.owner_id)
            .bind(q.as_deref())
            .bind(product_code.as_deref())
            .bind(batch_no.as_deref())
            .bind(location_code.as_deref())
            .bind(location_type.as_deref())
            .bind(zone_code.as_deref())
            .bind(temperature_zone.as_deref())
            .bind(quality_status.as_deref())
            .bind(production_from)
            .bind(production_to)
            .bind(expiry_from)
            .bind(expiry_to)
            .bind(created_from)
            .bind(created_to)
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error)?;

        Ok(rows.into_iter().map(map_inventory_batch).collect())
    }
}
