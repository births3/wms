use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;
use wms_domain::{
    BillingAccount, BillingContract, BillingRule, CancelInventoryRecallRequest,
    ChangeInventoryStatusRequest, ColdChainDevice, CreateBillingAccountRequest,
    CreateBillingContractRequest, CreateBillingRuleRequest, CreateColdChainDeviceRequest,
    CreateReceivingOrderRequest, ErrorResponse, ExpireInventoryBatchesRequest,
    IngestTemperatureExcursionRequest, IngestTemperatureReadingRequest,
    InspectReceivingOrderRequest, InspectionSignatureRecord, InventoryBatch,
    InventoryBatchListResponse, InventoryBatchQuery, InventoryBatchTrace, LocationHistoryQuery,
    LocationHistoryResponse, MarkInventoryRecallRequest, PageMeta, PutawayInventoryRequest,
    PutawayRecord, PutawayRequest,
    ReceiveReceivingOrderRequest, ReceivingDashboardQuery, ReceivingDashboardResponse,
    ReceivingInspectionRecord, ReceivingOrder, ReceivingOrderListResponse, ReceivingOrderPrintData,
    ReceivingOrderReceipt, RejectReceivingOrderRequest, TemperatureExcursionEvent,
    TemperatureReading, UpdateColdChainDeviceRequest, UpdateReceivingOrderRequest,
};

use crate::{
    audit::{AuditDiff, AuditLog, AuditWriteRequest},
    auth::{AuthContext, AuthError},
    billing::{BillingError, BillingStore},
    cold_chain::{ColdChainError, ColdChainService},
    config_center::{
        ConfigCenterAppState, ConfigCenterError, CONFIG_FLAG_DISABLED_CODE,
        CONFIG_FLAG_MISSING_CODE, CONFIG_FLAG_SOURCE_INVALID_CODE,
    },
    inbound::{ReceivingOrderError, ReceivingOrderStore},
    inventory::{InventoryError, InventoryStore},
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};

#[path = "wave3_handlers/receiving_handlers.rs"]
mod receiving_handlers;
use receiving_handlers::apply_receiving_order_routes;
#[cfg(test)]
use receiving_handlers::{receive_receiving_order_handler, update_receiving_order_handler};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const EXTERNAL_API_KEY_HEADER: &str = "x-wms-api-key";
const COLD_CHAIN_API_KEY_SHA256_ENV: &str = "WMS_M5_COLD_CHAIN_API_KEY_SHA256";
const COLD_CHAIN_OWNER_ID_ENV: &str = "WMS_M5_COLD_CHAIN_OWNER_ID";
pub const INVENTORY_BATCHES_SMOKE_FLAG: &str = "m3_inventory_batches_config_center_smoke";

#[derive(Clone, Debug)]
pub struct ExternalApiKeyConfig {
    pub key_sha256: String,
    pub owner_id: Uuid,
    pub actor_name: String,
}

