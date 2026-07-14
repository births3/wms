//! Wave 5 repository for value-added modules.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    BillingChargeCalculation, BillingStatement, ConfirmBillingStatementRequest,
    ConfirmContainerRecoveryRequest, ContainerRecovery, CreateCrossdockPlanRequest,
    CreatePackJobRequest, CreatePackingStationRequest, CreateRetailReplenishmentSuggestionRequest,
    CrossdockPlan, GenerateBillingStatementRequest, IngestTransitTemperatureRequest, PackJob,
    PackingStation, PrintWaybillRequest, ReceiveTmsDispatchRequest, RetailReplenishmentSuggestion,
    TmsDispatch, TransitTemperatureReading, WeighPackJobRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    auth::AuthContext,
    packing_station::{PackingStationError, PackingStationService},
    retail_chain::{RetailChainError, RetailChainService},
    tms_plus::{
        ReceiveTmsRoutePlanRequest, TmsPlusError, TmsPlusService, TmsRoutePlan, TmsRouteStop,
    },
};

#[derive(Clone, Debug)]
pub struct PgWave5Repository {
    pool: PgPool,
    packing: PackingStationService,
    retail: RetailChainService,
    tms: TmsPlusService,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Wave5RepositoryError {
    NotFound,
    InvalidInput,
    DuplicateCode,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

impl From<PackingStationError> for Wave5RepositoryError {
    fn from(_value: PackingStationError) -> Self {
        Self::InvalidInput
    }
}

impl From<RetailChainError> for Wave5RepositoryError {
    fn from(_value: RetailChainError) -> Self {
        Self::InvalidInput
    }
}

impl From<TmsPlusError> for Wave5RepositoryError {
    fn from(_value: TmsPlusError) -> Self {
        Self::InvalidInput
    }
}

#[derive(FromRow)]
struct PackingStationRow {
    id: Uuid,
    owner_id: Uuid,
    station_code: String,
    station_name: String,
    printer_code: Option<String>,
    scale_code: Option<String>,
    temperature_zone: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct PackJobRow {
    id: Uuid,
    owner_id: Uuid,
    outbound_order_id: Uuid,
    station_id: Option<Uuid>,
    job_no: String,
    pack_mode: String,
    recommended_box_type: String,
    actual_box_type: String,
    adjustment_reason: Option<String>,
    outbound_lpn: String,
    trace_codes: Vec<String>,
    status: String,
    weight_grams: Option<i64>,
    waybill_no: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct RetailReplenishmentSuggestionRow {
    id: Uuid,
    owner_id: Uuid,
    store_id: Uuid,
    product_code: String,
    period_key: String,
    min_qty: i64,
    max_qty: i64,
    current_qty: i64,
    in_transit_qty: i64,
    daily_sales_avg: i64,
    suggested_qty: i64,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct CrossdockPlanRow {
    id: Uuid,
    owner_id: Uuid,
    asn_id: Uuid,
    outbound_order_id: Uuid,
    store_id: Uuid,
    product_code: String,
    qty: i64,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct BillingChargeCalculationRow {
    id: Uuid,
    owner_id: Uuid,
    contract_id: Uuid,
    period_start: chrono::NaiveDate,
    period_end: chrono::NaiveDate,
    charge_item: String,
    quantity: i64,
    amount_cents: i64,
    source_refs: Vec<String>,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct BillingStatementRow {
    id: Uuid,
    owner_id: Uuid,
    contract_id: Uuid,
    period_start: chrono::NaiveDate,
    period_end: chrono::NaiveDate,
    status: String,
    total_amount_cents: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct TmsDispatchRow {
    id: Uuid,
    owner_id: Uuid,
    dispatch_no: String,
    outbound_order_id: Uuid,
    delivery_provider_type: String,
    vehicle_no: Option<String>,
    plate_no: Option<String>,
    driver_user_id: Option<Uuid>,
    carrier_code: Option<String>,
    waybill_no: Option<String>,
    status: String,
    version: i32,
    scheduled_load_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct TmsRoutePlanRow {
    id: Uuid,
    owner_id: Uuid,
    dispatch_result_id: String,
    delivery_date: NaiveDate,
    vehicle_no: String,
    plate_no: String,
    driver_user_id: Uuid,
    status: String,
    version: i32,
    payload_hash: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct TmsRouteStopRow {
    id: Uuid,
    store_id: Uuid,
    sequence: i32,
    estimated_arrival_at: DateTime<Utc>,
    outbound_order_ids: Vec<Uuid>,
}

#[derive(FromRow)]
struct TransitTemperatureReadingRow {
    id: Uuid,
    owner_id: Uuid,
    dispatch_id: Uuid,
    device_code: String,
    plate_no: String,
    measured_at: DateTime<Utc>,
    temperature_celsius: f64,
    humidity_percent: Option<f64>,
    is_exceeded: bool,
    external_trace_url: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct ContainerRecoveryRow {
    id: Uuid,
    owner_id: Uuid,
    container_lpn: String,
    dispatch_id: Option<Uuid>,
    customer_id: Uuid,
    delivery_provider_type: String,
    status: String,
    shipped_at: DateTime<Utc>,
    recovered_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

include!("wave5_repository_part1.rs");
include!("wave5_repository_part2.rs");

#[allow(clippy::too_many_arguments)]
async fn finish_mutation<T: Serialize>(
    mut tx: Transaction<'_, Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    idempotency_resource_type: &str,
    resource_id: Uuid,
    response: &T,
    audit: Option<AuditWriteRequest>,
    action: &str,
    module: &str,
    audit_resource_type: &str,
    now: DateTime<Utc>,
) -> Result<(), Wave5RepositoryError> {
    store_idempotency_success(
        &mut tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        idempotency_resource_type,
        resource_id.to_string(),
        response,
        now,
    )
    .await?;
    append_wave5_audit(
        &mut tx,
        ctx,
        audit,
        action,
        module,
        audit_resource_type,
        resource_id,
        now,
    )
    .await?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(())
}

async fn append_wave5_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    audit: Option<AuditWriteRequest>,
    action: &str,
    module: &str,
    resource_type: &str,
    resource_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), Wave5RepositoryError> {
    let mut audit = audit.unwrap_or_else(|| {
        AuditWriteRequest::from_auth_context(
            ctx,
            action,
            module,
            resource_type,
            resource_id.to_string(),
            None,
        )
    });
    audit.action = action.to_string();
    audit.module = module.to_string();
    audit.resource_type = resource_type.to_string();
    audit.resource_id = resource_id.to_string();
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| Wave5RepositoryError::Audit(format!("{error:?}")))
}

async fn ensure_outbound_order(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
) -> Result<(), Wave5RepositoryError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM outbound_orders WHERE owner_id = $1 AND id = $2)",
    )
    .bind(owner_id)
    .bind(order_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(Wave5RepositoryError::NotFound)
    }
}

async fn ensure_route_orders(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_ids: &[Uuid],
) -> Result<(), Wave5RepositoryError> {
    let matched: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM outbound_orders WHERE owner_id = $1 AND id = ANY($2)",
    )
    .bind(owner_id)
    .bind(order_ids)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if usize::try_from(matched).ok() == Some(order_ids.len()) {
        Ok(())
    } else {
        Err(Wave5RepositoryError::NotFound)
    }
}

async fn ensure_route_driver(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    driver_user_id: Uuid,
) -> Result<(), Wave5RepositoryError> {
    let active: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1
              FROM auth_user_owner_bindings binding
              JOIN auth_users user_account ON user_account.id = binding.user_id
             WHERE binding.owner_id = $1
               AND binding.user_id = $2
               AND binding.is_active = TRUE
               AND user_account.status = 'active'
        )
        "#,
    )
    .bind(owner_id)
    .bind(driver_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if active {
        Ok(())
    } else {
        Err(Wave5RepositoryError::NotFound)
    }
}

async fn load_tms_route_plan(
    tx: &mut Transaction<'_, Postgres>,
    row: TmsRoutePlanRow,
) -> Result<TmsRoutePlan, Wave5RepositoryError> {
    let stop_rows = sqlx::query_as::<_, TmsRouteStopRow>(
        r#"
        SELECT stop.id, stop.store_id, stop.stop_sequence AS sequence,
               stop.estimated_arrival_at,
               COALESCE(
                   array_agg(route_order.outbound_order_id ORDER BY route_order.created_at)
                       FILTER (WHERE route_order.outbound_order_id IS NOT NULL),
                   ARRAY[]::UUID[]
               ) AS outbound_order_ids
          FROM tms_route_stops stop
          LEFT JOIN tms_route_orders route_order
            ON route_order.owner_id = stop.owner_id
           AND route_order.route_stop_id = stop.id
         WHERE stop.owner_id = $1 AND stop.route_plan_id = $2
         GROUP BY stop.id
         ORDER BY stop.stop_sequence
        "#,
    )
    .bind(row.owner_id)
    .bind(row.id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let stops = stop_rows
        .into_iter()
        .map(|stop| TmsRouteStop {
            id: stop.id,
            store_id: stop.store_id,
            sequence: stop.sequence,
            estimated_arrival_at: stop.estimated_arrival_at,
            outbound_order_ids: stop.outbound_order_ids,
        })
        .collect();
    Ok(map_tms_route_plan(row, stops))
}

async fn ensure_packing_station(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    station_id: Uuid,
) -> Result<(), Wave5RepositoryError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM packing_stations WHERE owner_id = $1 AND id = $2)",
    )
    .bind(owner_id)
    .bind(station_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(Wave5RepositoryError::NotFound)
    }
}

async fn ensure_dispatch(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    dispatch_id: Uuid,
) -> Result<(), Wave5RepositoryError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tms_dispatches WHERE owner_id = $1 AND id = $2)",
    )
    .bind(owner_id)
    .bind(dispatch_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(Wave5RepositoryError::NotFound)
    }
}

