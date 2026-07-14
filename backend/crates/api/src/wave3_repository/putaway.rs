use super::*;
use serde_json::Value;

#[derive(FromRow)]
struct PutawayProductPolicyRow {
    storage_condition: String,
    attrs: Value,
}

#[derive(FromRow)]
struct PutawayLocationRow {
    location_id: Uuid,
    location_code: String,
    temperature_zone: String,
    quality_color: String,
    available_volume_cm3: i64,
    same_product: bool,
    #[sqlx(rename = "same_product_distance")]
    _same_product_distance: Option<i64>,
}

impl PgWave3Repository {
    pub async fn recommend_putaway_locations(
        &self,
        ctx: &AuthContext,
        receiving_order_id: Uuid,
        query: PutawayRecommendationQuery,
    ) -> Result<PutawayRecommendationResponse, Wave3RepositoryError> {
        if query.qty <= 0 || query.limit == Some(0) {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let limit = query.limit.unwrap_or(5).min(50);
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let order = lock_receiving_order(&mut tx, ctx.owner_id, receiving_order_id).await?;
        if order.status != "putaway" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "putaway".to_string(),
                actual: order.status,
            });
        }

        let valid_line: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM receiving_order_lines WHERE receiving_order_id = $1 AND owner_id = $2 AND product_code = $3 AND batch_no = $4)",
        )
        .bind(receiving_order_id)
        .bind(ctx.owner_id)
        .bind(&query.product_code)
        .bind(&query.batch_no)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if !valid_line {
            return Err(Wave3RepositoryError::NotFound);
        }

        let (accepted_qty, putaway_qty): (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COALESCE((
                    SELECT SUM(accepted_qty)
                      FROM receiving_inspections
                     WHERE receiving_order_id = $1
                       AND owner_id = $2
                       AND batch_no = $3
                ), 0)::BIGINT,
                COALESCE((
                    SELECT SUM(qty)
                      FROM receiving_putaways
                     WHERE receiving_order_id = $1
                       AND owner_id = $2
                       AND product_code = $4
                       AND batch_no = $3
                ), 0)::BIGINT
            "#,
        )
        .bind(receiving_order_id)
        .bind(ctx.owner_id)
        .bind(&query.batch_no)
        .bind(&query.product_code)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if accepted_qty <= 0 {
            return Err(Wave3RepositoryError::NotFound);
        }
        let remaining_qty = accepted_qty
            .checked_sub(putaway_qty)
            .ok_or(Wave3RepositoryError::QuantityClosureMismatch)?;
        if query.qty > remaining_qty {
            return Err(Wave3RepositoryError::QuantityClosureMismatch);
        }

        let product = sqlx::query_as::<_, PutawayProductPolicyRow>(
            "SELECT storage_condition, attrs FROM products WHERE owner_id = $1 AND product_code = $2 AND status = 'active'",
        )
        .bind(ctx.owner_id)
        .bind(&query.product_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let unit_volume_cm3 = product_unit_volume_cm3(&product.attrs)?;
        let required_volume_cm3 = unit_volume_cm3
            .checked_mul(query.qty)
            .ok_or(Wave3RepositoryError::InvalidQuantity)?;
        let quality_color =
            resolve_quality_color(&mut tx, ctx.owner_id, &query.quality_status, Utc::now()).await?;

        let locations = sqlx::query_as::<_, PutawayLocationRow>(
            r#"
            SELECT
                location.id AS location_id,
                location.location_code,
                zone.temperature_zone,
                zone.quality_color,
                location.max_volume_cm3 - location.used_volume_cm3 AS available_volume_cm3,
                COUNT(inventory.id) FILTER (
                    WHERE inventory.product_code = $5
                ) > 0 AS same_product,
                MIN(
                    CASE WHEN inventory.product_code = $5 THEN
                        ABS((location.row_no - same_product_location.row_no)::BIGINT)
                        + ABS((location.column_no - same_product_location.column_no)::BIGINT)
                        + ABS((location.layer_no - same_product_location.layer_no)::BIGINT)
                    END
                ) AS same_product_distance
              FROM warehouse_locations AS location
              JOIN warehouse_zones AS zone
                ON zone.id = location.zone_id
               AND zone.owner_id = location.owner_id
             LEFT JOIN inventory_batches AS inventory
                ON inventory.owner_id = location.owner_id
               AND inventory.location_id = location.id
             LEFT JOIN warehouse_locations AS same_product_location
                ON same_product_location.owner_id = inventory.owner_id
               AND same_product_location.id = inventory.location_id
               AND same_product_location.warehouse_id = location.warehouse_id
             WHERE location.owner_id = $1
               AND location.warehouse_id = $2
               AND (location.bound_owner_id IS NULL OR location.bound_owner_id = $1)
               AND location.status IN ('available', 'occupied')
               AND zone.status = 'active'
               AND zone.temperature_zone = $3
               AND zone.quality_color = $4
             GROUP BY location.id, location.location_code, location.max_volume_cm3,
                      location.used_volume_cm3, location.max_sku_count,
                      zone.temperature_zone, zone.quality_color,
                      location.row_no, location.column_no, location.layer_no
             HAVING location.max_volume_cm3 - location.used_volume_cm3 >= $6
                AND (
                    COUNT(inventory.id) FILTER (WHERE inventory.product_code = $5) > 0
                    OR COUNT(DISTINCT inventory.product_code) < location.max_sku_count
                )
             ORDER BY same_product DESC, same_product_distance NULLS LAST,
                      available_volume_cm3, location.location_code
             LIMIT $7
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order.warehouse_id)
        .bind(&product.storage_condition)
        .bind(&quality_color)
        .bind(&query.product_code)
        .bind(required_volume_cm3)
        .bind(i64::from(limit))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if locations.is_empty() {
            return Err(Wave3RepositoryError::NoAvailableLocation);
        }

        let data = locations
            .into_iter()
            .map(|location| PutawayLocationRecommendation {
                location_id: location.location_id,
                location_code: location.location_code,
                temperature_zone: location.temperature_zone,
                quality_color: location.quality_color,
                available_volume_cm3: location.available_volume_cm3,
                required_volume_cm3,
                same_product: location.same_product,
            })
            .collect();
        tx.commit().await.map_err(map_db_error)?;
        Ok(PutawayRecommendationResponse {
            receiving_order_id,
            owner_id: ctx.owner_id,
            product_code: query.product_code,
            batch_no: query.batch_no,
            qty: query.qty,
            quality_status: query.quality_status,
            data,
        })
    }
}

pub(super) fn product_unit_volume_cm3(attrs: &Value) -> Result<i64, Wave3RepositoryError> {
    let value = attrs
        .get("unit_volume_cm3")
        .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok()))
        .filter(|value| *value > 0)
        .ok_or(Wave3RepositoryError::InvalidProductVolume)?;
    Ok(value)
}