impl ExternalApiKeyConfig {
    pub fn from_env() -> Result<Self, Wave3HandlerError> {
        let key_sha256 = std::env::var(COLD_CHAIN_API_KEY_SHA256_ENV)
            .map_err(|_| Wave3HandlerError::ExternalAuthConfigMissing)?;
        let owner_id = std::env::var(COLD_CHAIN_OWNER_ID_ENV)
            .map_err(|_| Wave3HandlerError::ExternalAuthConfigMissing)?
            .parse()
            .map_err(|_| Wave3HandlerError::ExternalAuthConfigInvalid)?;
        Ok(Self {
            key_sha256,
            owner_id,
            actor_name: "external-cold-chain".to_string(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Wave3AppState {
    pub inbound_store: Arc<Mutex<ReceivingOrderStore>>,
    pub inventory_store: Arc<Mutex<InventoryStore>>,
    pub cold_chain_service: Arc<Mutex<ColdChainService>>,
    pub billing_store: Arc<Mutex<BillingStore>>,
    pub audit_log: Arc<Mutex<AuditLog>>,
    pub wave3_repository: Option<Arc<PgWave3Repository>>,
    pub cold_chain_api_key: Option<ExternalApiKeyConfig>,
    pub config_center_state: Option<ConfigCenterAppState>,
}

impl Default for Wave3AppState {
    fn default() -> Self {
        Self {
            inbound_store: Arc::new(Mutex::new(ReceivingOrderStore::default())),
            inventory_store: Arc::new(Mutex::new(InventoryStore::default())),
            cold_chain_service: Arc::new(Mutex::new(ColdChainService::default())),
            billing_store: Arc::new(Mutex::new(BillingStore::default())),
            audit_log: Arc::new(Mutex::new(AuditLog::default())),
            wave3_repository: None,
            cold_chain_api_key: None,
            config_center_state: None,
        }
    }
}

impl Wave3AppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            wave3_repository: Some(Arc::new(PgWave3Repository::new(pool))),
            cold_chain_api_key: ExternalApiKeyConfig::from_env().ok(),
            ..Self::default()
        }
    }

    pub fn with_config_center(mut self, config_center_state: ConfigCenterAppState) -> Self {
        self.config_center_state = Some(config_center_state);
        self
    }

    pub fn with_postgres_and_cold_chain_api_key(
        pool: PgPool,
        cold_chain_api_key: ExternalApiKeyConfig,
    ) -> Self {
        Self {
            wave3_repository: Some(Arc::new(PgWave3Repository::new(pool))),
            cold_chain_api_key: Some(cold_chain_api_key),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Wave3HandlerError {
    Auth(AuthError),
    Receiving(ReceivingOrderError),
    Inventory(InventoryError),
    ColdChain(ColdChainError),
    Billing(BillingError),
    ConfigCenter(ConfigCenterError),
    Repository(Wave3RepositoryError),
    MissingIdempotencyKey,
    ExternalAuthMissing,
    ExternalAuthInvalid,
    ExternalAuthConfigMissing,
    ExternalAuthConfigInvalid,
}

impl From<AuthError> for Wave3HandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<ReceivingOrderError> for Wave3HandlerError {
    fn from(value: ReceivingOrderError) -> Self {
        Self::Receiving(value)
    }
}

impl From<InventoryError> for Wave3HandlerError {
    fn from(value: InventoryError) -> Self {
        Self::Inventory(value)
    }
}

impl From<ColdChainError> for Wave3HandlerError {
    fn from(value: ColdChainError) -> Self {
        Self::ColdChain(value)
    }
}

impl From<BillingError> for Wave3HandlerError {
    fn from(value: BillingError) -> Self {
        Self::Billing(value)
    }
}

impl From<ConfigCenterError> for Wave3HandlerError {
    fn from(value: ConfigCenterError) -> Self {
        Self::ConfigCenter(value)
    }
}

impl From<Wave3RepositoryError> for Wave3HandlerError {
    fn from(value: Wave3RepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl IntoResponse for Wave3HandlerError {
    fn into_response(self) -> Response {
        if let Wave3HandlerError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            Wave3HandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "W3-IDEMPOTENCY-REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Wave3HandlerError::ExternalAuthMissing => (
                StatusCode::UNAUTHORIZED,
                "HINT-AUTH-MISSING",
                "缺少外部系统 API Key",
            ),
            Wave3HandlerError::ExternalAuthInvalid => (
                StatusCode::UNAUTHORIZED,
                "HINT-AUTH-INVALID",
                "外部系统 API Key 无效",
            ),
            Wave3HandlerError::ExternalAuthConfigMissing
            | Wave3HandlerError::ExternalAuthConfigInvalid => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "HINT-AUTH-CONFIG",
                "外部系统凭证配置不可用",
            ),
            Wave3HandlerError::Receiving(ReceivingOrderError::NotFound)
            | Wave3HandlerError::Inventory(InventoryError::NotFound)
            | Wave3HandlerError::ColdChain(ColdChainError::DeviceNotFound(_))
            | Wave3HandlerError::Billing(BillingError::NotFound)
            | Wave3HandlerError::Repository(Wave3RepositoryError::NotFound)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InventoryCountLineNotFound) => {
                (StatusCode::NOT_FOUND, "W3-404", "资源不存在")
            }
            Wave3HandlerError::ConfigCenter(ConfigCenterError::MissingFlag(_)) => (
                StatusCode::NOT_FOUND,
                CONFIG_FLAG_MISSING_CODE,
                "Feature Flag 不存在",
            ),
            Wave3HandlerError::ConfigCenter(ConfigCenterError::DisabledFlag(_)) => (
                StatusCode::NOT_FOUND,
                CONFIG_FLAG_DISABLED_CODE,
                "Feature Flag 未启用",
            ),
            Wave3HandlerError::Receiving(ReceivingOrderError::DuplicateReceiptNo(_))
            | Wave3HandlerError::ColdChain(ColdChainError::DuplicateDevice(_))
            | Wave3HandlerError::Billing(BillingError::DuplicateAccountCode(_))
            | Wave3HandlerError::Billing(BillingError::DuplicateContractNo(_))
            | Wave3HandlerError::Billing(BillingError::BillingRuleConflict)
            | Wave3HandlerError::Repository(Wave3RepositoryError::DuplicateReceipt)
            | Wave3HandlerError::Repository(Wave3RepositoryError::DuplicateCode)
            | Wave3HandlerError::Repository(Wave3RepositoryError::RecallAlreadyActive)
            | Wave3HandlerError::Repository(Wave3RepositoryError::RecallStateChanged)
            | Wave3HandlerError::Repository(Wave3RepositoryError::IdempotencyConflict)
            | Wave3HandlerError::Repository(Wave3RepositoryError::BillingRuleConflict)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InventoryCountAlreadyActive)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InventoryCountLineAlreadySubmitted)
            | Wave3HandlerError::Repository(Wave3RepositoryError::PendingErpCancel) => {
                (StatusCode::CONFLICT, "W3-409", "资源重复")
            }
            Wave3HandlerError::Repository(Wave3RepositoryError::LpnMixDenied) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_LPN_MIX_DENIED",
                "该容器类型不允许混批或混品上架",
            ),
            Wave3HandlerError::Repository(Wave3RepositoryError::PutawayZoneCategoryDenied) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_PUTAWAY_ZONE_CATEGORY_DENIED",
                "商品品类与目标库区准入大区不匹配（6 维①）",
            ),
            Wave3HandlerError::Repository(Wave3RepositoryError::PutawayTemperatureMismatch) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_PUTAWAY_TEMPERATURE_MISMATCH",
                "目标库位温区不满足商品存储温区要求（6 维②）",
            ),
            Wave3HandlerError::Repository(Wave3RepositoryError::PutawayQualityLocked) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_PUTAWAY_QUALITY_LOCKED",
                "容器质量锁/批次状态非合格，禁止上架合格位（6 维③）",
            ),
            Wave3HandlerError::Repository(Wave3RepositoryError::PutawaySpecialDualRequired) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_PUTAWAY_SPECIAL_DUAL_REQUIRED",
                "特药上架需要双人核验（6 维④）",
            ),
            Wave3HandlerError::Repository(Wave3RepositoryError::PutawayPackGranularityInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_PUTAWAY_PACK_GRANULARITY_INVALID",
                "包装粒度与目标位作业形态不符（6 维⑤）",
            ),
            Wave3HandlerError::Repository(Wave3RepositoryError::PutawayExternalFragrantConflict) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_PUTAWAY_EXTERNAL_FRAGRANT_CONFLICT",
                "外用/易串味商品与目标库区互斥（6 维⑥）",
            ),
            Wave3HandlerError::Repository(Wave3RepositoryError::PutawayCapacityExceeded) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_PUTAWAY_CAPACITY_EXCEEDED",
                "目标库位剩余容量不足（6 维⑥）",
            ),
            Wave3HandlerError::Repository(Wave3RepositoryError::LpnNotFound) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_LPN_NOT_FOUND",
                "上架填写的 LPN 在容器主档中不存在",
            ),
            Wave3HandlerError::Repository(Wave3RepositoryError::LpnNotUsable) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_LPN_BINDNG_FAILED",
                "LPN 当前状态或库位不可用于上架绑定",
            ),
            Wave3HandlerError::Receiving(ReceivingOrderError::UnauthorizedSigner)
            | Wave3HandlerError::Repository(Wave3RepositoryError::UnauthorizedSigner) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_VERIFIER_UNAUTHORIZED",
                "签字人不是当前货主的有效验收岗用户",
            ),
            Wave3HandlerError::Receiving(ReceivingOrderError::MissingSecondSigner)
            | Wave3HandlerError::Repository(Wave3RepositoryError::MissingSecondSigner) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_DUAL_PERSON_REQUIRED",
                "M-VR 策略要求第二验收签字人",
            ),
            Wave3HandlerError::Receiving(ReceivingOrderError::SameSigner)
            | Wave3HandlerError::Repository(Wave3RepositoryError::SameSigner) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_DUAL_PERSON_SAME_USER",
                "双人验收的两名签字人不能相同",
            ),
            Wave3HandlerError::Repository(Wave3RepositoryError::DualPersonApprovalRequired) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M2_DUAL_PERSON_APPROVAL_REQUIRED",
                "M-VR 策略要求先完成主管审批",
            ),
            Wave3HandlerError::Repository(
                Wave3RepositoryError::DrugInspectionMissingBlocked,
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_DI_REPORT_REQUIRED",
                "当前批号缺少已确认药检单，货主规则已阻塞验收",
            ),
            Wave3HandlerError::Repository(
                Wave3RepositoryError::DrugInspectionUnqualifiedBlocked,
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_DI_REPORT_UNQUALIFIED",
                "当前批号药检结论不合格，已创建质量联系单并阻塞验收",
            ),
            Wave3HandlerError::Receiving(ReceivingOrderError::EmptyLines)
            | Wave3HandlerError::Receiving(ReceivingOrderError::InvalidStatus { .. })
            | Wave3HandlerError::Receiving(ReceivingOrderError::QuantityClosureMismatch)
            | Wave3HandlerError::Receiving(ReceivingOrderError::OverReceiptNotAllowed)
            | Wave3HandlerError::Receiving(ReceivingOrderError::InvalidQuantity)
            | Wave3HandlerError::Receiving(ReceivingOrderError::MissingSupplier)
            | Wave3HandlerError::Receiving(ReceivingOrderError::MissingExpectedArrival)
            | Wave3HandlerError::Receiving(ReceivingOrderError::InvalidExpectedArrival)
            | Wave3HandlerError::Receiving(ReceivingOrderError::MissingProduct)
            | Wave3HandlerError::Receiving(ReceivingOrderError::MultipleProducts)
            | Wave3HandlerError::Receiving(ReceivingOrderError::InvalidReason)
            | Wave3HandlerError::Receiving(ReceivingOrderError::InvalidDocumentType)
            | Wave3HandlerError::Receiving(ReceivingOrderError::InvalidBatchPolicy)
            | Wave3HandlerError::Receiving(ReceivingOrderError::BatchExpired)
            | Wave3HandlerError::Inventory(InventoryError::InvalidQuantity)
            | Wave3HandlerError::ColdChain(ColdChainError::InvalidDeviceType(_))
            | Wave3HandlerError::ColdChain(ColdChainError::ActiveMonitoring(_))
            | Wave3HandlerError::Inventory(InventoryError::ExpiredBatch)
            | Wave3HandlerError::Inventory(InventoryError::InvalidReason)
            | Wave3HandlerError::Inventory(InventoryError::MissingApprovalSource)
            | Wave3HandlerError::Inventory(InventoryError::RecallNotActive)
            | Wave3HandlerError::Inventory(InventoryError::SameApprover)
            | Wave3HandlerError::Inventory(InventoryError::RecallStateChanged)
            | Wave3HandlerError::Inventory(InventoryError::RecallAlreadyActive)
            | Wave3HandlerError::Inventory(InventoryError::InvalidStateTransition { .. })
            | Wave3HandlerError::ColdChain(ColdChainError::FutureTimestamp)
            | Wave3HandlerError::Billing(BillingError::InvalidRate)
            | Wave3HandlerError::Billing(BillingError::InvalidChargeItem)
            | Wave3HandlerError::Billing(BillingError::InvalidUnit)
            | Wave3HandlerError::Billing(BillingError::InvalidBillingCycle)
            | Wave3HandlerError::Billing(BillingError::InvalidQuantity)
            | Wave3HandlerError::Billing(BillingError::InvalidEffectiveWindow)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidStatus { .. })
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidQuantity)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InsufficientQuantity)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidDeviceType)
            | Wave3HandlerError::Repository(Wave3RepositoryError::ActiveMonitoring)
            | Wave3HandlerError::Repository(Wave3RepositoryError::MissingSupplier)
            | Wave3HandlerError::Repository(Wave3RepositoryError::MissingExpectedArrival)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidExpectedArrival)
            | Wave3HandlerError::Repository(Wave3RepositoryError::MissingProduct)
            | Wave3HandlerError::Repository(Wave3RepositoryError::MultipleProducts)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidDocumentType)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidBatchPolicy)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidQualityStatus)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidLocation)
            | Wave3HandlerError::Repository(Wave3RepositoryError::LocationQualityMismatch)
            | Wave3HandlerError::Repository(Wave3RepositoryError::LocationTemperatureMismatch)
            | Wave3HandlerError::Repository(Wave3RepositoryError::LocationCapacityExceeded)
            | Wave3HandlerError::Repository(Wave3RepositoryError::LocationSkuLimitExceeded)
            | Wave3HandlerError::Repository(Wave3RepositoryError::NoAvailableLocation)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidProductVolume)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidDate(_))
            | Wave3HandlerError::Repository(Wave3RepositoryError::BatchExpired)
            | Wave3HandlerError::Repository(Wave3RepositoryError::QuantityClosureMismatch)
            | Wave3HandlerError::Repository(Wave3RepositoryError::OverReceiptNotAllowed)
            | Wave3HandlerError::Repository(Wave3RepositoryError::DuplicateTraceCode)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidReason)
            | Wave3HandlerError::Repository(Wave3RepositoryError::MissingRequiredField(_))
            | Wave3HandlerError::Repository(
                Wave3RepositoryError::TemperatureExcursionRequiresDisposition,
            )
            | Wave3HandlerError::Repository(Wave3RepositoryError::SupplierQualificationExpired)
            | Wave3HandlerError::Repository(Wave3RepositoryError::MissingApprovalSource)
            | Wave3HandlerError::Repository(Wave3RepositoryError::RecallNotActive)
            | Wave3HandlerError::Repository(Wave3RepositoryError::SameApprover)
            | Wave3HandlerError::Repository(Wave3RepositoryError::SecondApproverNotAuthorized)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidStateTransition {
                ..
            })
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidEffectiveWindow)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidBillingRuleField)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidRate)
            | Wave3HandlerError::Repository(Wave3RepositoryError::FutureTimestamp)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidInventoryState)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidMaintenanceTaskState)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidMaintenanceResult)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidInventoryCountType)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidInventoryCountState)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InventoryCountNotReady)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InventoryCountQuantityConflict)
            | Wave3HandlerError::Repository(Wave3RepositoryError::NoInventoryData) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "W3-422",
                "业务规则校验失败",
            ),
            Wave3HandlerError::ConfigCenter(ConfigCenterError::InvalidFeatureFlagSource(_)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                CONFIG_FLAG_SOURCE_INVALID_CODE,
                "Feature Flag 读取源无效",
            ),
            Wave3HandlerError::Repository(Wave3RepositoryError::Audit(_))
            | Wave3HandlerError::Repository(Wave3RepositoryError::Database(_))
            | Wave3HandlerError::Repository(Wave3RepositoryError::DocumentNumbering(_))
            | Wave3HandlerError::Repository(Wave3RepositoryError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "W3-500",
                "持久化或审计写入失败",
            ),
            Wave3HandlerError::Auth(_) => unreachable!("auth error returned above"),
        };

        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

