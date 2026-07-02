//! Wave 4 repository helpers for cross-module business closures.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    CompletePickTaskRequest, CreateOutboundOrderRequest, CreateOutboundWaveRequest, InventoryBatch,
    OutboundOrder, OutboundOrderLine, OutboundWave, ReviewOutboundOrderRequest,
    ShipOutboundOrderRequest, TemperatureExcursionEvent, TraceabilityOutboundReport,
    TraceabilityOutboundReportRequest, TraceabilityStatusChangeEvent,
};

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    auth::AuthContext,
    inventory::{allowed_transition, STATUS_QUALIFIED, STATUS_QUARANTINED},
    outbound::{
        all_lines_reviewed_for_ship, short_pick_qty, status_after_pick, status_after_review,
        OUTBOUND_STATUS_CONFIRMED, OUTBOUND_STATUS_IN_WAVE, OUTBOUND_STATUS_REVIEWED,
        OUTBOUND_STATUS_REVIEWED_SHORT, OUTBOUND_STATUS_SHIPPED,
    },
    traceability_code::{
        TraceabilityCodeService, TraceabilityPlatformResponse, TraceabilityReplayDecision,
    },
};

pub const APPROVAL_SOURCE_TEMPERATURE_EXCURSION: &str = "M5-TEMP_EXCURSION";

#[derive(Clone, Debug)]
pub struct PgWave4Repository {
    pool: PgPool,
}

