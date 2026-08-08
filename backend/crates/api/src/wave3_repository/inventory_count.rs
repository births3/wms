use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    calculate_variance, count_requires_elevated_approval, validate_approval_for_variance,
    validate_count_type, validate_physical_quantity, ApproveInventoryCountRequest,
    CreateInventoryCountRequest, InventoryCount, InventoryCountLine,
    SubmitInventoryCountLineRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    operation_context::OperationContext as AuthContext,
};

use super::{
    map_db_error, replay_idempotency, request_hash, store_idempotency_success, IdempotentMutation,
    PgWave3Repository, Wave3RepositoryError,
};

mod approval;

#[derive(Clone, FromRow)]
struct InventoryCountRow {
    id: Uuid,
    owner_id: Uuid,
    count_type: String,
    warehouse_id: Option<Uuid>,
    zone_id: Option<Uuid>,
    product_code: Option<String>,
    status: String,
    started_at: DateTime<Utc>,
    created_by: Uuid,
    approved_by: Option<Uuid>,
    approved_at: Option<DateTime<Utc>>,
    approval_source: Option<String>,
    approval_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, FromRow)]
struct InventoryCountLineRow {
    id: Uuid,
    count_id: Uuid,
    owner_id: Uuid,
    inventory_batch_id: Uuid,
    location_id: Uuid,
    location_code: String,
    product_code: String,
    batch_no: String,
    book_qty: wms_domain::Quantity,
    physical_qty: Option<wms_domain::Quantity>,
    variance_qty: Option<wms_domain::Quantity>,
    variance_type: Option<String>,
}

#[derive(Clone, FromRow)]
struct InventoryCountBatchRow {
    id: Uuid,
    warehouse_id: Uuid,
    product_code: String,
    batch_no: String,
    expiry_date: NaiveDate,
    quality_status: String,
    qty_on_hand: wms_domain::Quantity,
    qty_locked: wms_domain::Quantity,
}

impl PgWave3Repository {
    pub async fn create_inventory_count_with_audit(
        &self,
        ctx: &AuthContext,
        req: CreateInventoryCountRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<InventoryCount>, Wave3RepositoryError> {
        validate_count_type(&req.count_type)
            .map_err(|_| Wave3RepositoryError::InvalidInventoryCountType)?;
        let product_code = req
            .product_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let request_hash = request_hash(&json!({ "request": &req }))?;
        let mut tx = self.begin().await?;
        super::lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<InventoryCount>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let snapshots = sqlx::query_as::<_, InventoryCountLineRow>(
            r#"
            SELECT batch.id,
                   $3::UUID AS count_id,
                   batch.owner_id,
                   batch.id AS inventory_batch_id,
                   batch.location_id,
                   batch.location_code,
                   batch.product_code,
                   batch.batch_no,
                   batch.qty_on_hand - batch.qty_locked AS book_qty,
                   NULL::NUMERIC AS physical_qty,
                   NULL::NUMERIC AS variance_qty,
                   NULL::TEXT AS variance_type
              FROM inventory_batches AS batch
              JOIN warehouse_locations AS location
                ON location.owner_id = batch.owner_id
               AND location.id = batch.location_id
             WHERE batch.owner_id = $1
               AND batch.qty_on_hand - batch.qty_locked > 0
               AND ($2::UUID IS NULL OR location.warehouse_id = $2)
               AND ($4::UUID IS NULL OR location.zone_id = $4)
               AND ($5::TEXT IS NULL OR batch.product_code = $5)
             ORDER BY batch.location_code, batch.product_code, batch.batch_no, batch.id
             FOR UPDATE OF batch
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.warehouse_id)
        .bind(Uuid::nil())
        .bind(req.zone_id)
        .bind(product_code.as_deref())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if snapshots.is_empty() {
            return Err(Wave3RepositoryError::NoInventoryData);
        }

        let batch_ids: Vec<Uuid> = snapshots
            .iter()
            .map(|line| line.inventory_batch_id)
            .collect();
        let active_lock: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                  FROM inventory_count_lines line
                  JOIN inventory_counts count_sheet
                    ON count_sheet.owner_id = line.owner_id
                   AND count_sheet.id = line.count_id
                 WHERE line.owner_id = $1
                   AND line.inventory_batch_id = ANY($2)
                   AND count_sheet.status IN ('in_progress', 'pending_approval')
            )
            "#,
        )
        .bind(ctx.owner_id)
        .bind(&batch_ids)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if active_lock {
            return Err(Wave3RepositoryError::InventoryCountAlreadyActive);
        }