pub fn wave3_router(state: Wave3AppState) -> Router {
    apply_m3_ops_routes(apply_inventory_count_routes(apply_maintenance_routes(
        apply_receiving_order_routes().route(
            "/api/v1/inbound/receiving-orders/:id/putaway-recommendations",
            get(m2_putaway::recommend_putaway_locations_handler),
        ),
    )))
        .route(
            "/api/v1/inventory/batches",
            get(list_inventory_batches_handler),
        )
        .route(
            "/api/v1/inventory/batches/near-expiry-report",
            get(near_expiry_report_handler),
        )
        .route(
            "/api/v1/inventory/batches/:id/trace",
            get(get_inventory_batch_trace_handler),
        )
        .route(
            "/api/v1/inventory/locations/history",
            get(list_location_history_handler),
        )
        .route(
            "/api/v1/inventory/batches/putaway",
            post(putaway_inventory_batch_handler),
        )
        .route(
            "/api/v1/inventory/batches/status",
            post(change_inventory_batch_status_handler),
        )
        .route(
            "/api/v1/inventory/batches/recall",
            post(mark_inventory_recall_handler),
        )
        .route(
            "/api/v1/inventory/batches/recall/cancel",
            post(cancel_inventory_recall_handler),
        )
        .route(
            "/api/v1/inventory/batches/expire",
            post(isolate_expired_inventory_batches_handler),
        )
        .route(
            "/api/v1/cold-chain/devices",
            get(list_cold_chain_devices_handler).post(create_cold_chain_device_handler),
        )
        .route(
            "/api/v1/cold-chain/devices/:device_code",
            patch(update_cold_chain_device_handler),
        )
        .route(
            "/api/v1/cold-chain/devices/:device_code/disable",
            post(disable_cold_chain_device_handler),
        )
        .route(
            "/api/v1/cold-chain/readings",
            post(ingest_temperature_reading_handler),
        )
        .route(
            "/api/v1/cold-chain/excursions",
            post(ingest_temperature_excursion_handler),
        )
        .route(
            "/api/v1/billing/accounts",
            post(create_billing_account_handler),
        )
        .route(
            "/api/v1/billing/contracts",
            post(create_billing_contract_handler),
        )
        .route("/api/v1/billing/rules", post(create_billing_rule_handler))
        .with_state(state)
}

