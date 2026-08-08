//! Wave 4 repository helpers for cross-module business closures.
// @governance: skip-page-size shared row types and transaction helpers serve five include! slices.

use std::{collections::HashSet, future::Future, pin::Pin};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    validate_review_submission, validate_ship_outbound_request, CompletePickTaskRequest,
    CreateOutboundOrderRequest, CreateOutboundWaveRequest, CreatePurchaseReturnRequest,
    InventoryBatch, OutboundColdChainPackage, OutboundOrder, OutboundOrderLine, OutboundShipment,
    OutboundWave, PurchaseReturnOrder, RejectPurchaseReturnRequest, ReviewOutboundOrderRequest,
    ReviewValidationError, ShipOutboundOrderRequest, ShipOutboundValidationError,
    TemperatureExcursionEvent, TraceabilityOutboundReport, TraceabilityOutboundReportRequest,
    TraceabilityStatusChangeEvent,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    document_numbering::{GenerateDocumentNumberRequest, PgDocumentNumberingService},
    h2_lifecycle::publish_event_in_tx,
    idempotency,
    inventory::{STATUS_QUALIFIED, STATUS_QUARANTINED},
    operation_context::OperationContext as AuthContext,
    outbound::{
        all_lines_reviewed_for_ship, short_pick_qty, status_after_pick, status_after_review,
        OUTBOUND_STATUS_CONFIRMED, OUTBOUND_STATUS_IN_WAVE, OUTBOUND_STATUS_PENDING_VALIDATION,
        OUTBOUND_STATUS_REVIEWED, OUTBOUND_STATUS_REVIEWED_SHORT, OUTBOUND_STATUS_SHIPPED,
        OUTBOUND_STATUS_VALIDATION_EXCEPTION, OUTBOUND_STATUS_VOID_REQUESTED,
    },
    print_orchestration::{freeze_outbound_route_in_tx, PrintOrchestrationError},
    traceability_code::{
        TraceabilityCodeService, TraceabilityPlatformResponse, TraceabilityReplayDecision,
    },
};

pub const APPROVAL_SOURCE_TEMPERATURE_EXCURSION: &str = "M5-TEMP_EXCURSION";

mod integrations;

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

pub type ShipOutboundOrderFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<IdempotentMutation<OutboundOrder>, Wave4RepositoryError>>
            + Send
            + 'a,
    >,
>;

pub trait ShipOutboundOrderPort: Send + Sync {
    /// Persists one complete shipment operation in the repository transaction boundary.
    fn persist_ship_outbound_order<'a>(
        &'a self,
        ctx: &'a AuthContext,
        order_id: Uuid,
        req: ShipOutboundOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &'a str,
        audit: Option<AuditWriteRequest>,
    ) -> ShipOutboundOrderFuture<'a>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Wave4RepositoryError {
    NotFound,
    DuplicateCode,
    EmptySelection,
    InvalidDocumentType,
    InvalidDeliveryAddress,
    RouteBindingUnavailable,
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
    ShipmentValidation(ShipOutboundValidationError),
    InvalidQuantity,
    DocumentNumbering(String),
    InvalidTraceabilityEvent,
    IdempotencyConflict,
    OrderAlreadyInWave,
    PendingErpCancel,
    ShortPickNotReplenished,
    MissingSecondReviewer,
    UnqualifiedSecondReviewer,
    DualPersonApprovalRequired,
    MissingRejectReason,
    MissingRequiredField(&'static str),
    ErpGoodsMappingIncomplete,
    InvalidDriver,
    InvalidSignatureAttachment,
    Audit(String),
    Database(String),
    Serialize(String),
}

impl From<crate::idempotency::IdempotencyError> for Wave4RepositoryError {
    fn from(error: crate::idempotency::IdempotencyError) -> Self {
        match error {
            crate::idempotency::IdempotencyError::Conflict => Self::IdempotencyConflict,
            crate::idempotency::IdempotencyError::Database(error) => {
                Self::Database(error.to_string())
            }
            crate::idempotency::IdempotencyError::Serialize(error) => Self::Serialize(error),
        }
    }
}

