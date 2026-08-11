use chrono::{DateTime, Duration, NaiveDate, Utc};
use sqlx::{query_as, FromRow};
use uuid::Uuid;
use wms_domain::{
    InventoryBatchTrace, InventoryMovement, InventoryStatusChange, LocationHistoryProductShare,
    LocationHistoryQuery, LocationHistoryResponse, LocationHistoryRisk, PageMeta,
};

use super::{map_inventory_batch, InventoryBatchRow, PgWave3Repository, Wave3RepositoryError};
use crate::operation_context::OperationContext as AuthContext;

#[derive(Clone, FromRow)]
struct InventoryMovementRow {
    id: Uuid,
    owner_id: Uuid,
    batch_id: Uuid,
    movement_type: String,
    qty_delta: wms_domain::Quantity,
    source_document_type: String,
    source_document_id: Uuid,
    occurred_at: DateTime<Utc>,
    location_code: Option<String>,
    from_location_code: Option<String>,
    to_location_code: Option<String>,
    lpn_code: Option<String>,
    operator_user_id: Option<Uuid>,
    operator_name: Option<String>,
    volume_delta_cm3: Option<i64>,
    product_code: Option<String>,
    product_name: Option<String>,
    batch_no: Option<String>,
    expiry_date: Option<String>,
}

#[derive(Clone, FromRow)]
struct InventoryStatusChangeRow {
    id: Uuid,
    owner_id: Uuid,
    batch_id: Uuid,
    from_status: String,
    to_status: String,
    reason: String,
    approval_source: String,
    approval_id: String,
    occurred_at: DateTime<Utc>,
}