/// 入库单列表分页查询参数（offset 分页，契约见 docs/api/api-pagination-standards.md）。
#[derive(Clone, Debug, serde::Deserialize)]
pub struct ReceivingOrderListQuery {
    /// 页码，从 1 开始；缺省为 1。
    pub page: Option<u32>,
    /// 每页条数；缺省为 20，上限 200。
    pub page_size: Option<u32>,
}

impl ReceivingOrderListQuery {
    fn page(&self) -> u32 {
        self.page.filter(|p| *p >= 1).unwrap_or(1)
    }

    fn page_size(&self) -> u32 {
        self.page_size.filter(|s| *s >= 1).unwrap_or(20).min(200)
    }
}

async fn list_receiving_orders_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Query(query): Query<ReceivingOrderListQuery>,
) -> Result<Json<ReceivingOrderListResponse>, Wave3HandlerError> {
    let page = query.page();
    let page_size = query.page_size();
    let (data, total) = if let Some(repository) = &state.wave3_repository {
        repository
            .list_receiving_orders(&ctx, page, page_size)
            .await?
    } else {
        // 内存兜底路径：与 SQL 路径同语义做内存分页（page/page_size 已钳制）。
        let store = state.inbound_store.lock().await;
        let rows = store.list(&ctx);
        let total = rows.len() as i64;
        let start = ((page - 1) as usize) * (page_size as usize);
        let data = rows
            .into_iter()
            .skip(start)
            .take(page_size as usize)
            .collect();
        (data, total)
    };
    Ok(Json(ReceivingOrderListResponse {
        page: PageMeta {
            count: data.len() as u32,
            next_cursor: None,
            total: Some(total.clamp(0, u32::MAX as i64) as u32),
        },
        data,
    }))
}