#[derive(Clone, Debug)]
pub struct TemperatureExcursionDisposition {
    pub event: TemperatureExcursionEvent,
    pub quarantined_batches: Vec<InventoryBatch>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Wave4RepositoryError {
    NotFound,
    DuplicateCode,
    EmptySelection,
    BatchNotAffected(Uuid),
    InvalidStatus {
        expected: String,
        actual: String,
    },
    InvalidStateTransition {
        from: String,
        to: String,
        approval_source: String,
    },
    InvalidQuantity,
    InvalidTraceabilityEvent,
    IdempotencyConflict,
    ShortPickNotReplenished,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(FromRow)]
struct InventoryBatchRow {
    id: Uuid,
    owner_id: Uuid,
    product_code: String,
    batch_no: String,
    production_date: NaiveDate,
    expiry_date: NaiveDate,
    qty_on_hand: i64,
    qty_locked: i64,
    quality_status: String,
    location_id: Uuid,
    location_code: String,
    recall_flag: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct TemperatureExcursionEventRow {
    id: Uuid,
    owner_id: Uuid,
    external_event_id: String,
    device_code: String,
    location_code: Option<String>,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    min_temperature_celsius: Option<f64>,
    max_temperature_celsius: Option<f64>,
    affected_batch_ids: Vec<Uuid>,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct OutboundOrderRow {
    id: Uuid,
    owner_id: Uuid,
    wms_order_no: String,
    erp_order_no: Option<String>,
    customer_id: Uuid,
    warehouse_id: Uuid,
    required_ship_at: Option<DateTime<Utc>>,
    status: String,
    short_pick: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct OutboundOrderLineRow {
    line_no: i32,
    product_code: String,
    batch_no: String,
    planned_qty: i64,
    picked_qty: i64,
    reviewed_qty: i64,
    shipped_qty: i64,
    short_pick_qty: i64,
}

#[derive(FromRow)]
struct OutboundWaveRow {
    id: Uuid,
    owner_id: Uuid,
    wave_no: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct TraceabilityOutboundReportRow {
    id: Uuid,
    platform: String,
    status: String,
    queued_count: i32,
    generated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct TraceabilityOutboundReportEventRow {
    event_id: Uuid,
    trace_code: String,
    status_change_type: String,
    occurred_at: DateTime<Utc>,
}

impl PgWave4Repository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_outbound_orders(
        &self,
        ctx: &AuthContext,
        status: Option<&str>,
        q: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<OutboundOrder>, Wave4RepositoryError> {
        let status = non_empty_filter(status);
        let q = non_empty_filter(q);
        let limit = i64::from(limit.unwrap_or(50).clamp(1, 200));
        let rows = sqlx::query_as::<_, OutboundOrderRow>(
            r#"
            SELECT id, owner_id, wms_order_no, erp_order_no, customer_id,
                   warehouse_id, required_ship_at, status, short_pick,
                   created_at, updated_at
              FROM outbound_orders
             WHERE owner_id = $1
               AND ($2::TEXT IS NULL OR status = $2)
               AND (
                    $3::TEXT IS NULL
                    OR wms_order_no ILIKE '%' || $3 || '%'
                    OR erp_order_no ILIKE '%' || $3 || '%'
               )
             ORDER BY updated_at DESC, wms_order_no ASC
             LIMIT $4
            "#,
        )
        .bind(ctx.owner_id)
        .bind(status)
        .bind(q)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        let mut orders = Vec::with_capacity(rows.len());
        for row in rows {
            let lines =
                load_outbound_order_lines_from_pool(&self.pool, ctx.owner_id, row.id).await?;
            orders.push(map_outbound_order(row, lines));
        }
        Ok(orders)
    }

    pub async fn get_outbound_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<OutboundOrder, Wave4RepositoryError> {
        let row = sqlx::query_as::<_, OutboundOrderRow>(
            r#"
            SELECT id, owner_id, wms_order_no, erp_order_no, customer_id,
                   warehouse_id, required_ship_at, status, short_pick,
                   created_at, updated_at
              FROM outbound_orders
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;
        let lines = load_outbound_order_lines_from_pool(&self.pool, ctx.owner_id, id).await?;
        Ok(map_outbound_order(row, lines))
    }

    pub async fn create_outbound_order(
        &self,
        ctx: &AuthContext,
        req: CreateOutboundOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundOrder>, Wave4RepositoryError> {
        if req.lines.is_empty() || req.lines.iter().any(|line| line.planned_qty <= 0) {
            return Err(Wave4RepositoryError::InvalidQuantity);
        }
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let order_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO outbound_orders (
                id, owner_id, wms_order_no, erp_order_no, customer_id, warehouse_id,
                required_ship_at, status, short_pick, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, FALSE, $9, $9)
            "#,
        )
        .bind(order_id)
        .bind(ctx.owner_id)
        .bind(&req.wms_order_no)
        .bind(&req.erp_order_no)
        .bind(req.customer_id)
        .bind(req.warehouse_id)
        .bind(req.required_ship_at)
        .bind(OUTBOUND_STATUS_CONFIRMED)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_insert_error)?;

        for line in &req.lines {
            sqlx::query(
                r#"
                INSERT INTO outbound_order_lines (
                    id, outbound_order_id, owner_id, line_no, product_code,
                    batch_no, planned_qty
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(order_id)
            .bind(ctx.owner_id)
            .bind(i32::try_from(line.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?)
            .bind(&line.product_code)
            .bind(&line.batch_no)
            .bind(line.planned_qty)
            .execute(&mut *tx)
            .await
            .map_err(map_insert_error)?;
        }

        let order = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/orders",
            "outbound_order",
            order.id.to_string(),
            &order,
            now,
        )
        .await?;
        append_outbound_audit(&mut tx, ctx, audit, "create_outbound_order", order.id, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: order,
            replayed: false,
        })
    }

    pub async fn create_outbound_wave(
        &self,
        ctx: &AuthContext,
        req: CreateOutboundWaveRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundWave>, Wave4RepositoryError> {
        if req.order_ids.is_empty() {
            return Err(Wave4RepositoryError::EmptySelection);
        }
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let wave_row = sqlx::query_as::<_, OutboundWaveRow>(
            r#"
            INSERT INTO outbound_waves (id, owner_id, wave_no, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $5)
            RETURNING id, owner_id, wave_no, status, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(&req.wave_no)
        .bind("released")
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_insert_error)?;

        for order_id in &req.order_ids {
            let order = lock_outbound_order(&mut tx, ctx.owner_id, *order_id).await?;
            if order.status != OUTBOUND_STATUS_CONFIRMED {
                return Err(Wave4RepositoryError::InvalidStatus {
                    expected: OUTBOUND_STATUS_CONFIRMED.to_string(),
                    actual: order.status,
                });
            }
            sqlx::query(
                r#"
                UPDATE outbound_orders
                   SET status = $3, updated_at = $4, version = version + 1
                 WHERE owner_id = $1 AND id = $2
                "#,
            )
            .bind(ctx.owner_id)
            .bind(order.id)
            .bind(OUTBOUND_STATUS_IN_WAVE)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            sqlx::query(
                r#"
                INSERT INTO outbound_wave_orders (id, owner_id, wave_id, outbound_order_id, created_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(wave_row.id)
            .bind(order.id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_insert_error)?;
        }

        let wave = map_outbound_wave(wave_row, req.order_ids);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/waves",
            "outbound_wave",
            wave.id.to_string(),
            &wave,
            now,
        )
        .await?;
        append_outbound_audit(&mut tx, ctx, audit, "create_outbound_wave", wave.id, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: wave,
            replayed: false,
        })
    }

    pub async fn complete_pick_task(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        req: CompletePickTaskRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundOrder>, Wave4RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "outbound_order_id": order_id,
            "request": req,
        }))?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let order = lock_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        if !matches!(
            order.status.as_str(),
            OUTBOUND_STATUS_IN_WAVE | "picked" | "picked_short" | OUTBOUND_STATUS_REVIEWED_SHORT
        ) {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "in_wave|picked|picked_short|reviewed_short".to_string(),
                actual: order.status,
            });
        }

        let planned_qty: i64 = sqlx::query_scalar(
            r#"
            SELECT planned_qty
              FROM outbound_order_lines
             WHERE owner_id = $1 AND outbound_order_id = $2 AND line_no = $3
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(i32::try_from(req.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;
        let short_qty = short_pick_qty(planned_qty, req.picked_qty)
            .map_err(|_| Wave4RepositoryError::InvalidQuantity)?;

        sqlx::query(
            r#"
            UPDATE outbound_order_lines
               SET picked_qty = $4,
                   short_pick_qty = $5
             WHERE owner_id = $1 AND outbound_order_id = $2 AND line_no = $3
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(i32::try_from(req.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?)
        .bind(req.picked_qty)
        .bind(short_qty)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let mut updated = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        let next_status = status_after_pick(&updated.lines);
        let short_pick = updated.lines.iter().any(|line| line.short_pick_qty > 0);
        sqlx::query(
            r#"
            UPDATE outbound_orders
               SET status = $3,
                   short_pick = $4,
                   updated_at = $5,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(next_status)
        .bind(short_pick)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        updated = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/pick-tasks/{id}/complete",
            "outbound_order",
            updated.id.to_string(),
            &updated,
            now,
        )
        .await?;
        append_outbound_audit(&mut tx, ctx, audit, "complete_pick_task", updated.id, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: updated,
            replayed: false,
        })
    }

    pub async fn review_outbound_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        req: ReviewOutboundOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundOrder>, Wave4RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "outbound_order_id": order_id,
            "request": req,
        }))?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let order = lock_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        if !matches!(order.status.as_str(), "picked" | "picked_short") {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "picked|picked_short".to_string(),
                actual: order.status,
            });
        }
        sqlx::query(
            r#"
            UPDATE outbound_order_lines
               SET reviewed_qty = picked_qty
             WHERE owner_id = $1 AND outbound_order_id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let mut updated = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        let next_status = status_after_review(&updated.lines);
        sqlx::query(
            r#"
            UPDATE outbound_orders
               SET status = $3,
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(next_status)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        updated = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/orders/{id}/review",
            "outbound_order",
            updated.id.to_string(),
            &updated,
            now,
        )
        .await?;
        append_outbound_audit(
            &mut tx,
            ctx,
            audit,
            "review_outbound_order",
            updated.id,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: updated,
            replayed: false,
        })
    }