        let count_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO inventory_counts (
                id, owner_id, count_type, warehouse_id, zone_id, product_code,
                status, started_at, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'in_progress', $7, $8, $7, $7)
            "#,
        )
        .bind(count_id)
        .bind(ctx.owner_id)
        .bind(&req.count_type)
        .bind(req.warehouse_id)
        .bind(req.zone_id)
        .bind(&product_code)
        .bind(now)
        .bind(ctx.user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        for line in &snapshots {
            sqlx::query(
                r#"
                INSERT INTO inventory_count_lines (
                    id, count_id, owner_id, inventory_batch_id, location_id,
                    location_code, product_code, batch_no, book_qty
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(count_id)
            .bind(ctx.owner_id)
            .bind(line.inventory_batch_id)
            .bind(line.location_id)
            .bind(&line.location_code)
            .bind(&line.product_code)
            .bind(&line.batch_no)
            .bind(line.book_qty)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        let count = load_inventory_count_in_tx(&mut tx, ctx.owner_id, count_id).await?;
        let mut audit_event = audit.unwrap_or_else(|| {
            AuditWriteRequest::from_auth_context(
                ctx,
                "create_inventory_count",
                "M3",
                "inventory_count",
                count_id.to_string(),
                None,
            )
        });
        audit_event.occurred_at = now;
        audit_event.resource_id = count_id.to_string();
        append_event_in_tx(&mut tx, &audit_event)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inventory/counts",
            "inventory_count",
            count_id.to_string(),
            &count,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: count,
            replayed: false,
        })
    }

    pub async fn get_inventory_count(
        &self,
        ctx: &AuthContext,
        count_id: Uuid,
    ) -> Result<InventoryCount, Wave3RepositoryError> {
        load_inventory_count(&self.pool, ctx.owner_id, count_id).await
    }

    pub async fn list_inventory_counts(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<InventoryCount>, Wave3RepositoryError> {
        let rows = sqlx::query_as::<_, InventoryCountRow>(
            r#"
            SELECT id, owner_id, count_type, warehouse_id, zone_id, product_code, status,
                   started_at, created_by, approved_by, approved_at, approval_source, approval_id,
                   created_at, updated_at
              FROM inventory_counts
             WHERE owner_id = $1
             ORDER BY started_at DESC, id DESC
             LIMIT 100
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let mut counts = Vec::with_capacity(rows.len());
        for row in rows {
            let lines = sqlx::query_as::<_, InventoryCountLineRow>(
                r#"
                SELECT id, count_id, owner_id, inventory_batch_id, location_id, location_code,
                       product_code, batch_no, book_qty, physical_qty, variance_qty, variance_type
                  FROM inventory_count_lines
                 WHERE owner_id = $1 AND count_id = $2
                 ORDER BY location_code, product_code, batch_no, id
                "#,
            )
            .bind(ctx.owner_id)
            .bind(row.id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error)?;
            counts.push(map_inventory_count(row, lines));
        }
        Ok(counts)
    }

    pub async fn submit_inventory_count_line_with_audit(
        &self,
        ctx: &AuthContext,
        count_id: Uuid,
        line_id: Uuid,
        req: SubmitInventoryCountLineRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<InventoryCountLine>, Wave3RepositoryError> {
        validate_physical_quantity(req.physical_qty)
            .map_err(|_| Wave3RepositoryError::InvalidQuantity)?;
        let request_hash = request_hash(&json!({
            "count_id": count_id,
            "line_id": line_id,
            "request": &req,
        }))?;
        let mut tx = self.begin().await?;
        super::lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<InventoryCountLine>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let count = lock_inventory_count(&mut tx, ctx.owner_id, count_id).await?;
        if count.status != "in_progress" {
            return Err(Wave3RepositoryError::InvalidInventoryCountState);
        }
        let line = sqlx::query_as::<_, InventoryCountLineRow>(
            r#"
            SELECT id, count_id, owner_id, inventory_batch_id, location_id,
                   location_code, product_code, batch_no, book_qty, physical_qty,
                   variance_qty, variance_type
              FROM inventory_count_lines
             WHERE owner_id = $1 AND count_id = $2 AND id = $3
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(count_id)
        .bind(line_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::InventoryCountLineNotFound)?;
        if line.physical_qty.is_some() {
            return Err(Wave3RepositoryError::InventoryCountLineAlreadySubmitted);
        }
        let (variance_qty, variance_type) = calculate_variance(line.book_qty, req.physical_qty);
        sqlx::query(
            r#"
            UPDATE inventory_count_lines
               SET physical_qty = $4, variance_qty = $5, variance_type = $6, updated_at = $7
             WHERE owner_id = $1 AND count_id = $2 AND id = $3
            "#,
        )
        .bind(ctx.owner_id)
        .bind(count_id)
        .bind(line_id)
        .bind(req.physical_qty)
        .bind(variance_qty)
        .bind(variance_type)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let incomplete: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM inventory_count_lines WHERE owner_id = $1 AND count_id = $2 AND physical_qty IS NULL)",
        )
        .bind(ctx.owner_id)
        .bind(count_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if !incomplete {
            sqlx::query(
                "UPDATE inventory_counts SET status = 'pending_approval', updated_at = $3 WHERE owner_id = $1 AND id = $2 AND status = 'in_progress'",
            )
            .bind(ctx.owner_id)
            .bind(count_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        let submitted = InventoryCountLine {
            id: line.id,
            count_id: line.count_id,
            owner_id: line.owner_id,
            inventory_batch_id: line.inventory_batch_id,
            location_id: line.location_id,
            location_code: line.location_code,
            product_code: line.product_code,
            batch_no: line.batch_no,
            book_qty: line.book_qty,
            physical_qty: Some(req.physical_qty),
            variance_qty: Some(variance_qty),
            variance_type: Some(variance_type.to_string()),
        };
        let mut audit_event = audit.unwrap_or_else(|| {
            AuditWriteRequest::from_auth_context(
                ctx,
                "submit_inventory_count_line",
                "M3",
                "inventory_count_line",
                line_id.to_string(),
                Some(AuditDiff::compute(
                    json!({ "physical_qty": null }),
                    json!({
                        "physical_qty": req.physical_qty,
                        "variance_qty": variance_qty,
                        "variance_type": variance_type,
                    }),
                )),
            )
        });
        audit_event.occurred_at = now;
        audit_event.resource_id = line_id.to_string();
        append_event_in_tx(&mut tx, &audit_event)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &format!("/api/v1/inventory/counts/{count_id}/lines/{line_id}/submit"),
            "inventory_count_line",
            line_id.to_string(),
            &submitted,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: submitted,
            replayed: false,
        })
    }
}

async fn lock_inventory_count(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    count_id: Uuid,
) -> Result<InventoryCountRow, Wave3RepositoryError> {
    sqlx::query_as::<_, InventoryCountRow>(
        r#"
        SELECT id, owner_id, count_type, warehouse_id, zone_id, product_code,
               status, started_at, created_by, approved_by, approved_at,
               approval_source, approval_id, created_at, updated_at
          FROM inventory_counts
         WHERE owner_id = $1 AND id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(count_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave3RepositoryError::NotFound)
}

async fn load_inventory_count_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    count_id: Uuid,
) -> Result<InventoryCount, Wave3RepositoryError> {
    let count = sqlx::query_as::<_, InventoryCountRow>(
        r#"
        SELECT id, owner_id, count_type, warehouse_id, zone_id, product_code,
               status, started_at, created_by, approved_by, approved_at,
               approval_source, approval_id, created_at, updated_at
          FROM inventory_counts
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(count_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave3RepositoryError::NotFound)?;
    let lines = sqlx::query_as::<_, InventoryCountLineRow>(
        r#"
        SELECT id, count_id, owner_id, inventory_batch_id, location_id,
               location_code, product_code, batch_no, book_qty, physical_qty,
               variance_qty, variance_type
          FROM inventory_count_lines
         WHERE owner_id = $1 AND count_id = $2
         ORDER BY location_code, product_code, batch_no, id
        "#,
    )
    .bind(owner_id)
    .bind(count_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(map_inventory_count(count, lines))
}

async fn load_inventory_count(
    pool: &sqlx::PgPool,
    owner_id: Uuid,
    count_id: Uuid,
) -> Result<InventoryCount, Wave3RepositoryError> {
    let count = sqlx::query_as::<_, InventoryCountRow>(
        r#"
        SELECT id, owner_id, count_type, warehouse_id, zone_id, product_code,
               status, started_at, created_by, approved_by, approved_at,
               approval_source, approval_id, created_at, updated_at
          FROM inventory_counts
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(count_id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave3RepositoryError::NotFound)?;
    let lines = sqlx::query_as::<_, InventoryCountLineRow>(
        r#"
        SELECT id, count_id, owner_id, inventory_batch_id, location_id,
               location_code, product_code, batch_no, book_qty, physical_qty,
               variance_qty, variance_type
          FROM inventory_count_lines
         WHERE owner_id = $1 AND count_id = $2
         ORDER BY location_code, product_code, batch_no, id
        "#,
    )
    .bind(owner_id)
    .bind(count_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    Ok(map_inventory_count(count, lines))
}

fn map_inventory_count(
    count: InventoryCountRow,
    lines: Vec<InventoryCountLineRow>,
) -> InventoryCount {
    let blind = count.count_type == "blind";
    let redact_book = blind && matches!(count.status.as_str(), "in_progress" | "pending_approval");
    InventoryCount {
        id: count.id,
        owner_id: count.owner_id,
        count_type: count.count_type,
        warehouse_id: count.warehouse_id,
        zone_id: count.zone_id,
        product_code: count.product_code,
        status: count.status,
        started_at: count.started_at,
        created_by: count.created_by,
        approved_by: count.approved_by,
        approved_at: count.approved_at,
        approval_source: count.approval_source,
        approval_id: count.approval_id,
        created_at: count.created_at,
        updated_at: count.updated_at,
        lines: lines
            .into_iter()
            .map(|line| map_inventory_count_line(line, redact_book))
            .collect(),
    }
}

fn map_inventory_count_line(line: InventoryCountLineRow, redact_book: bool) -> InventoryCountLine {
    // 盲盘在实盘提交前不回显账面数量（DB 仍保留 book_qty 用于差异计算）。
    let hide = redact_book && line.physical_qty.is_none();
    InventoryCountLine {
        id: line.id,
        count_id: line.count_id,
        owner_id: line.owner_id,
        inventory_batch_id: line.inventory_batch_id,
        location_id: line.location_id,
        location_code: line.location_code,
        product_code: line.product_code,
        batch_no: line.batch_no,
        book_qty: if hide {
            wms_domain::Quantity::ZERO
        } else {
            line.book_qty
        },
        physical_qty: line.physical_qty,
        variance_qty: if hide { None } else { line.variance_qty },
        variance_type: if hide { None } else { line.variance_type },
    }
}
