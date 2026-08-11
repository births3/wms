use super::*;
use wms_domain::{
    HandleInventoryAlertRequest, InventoryAlertEvent, InventoryAlertListResponse,
    InventoryAlertQuery, PageMeta,
};

#[derive(FromRow)]
struct AlertRow {
    id: Uuid,
    owner_id: Uuid,
    alert_type: String,
    product_code: Option<String>,
    batch_id: Option<Uuid>,
    batch_no: Option<String>,
    location_code: Option<String>,
    severity: String,
    title: String,
    message: String,
    lifecycle_status: String,
    handled_by: Option<Uuid>,
    handled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PgWave3Repository {
    pub async fn list_inventory_alerts(
        &self,
        ctx: &AuthContext,
        query: &InventoryAlertQuery,
    ) -> Result<InventoryAlertListResponse, Wave3RepositoryError> {
        let alert_type = normalize_filter(query.alert_type.as_deref());
        let lifecycle = normalize_filter(query.lifecycle_status.as_deref());
        let product = normalize_filter(query.product_code.as_deref());
        let rows = sqlx::query_as::<_, AlertRow>(
            r#"
            SELECT id, owner_id, alert_type, product_code, batch_id, batch_no, location_code,
                   severity, title, message, lifecycle_status, handled_by, handled_at,
                   created_at, updated_at
              FROM inventory_alert_events
             WHERE owner_id = $1
               AND ($2::text IS NULL OR alert_type = $2)
               AND ($3::text IS NULL OR lifecycle_status = $3)
               AND ($4::text IS NULL OR product_code ILIKE '%' || $4 || '%')
             ORDER BY created_at DESC, id DESC
             LIMIT 200
            "#,
        )
        .bind(ctx.owner_id)
        .bind(alert_type.as_deref())
        .bind(lifecycle.as_deref())
        .bind(product.as_deref())
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let data: Vec<_> = rows.into_iter().map(map_alert).collect();
        let count = data.len() as u32;
        Ok(InventoryAlertListResponse {
            data,
            page: PageMeta {
                next_cursor: None,
                count,
                total: None,
            },
        })
    }

    pub async fn handle_inventory_alert(
        &self,
        ctx: &AuthContext,
        alert_id: Uuid,
        req: HandleInventoryAlertRequest,
        now: DateTime<Utc>,
    ) -> Result<InventoryAlertEvent, Wave3RepositoryError> {
        let status = req.lifecycle_status.trim();
        if !matches!(status, "handled" | "ignored" | "open") {
            return Err(Wave3RepositoryError::InvalidReason);
        }
        let mut tx = self.begin().await?;
        let row = sqlx::query_as::<_, AlertRow>(
            r#"
            UPDATE inventory_alert_events
               SET lifecycle_status = $3,
                   handled_by = CASE WHEN $3 = 'open' THEN NULL ELSE $4 END,
                   handled_at = CASE WHEN $3 = 'open' THEN NULL ELSE $5 END,
                   updated_at = $5
             WHERE id = $1 AND owner_id = $2
            RETURNING id, owner_id, alert_type, product_code, batch_id, batch_no, location_code,
                      severity, title, message, lifecycle_status, handled_by, handled_at,
                      created_at, updated_at
            "#,
        )
        .bind(alert_id)
        .bind(ctx.owner_id)
        .bind(status)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let event = map_alert(row);
        let mut audit_event = AuditWriteRequest::from_auth_context(
            ctx,
            "handle_inventory_alert",
            "M3",
            "inventory_alert_event",
            event.id.to_string(),
            None,
        );
        audit_event.occurred_at = now;
        append_event_in_tx(&mut tx, &audit_event)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(event)
    }

    pub async fn generate_near_expiry_alerts(
        &self,
        ctx: &AuthContext,
        now: DateTime<Utc>,
        warning_days: Option<i64>,
    ) -> Result<usize, Wave3RepositoryError> {
        let as_of = now.date_naive();
        let warning_days = match warning_days {
            Some(days) => days.clamp(1, 3650),
            None => self
                .resolve_expiry_warning_days(ctx, as_of)
                .await
                .unwrap_or(180),
        };
        let until = as_of + chrono::Duration::days(warning_days);
        let mut tx = self.begin().await?;
        let rows: Vec<(Uuid, String, String, Option<String>, chrono::NaiveDate)> = sqlx::query_as(
            r#"
            SELECT id, product_code, batch_no, location_code, expiry_date
              FROM inventory_batches
             WHERE owner_id = $1
               AND quality_status = 'qualified'
               AND expiry_date >= $2
               AND expiry_date <= $3
               AND qty_on_hand > 0
            "#,
        )
        .bind(ctx.owner_id)
        .bind(as_of)
        .bind(until)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let mut created = 0;
        for (batch_id, product_code, batch_no, location_code, expiry_date) in rows {
            let exists: bool = sqlx::query_scalar(
                r#"
                SELECT EXISTS (
                    SELECT 1 FROM inventory_alert_events
                     WHERE owner_id = $1 AND batch_id = $2 AND alert_type = 'near_expiry'
                       AND lifecycle_status = 'open'
                )
                "#,
            )
            .bind(ctx.owner_id)
            .bind(batch_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            if exists {
                continue;
            }
            let days_left = (expiry_date - as_of).num_days();
            sqlx::query(
                r#"
                INSERT INTO inventory_alert_events (
                    id, owner_id, alert_type, product_code, batch_id, batch_no, location_code,
                    severity, title, message, lifecycle_status, created_at, updated_at
                ) VALUES (
                    $1,$2,'near_expiry',$3,$4,$5,$6,
                    CASE WHEN $7 <= 30 THEN 'high' ELSE 'medium' END,
                    '近效期预警',
                    $8,
                    'open',$9,$9
                )
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(&product_code)
            .bind(batch_id)
            .bind(&batch_no)
            .bind(&location_code)
            .bind(days_left)
            .bind(format!(
                "商品 {product_code} 批号 {batch_no} 将于 {expiry_date} 过期，剩余 {days_left} 天"
            ))
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            created += 1;
        }
        let mut audit_event = AuditWriteRequest::from_auth_context(
            ctx,
            "generate_near_expiry_alerts",
            "M3",
            "inventory_alert_event",
            as_of.to_string(),
            None,
        );
        audit_event.occurred_at = now;
        append_event_in_tx(&mut tx, &audit_event)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(created)
    }

    pub async fn list_shipped_customers_for_batch(
        &self,
        ctx: &AuthContext,
        batch_id: Uuid,
    ) -> Result<Vec<wms_domain::ShippedCustomerHint>, Wave3RepositoryError> {
        let batch_no: Option<String> = sqlx::query_scalar(
            "SELECT batch_no FROM inventory_batches WHERE id = $1 AND owner_id = $2",
        )
        .bind(batch_id)
        .bind(ctx.owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?;
        let Some(batch_no) = batch_no else {
            return Err(Wave3RepositoryError::NotFound);
        };
        let rows: Vec<(Uuid, Uuid, Option<String>, wms_domain::Quantity)> = match sqlx::query_as(
            r#"
            SELECT order_row.customer_id, order_row.id, order_row.wms_order_no,
                   COALESCE(line.shipped_qty, line.planned_qty, 0)
              FROM outbound_order_lines line
              JOIN outbound_orders order_row
                ON order_row.id = line.order_id AND order_row.owner_id = line.owner_id
             WHERE line.owner_id = $1
               AND line.batch_no = $2
               AND order_row.status IN ('shipped', 'completed', 'closed')
            "#,
        )
        .bind(ctx.owner_id)
        .bind(&batch_no)
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(sqlx::Error::Database(db)) if db.message().contains("does not exist") => Vec::new(),
            Err(error) => return Err(map_db_error(error)),
        };
        Ok(rows
            .into_iter()
            .map(|(customer_id, order_id, wms_order_no, shipped_qty)| {
                wms_domain::ShippedCustomerHint {
                    customer_id,
                    order_id,
                    wms_order_no,
                    shipped_qty,
                }
            })
            .collect())
    }
}

fn map_alert(row: AlertRow) -> InventoryAlertEvent {
    InventoryAlertEvent {
        id: row.id,
        owner_id: row.owner_id,
        alert_type: row.alert_type,
        product_code: row.product_code,
        batch_id: row.batch_id,
        batch_no: row.batch_no,
        location_code: row.location_code,
        severity: row.severity,
        title: row.title,
        message: row.message,
        lifecycle_status: row.lifecycle_status,
        handled_by: row.handled_by,
        handled_at: row.handled_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn normalize_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
