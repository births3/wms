//! Wave 4 repository helpers for cross-module business closures.

use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    validate_review_submission, CompletePickTaskRequest, CreateOutboundOrderRequest,
    CreateOutboundWaveRequest, InventoryBatch, OutboundOrder, OutboundOrderLine, OutboundWave,
    ReviewOutboundOrderRequest, ReviewValidationError, ShipOutboundOrderRequest,
    TemperatureExcursionEvent, TraceabilityOutboundReport, TraceabilityOutboundReportRequest,
    TraceabilityStatusChangeEvent,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    document_numbering::{GenerateDocumentNumberRequest, PgDocumentNumberingService},
    inventory::{STATUS_QUALIFIED, STATUS_QUARANTINED},
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
    InvalidDocumentType,
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
    ReviewValidation(ReviewValidationError),
    InvalidQuantity,
    DocumentNumbering(String),
    InvalidTraceabilityEvent,
    IdempotencyConflict,
    OrderAlreadyInWave,
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
    document_type: String,
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

struct PickTaskDraft {
    order_id: Uuid,
    line_no: i32,
    batch_id: Uuid,
    product_code: String,
    batch_no: String,
    location_id: Uuid,
    location_code: String,
    planned_qty: i64,
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

include!("wave4_repository_part1.rs");
include!("wave4_repository_part2.rs");
include!("wave4_repository_waves.rs");

async fn lock_outbound_order(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<OutboundOrderRow, Wave4RepositoryError> {
    sqlx::query_as::<_, OutboundOrderRow>(
        r#"
        SELECT id, owner_id, document_type, wms_order_no, erp_order_no, customer_id,
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

async fn ensure_outbound_document_type(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    document_type: &str,
) -> Result<(), Wave4RepositoryError> {
    let valid: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM system_dictionary_items
             WHERE dict_code = 'document_type'
               AND item_code = $1
               AND (owner_id IS NULL OR owner_id = $2)
               AND enabled = TRUE
               AND params->>'direction' = 'outbound'
        )
        "#,
    )
    .bind(document_type)
    .bind(owner_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if valid {
        Ok(())
    } else {
        Err(Wave4RepositoryError::InvalidDocumentType)
    }
}

async fn load_outbound_order(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<OutboundOrder, Wave4RepositoryError> {
    let row = sqlx::query_as::<_, OutboundOrderRow>(
        r#"
        SELECT id, owner_id, document_type, wms_order_no, erp_order_no, customer_id,
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

async fn load_outbound_pick_operator_ids(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
) -> Result<Vec<Uuid>, Wave4RepositoryError> {
    sqlx::query_scalar(
        r#"
        SELECT DISTINCT actor_id
          FROM audit_event
         WHERE owner_id = $1
           AND module = 'M4'
           AND action = 'complete_pick_task'
           AND resource_type = 'outbound_order'
           AND resource_id = $2
         ORDER BY actor_id
        "#,
    )
    .bind(owner_id)
    .bind(order_id.to_string())
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
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
               AND recall_flag = FALSE
               AND qty_on_hand - qty_locked > 0
             ORDER BY location_code ASC, id ASC
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
        append_outbound_inventory_movement(tx, owner_id, batch_id, -deducted, order_id, now)
            .await?;
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
        document_type: row.document_type,
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