impl PgWave3Repository {
    pub async fn get_inventory_batch_trace(
        &self,
        ctx: &AuthContext,
        batch_id: Uuid,
    ) -> Result<InventoryBatchTrace, Wave3RepositoryError> {
        let batch = query_as::<_, InventoryBatchRow>(
            r#"
            SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                   qty_on_hand, qty_locked, quality_status, location_id, location_code,
                   recall_flag, created_at, updated_at
              FROM inventory_batches
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(batch_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(super::map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;

        let movements = query_as::<_, InventoryMovementRow>(
            r#"
            SELECT m.id, m.owner_id, m.batch_id, m.movement_type, m.qty_delta,
                   m.source_document_type, m.source_document_id, m.occurred_at,
                   m.location_code, m.from_location_code, m.to_location_code,
                   m.lpn_code, m.operator_user_id, m.operator_name, m.volume_delta_cm3,
                   b.product_code AS product_code,
                   p.product_name,
                   b.batch_no AS batch_no,
                   to_char(b.expiry_date, 'YYYY-MM-DD') AS expiry_date
              FROM inventory_movements m
              JOIN inventory_batches b
                ON b.id = m.batch_id AND b.owner_id = m.owner_id
              LEFT JOIN products p
                ON p.owner_id = m.owner_id AND p.product_code = b.product_code
             WHERE m.owner_id = $1 AND m.batch_id = $2
             ORDER BY m.occurred_at ASC, m.id ASC
            "#,
        )
        .bind(ctx.owner_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .map_err(super::map_db_error)?
        .into_iter()
        .map(map_inventory_movement)
        .collect();

        let status_changes = query_as::<_, InventoryStatusChangeRow>(
            r#"
            SELECT id, owner_id, batch_id, from_status, to_status,
                   reason, approval_source, approval_id, occurred_at
              FROM inventory_status_changes
             WHERE owner_id = $1 AND batch_id = $2
             ORDER BY occurred_at ASC, id ASC
            "#,
        )
        .bind(ctx.owner_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .map_err(super::map_db_error)?
        .into_iter()
        .map(|row| InventoryStatusChange {
            id: row.id,
            owner_id: row.owner_id,
            batch_id: row.batch_id,
            from_status: row.from_status,
            to_status: row.to_status,
            reason: row.reason,
            approval_source: row.approval_source,
            approval_id: row.approval_id,
            occurred_at: row.occurred_at,
        })
        .collect();

        Ok(InventoryBatchTrace {
            batch: map_inventory_batch(batch),
            movements,
            status_changes,
        })
    }

    pub async fn list_location_history(
        &self,
        ctx: &AuthContext,
        query: &LocationHistoryQuery,
    ) -> Result<LocationHistoryResponse, Wave3RepositoryError> {
        let location_code = query
            .location_code
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .ok_or(Wave3RepositoryError::InvalidLocation)?
            .to_string();

        let location_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                  FROM warehouse_locations location
                 WHERE location.owner_id = $1
                   AND location.location_code = $2
            )
            "#,
        )
        .bind(ctx.owner_id)
        .bind(&location_code)
        .fetch_one(&self.pool)
        .await
        .map_err(super::map_db_error)?;
        if !location_exists {
            let has_movement: bool = sqlx::query_scalar(
                r#"
                SELECT EXISTS (
                    SELECT 1
                      FROM inventory_movements
                     WHERE owner_id = $1
                       AND (
                            location_code = $2
                         OR from_location_code = $2
                         OR to_location_code = $2
                       )
                )
                "#,
            )
            .bind(ctx.owner_id)
            .bind(&location_code)
            .fetch_one(&self.pool)
            .await
            .map_err(super::map_db_error)?;
            if !has_movement {
                return Err(Wave3RepositoryError::NotFound);
            }
        }

        let days = query.days.unwrap_or(30).clamp(1, 3650);
        let default_from = Utc::now() - Duration::days(days);
        let from = parse_history_datetime(query.from.as_deref())?.unwrap_or(default_from);
        let to = parse_history_datetime(query.to.as_deref())?.unwrap_or_else(Utc::now);
        let movement_type = normalize_optional_filter(query.movement_type.as_deref());
        let product_code = normalize_optional_filter(query.product_code.as_deref());
        let batch_no = normalize_optional_filter(query.batch_no.as_deref());

        let rows = query_as::<_, InventoryMovementRow>(
            r#"
            SELECT m.id, m.owner_id, m.batch_id, m.movement_type, m.qty_delta,
                   m.source_document_type, m.source_document_id, m.occurred_at,
                   m.location_code, m.from_location_code, m.to_location_code,
                   m.lpn_code, m.operator_user_id, m.operator_name, m.volume_delta_cm3,
                   b.product_code AS product_code,
                   p.product_name,
                   b.batch_no AS batch_no,
                   to_char(b.expiry_date, 'YYYY-MM-DD') AS expiry_date
              FROM inventory_movements m
              JOIN inventory_batches b
                ON b.id = m.batch_id AND b.owner_id = m.owner_id
              LEFT JOIN products p
                ON p.owner_id = m.owner_id AND p.product_code = b.product_code
             WHERE m.owner_id = $1
               AND (
                    m.location_code = $2
                 OR m.from_location_code = $2
                 OR m.to_location_code = $2
                 OR (m.location_code IS NULL AND b.location_code = $2)
               )
               AND m.occurred_at >= $3
               AND m.occurred_at <= $4
               AND ($5::text IS NULL OR m.movement_type = $5)
               AND ($6::text IS NULL OR b.product_code ILIKE '%' || $6 || '%')
               AND ($7::text IS NULL OR b.batch_no ILIKE '%' || $7 || '%')
             ORDER BY m.occurred_at DESC, m.id DESC
             LIMIT 500
            "#,
        )
        .bind(ctx.owner_id)
        .bind(&location_code)
        .bind(from)
        .bind(to)
        .bind(movement_type.as_deref())
        .bind(product_code.as_deref())
        .bind(batch_no.as_deref())
        .fetch_all(&self.pool)
        .await
        .map_err(super::map_db_error)?;

        let data: Vec<InventoryMovement> = rows.into_iter().map(map_inventory_movement).collect();
        let risks = detect_location_risks(&location_code, &data, ctx.owner_id, &self.pool).await?;
        let product_shares = build_product_shares(&data);
        let count = data.len() as u32;
        Ok(LocationHistoryResponse {
            location_code,
            data,
            risks,
            product_shares,
            page: PageMeta {
                next_cursor: None,
                count,
                total: None,
            },
        })
    }
}

fn map_inventory_movement(row: InventoryMovementRow) -> InventoryMovement {
    InventoryMovement {
        id: row.id,
        owner_id: row.owner_id,
        batch_id: row.batch_id,
        movement_type: row.movement_type,
        qty_delta: row.qty_delta,
        source_document_type: row.source_document_type,
        source_document_id: row.source_document_id,
        occurred_at: row.occurred_at,
        location_code: row.location_code,
        from_location_code: row.from_location_code,
        to_location_code: row.to_location_code,
        lpn_code: row.lpn_code,
        operator_user_id: row.operator_user_id,
        operator_name: row.operator_name,
        volume_delta_cm3: row.volume_delta_cm3,
        product_code: row.product_code,
        product_name: row.product_name,
        batch_no: row.batch_no,
        expiry_date: row.expiry_date,
    }
}

fn normalize_optional_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_history_datetime(
    value: Option<&str>,
) -> Result<Option<DateTime<Utc>>, Wave3RepositoryError> {
    let Some(raw) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
        return Ok(Some(parsed.with_timezone(&Utc)));
    }
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map(|date| {
            date.and_hms_opt(0, 0, 0)
                .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
        })
        .ok()
        .flatten()
        .map(Some)
        .ok_or_else(|| Wave3RepositoryError::InvalidDate(raw.to_string()))
}

async fn detect_location_risks(
    location_code: &str,
    movements: &[InventoryMovement],
    owner_id: Uuid,
    pool: &sqlx::PgPool,
) -> Result<Vec<LocationHistoryRisk>, Wave3RepositoryError> {
    let mut risks = Vec::new();
    let location_zone: Option<String> = sqlx::query_scalar(
        r#"
        SELECT zones.temperature_zone
          FROM warehouse_locations location
          JOIN warehouse_zones zones
            ON zones.id = location.zone_id AND zones.owner_id = location.owner_id
         WHERE location.owner_id = $1 AND location.location_code = $2
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .bind(location_code)
    .fetch_optional(pool)
    .await
    .map_err(super::map_db_error)?;

    if let Some(zone) = location_zone.as_deref() {
        let mismatched = movements.iter().any(|movement| {
            movement.product_code.as_deref().is_some_and(|code| {
                // 冷链商品编码约定：含 COLD / 冷 视为冷链；与常温库位交叉视为错放风险。
                let cold_product = code.to_uppercase().contains("COLD") || code.contains('冷');
                cold_product && zone.eq_ignore_ascii_case("normal")
            })
        });
        if mismatched {
            risks.push(LocationHistoryRisk {
                risk_code: "temperature_mismatch".to_string(),
                severity: "high".to_string(),
                message: format!(
                    "库位 {location_code} 温区为 {zone}，历史存在冷链相关商品记录，需复核清洁状态"
                ),
            });
        }
    }

    let unqualified_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
          FROM inventory_status_changes change
          JOIN inventory_batches batch
            ON batch.id = change.batch_id AND batch.owner_id = change.owner_id
         WHERE change.owner_id = $1
           AND batch.location_code = $2
           AND change.to_status IN ('unqualified', 'pending_destruction')
        "#,
    )
    .bind(owner_id)
    .bind(location_code)
    .fetch_one(pool)
    .await
    .map_err(super::map_db_error)?;
    if unqualified_count > 0 {
        risks.push(LocationHistoryRisk {
            risk_code: "unqualified_history".to_string(),
            severity: "medium".to_string(),
            message: format!("库位 {location_code} 曾存放不合格或待销毁库存，请复核清洁状态"),
        });
    }

    let mut batch_keys = std::collections::BTreeSet::new();
    for movement in movements {
        if let (Some(product), Some(batch_no), Some(expiry)) = (
            movement.product_code.as_deref(),
            movement.batch_no.as_deref(),
            movement.expiry_date.as_deref(),
        ) {
            batch_keys.insert((
                product.to_string(),
                batch_no.to_string(),
                expiry.to_string(),
            ));
        }
    }
    let product_batch_groups = batch_keys.iter().fold(
        std::collections::BTreeMap::<&str, usize>::new(),
        |mut map, (product, _, _)| {
            *map.entry(product.as_str()).or_default() += 1;
            map
        },
    );
    if product_batch_groups.values().any(|count| *count > 1) {
        risks.push(LocationHistoryRisk {
            risk_code: "mixed_batches".to_string(),
            severity: "medium".to_string(),
            message: format!("库位 {location_code} 历史存在同商品多效期批次混放记录"),
        });
    }

    let _ = location_code;
    Ok(risks)
}

fn build_product_shares(movements: &[InventoryMovement]) -> Vec<LocationHistoryProductShare> {
    let mut map = std::collections::BTreeMap::<String, LocationHistoryProductShare>::new();
    for movement in movements {
        let Some(product_code) = movement.product_code.clone() else {
            continue;
        };
        let entry = map
            .entry(product_code.clone())
            .or_insert(LocationHistoryProductShare {
                product_code,
                product_name: movement.product_name.clone(),
                event_count: 0,
                total_qty_delta: wms_domain::Quantity::ZERO,
            });
        entry.event_count += 1;
        entry.total_qty_delta += movement.qty_delta;
        if entry.product_name.is_none() {
            entry.product_name = movement.product_name.clone();
        }
    }
    let mut shares: Vec<_> = map.into_values().collect();
    shares.sort_by_key(|share| std::cmp::Reverse(share.event_count));
    shares
}
