//! Wave 3 PostgreSQL repository, aligned with ADR-0034.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    validate_billing_rule_request, validate_create_receiving_order_request, BillingAccount,
    BillingContract, BillingRule, BillingRuleValidationError, ChangeInventoryStatusRequest,
    ColdChainDevice, CreateBillingAccountRequest, CreateBillingContractRequest,
    CreateBillingRuleRequest, CreateColdChainDeviceRequest, CreateReceivingOrderRequest,
    IngestTemperatureExcursionRequest, IngestTemperatureReadingRequest,
    InspectReceivingOrderRequest, InspectionSignatureRecord, InventoryBatch, InventoryMovement,
    PutawayLocationRecommendation, PutawayRecommendationQuery, PutawayRecommendationResponse,
    PutawayRecord, PutawayRequest, ReceiveReceivingOrderRequest, ReceivingDashboardQuery,
    ReceivingDashboardRow, ReceivingInspectionRecord, ReceivingOrder, ReceivingOrderLine,
    ReceivingOrderPrintData, ReceivingOrderReceipt, ReceivingOrderRequestValidationError,
    ReceivingReceiptDetails, RejectReceivingOrderRequest, SignInspectionRequest,
    TemperatureExcursionEvent, TemperatureReading, UpdateColdChainDeviceRequest,
    UpdateReceivingOrderRequest, RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND,
    RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    inventory::STATUS_QUALIFIED,
};

mod abc;
mod alerts;
mod billing;
mod cold_chain;
mod erp_outbox;
mod expiry;
mod integrations;
mod inventory_count;
mod maintenance;
mod mappings;
mod putaway;
mod query;
mod recall;
mod receiving_read;
mod receiving_update;
mod relocation;
mod trace;

use mappings::{
    map_cold_chain_device, map_inspection_signature, map_inventory_batch, map_receiving_inspection,
    map_receiving_order, map_receiving_order_line, map_receiving_order_receipt,
    map_temperature_excursion, map_temperature_reading,
};
use query::parse_optional_date;

