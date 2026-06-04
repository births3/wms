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
    tms_plus::{TmsPlusError, TmsPlusService},
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
    period_start: String,
    period_end: String,
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
    period_start: String,
    period_end: String,
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

impl PgWave5Repository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            packing: PackingStationService,
            retail: RetailChainService,
            tms: TmsPlusService,
        }
    }

    pub async fn create_packing_station(
        &self,
        ctx: &AuthContext,
        req: CreatePackingStationRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PackingStation>, Wave5RepositoryError> {
        self.packing.validate_station(&req)?;
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

        let id = Uuid::new_v4();
        let station = map_packing_station(
            sqlx::query_as::<_, PackingStationRow>(
                r#"
            INSERT INTO packing_stations (
                id, owner_id, station_code, station_name, printer_code, scale_code,
                temperature_zone, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'idle', $8, $8)
            RETURNING id, owner_id, station_code, station_name, printer_code, scale_code,
                      temperature_zone, status, created_at, updated_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(&req.station_code)
            .bind(&req.station_name)
            .bind(&req.printer_code)
            .bind(&req.scale_code)
            .bind(&req.temperature_zone)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/packing/stations",
            "packing_station",
            station.id,
            &station,
            audit,
            "create_packing_station",
            "M-PK",
            "packing_station",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: station,
            replayed: false,
        })
    }

    pub async fn create_pack_job(
        &self,
        ctx: &AuthContext,
        req: CreatePackJobRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PackJob>, Wave5RepositoryError> {
        self.packing.validate_pack_job(&req)?;
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
        ensure_outbound_order(&mut tx, ctx.owner_id, req.outbound_order_id).await?;
        if let Some(station_id) = req.station_id {
            ensure_packing_station(&mut tx, ctx.owner_id, station_id).await?;
        }

        let id = Uuid::new_v4();
        let job = map_pack_job(
            sqlx::query_as::<_, PackJobRow>(
                r#"
            INSERT INTO packing_jobs (
                id, owner_id, outbound_order_id, station_id, job_no, pack_mode,
                recommended_box_type, actual_box_type, adjustment_reason,
                outbound_lpn, trace_codes, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'packed', $12, $12)
            RETURNING id, owner_id, outbound_order_id, station_id, job_no, pack_mode,
                      recommended_box_type, actual_box_type, adjustment_reason,
                      outbound_lpn, trace_codes, status, weight_grams, waybill_no,
                      created_at, updated_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(req.outbound_order_id)
            .bind(req.station_id)
            .bind(&req.job_no)
            .bind(&req.pack_mode)
            .bind(&req.recommended_box_type)
            .bind(&req.actual_box_type)
            .bind(&req.adjustment_reason)
            .bind(&req.outbound_lpn)
            .bind(&req.trace_codes)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/packing/jobs",
            "packing_job",
            job.id,
            &job,
            audit,
            "create_pack_job",
            "M-PK",
            "packing_job",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: job,
            replayed: false,
        })
    }

    pub async fn weigh_pack_job(
        &self,
        ctx: &AuthContext,
        job_id: Uuid,
        req: WeighPackJobRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PackJob>, Wave5RepositoryError> {
        self.packing.validate_weight(&req)?;
        let request_hash = request_hash(&serde_json::json!({ "job_id": job_id, "request": req }))?;
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
        let job = map_pack_job(
            sqlx::query_as::<_, PackJobRow>(
                r#"
            UPDATE packing_jobs
               SET weight_grams = $3,
                   status = 'weighed',
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            RETURNING id, owner_id, outbound_order_id, station_id, job_no, pack_mode,
                      recommended_box_type, actual_box_type, adjustment_reason,
                      outbound_lpn, trace_codes, status, weight_grams, waybill_no,
                      created_at, updated_at
            "#,
            )
            .bind(ctx.owner_id)
            .bind(job_id)
            .bind(req.actual_weight_grams)
            .bind(now)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .ok_or(Wave5RepositoryError::NotFound)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/packing/jobs/{id}/weigh",
            "packing_job",
            job.id,
            &job,
            audit,
            "weigh_pack_job",
            "M-PK",
            "packing_job",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: job,
            replayed: false,
        })
    }

    pub async fn print_pack_job_waybill(
        &self,
        ctx: &AuthContext,
        job_id: Uuid,
        req: PrintWaybillRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PackJob>, Wave5RepositoryError> {
        self.packing.validate_waybill(&req)?;
        let request_hash = request_hash(&serde_json::json!({ "job_id": job_id, "request": req }))?;
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
        let waybill_no = req
            .waybill_no
            .unwrap_or_else(|| format!("{}-{}", req.carrier_code, job_id.simple()));
        let job = map_pack_job(
            sqlx::query_as::<_, PackJobRow>(
                r#"
            UPDATE packing_jobs
               SET waybill_no = $3,
                   status = 'waybill_printed',
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            RETURNING id, owner_id, outbound_order_id, station_id, job_no, pack_mode,
                      recommended_box_type, actual_box_type, adjustment_reason,
                      outbound_lpn, trace_codes, status, weight_grams, waybill_no,
                      created_at, updated_at
            "#,
            )
            .bind(ctx.owner_id)
            .bind(job_id)
            .bind(&waybill_no)
            .bind(now)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .ok_or(Wave5RepositoryError::NotFound)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/packing/jobs/{id}/waybill",
            "packing_job",
            job.id,
            &job,
            audit,
            "print_pack_job_waybill",
            "M-PK",
            "packing_job",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: job,
            replayed: false,
        })
    }

    pub async fn create_replenishment_suggestion(
        &self,
        ctx: &AuthContext,
        req: CreateRetailReplenishmentSuggestionRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<RetailReplenishmentSuggestion>, Wave5RepositoryError> {
        let suggested_qty = self.retail.suggested_qty(&req)?;
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
        let id = Uuid::new_v4();
        let suggestion = map_replenishment(sqlx::query_as::<_, RetailReplenishmentSuggestionRow>(
            r#"
            INSERT INTO retail_replenishment_suggestions (
                id, owner_id, store_id, product_code, period_key, min_qty, max_qty,
                current_qty, in_transit_qty, daily_sales_avg, suggested_qty, status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'pending_approval', $12)
            RETURNING id, owner_id, store_id, product_code, period_key, min_qty, max_qty,
                      current_qty, in_transit_qty, daily_sales_avg, suggested_qty, status, created_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(req.store_id)
        .bind(&req.product_code)
        .bind(&req.period_key)
        .bind(req.min_qty)
        .bind(req.max_qty)
        .bind(req.current_qty)
        .bind(req.in_transit_qty)
        .bind(req.daily_sales_avg)
        .bind(suggested_qty)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?);
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/retail/replenishment-suggestions",
            "retail_replenishment_suggestion",
            suggestion.id,
            &suggestion,
            audit,
            "create_replenishment_suggestion",
            "M8",
            "retail_replenishment_suggestion",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: suggestion,
            replayed: false,
        })
    }

    pub async fn create_crossdock_plan(
        &self,
        ctx: &AuthContext,
        req: CreateCrossdockPlanRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<CrossdockPlan>, Wave5RepositoryError> {
        self.retail.validate_crossdock(&req)?;
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
        ensure_outbound_order(&mut tx, ctx.owner_id, req.outbound_order_id).await?;
        let id = Uuid::new_v4();
        let plan = map_crossdock_plan(
            sqlx::query_as::<_, CrossdockPlanRow>(
                r#"
            INSERT INTO crossdock_plans (
                id, owner_id, asn_id, outbound_order_id, store_id, product_code,
                qty, status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'planned', $8)
            RETURNING id, owner_id, asn_id, outbound_order_id, store_id, product_code,
                      qty, status, created_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(req.asn_id)
            .bind(req.outbound_order_id)
            .bind(req.store_id)
            .bind(&req.product_code)
            .bind(req.qty)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/retail/crossdock-plans",
            "crossdock_plan",
            plan.id,
            &plan,
            audit,
            "create_crossdock_plan",
            "M8",
            "crossdock_plan",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: plan,
            replayed: false,
        })
    }

    pub async fn calculate_period_charges(
        &self,
        ctx: &AuthContext,
        req: wms_domain::CalculateBillingChargesRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<BillingChargeCalculation>, Wave5RepositoryError> {
        if req.quantity < 0 || req.period_start.is_empty() || req.period_end.is_empty() {
            return Err(Wave5RepositoryError::InvalidInput);
        }
        let period_start = parse_billing_date(&req.period_start)?;
        let period_end = parse_billing_date(&req.period_end)?;
        if period_end < period_start {
            return Err(Wave5RepositoryError::InvalidInput);
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
        let unit_price: i64 = sqlx::query_scalar(
            r#"
            SELECT unit_price_cents
              FROM billing_rules
             WHERE owner_id = $1
               AND contract_id = $2
               AND charge_item = $3
               AND effective_from <= $4
               AND effective_to >= $5
             ORDER BY created_at DESC
             LIMIT 1
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.contract_id)
        .bind(&req.charge_item)
        .bind(period_end)
        .bind(period_start)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave5RepositoryError::NotFound)?;
        let amount_cents = unit_price
            .checked_mul(req.quantity)
            .ok_or(Wave5RepositoryError::InvalidInput)?;
        let id = Uuid::new_v4();
        let charge = map_charge(
            sqlx::query_as::<_, BillingChargeCalculationRow>(
                r#"
            INSERT INTO billing_charge_calculations (
                id, owner_id, contract_id, period_start, period_end, charge_item,
                quantity, amount_cents, source_refs, status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'calculated', $10)
            RETURNING id, owner_id, contract_id, period_start, period_end, charge_item,
                      quantity, amount_cents, source_refs, status, created_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(req.contract_id)
            .bind(period_start.to_string())
            .bind(period_end.to_string())
            .bind(&req.charge_item)
            .bind(req.quantity)
            .bind(amount_cents)
            .bind(&req.source_refs)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/billing/charges/calculate",
            "billing_charge_calculation",
            charge.id,
            &charge,
            audit,
            "calculate_period_charges",
            "M9",
            "billing_charge_calculation",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: charge,
            replayed: false,
        })
    }

    pub async fn generate_billing_statement(
        &self,
        ctx: &AuthContext,
        req: GenerateBillingStatementRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<BillingStatement>, Wave5RepositoryError> {
        if req.charge_ids.is_empty()
            || has_duplicate_uuids(&req.charge_ids)
            || req.period_start.is_empty()
            || req.period_end.is_empty()
        {
            return Err(Wave5RepositoryError::InvalidInput);
        }
        let period_start = parse_billing_date(&req.period_start)?;
        let period_end = parse_billing_date(&req.period_end)?;
        if period_end < period_start {
            return Err(Wave5RepositoryError::InvalidInput);
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
        let (selected_count, period_count, total): (i64, i64, Option<i64>) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*)::BIGINT,
                COUNT(*) FILTER (WHERE period_start = $4 AND period_end = $5)::BIGINT,
                SUM(amount_cents) FILTER (WHERE period_start = $4 AND period_end = $5)::BIGINT
              FROM billing_charge_calculations
             WHERE owner_id = $1 AND contract_id = $2 AND id = ANY($3)
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.contract_id)
        .bind(&req.charge_ids)
        .bind(period_start.to_string())
        .bind(period_end.to_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if usize::try_from(selected_count).ok() != Some(req.charge_ids.len()) {
            return Err(Wave5RepositoryError::NotFound);
        }
        if usize::try_from(period_count).ok() != Some(req.charge_ids.len()) {
            return Err(Wave5RepositoryError::InvalidInput);
        }
        let id = Uuid::new_v4();
        let statement = map_statement(
            sqlx::query_as::<_, BillingStatementRow>(
                r#"
                INSERT INTO billing_statements (
                    id, owner_id, contract_id, period_start, period_end, status,
                    total_amount_cents, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, 'pending_confirmation', $6, $7, $7)
                RETURNING id, owner_id, contract_id, period_start, period_end, status,
                          total_amount_cents, created_at, updated_at
                "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(req.contract_id)
            .bind(period_start.to_string())
            .bind(period_end.to_string())
            .bind(total.unwrap_or(0))
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
            req.charge_ids.clone(),
        );
        for charge_id in &req.charge_ids {
            sqlx::query(
                r#"
                INSERT INTO billing_statement_charges (id, owner_id, statement_id, charge_id, created_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(statement.id)
            .bind(charge_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/billing/statements",
            "billing_statement",
            statement.id,
            &statement,
            audit,
            "generate_billing_statement",
            "M9",
            "billing_statement",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: statement,
            replayed: false,
        })
    }

    pub async fn confirm_billing_statement(
        &self,
        ctx: &AuthContext,
        statement_id: Uuid,
        req: ConfirmBillingStatementRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<BillingStatement>, Wave5RepositoryError> {
        let request_hash =
            request_hash(&serde_json::json!({ "statement_id": statement_id, "request": req }))?;
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
        let current = sqlx::query_as::<_, BillingStatementRow>(
            r#"
            SELECT id, owner_id, contract_id, period_start, period_end, status,
                   total_amount_cents, created_at, updated_at
              FROM billing_statements
             WHERE owner_id = $1 AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(statement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave5RepositoryError::NotFound)?;
        let charge_ids = load_statement_charge_ids(&mut tx, ctx.owner_id, statement_id).await?;
        if current.status == "confirmed" {
            let statement = map_statement(current, charge_ids);
            store_idempotency_success(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &request_hash,
                "POST",
                "/api/v1/billing/statements/{id}/confirm",
                "billing_statement",
                statement.id.to_string(),
                &statement,
                now,
            )
            .await?;
            tx.commit().await.map_err(map_db_error)?;
            return Ok(IdempotentMutation {
                value: statement,
                replayed: false,
            });
        }
        if current.status != "pending_confirmation" {
            return Err(Wave5RepositoryError::InvalidInput);
        }
        let row = sqlx::query_as::<_, BillingStatementRow>(
            r#"
            UPDATE billing_statements
               SET status = 'confirmed',
                   updated_at = $3,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            RETURNING id, owner_id, contract_id, period_start, period_end, status,
                      total_amount_cents, created_at, updated_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(statement_id)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave5RepositoryError::NotFound)?;
        let statement = map_statement(row, charge_ids);
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/billing/statements/{id}/confirm",
            "billing_statement",
            statement.id,
            &statement,
            audit,
            "confirm_billing_statement",
            "M9",
            "billing_statement",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: statement,
            replayed: false,
        })
    }

    pub async fn receive_tms_dispatch(
        &self,
        ctx: &AuthContext,
        req: ReceiveTmsDispatchRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<TmsDispatch>, Wave5RepositoryError> {
        self.tms.validate_dispatch(&req)?;
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
        ensure_outbound_order(&mut tx, ctx.owner_id, req.outbound_order_id).await?;
        let id = Uuid::new_v4();
        let dispatch = map_tms_dispatch(
            sqlx::query_as::<_, TmsDispatchRow>(
                r#"
            INSERT INTO tms_dispatches (
                id, owner_id, dispatch_no, outbound_order_id, delivery_provider_type,
                vehicle_no, plate_no, driver_user_id, carrier_code, waybill_no,
                status, dispatch_version, scheduled_load_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'received', $11, $12, $13, $13)
            RETURNING id, owner_id, dispatch_no, outbound_order_id, delivery_provider_type,
                      vehicle_no, plate_no, driver_user_id, carrier_code, waybill_no,
                      status, dispatch_version AS version, scheduled_load_at, created_at, updated_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(&req.dispatch_no)
            .bind(req.outbound_order_id)
            .bind(&req.delivery_provider_type)
            .bind(&req.vehicle_no)
            .bind(&req.plate_no)
            .bind(req.driver_user_id)
            .bind(&req.carrier_code)
            .bind(&req.waybill_no)
            .bind(req.version)
            .bind(req.scheduled_load_at)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/tms/dispatches",
            "tms_dispatch",
            dispatch.id,
            &dispatch,
            audit,
            "receive_tms_dispatch",
            "M10",
            "tms_dispatch",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: dispatch,
            replayed: false,
        })
    }

    pub async fn ingest_transit_temperature(
        &self,
        ctx: &AuthContext,
        req: IngestTransitTemperatureRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<TransitTemperatureReading>, Wave5RepositoryError> {
        self.tms.validate_temperature(&req, now)?;
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
        ensure_dispatch(&mut tx, ctx.owner_id, req.dispatch_id).await?;
        let id = Uuid::new_v4();
        let reading = map_transit_temperature(
            sqlx::query_as::<_, TransitTemperatureReadingRow>(
                r#"
            INSERT INTO transit_temperature_readings (
                id, owner_id, dispatch_id, device_code, plate_no, measured_at,
                temperature_celsius, humidity_percent, is_exceeded, external_trace_url, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, owner_id, dispatch_id, device_code, plate_no, measured_at,
                      temperature_celsius, humidity_percent, is_exceeded, external_trace_url,
                      created_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(req.dispatch_id)
            .bind(&req.device_code)
            .bind(&req.plate_no)
            .bind(req.measured_at)
            .bind(req.temperature_celsius)
            .bind(req.humidity_percent)
            .bind(req.is_exceeded)
            .bind(&req.external_trace_url)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/tms/transit-temperature-readings",
            "transit_temperature_reading",
            reading.id,
            &reading,
            audit,
            "ingest_transit_temperature",
            "M10",
            "transit_temperature_reading",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: reading,
            replayed: false,
        })
    }

    pub async fn confirm_container_recovery(
        &self,
        ctx: &AuthContext,
        req: ConfirmContainerRecoveryRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<ContainerRecovery>, Wave5RepositoryError> {
        self.tms.validate_recovery(&req)?;
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
        if let Some(dispatch_id) = req.dispatch_id {
            ensure_dispatch(&mut tx, ctx.owner_id, dispatch_id).await?;
        }
        let shipped_at = req.shipped_at.unwrap_or(now);
        let id = Uuid::new_v4();
        let recovery = map_container_recovery(
            sqlx::query_as::<_, ContainerRecoveryRow>(
                r#"
            INSERT INTO container_recoveries (
                id, owner_id, container_lpn, dispatch_id, customer_id,
                delivery_provider_type, status, shipped_at, recovered_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'recovered', $7, $8, $8, $8)
            RETURNING id, owner_id, container_lpn, dispatch_id, customer_id,
                      delivery_provider_type, status, shipped_at, recovered_at,
                      created_at, updated_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(&req.container_lpn)
            .bind(req.dispatch_id)
            .bind(req.customer_id)
            .bind(&req.delivery_provider_type)
            .bind(shipped_at)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/tms/container-recoveries",
            "container_recovery",
            recovery.id,
            &recovery,
            audit,
            "confirm_container_recovery",
            "M10",
            "container_recovery",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: recovery,
            replayed: false,
        })
    }
}

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
        period_start: row.period_start,
        period_end: row.period_end,
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
        period_start: row.period_start,
        period_end: row.period_end,
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
