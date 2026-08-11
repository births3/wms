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

/// 库存批次列表查询的 FROM/JOIN（列表与 count(*) 共用，保证过滤条件一致）。
const INVENTORY_BATCHES_LIST_FROM: &str = r#"
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
"#;

/// 库存批次列表过滤条件（WHERE，$1..$15 绑定顺序与主查询一致）。
const INVENTORY_BATCHES_LIST_FILTERS: &str = r#"
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
"#;

/// 已解析并校验的库存批次过滤条件（分页与全量扫描共用）。
struct ParsedInventoryBatchQuery {
    q: Option<String>,
    product_code: Option<String>,
    batch_no: Option<String>,
    location_code: Option<String>,
    location_type: Option<String>,
    zone_code: Option<String>,
    temperature_zone: Option<String>,
    quality_status: Option<String>,
    production_from: Option<NaiveDate>,
    production_to: Option<NaiveDate>,
    expiry_from: Option<NaiveDate>,
    expiry_to: Option<NaiveDate>,
    created_from: Option<DateTime<Utc>>,
    created_to: Option<DateTime<Utc>>,
}

fn parse_inventory_batch_query(
    query: InventoryBatchQuery,
) -> Result<ParsedInventoryBatchQuery, Wave3RepositoryError> {
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
    Ok(ParsedInventoryBatchQuery {
        q,
        product_code,
        batch_no,
        location_code,
        location_type,
        zone_code,
        temperature_zone,
        quality_status,
        production_from,
        production_to,
        expiry_from,
        expiry_to,
        created_from,
        created_to,
    })
}

impl PgWave3Repository {
    /// 列表分页查询：返回 (本页数据, 满足过滤条件的总行数)。
    pub async fn list_inventory_batches_with_query(
        &self,
        ctx: &super::AuthContext,
        query: InventoryBatchQuery,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<InventoryBatch>, i64), Wave3RepositoryError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 200);
        let offset = ((page - 1) as i64) * (page_size as i64);
        let parsed = parse_inventory_batch_query(query)?;
        let total = self.count_inventory_batches(ctx, &parsed).await?;
        let items = self
            .fetch_inventory_batches_page(ctx, &parsed, Some(page_size as i64), offset)
            .await?;
        Ok((items, total))
    }

    /// 全量扫描（近效期报表/召回影响等内部用途），语义不变。
    pub async fn list_inventory_batches(
        &self,
        ctx: &super::AuthContext,
    ) -> Result<Vec<InventoryBatch>, Wave3RepositoryError> {
        let parsed = parse_inventory_batch_query(InventoryBatchQuery::default())?;
        self.fetch_inventory_batches_page(ctx, &parsed, None, 0)
            .await
    }

    async fn fetch_inventory_batches_page(
        &self,
        ctx: &super::AuthContext,
        parsed: &ParsedInventoryBatchQuery,
        limit: Option<i64>,
        offset: i64,
    ) -> Result<Vec<InventoryBatch>, Wave3RepositoryError> {
        let order_by = if parsed.expiry_from.is_some() || parsed.expiry_to.is_some() {
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
              {from}
              {filters}
              ORDER BY {order_by}
              {pagination}
            "#,
            from = INVENTORY_BATCHES_LIST_FROM,
            filters = INVENTORY_BATCHES_LIST_FILTERS,
            order_by = order_by,
            pagination = if limit.is_some() {
                "LIMIT $16 OFFSET $17"
            } else {
                ""
            },
        );
        let mut query = query_as::<_, InventoryBatchRow>(&sql)
            .bind(ctx.owner_id)
            .bind(parsed.q.as_deref())
            .bind(parsed.product_code.as_deref())
            .bind(parsed.batch_no.as_deref())
            .bind(parsed.location_code.as_deref())
            .bind(parsed.location_type.as_deref())
            .bind(parsed.zone_code.as_deref())
            .bind(parsed.temperature_zone.as_deref())
            .bind(parsed.quality_status.as_deref())
            .bind(parsed.production_from)
            .bind(parsed.production_to)
            .bind(parsed.expiry_from)
            .bind(parsed.expiry_to)
            .bind(parsed.created_from)
            .bind(parsed.created_to);
        if let Some(limit) = limit {
            query = query.bind(limit).bind(offset);
        }
        let rows = query.fetch_all(&self.pool).await.map_err(map_db_error)?;

        Ok(rows.into_iter().map(map_inventory_batch).collect())
    }

    async fn count_inventory_batches(
        &self,
        ctx: &super::AuthContext,
        parsed: &ParsedInventoryBatchQuery,
    ) -> Result<i64, Wave3RepositoryError> {
        let sql = format!(
            r#"SELECT count(*) {from} {filters}"#,
            from = INVENTORY_BATCHES_LIST_FROM,
            filters = INVENTORY_BATCHES_LIST_FILTERS,
        );
        let total: i64 = sqlx::query_scalar(&sql)
            .bind(ctx.owner_id)
            .bind(parsed.q.as_deref())
            .bind(parsed.product_code.as_deref())
            .bind(parsed.batch_no.as_deref())
            .bind(parsed.location_code.as_deref())
            .bind(parsed.location_type.as_deref())
            .bind(parsed.zone_code.as_deref())
            .bind(parsed.temperature_zone.as_deref())
            .bind(parsed.quality_status.as_deref())
            .bind(parsed.production_from)
            .bind(parsed.production_to)
            .bind(parsed.expiry_from)
            .bind(parsed.expiry_to)
            .bind(parsed.created_from)
            .bind(parsed.created_to)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_error)?;
        Ok(total)
    }
}