#[derive(FromRow)]
struct InventoryBatchRow {
    id: Uuid,
    owner_id: Uuid,
    product_code: String,
    batch_no: String,
    production_date: NaiveDate,
    expiry_date: NaiveDate,
    qty_on_hand: wms_domain::Quantity,
    qty_locked: wms_domain::Quantity,
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
    invoice_no: Option<String>,
    transport_mode_code: Option<String>,
    department_code: Option<String>,
    sales_group_code: Option<String>,
    order_group_no: Option<String>,
    business_type_code: Option<String>,
    customer_id: Uuid,
    delivery_address_id: Uuid,
    delivery_address_snapshot: serde_json::Value,
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
    planned_qty: wms_domain::Quantity,
    picked_qty: wms_domain::Quantity,
    reviewed_qty: wms_domain::Quantity,
    shipped_qty: wms_domain::Quantity,
    short_pick_qty: wms_domain::Quantity,
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
    order_no: String,
    warehouse_id: Uuid,
    line_no: i32,
    batch_id: Uuid,
    product_code: String,
    batch_no: String,
    location_id: Uuid,
    location_code: String,
    planned_qty: wms_domain::Quantity,
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
include!("wave4_repository_shipment.rs");
include!("wave4_repository_waves.rs");
include!("wave4_repository_actions.rs");
include!("wave4_repository_returns.rs");
include!("wave4_repository_customer_portal.rs");

impl ShipOutboundOrderPort for PgWave4Repository {
    fn persist_ship_outbound_order<'a>(
        &'a self,
        ctx: &'a AuthContext,
        order_id: Uuid,
        req: ShipOutboundOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &'a str,
        audit: Option<AuditWriteRequest>,
    ) -> ShipOutboundOrderFuture<'a> {
        Box::pin(PgWave4Repository::ship_outbound_order(
            self,
            ctx,
            order_id,
            req,
            now,
            idempotency_key,
            audit,
        ))
    }
}

async fn lock_outbound_order(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<OutboundOrderRow, Wave4RepositoryError> {
    sqlx::query_as::<_, OutboundOrderRow>(
        r#"
        SELECT id, owner_id, document_type, wms_order_no, erp_order_no,
               invoice_no, transport_mode_code, department_code, sales_group_code,
               order_group_no, business_type_code, customer_id,
               delivery_address_id, delivery_address_snapshot, warehouse_id,
               required_ship_at, status, short_pick,
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
        SELECT id, owner_id, document_type, wms_order_no, erp_order_no,
               invoice_no, transport_mode_code, department_code, sales_group_code,
               order_group_no, business_type_code, customer_id,
               delivery_address_id, delivery_address_snapshot, warehouse_id,
               required_ship_at, status, short_pick,
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
    let shipment = load_outbound_shipment(tx, owner_id, id).await?;
    Ok(map_outbound_order(row, lines, shipment))
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
    while remaining > wms_domain::Quantity::ZERO {
        let row: Option<(Uuid, wms_domain::Quantity)> = sqlx::query_as(
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
    idempotency::replay_hash_only(tx, owner_id, idempotency_key, request_hash, now)
        .await
        .map_err(Into::into)
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), Wave4RepositoryError> {
    idempotency::lock_key(tx, "wave4", owner_id, idempotency_key)
        .await
        .map_err(Into::into)
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
    idempotency::store_success(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        resource_type,
        &resource_id,
        response,
        now,
    )
    .await
    .map_err(Into::into)
}

fn request_hash(value: &serde_json::Value) -> Result<String, Wave4RepositoryError> {
    idempotency::request_hash(value).map_err(Into::into)
}

fn non_empty_filter(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn map_outbound_order(
    row: OutboundOrderRow,
    lines: Vec<OutboundOrderLine>,
    shipment: Option<OutboundShipment>,
) -> OutboundOrder {
    OutboundOrder {
        id: row.id,
        owner_id: row.owner_id,
        document_type: row.document_type,
        wms_order_no: row.wms_order_no,
        erp_order_no: row.erp_order_no,
        invoice_no: row.invoice_no,
        transport_mode_code: row.transport_mode_code,
        department_code: row.department_code,
        sales_group_code: row.sales_group_code,
        order_group_no: row.order_group_no,
        business_type_code: row.business_type_code,
        customer_id: row.customer_id,
        delivery_address_id: row.delivery_address_id,
        delivery_address_snapshot: row.delivery_address_snapshot,
        warehouse_id: row.warehouse_id,
        required_ship_at: row.required_ship_at,
        status: row.status,
        short_pick: row.short_pick,
        lines,
        shipment,
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
        product_name: None,
        specification: None,
        manufacturer: None,
        batch_no: row.batch_no,
        production_date: row.production_date.to_string(),
        expiry_date: row.expiry_date.to_string(),
        qty_on_hand: row.qty_on_hand,
        qty_locked: row.qty_locked,
        quality_status: row.quality_status,
        location_id: row.location_id,
        location_code: row.location_code,
        row_no: None,
        column_no: None,
        layer_no: None,
        zone_code: None,
        temperature_zone: None,
        quality_color: None,
        max_volume_cm3: None,
        used_volume_cm3: None,
        remaining_volume_cm3: None,
        max_sku_count: None,
        current_sku_count: None,
        container_lpn: None,
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

fn map_route_freeze_error(error: PrintOrchestrationError) -> Wave4RepositoryError {
    match error {
        PrintOrchestrationError::RouteBindingNotFound
        | PrintOrchestrationError::EffectivePeriodOverlap
        | PrintOrchestrationError::InvalidRequest => Wave4RepositoryError::RouteBindingUnavailable,
        other => Wave4RepositoryError::Database(format!("{other:?}")),
    }
}

fn map_insert_error(error: sqlx::Error) -> Wave4RepositoryError {
    map_db_error(error)
}