    pub async fn ship_outbound_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        req: ShipOutboundOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundOrder>, Wave4RepositoryError> {
        if req.package_count == 0 {
            return Err(Wave4RepositoryError::InvalidQuantity);
        }
        let request_hash = request_hash(&serde_json::json!({
            "outbound_order_id": order_id,
            "request": req,
        }))?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let order_row = lock_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        if !matches!(
            order_row.status.as_str(),
            OUTBOUND_STATUS_REVIEWED | OUTBOUND_STATUS_REVIEWED_SHORT
        ) {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "reviewed|reviewed_short".to_string(),
                actual: order_row.status,
            });
        }
        let order = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        all_lines_reviewed_for_ship(&order.lines)
            .map_err(|_| Wave4RepositoryError::ShortPickNotReplenished)?;

        for line in &order.lines {
            deduct_inventory_for_outbound(&mut tx, ctx.owner_id, order_id, line, now).await?;
        }
        sqlx::query(
            r#"
            UPDATE outbound_order_lines
               SET shipped_qty = planned_qty
             WHERE owner_id = $1 AND outbound_order_id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query(
            r#"
            INSERT INTO outbound_shipments (
                id, owner_id, outbound_order_id, carrier_type, handover_to,
                package_count, shipped_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(&req.carrier_type)
        .bind(&req.handover_to)
        .bind(i32::try_from(req.package_count).map_err(|_| Wave4RepositoryError::InvalidQuantity)?)
        .bind(req.shipped_at.unwrap_or(now))
        .execute(&mut *tx)
        .await
        .map_err(map_insert_error)?;
        sqlx::query(
            r#"
            UPDATE outbound_orders
               SET status = $3,
                   short_pick = FALSE,
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(OUTBOUND_STATUS_SHIPPED)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let shipped = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/orders/{id}/ship",
            "outbound_order",
            shipped.id.to_string(),
            &shipped,
            now,
        )
        .await?;
        append_outbound_audit(&mut tx, ctx, audit, "ship_outbound_order", shipped.id, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: shipped,
            replayed: false,
        })
    }

    pub async fn create_traceability_outbound_report(
        &self,
        ctx: &AuthContext,
        req: TraceabilityOutboundReportRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<TraceabilityOutboundReport>, Wave4RepositoryError> {
        let report = TraceabilityCodeService
            .traceability_report_at(req, now)
            .map_err(|_| Wave4RepositoryError::InvalidTraceabilityEvent)?;
        let request_hash = request_hash(&serde_json::json!({ "request": report.events }))?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        sqlx::query(
            r#"
            INSERT INTO traceability_outbound_reports (
                id, owner_id, platform, status, queued_count, generated_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $6, $6)
            "#,
        )
        .bind(report.report_id)
        .bind(ctx.owner_id)
        .bind(&report.platform)
        .bind(&report.status)
        .bind(
            i32::try_from(report.queued_count)
                .map_err(|_| Wave4RepositoryError::InvalidQuantity)?,
        )
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_insert_error)?;

        for event in &report.events {
            sqlx::query(
                r#"
                INSERT INTO traceability_outbound_report_events (
                    event_id, owner_id, report_id, trace_code, status_change_type,
                    occurred_at, report_status, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
                "#,
            )
            .bind(event.event_id)
            .bind(ctx.owner_id)
            .bind(report.report_id)
            .bind(&event.trace_code)
            .bind(&event.status_change_type)
            .bind(event.occurred_at)
            .bind(&report.status)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_insert_error)?;
        }

        let persisted =
            load_traceability_outbound_report(&mut tx, ctx.owner_id, report.report_id).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/traceability/outbound-reports",
            "traceability_outbound_report",
            persisted.report_id.to_string(),
            &persisted,
            now,
        )
        .await?;
        append_traceability_audit(
            &mut tx,
            ctx,
            audit,
            "create_traceability_outbound_report",
            persisted.report_id,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: persisted,
            replayed: false,
        })
    }

    pub async fn apply_traceability_platform_response(
        &self,
        ctx: &AuthContext,
        event_id: Uuid,
        response: TraceabilityPlatformResponse,
        now: DateTime<Utc>,
        audit: Option<AuditWriteRequest>,
    ) -> Result<TraceabilityReplayDecision, Wave4RepositoryError> {
        let decision = TraceabilityCodeService
            .classify_platform_response(response)
            .map_err(|_| Wave4RepositoryError::InvalidTraceabilityEvent)?;

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let report_id: Uuid = sqlx::query_scalar(
            r#"
            SELECT report_id
              FROM traceability_outbound_report_events
             WHERE owner_id = $1 AND event_id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(event_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;

        sqlx::query(
            r#"
            UPDATE traceability_outbound_report_events
               SET report_status = $3,
                   retry_count = retry_count + CASE WHEN $4 THEN 1 ELSE 0 END,
                   last_error_code = $5,
                   platform_receipt_id = $6,
                   updated_at = $7
             WHERE owner_id = $1 AND event_id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(event_id)
        .bind(&decision.status)
        .bind(decision.should_retry)
        .bind(&decision.error_code)
        .bind(&decision.platform_receipt_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        refresh_traceability_report_status(&mut tx, ctx.owner_id, report_id, now).await?;
        append_traceability_event_audit(&mut tx, ctx, audit, &decision.audit_action, event_id, now)
            .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(decision)
    }

    pub async fn list_pending_temperature_excursions(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<TemperatureExcursionEvent>, Wave4RepositoryError> {
        let rows = sqlx::query_as::<_, TemperatureExcursionEventRow>(
            r#"
            SELECT id, owner_id, external_event_id, device_code, location_code,
                   started_at, ended_at, min_temperature_celsius,
                   max_temperature_celsius, affected_batch_ids, status, created_at
              FROM temperature_excursion_events
             WHERE owner_id = $1 AND status = 'pending_disposition'
             ORDER BY created_at DESC, external_event_id ASC
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        Ok(rows.into_iter().map(map_temperature_excursion).collect())
    }

    pub async fn dispose_temperature_excursion_and_quarantine_batches(
        &self,
        ctx: &AuthContext,
        external_event_id: &str,
        selected_batch_ids: Vec<Uuid>,
        now: DateTime<Utc>,
        audit: Option<AuditWriteRequest>,
    ) -> Result<TemperatureExcursionDisposition, Wave4RepositoryError> {
        if selected_batch_ids.is_empty() {
            return Err(Wave4RepositoryError::EmptySelection);
        }

        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let event_row = sqlx::query_as::<_, TemperatureExcursionEventRow>(
            r#"
            SELECT id, owner_id, external_event_id, device_code, location_code,
                   started_at, ended_at, min_temperature_celsius,
                   max_temperature_celsius, affected_batch_ids, status, created_at
              FROM temperature_excursion_events
             WHERE owner_id = $1 AND external_event_id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(external_event_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;

        if event_row.status != "pending_disposition" {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "pending_disposition".to_string(),
                actual: event_row.status,
            });
        }

        for batch_id in &selected_batch_ids {
            if !event_row.affected_batch_ids.contains(batch_id) {
                return Err(Wave4RepositoryError::BatchNotAffected(*batch_id));
            }
        }

        let mut quarantined_batches = Vec::new();
        for batch_id in selected_batch_ids {
            let batch_row = sqlx::query_as::<_, InventoryBatchRow>(
                r#"
                SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                       qty_on_hand, qty_locked, quality_status, location_id, location_code,
                       recall_flag, created_at, updated_at
                  FROM inventory_batches
                 WHERE owner_id = $1 AND id = $2
                 FOR UPDATE
                "#,
            )
            .bind(ctx.owner_id)
            .bind(batch_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .ok_or(Wave4RepositoryError::NotFound)?;

            let from_status = batch_row.quality_status.clone();
            let batch = if from_status == STATUS_QUARANTINED {
                map_inventory_batch(batch_row)
            } else {
                if !allowed_transition(
                    &from_status,
                    STATUS_QUARANTINED,
                    APPROVAL_SOURCE_TEMPERATURE_EXCURSION,
                ) {
                    return Err(Wave4RepositoryError::InvalidStateTransition {
                        from: from_status,
                        to: STATUS_QUARANTINED.to_string(),
                        approval_source: APPROVAL_SOURCE_TEMPERATURE_EXCURSION.to_string(),
                    });
                }

                sqlx::query(
                    r#"
                    INSERT INTO inventory_status_changes (
                        id, owner_id, batch_id, from_status, to_status,
                        reason, approval_source, approval_id, occurred_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    "#,
                )
                .bind(Uuid::new_v4())
                .bind(ctx.owner_id)
                .bind(batch_id)
                .bind(&from_status)
                .bind(STATUS_QUARANTINED)
                .bind("temperature excursion disposition")
                .bind(APPROVAL_SOURCE_TEMPERATURE_EXCURSION)
                .bind(external_event_id)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;

                let updated = sqlx::query_as::<_, InventoryBatchRow>(
                    r#"
                    UPDATE inventory_batches
                       SET quality_status = $3,
                           updated_at = $4,
                           version = version + 1
                     WHERE owner_id = $1 AND id = $2
                    RETURNING id, owner_id, product_code, batch_no, production_date, expiry_date,
                              qty_on_hand, qty_locked, quality_status, location_id, location_code,
                              recall_flag, created_at, updated_at
                    "#,
                )
                .bind(ctx.owner_id)
                .bind(batch_id)
                .bind(STATUS_QUARANTINED)
                .bind(now)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_db_error)?;
                map_inventory_batch(updated)
            };
            quarantined_batches.push(batch);
        }

        let event_row = sqlx::query_as::<_, TemperatureExcursionEventRow>(
            r#"
            UPDATE temperature_excursion_events
               SET status = 'disposed'
             WHERE owner_id = $1 AND external_event_id = $2
            RETURNING id, owner_id, external_event_id, device_code, location_code,
                      started_at, ended_at, min_temperature_celsius,
                      max_temperature_celsius, affected_batch_ids, status, created_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(external_event_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let event = map_temperature_excursion(event_row);

        let mut audit = audit.unwrap_or_else(|| {
            AuditWriteRequest::from_auth_context(
                ctx,
                "dispose_temperature_excursion",
                "M5",
                "temperature_excursion",
                event.id.to_string(),
                None,
            )
        });
        audit.resource_id = event.id.to_string();
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave4RepositoryError::Audit(format!("{error:?}")))?;

        tx.commit().await.map_err(map_db_error)?;
        Ok(TemperatureExcursionDisposition {
            event,
            quarantined_batches,
        })
    }
}

async fn lock_outbound_order(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<OutboundOrderRow, Wave4RepositoryError> {
    sqlx::query_as::<_, OutboundOrderRow>(
        r#"
        SELECT id, owner_id, wms_order_no, erp_order_no, customer_id,
               warehouse_id, required_ship_at, status, short_pick,
               created_at, updated_at
          FROM outbound_orders
         WHERE owner_id = $1 AND id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave4RepositoryError::NotFound)
}

async fn load_outbound_order(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<OutboundOrder, Wave4RepositoryError> {
    let row = sqlx::query_as::<_, OutboundOrderRow>(
        r#"
        SELECT id, owner_id, wms_order_no, erp_order_no, customer_id,
               warehouse_id, required_ship_at, status, short_pick,
               created_at, updated_at
          FROM outbound_orders
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave4RepositoryError::NotFound)?;
    let line_rows = sqlx::query_as::<_, OutboundOrderLineRow>(
        r#"
        SELECT line_no, product_code, batch_no, planned_qty, picked_qty,
               reviewed_qty, shipped_qty, short_pick_qty
          FROM outbound_order_lines
         WHERE owner_id = $1 AND outbound_order_id = $2
         ORDER BY line_no
        "#,
    )
    .bind(owner_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let lines = line_rows
        .into_iter()
        .map(map_outbound_order_line)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(map_outbound_order(row, lines))
}

async fn load_outbound_order_lines_from_pool(
    pool: &PgPool,
    owner_id: Uuid,
    id: Uuid,
) -> Result<Vec<OutboundOrderLine>, Wave4RepositoryError> {
    let line_rows = sqlx::query_as::<_, OutboundOrderLineRow>(
        r#"
        SELECT line_no, product_code, batch_no, planned_qty, picked_qty,
               reviewed_qty, shipped_qty, short_pick_qty
          FROM outbound_order_lines
         WHERE owner_id = $1 AND outbound_order_id = $2
         ORDER BY line_no
        "#,
    )
    .bind(owner_id)
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    line_rows
        .into_iter()
        .map(map_outbound_order_line)
        .collect::<Result<Vec<_>, _>>()
}

async fn load_traceability_outbound_report(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    report_id: Uuid,
) -> Result<TraceabilityOutboundReport, Wave4RepositoryError> {
    let row = sqlx::query_as::<_, TraceabilityOutboundReportRow>(
        r#"
        SELECT id, platform, status, queued_count, generated_at
          FROM traceability_outbound_reports
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(report_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave4RepositoryError::NotFound)?;

    let event_rows = sqlx::query_as::<_, TraceabilityOutboundReportEventRow>(
        r#"
        SELECT event_id, trace_code, status_change_type, occurred_at
          FROM traceability_outbound_report_events
         WHERE owner_id = $1 AND report_id = $2
         ORDER BY occurred_at ASC, trace_code ASC
        "#,
    )
    .bind(owner_id)
    .bind(row.id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;

    Ok(TraceabilityOutboundReport {
        report_id: row.id,
        platform: row.platform,
        status: row.status,
        queued_count: u32::try_from(row.queued_count)
            .map_err(|_| Wave4RepositoryError::InvalidQuantity)?,
        generated_at: row.generated_at,
        events: event_rows
            .into_iter()
            .map(|event| TraceabilityStatusChangeEvent {
                event_id: event.event_id,
                trace_code: event.trace_code,
                status_change_type: event.status_change_type,
                occurred_at: event.occurred_at,
            })
            .collect(),
    })
}

async fn deduct_inventory_for_outbound(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
    line: &OutboundOrderLine,
    now: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    let mut remaining = line.planned_qty;
    while remaining > 0 {
        let row: Option<(Uuid, i64)> = sqlx::query_as(
            r#"
            SELECT id, qty_on_hand - qty_locked AS available_qty
              FROM inventory_batches
             WHERE owner_id = $1
               AND product_code = $2
               AND batch_no = $3
               AND quality_status = $4
               AND qty_on_hand - qty_locked > 0
             ORDER BY expiry_date ASC, location_code ASC
             LIMIT 1
             FOR UPDATE
            "#,
        )
        .bind(owner_id)
        .bind(&line.product_code)
        .bind(&line.batch_no)
        .bind(STATUS_QUALIFIED)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
        let Some((batch_id, available_qty)) = row else {
            return Err(Wave4RepositoryError::InvalidQuantity);
        };
        let deducted = available_qty.min(remaining);
        sqlx::query(
            r#"
            UPDATE inventory_batches
               SET qty_on_hand = qty_on_hand - $3,
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(batch_id)
        .bind(deducted)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query(
            r#"
            INSERT INTO inventory_movements (
                id, owner_id, batch_id, movement_type, qty_delta,
                source_document_type, source_document_id, occurred_at
            )
            VALUES ($1, $2, $3, 'outbound_ship', $4, 'outbound_order', $5, $6)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(batch_id)
        .bind(-deducted)
        .bind(order_id)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        remaining -= deducted;
    }
    Ok(())
}

async fn append_outbound_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    audit: Option<AuditWriteRequest>,
    action: &str,
    resource_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    let mut audit = audit.unwrap_or_else(|| {
        AuditWriteRequest::from_auth_context(
            ctx,
            action,
            "M4",
            "outbound_order",
            resource_id.to_string(),
            None,
        )
    });
    audit.action = action.to_string();
    audit.module = "M4".to_string();
    audit.resource_type = "outbound_order".to_string();
    audit.resource_id = resource_id.to_string();
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| Wave4RepositoryError::Audit(format!("{error:?}")))
}

async fn append_traceability_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    audit: Option<AuditWriteRequest>,
    action: &str,
    resource_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    let mut audit = audit.unwrap_or_else(|| {
        AuditWriteRequest::from_auth_context(
            ctx,
            action,
            "M-TC",
            "traceability_outbound_report",
            resource_id.to_string(),
            None,
        )
    });
    audit.action = action.to_string();
    audit.module = "M-TC".to_string();
    audit.resource_type = "traceability_outbound_report".to_string();
    audit.resource_id = resource_id.to_string();
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| Wave4RepositoryError::Audit(format!("{error:?}")))
}

async fn append_traceability_event_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    audit: Option<AuditWriteRequest>,
    action: &str,
    resource_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    let mut audit = audit.unwrap_or_else(|| {
        AuditWriteRequest::from_auth_context(
            ctx,
            action,
            "M-TC",
            "traceability_outbound_report_event",
            resource_id.to_string(),
            None,
        )
    });
    audit.action = action.to_string();
    audit.module = "M-TC".to_string();
    audit.resource_type = "traceability_outbound_report_event".to_string();
    audit.resource_id = resource_id.to_string();
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| Wave4RepositoryError::Audit(format!("{error:?}")))
}

async fn refresh_traceability_report_status(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    report_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    let (total, reported, pending_replay): (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::BIGINT,
            COUNT(*) FILTER (WHERE report_status = 'reported')::BIGINT,
            COUNT(*) FILTER (WHERE report_status = 'pending_replay')::BIGINT
          FROM traceability_outbound_report_events
         WHERE owner_id = $1 AND report_id = $2
        "#,
    )
    .bind(owner_id)
    .bind(report_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let next_status = if total > 0 && reported == total {
        "reported"
    } else if pending_replay > 0 {
        "pending_replay"
    } else {
        "queued"
    };
    sqlx::query(
        r#"
        UPDATE traceability_outbound_reports
           SET status = $3,
               updated_at = $4,
               version = version + 1
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(report_id)
    .bind(next_status)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, Wave4RepositoryError> {
    let row: Option<(String, serde_json::Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT request_hash, response_body, expires_at
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((stored_hash, response_body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        return Ok(None);
    }
    if stored_hash != request_hash {
        return Err(Wave4RepositoryError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| Wave4RepositoryError::Serialize(error.to_string()))
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), Wave4RepositoryError> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(idempotency_lock_id(owner_id, idempotency_key))
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| Wave4RepositoryError::Serialize(error.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 200, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(method)
    .bind(path)
    .bind(response_body)
    .bind(resource_type)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

fn request_hash(value: &serde_json::Value) -> Result<String, Wave4RepositoryError> {
    let text = serde_json::to_string(value)
        .map_err(|error| Wave4RepositoryError::Serialize(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn idempotency_lock_id(owner_id: Uuid, idempotency_key: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(owner_id.as_bytes());
    hasher.update([0]);
    hasher.update(idempotency_key.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

fn non_empty_filter(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn map_outbound_order(row: OutboundOrderRow, lines: Vec<OutboundOrderLine>) -> OutboundOrder {
    OutboundOrder {
        id: row.id,
        owner_id: row.owner_id,
        wms_order_no: row.wms_order_no,
        erp_order_no: row.erp_order_no,
        customer_id: row.customer_id,
        warehouse_id: row.warehouse_id,
        required_ship_at: row.required_ship_at,
        status: row.status,
        short_pick: row.short_pick,
        lines,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_outbound_order_line(
    row: OutboundOrderLineRow,
) -> Result<OutboundOrderLine, Wave4RepositoryError> {
    Ok(OutboundOrderLine {
        line_no: u32::try_from(row.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?,
        product_code: row.product_code,
        batch_no: row.batch_no,
        planned_qty: row.planned_qty,
        picked_qty: row.picked_qty,
        reviewed_qty: row.reviewed_qty,
        shipped_qty: row.shipped_qty,
        short_pick_qty: row.short_pick_qty,
    })
}

fn map_outbound_wave(row: OutboundWaveRow, order_ids: Vec<Uuid>) -> OutboundWave {
    OutboundWave {
        id: row.id,
        owner_id: row.owner_id,
        wave_no: row.wave_no,
        status: row.status,
        order_ids,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_inventory_batch(row: InventoryBatchRow) -> InventoryBatch {
    InventoryBatch {
        id: row.id,
        owner_id: row.owner_id,
        product_code: row.product_code,
        batch_no: row.batch_no,
        production_date: row.production_date.to_string(),
        expiry_date: row.expiry_date.to_string(),
        qty_on_hand: row.qty_on_hand,
        qty_locked: row.qty_locked,
        quality_status: row.quality_status,
        location_id: row.location_id,
        location_code: row.location_code,
        recall_flag: row.recall_flag,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_temperature_excursion(row: TemperatureExcursionEventRow) -> TemperatureExcursionEvent {
    TemperatureExcursionEvent {
        id: row.id,
        owner_id: row.owner_id,
        external_event_id: row.external_event_id,
        device_code: row.device_code,
        location_code: row.location_code,
        started_at: row.started_at,
        ended_at: row.ended_at,
        min_temperature_celsius: row.min_temperature_celsius,
        max_temperature_celsius: row.max_temperature_celsius,
        affected_batch_ids: row.affected_batch_ids,
        status: row.status,
        created_at: row.created_at,
    }
}

fn map_db_error(error: sqlx::Error) -> Wave4RepositoryError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return Wave4RepositoryError::DuplicateCode;
        }
    }
    Wave4RepositoryError::Database(error.to_string())
}

fn map_insert_error(error: sqlx::Error) -> Wave4RepositoryError {
    map_db_error(error)
}