async fn list_receiving_dashboard_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Query(query): Query<ReceivingDashboardQuery>,
) -> Result<Json<ReceivingDashboardResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m2.read", "m2.write"])?;
    let data = if let Some(repository) = &state.wave3_repository {
        repository.list_receiving_dashboard(&ctx, &query).await?
    } else {
        let rows = state.inbound_store.lock().await.list(&ctx);
        let mut grouped = std::collections::BTreeMap::<
            String,
            (i64, wms_domain::Quantity, chrono::DateTime<Utc>),
        >::new();
        for row in rows {
            let created_at = row.created_at;
            let entry = grouped.entry(row.status).or_insert((
                0,
                wms_domain::Quantity::ZERO,
                created_at,
            ));
            entry.0 += 1;
            entry.1 += row
                .lines
                .iter()
                .map(|line| line.expected_qty)
                .sum::<wms_domain::Quantity>();
            entry.2 = entry.2.max(created_at);
        }
        grouped
            .into_iter()
            .map(|(status, (order_count, expected_qty, created_at))| {
                wms_domain::ReceivingDashboardRow {
                    created_at,
                    abnormal: matches!(status.as_str(), "closed_rejected" | "exception"),
                    status,
                    order_count,
                    expected_qty,
                }
            })
            .collect()
    };
    Ok(Json(ReceivingDashboardResponse {
        data,
        refreshed_at: Utc::now(),
    }))
}

