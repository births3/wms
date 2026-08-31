//! Wave 3 PostgreSQL repository, aligned with ADR-0034.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    validate_billing_rule_request, validate_create_receiving_order_request, BillingAccount,
    BillingContract, BillingRule, BillingRuleValidationError, CancelReceivingOrderRequest,
    ChangeInventoryStatusRequest, ColdChainDevice, CreateBillingAccountRequest,
    CreateBillingContractRequest, CreateBillingRuleRequest, CreateColdChainDeviceRequest,
    CreateReceivingOrderRequest, ForceCloseShortageRequest, IngestTemperatureExcursionRequest,
    IngestTemperatureReadingRequest, InspectReceivingOrderRequest, InspectionSignatureRecord,
    InventoryBatch, InventoryMovement, PutawayLocationRecommendation, PutawayRecommendationQuery,
    PutawayRecommendationResponse, PutawayRecord, PutawayRequest, PutawayStrategyProfile,
    PutawayStrategyProfileListResponse, ReceiveReceivingOrderRequest, ReceivingDashboardQuery,
    ReceivingDashboardRow, ReceivingInspectionRecord, ReceivingOrder, ReceivingOrderLine,
    ReceivingOrderPrintData, ReceivingOrderReceipt, ReceivingOrderRequestValidationError,
    ReceivingReceiptDetails, RejectReceivingOrderRequest, SignInspectionRequest,
    TemperatureExcursionEvent, TemperatureReading, UpdateColdChainDeviceRequest,
    UpdateReceivingOrderRequest, UpsertPutawayStrategyProfileRequest,
    RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    idempotency,
    inventory::STATUS_QUALIFIED,
    operation_context::OperationContext as AuthContext,
};

mod abc;
mod alerts;
mod billing;
mod cold_chain;
mod erp_outbox;
mod expiry;
mod integrations;
mod inventory_count;
mod inventory_seed;
pub use inventory_seed::{ErpInventorySeedItem, ErpInventorySeedSnapshot};
mod maintenance;
mod mappings;
mod putaway;
mod putaway_validation;
mod query;
mod recall;
mod receiving_read;
mod receiving_update;
mod receiving_validation;
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
    DrugInspectionMissingBlocked,
    DrugInspectionUnqualifiedBlocked,
    InvalidDeviceType,
    ActiveMonitoring,
    DuplicateTraceCode,
    InvalidLocation,
    LocationUnreachable,
    LpnNotFound,
    LpnMixDenied,
    LpnNotUsable,
    InsufficientQuantity,
    LocationQualityMismatch,
    LocationTemperatureMismatch,
    LocationCapacityExceeded,
    LocationSkuLimitExceeded,
    PutawayZoneCategoryDenied,
    PutawayTemperatureMismatch,
    PutawayQualityLocked,
    PutawaySpecialDualRequired,
    PutawayPackGranularityInvalid,
    PutawayExternalFragrantConflict,
    PutawayCapacityExceeded,
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
    MissingRequiredField(String),
    TemperatureExcursionRequiresDisposition,
    PendingErpCancel,
    SupplierQualificationExpired,
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

impl From<crate::idempotency::IdempotencyError> for Wave3RepositoryError {
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

pub(crate) fn validated_pda_operated_at(
    requested: Option<DateTime<Utc>>,
    server_now: DateTime<Utc>,
) -> Result<DateTime<Utc>, Wave3RepositoryError> {
    let operated_at = requested.unwrap_or(server_now);
    if operated_at > server_now + chrono::Duration::minutes(5) {
        return Err(Wave3RepositoryError::FutureTimestamp);
    }
    if operated_at < server_now - chrono::Duration::hours(24) {
        return Err(Wave3RepositoryError::InvalidDate(
            "operated_at exceeds the 24-hour PDA offline window".to_string(),
        ));
    }
    Ok(operated_at)
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
    expected_qty: wms_domain::Quantity,
    batch_no: Option<String>,
    production_date: Option<NaiveDate>,
    expiry_date: Option<NaiveDate>,
}

#[derive(FromRow)]
struct ReceivingOrderReceiptRow {
    id: Uuid,
    receiving_order_id: Uuid,
    owner_id: Uuid,
    actual_qty: wms_domain::Quantity,
    shortage_qty: wms_domain::Quantity,
    rejected_qty: wms_domain::Quantity,
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
    accepted_qty: wms_domain::Quantity,
    rejected_qty: wms_domain::Quantity,
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
    qty_on_hand: wms_domain::Quantity,
    qty_frozen: wms_domain::Quantity,
    status: String,
    location_id: Uuid,
    location_code: String,
    #[sqlx(default)]
    qty_allocated: wms_domain::Quantity,
    #[sqlx(default)]
    qty_replenish_in_transit: wms_domain::Quantity,
    #[sqlx(default)]
    qty_replenish_out_transit: wms_domain::Quantity,
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
include!("wave3_repository_part1b.rs");
include!("wave3_repository_quality.rs");
include!("wave3_repository_part2.rs");
include!("wave3_repository_part2b.rs");
include!("wave3_repository_part3.rs");
include!("wave3_repository_drug_inspection.rs");

include!("wave3_repository_helpers.rs");