#[derive(Clone, Debug)]
pub struct PgWave3Repository {
    pool: PgPool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PutawayInventoryCommit {
    pub putaway: PutawayRecord,
    pub inventory_batch: InventoryBatch,
    pub inventory_movement: InventoryMovement,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Wave3RepositoryError {
    NotFound,
    DuplicateReceipt,
    DuplicateCode,
    InvalidStatus {
        expected: String,
        actual: String,
    },
    InvalidQuantity,
    MissingSupplier,
    MissingExpectedArrival,
    InvalidExpectedArrival,
    MissingProduct,
    MultipleProducts,
    InvalidDocumentType,
    InvalidBatchPolicy,
    InvalidQualityStatus,
    InvalidDeviceType,
    ActiveMonitoring,
    DuplicateTraceCode,
    InvalidLocation,
    InsufficientQuantity,
    LocationQualityMismatch,
    LocationTemperatureMismatch,
    LocationCapacityExceeded,
    LocationSkuLimitExceeded,
    NoAvailableLocation,
    InvalidProductVolume,
    DocumentNumbering(String),
    InvalidDate(String),
    BatchExpired,
    QuantityClosureMismatch,
    OverReceiptNotAllowed,
    MissingSecondSigner,
    DualPersonApprovalRequired,
    SameSigner,
    UnauthorizedSigner,
    InvalidReason,
    MissingApprovalSource,
    RecallAlreadyActive,
    RecallNotActive,
    RecallStateChanged,
    SameApprover,
    SecondApproverNotAuthorized,
    InvalidStateTransition {
        from: String,
        to: String,
        approval_source: String,
    },
    IdempotencyConflict,
    BillingRuleConflict,
    InvalidBillingRuleField,
    InvalidEffectiveWindow,
    InvalidRate,
    FutureTimestamp,
    InvalidInventoryState,
    InvalidMaintenanceTaskState,
    InvalidMaintenanceResult,
    InvalidInventoryCountType,
    InvalidInventoryCountState,
    InventoryCountAlreadyActive,
    InventoryCountLineNotFound,
    InventoryCountLineAlreadySubmitted,
    InventoryCountNotReady,
    InventoryCountQuantityConflict,
    NoInventoryData,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, FromRow)]
struct ReceivingOrderRow {
    id: Uuid,
    owner_id: Uuid,
    receipt_no: String,
    document_type: String,
    supplier_id: Option<Uuid>,
    warehouse_id: Uuid,
    external_ref: Option<String>,
    status: String,
    expected_arrival_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct ReceivingOrderLineRow {
    id: Uuid,
    line_no: i32,
    product_id: Option<Uuid>,
    product_code: String,
    expected_qty: i64,
    batch_no: Option<String>,
    production_date: Option<NaiveDate>,
    expiry_date: Option<NaiveDate>,
}

#[derive(FromRow)]
struct ReceivingOrderReceiptRow {
    id: Uuid,
    receiving_order_id: Uuid,
    owner_id: Uuid,
    actual_qty: i64,
    shortage_qty: i64,
    rejected_qty: i64,
    arrival_temperature_celsius: Option<f64>,
    exception_note: Option<String>,
    receiving_details: Option<sqlx::types::Json<ReceivingReceiptDetails>>,
    occurred_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct ReceivingInspectionRow {
    id: Uuid,
    receiving_order_id: Uuid,
    owner_id: Uuid,
    batch_no: String,
    accepted_qty: i64,
    rejected_qty: i64,
    quality_status: String,
    occurred_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct InspectionSignatureRow {
    id: Uuid,
    receiving_order_id: Uuid,
    owner_id: Uuid,
    first_signer_id: Uuid,
    second_signer_id: Option<Uuid>,
    strategy_rule_id: Option<Uuid>,
    approval_record_id: Option<Uuid>,
    signed_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct InventoryBatchRow {
    id: Uuid,
    owner_id: Uuid,
    product_code: String,
    #[sqlx(default)]
    product_name: Option<String>,
    #[sqlx(default)]
    specification: Option<String>,
    #[sqlx(default)]
    manufacturer: Option<String>,
    batch_no: String,
    production_date: NaiveDate,
    expiry_date: NaiveDate,
    qty_on_hand: i64,
    qty_locked: i64,
    quality_status: String,
    location_id: Uuid,
    location_code: String,
    #[sqlx(default)]
    row_no: Option<i32>,
    #[sqlx(default)]
    column_no: Option<i32>,
    #[sqlx(default)]
    layer_no: Option<i32>,
    #[sqlx(default)]
    zone_code: Option<String>,
    #[sqlx(default)]
    temperature_zone: Option<String>,
    #[sqlx(default)]
    quality_color: Option<String>,
    #[sqlx(default)]
    max_volume_cm3: Option<i64>,
    #[sqlx(default)]
    used_volume_cm3: Option<i64>,
    #[sqlx(default)]
    remaining_volume_cm3: Option<i64>,
    #[sqlx(default)]
    max_sku_count: Option<i32>,
    #[sqlx(default)]
    current_sku_count: Option<i64>,
    #[sqlx(default)]
    container_lpn: Option<String>,
    recall_flag: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct BillingContractRow {
    id: Uuid,
    owner_id: Uuid,
    account_id: Uuid,
    contract_no: String,
    valid_from: NaiveDate,
    valid_to: NaiveDate,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct TemperatureReadingRow {
    id: Uuid,
    owner_id: Uuid,
    device_code: String,
    temperature_celsius: f64,
    humidity_percent: Option<f64>,
    captured_at: DateTime<Utc>,
    external_report_url: Option<String>,
    out_of_range: bool,
}

#[derive(FromRow)]
struct ColdChainDeviceRow {
    id: Uuid,
    owner_id: Uuid,
    device_code: String,
    device_type: String,
    installed_at_location_code: Option<String>,
    calibration_due_at: Option<DateTime<Utc>>,
    status: String,
    created_at: DateTime<Utc>,
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

include!("wave3_repository_part1.rs");
include!("wave3_repository_quality.rs");
include!("wave3_repository_part2.rs");
include!("wave3_repository_part3.rs");

async fn lock_receiving_order(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<ReceivingOrderRow, Wave3RepositoryError> {
    sqlx::query_as::<_, ReceivingOrderRow>(
        r#"
        SELECT id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
               external_ref, status, expected_arrival_at, created_at, updated_at
          FROM receiving_orders
         WHERE id = $1 AND owner_id = $2
         FOR UPDATE
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave3RepositoryError::NotFound)
}

async fn load_receiving_order_lines_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<Vec<ReceivingOrderLine>, Wave3RepositoryError> {
    let rows = sqlx::query_as::<_, ReceivingOrderLineRow>(
        r#"
        SELECT id, line_no, product_id, product_code, expected_qty, batch_no,
               production_date, expiry_date
          FROM receiving_order_lines
         WHERE receiving_order_id = $1 AND owner_id = $2
         ORDER BY line_no
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(rows.into_iter().map(map_receiving_order_line).collect())
}

async fn ensure_owned_reference(
    tx: &mut Transaction<'_, Postgres>,
    table: &'static str,
    owner_id: Uuid,
    id: Uuid,
) -> Result<(), Wave3RepositoryError> {
    let query = match table {
        "suppliers" => "SELECT EXISTS(SELECT 1 FROM suppliers WHERE owner_id = $1 AND id = $2 AND status = 'active')",
        "warehouses" => "SELECT EXISTS(SELECT 1 FROM warehouses WHERE owner_id = $1 AND id = $2 AND status = 'active')",
        "products" => "SELECT EXISTS(SELECT 1 FROM products WHERE owner_id = $1 AND id = $2 AND status = 'active')",
        _ => {
            return Err(Wave3RepositoryError::Serialize(
                "invalid reference table".to_string(),
            ))
        }
    };
    let exists: bool = sqlx::query_scalar(query)
        .bind(owner_id)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(Wave3RepositoryError::NotFound)
    }
}

async fn ensure_cold_chain_device_active(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    device_code: &str,
) -> Result<(), Wave3RepositoryError> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM cold_chain_devices
             WHERE owner_id = $1 AND device_code = $2 AND status = 'active'
        )
        "#,
    )
    .bind(owner_id)
    .bind(device_code)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(Wave3RepositoryError::NotFound)
    }
}

async fn load_temperature_reading(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    device_code: &str,
    captured_at: DateTime<Utc>,
) -> Result<Option<TemperatureReading>, Wave3RepositoryError> {
    let row = sqlx::query_as::<_, TemperatureReadingRow>(
        r#"
        SELECT id, owner_id, device_code, temperature_celsius, humidity_percent,
               captured_at, external_report_url, out_of_range
          FROM temperature_readings
         WHERE owner_id = $1 AND device_code = $2 AND captured_at = $3
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(device_code)
    .bind(captured_at)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(row.map(map_temperature_reading))
}

async fn load_temperature_excursion(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    external_event_id: &str,
) -> Result<Option<TemperatureExcursionEvent>, Wave3RepositoryError> {
    let row = sqlx::query_as::<_, TemperatureExcursionEventRow>(
        r#"
        SELECT id, owner_id, external_event_id, device_code, location_code,
               started_at, ended_at, min_temperature_celsius,
               max_temperature_celsius, affected_batch_ids, status, created_at
          FROM temperature_excursion_events
         WHERE owner_id = $1 AND external_event_id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(external_event_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(row.map(map_temperature_excursion))
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, Wave3RepositoryError> {
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
        return Err(Wave3RepositoryError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| Wave3RepositoryError::Serialize(error.to_string()))
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), Wave3RepositoryError> {
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
) -> Result<(), Wave3RepositoryError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| Wave3RepositoryError::Serialize(error.to_string()))?;
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

fn request_hash(value: &serde_json::Value) -> Result<String, Wave3RepositoryError> {
    let text = serde_json::to_string(value)
        .map_err(|error| Wave3RepositoryError::Serialize(error.to_string()))?;
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

async fn insert_receiving_order_lines(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    receiving_order_id: Uuid,
    lines: &[ReceivingOrderLine],
) -> Result<(), Wave3RepositoryError> {
    for line in lines {
        sqlx::query(
            r#"
            INSERT INTO receiving_order_lines (
                id, receiving_order_id, owner_id, line_no, product_id,
                product_code, expected_qty, batch_no, production_date, expiry_date
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(receiving_order_id)
        .bind(owner_id)
        .bind(i32::try_from(line.line_no).map_err(|_| Wave3RepositoryError::InvalidQuantity)?)
        .bind(line.product_id)
        .bind(&line.product_code)
        .bind(line.expected_qty)
        .bind(&line.batch_no)
        .bind(parse_optional_date(line.production_date.as_deref())?)
        .bind(parse_optional_date(line.expiry_date.as_deref())?)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

async fn insert_receiving_order_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    mut req: CreateReceivingOrderRequest,
    now: DateTime<Utc>,
) -> Result<ReceivingOrder, Wave3RepositoryError> {
    let id = Uuid::new_v4();
    let receipt_no = if req.receipt_no.trim().is_empty() {
        crate::document_numbering::PgDocumentNumberingService::new()
            .generate_in_tx(
                tx,
                ctx,
                crate::document_numbering::GenerateDocumentNumberRequest {
                    document_type: req.document_type.clone(),
                    idempotency_key: format!("m2-asn-create:{id}"),
                    source_module: "M2".to_string(),
                    source_document_id: Some(id),
                },
                now,
            )
            .await
            .map_err(|error| Wave3RepositoryError::DocumentNumbering(format!("{error:?}")))?
            .value
            .generated_no
    } else {
        req.receipt_no.clone()
    };
    for line in &mut req.lines {
        if line.product_id.is_none() {
            line.product_id = sqlx::query_scalar(
                "SELECT id FROM products WHERE owner_id = $1 AND product_code = $2 AND status = 'active'",
            )
            .bind(ctx.owner_id)
            .bind(&line.product_code)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
        }
    }
    sqlx::query(
        r#"
        INSERT INTO receiving_orders (
            id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
            external_ref, status, expected_arrival_at, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'draft', $8, $9, $9)
        "#,
    )
    .bind(id)
    .bind(ctx.owner_id)
    .bind(&receipt_no)
    .bind(&req.document_type)
    .bind(req.supplier_id)
    .bind(req.warehouse_id)
    .bind(&req.external_ref)
    .bind(req.expected_arrival_at)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    insert_receiving_order_lines(&mut *tx, ctx.owner_id, id, &req.lines).await?;
    Ok(ReceivingOrder {
        id,
        owner_id: ctx.owner_id,
        receipt_no,
        document_type: req.document_type,
        supplier_id: req.supplier_id,
        warehouse_id: req.warehouse_id,
        external_ref: req.external_ref,
        status: "draft".to_string(),
        expected_arrival_at: req.expected_arrival_at,
        lines: req.lines,
        created_at: now,
        updated_at: now,
    })
}

fn validate_document_type(value: &str) -> Result<(), Wave3RepositoryError> {
    match value {
        RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND | RECEIVING_DOCUMENT_TYPE_SALES_RETURN => Ok(()),
        _ => Err(Wave3RepositoryError::InvalidDocumentType),
    }
}

fn map_request_validation_error(
    error: ReceivingOrderRequestValidationError,
) -> Wave3RepositoryError {
    match error {
        ReceivingOrderRequestValidationError::MissingSupplier => {
            Wave3RepositoryError::MissingSupplier
        }
        ReceivingOrderRequestValidationError::MissingExpectedArrival => {
            Wave3RepositoryError::MissingExpectedArrival
        }
        ReceivingOrderRequestValidationError::InvalidExpectedArrival => {
            Wave3RepositoryError::InvalidExpectedArrival
        }
        ReceivingOrderRequestValidationError::MissingProduct => {
            Wave3RepositoryError::MissingProduct
        }
        ReceivingOrderRequestValidationError::MultipleProducts => {
            Wave3RepositoryError::MultipleProducts
        }
    }
}

fn validate_receiving_order_lines(
    document_type: &str,
    lines: &[ReceivingOrderLine],
) -> Result<(), Wave3RepositoryError> {
    if lines.is_empty() {
        return Err(Wave3RepositoryError::InvalidQuantity);
    }

    for line in lines {
        if line.line_no == 0 || line.expected_qty <= 0 {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let has_batch = line
            .batch_no
            .as_deref()
            .is_some_and(|batch_no| !batch_no.trim().is_empty());
        match document_type {
            RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND
                if line.batch_no.is_some()
                    || line.production_date.is_some()
                    || line.expiry_date.is_some() =>
            {
                return Err(Wave3RepositoryError::InvalidBatchPolicy);
            }
            RECEIVING_DOCUMENT_TYPE_SALES_RETURN if !has_batch => {
                return Err(Wave3RepositoryError::InvalidBatchPolicy);
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_date(value: &str) -> Result<NaiveDate, Wave3RepositoryError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| Wave3RepositoryError::InvalidDate(value.to_string()))
}

fn map_db_error(error: sqlx::Error) -> Wave3RepositoryError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return Wave3RepositoryError::DuplicateCode;
        }
    }
    Wave3RepositoryError::Database(error.to_string())
}

fn map_receipt_insert_error(error: sqlx::Error) -> Wave3RepositoryError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return Wave3RepositoryError::DuplicateReceipt;
        }
    }
    map_db_error(error)
}