async fn create_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateReceivingOrderRequest>,
) -> Result<Json<ReceivingOrder>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    if let Some(warehouse_scope) = ctx.warehouse_scope {
        if req.warehouse_id != warehouse_scope {
            return Err(Wave3HandlerError::Repository(
                crate::wave3_repository::Wave3RepositoryError::NotFound,
            ));
        }
    }
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let order = if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "create",
            "M2",
            "receiving_order",
            "pending",
            None,
        );
        return Ok(Json(
            repository
                .create_receiving_order_with_audit(&ctx, req, now, &idempotency_key, audit)
                .await?
                .value,
        ));
    } else {
        let mut store = state.inbound_store.lock().await;
        store.create(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "create",
        "M2",
        "receiving_order",
        order.id.to_string(),
    )
    .await;
    Ok(Json(order))
}

async fn inspect_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<InspectReceivingOrderRequest>,
) -> Result<Json<ReceivingInspectionRecord>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "inspect",
            "M2",
            "receiving_order",
            id.to_string(),
            None,
        );
        let outcome = repository
            .inspect_receiving_order_with_audit(
                &ctx,
                id,
                req,
                now.date_naive(),
                now,
                &idempotency_key,
                Some(audit),
            )
            .await?;
        return Ok(Json(outcome.value));
    }
    let inspection = {
        let mut store = state.inbound_store.lock().await;
        store.inspect(&ctx, id, req, now.date_naive(), now)?
    };
    append_audit(
        &state,
        &ctx,
        "inspect",
        "M2",
        "receiving_order",
        id.to_string(),
    )
    .await;
    Ok(Json(inspection))
}

async fn sign_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<wms_domain::SignInspectionRequest>,
) -> Result<Json<InspectionSignatureRecord>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "sign",
            "M2",
            "receiving_order",
            id.to_string(),
            None,
        );
        let outcome = repository
            .sign_receiving_order_with_audit(&ctx, id, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let signature = {
        let mut store = state.inbound_store.lock().await;
        store.sign_inspection(&ctx, id, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "sign",
        "M2",
        "receiving_order",
        id.to_string(),
    )
    .await;
    Ok(Json(signature))
}