async fn load_statement_charge_ids(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    statement_id: Uuid,
) -> Result<Vec<Uuid>, Wave5RepositoryError> {
    sqlx::query_scalar(
        r#"
        SELECT charge_id
          FROM billing_statement_charges
         WHERE owner_id = $1 AND statement_id = $2
         ORDER BY created_at ASC
        "#,
    )
    .bind(owner_id)
    .bind(statement_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, Wave5RepositoryError> {
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
        return Err(Wave5RepositoryError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| Wave5RepositoryError::Serialize(error.to_string()))
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), Wave5RepositoryError> {
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
) -> Result<(), Wave5RepositoryError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| Wave5RepositoryError::Serialize(error.to_string()))?;
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

fn request_hash(value: &serde_json::Value) -> Result<String, Wave5RepositoryError> {
    let text = serde_json::to_string(value)
        .map_err(|error| Wave5RepositoryError::Serialize(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn parse_billing_date(value: &str) -> Result<NaiveDate, Wave5RepositoryError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| Wave5RepositoryError::InvalidInput)
}

fn has_duplicate_uuids(values: &[Uuid]) -> bool {
    let mut seen = std::collections::HashSet::with_capacity(values.len());
    values.iter().any(|value| !seen.insert(value))
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

fn map_packing_station(row: PackingStationRow) -> PackingStation {
    PackingStation {
        id: row.id,
        owner_id: row.owner_id,
        station_code: row.station_code,
        station_name: row.station_name,
        printer_code: row.printer_code,
        scale_code: row.scale_code,
        temperature_zone: row.temperature_zone,
        status: row.status,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_pack_job(row: PackJobRow) -> PackJob {
    PackJob {
        id: row.id,
        owner_id: row.owner_id,
        outbound_order_id: row.outbound_order_id,
        station_id: row.station_id,
        job_no: row.job_no,
        pack_mode: row.pack_mode,
        recommended_box_type: row.recommended_box_type,
        actual_box_type: row.actual_box_type,
        adjustment_reason: row.adjustment_reason,
        outbound_lpn: row.outbound_lpn,
        trace_codes: row.trace_codes,
        status: row.status,
        weight_grams: row.weight_grams,
        waybill_no: row.waybill_no,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_replenishment(row: RetailReplenishmentSuggestionRow) -> RetailReplenishmentSuggestion {
    RetailReplenishmentSuggestion {
        id: row.id,
        owner_id: row.owner_id,
        store_id: row.store_id,
        product_code: row.product_code,
        period_key: row.period_key,
        min_qty: row.min_qty,
        max_qty: row.max_qty,
        current_qty: row.current_qty,
        in_transit_qty: row.in_transit_qty,
        daily_sales_avg: row.daily_sales_avg,
        suggested_qty: row.suggested_qty,
        status: row.status,
        created_at: row.created_at,
    }
}

fn map_crossdock_plan(row: CrossdockPlanRow) -> CrossdockPlan {
    CrossdockPlan {
        id: row.id,
        owner_id: row.owner_id,
        asn_id: row.asn_id,
        outbound_order_id: row.outbound_order_id,
        store_id: row.store_id,
        product_code: row.product_code,
        qty: row.qty,
        status: row.status,
        created_at: row.created_at,
    }
}

fn map_charge(row: BillingChargeCalculationRow) -> BillingChargeCalculation {
    BillingChargeCalculation {
        id: row.id,
        owner_id: row.owner_id,
        contract_id: row.contract_id,
        period_start: row.period_start.to_string(),
        period_end: row.period_end.to_string(),
        charge_item: row.charge_item,
        quantity: row.quantity,
        amount_cents: row.amount_cents,
        source_refs: row.source_refs,
        status: row.status,
        created_at: row.created_at,
    }
}

fn map_statement(row: BillingStatementRow, charge_ids: Vec<Uuid>) -> BillingStatement {
    BillingStatement {
        id: row.id,
        owner_id: row.owner_id,
        contract_id: row.contract_id,
        period_start: row.period_start.to_string(),
        period_end: row.period_end.to_string(),
        status: row.status,
        total_amount_cents: row.total_amount_cents,
        charge_ids,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_tms_dispatch(row: TmsDispatchRow) -> TmsDispatch {
    TmsDispatch {
        id: row.id,
        owner_id: row.owner_id,
        dispatch_no: row.dispatch_no,
        outbound_order_id: row.outbound_order_id,
        delivery_provider_type: row.delivery_provider_type,
        vehicle_no: row.vehicle_no,
        plate_no: row.plate_no,
        driver_user_id: row.driver_user_id,
        carrier_code: row.carrier_code,
        waybill_no: row.waybill_no,
        status: row.status,
        version: row.version,
        scheduled_load_at: row.scheduled_load_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_tms_route_plan(row: TmsRoutePlanRow, stops: Vec<TmsRouteStop>) -> TmsRoutePlan {
    let outbound_order_ids = stops
        .iter()
        .flat_map(|stop| stop.outbound_order_ids.iter().copied())
        .collect();
    TmsRoutePlan {
        id: row.id,
        owner_id: row.owner_id,
        dispatch_result_id: row.dispatch_result_id,
        delivery_date: row.delivery_date,
        vehicle_no: row.vehicle_no,
        plate_no: row.plate_no,
        driver_user_id: row.driver_user_id,
        status: row.status,
        version: row.version,
        outbound_order_ids,
        stops,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_transit_temperature(row: TransitTemperatureReadingRow) -> TransitTemperatureReading {
    TransitTemperatureReading {
        id: row.id,
        owner_id: row.owner_id,
        dispatch_id: row.dispatch_id,
        device_code: row.device_code,
        plate_no: row.plate_no,
        measured_at: row.measured_at,
        temperature_celsius: row.temperature_celsius,
        humidity_percent: row.humidity_percent,
        is_exceeded: row.is_exceeded,
        external_trace_url: row.external_trace_url,
        created_at: row.created_at,
    }
}

fn map_container_recovery(row: ContainerRecoveryRow) -> ContainerRecovery {
    ContainerRecovery {
        id: row.id,
        owner_id: row.owner_id,
        container_lpn: row.container_lpn,
        dispatch_id: row.dispatch_id,
        customer_id: row.customer_id,
        delivery_provider_type: row.delivery_provider_type,
        status: row.status,
        shipped_at: row.shipped_at,
        recovered_at: row.recovered_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_db_error(error: sqlx::Error) -> Wave5RepositoryError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return Wave5RepositoryError::DuplicateCode;
        }
    }
    Wave5RepositoryError::Database(error.to_string())
}
