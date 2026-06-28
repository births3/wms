//! Wave 3 first-batch Axum handlers.
//!
//! These handlers wire Wave 3 services to AuthContext and H2 audit logging.
//! M2 receive/putaway can run against the PostgreSQL repository once a PgPool
//! is attached to the app state.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;
use wms_domain::{
    BillingAccount, BillingContract, BillingRule, ChangeInventoryStatusRequest, ColdChainDevice,
    CreateBillingAccountRequest, CreateBillingContractRequest, CreateBillingRuleRequest,
    CreateColdChainDeviceRequest, ErrorResponse, IngestTemperatureExcursionRequest,
    IngestTemperatureReadingRequest, InspectReceivingOrderRequest, InspectionSignatureRecord,
    InventoryBatch, InventoryBatchListResponse, PageMeta, PutawayInventoryRequest, PutawayRecord,
    PutawayRequest, ReceiveReceivingOrderRequest, ReceivingInspectionRecord, ReceivingOrderReceipt,
    RejectReceivingOrderRequest, TemperatureExcursionEvent, TemperatureReading,
};

use crate::{
    audit::{AuditLog, AuditWriteRequest},
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
            | Wave3HandlerError::Repository(Wave3RepositoryError::NotFound) => {
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
            | Wave3HandlerError::Repository(Wave3RepositoryError::IdempotencyConflict)
            | Wave3HandlerError::Repository(Wave3RepositoryError::BillingRuleConflict) => {
                (StatusCode::CONFLICT, "W3-409", "资源重复")
            }
            Wave3HandlerError::Receiving(ReceivingOrderError::EmptyLines)
            | Wave3HandlerError::Receiving(ReceivingOrderError::InvalidStatus { .. })
            | Wave3HandlerError::Receiving(ReceivingOrderError::QuantityClosureMismatch)
            | Wave3HandlerError::Receiving(ReceivingOrderError::OverReceiptNotAllowed)
            | Wave3HandlerError::Receiving(ReceivingOrderError::InvalidQuantity)
            | Wave3HandlerError::Receiving(ReceivingOrderError::InvalidReason)
            | Wave3HandlerError::Receiving(ReceivingOrderError::BatchExpired)
            | Wave3HandlerError::Receiving(ReceivingOrderError::SameSigner)
            | Wave3HandlerError::Receiving(ReceivingOrderError::MissingSecondSigner)
            | Wave3HandlerError::Inventory(InventoryError::InvalidQuantity)
            | Wave3HandlerError::Inventory(InventoryError::ExpiredBatch)
            | Wave3HandlerError::Inventory(InventoryError::MissingApprovalSource)
            | Wave3HandlerError::Inventory(InventoryError::InvalidStateTransition { .. })
            | Wave3HandlerError::ColdChain(ColdChainError::FutureTimestamp)
            | Wave3HandlerError::Billing(BillingError::InvalidRate)
            | Wave3HandlerError::Billing(BillingError::InvalidQuantity)
            | Wave3HandlerError::Billing(BillingError::InvalidEffectiveWindow)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidStatus { .. })
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidQuantity)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidDate(_))
            | Wave3HandlerError::Repository(Wave3RepositoryError::BatchExpired)
            | Wave3HandlerError::Repository(Wave3RepositoryError::QuantityClosureMismatch)
            | Wave3HandlerError::Repository(Wave3RepositoryError::OverReceiptNotAllowed)
            | Wave3HandlerError::Repository(Wave3RepositoryError::MissingSecondSigner)
            | Wave3HandlerError::Repository(Wave3RepositoryError::SameSigner)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidReason)
            | Wave3HandlerError::Repository(Wave3RepositoryError::MissingApprovalSource)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidStateTransition {
                ..
            })
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidEffectiveWindow)
            | Wave3HandlerError::Repository(Wave3RepositoryError::InvalidRate)
            | Wave3HandlerError::Repository(Wave3RepositoryError::FutureTimestamp) => (
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
    Router::new()
        .route(
            "/api/v1/inbound/receiving-orders/:id/receive",
            post(receive_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/reject",
            post(reject_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/inspect",
            post(inspect_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/sign",
            post(sign_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/putaway",
            post(putaway_receiving_order_handler),
        )
        .route(
            "/api/v1/inventory/batches",
            get(list_inventory_batches_handler),
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
            "/api/v1/cold-chain/devices",
            post(create_cold_chain_device_handler),
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

async fn receive_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ReceiveReceivingOrderRequest>,
) -> Result<Json<ReceivingOrderReceipt>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "receive",
            "M2",
            "receiving_order",
            id.to_string(),
            None,
        );
        let outcome = repository
            .receive_receiving_order_with_audit(&ctx, id, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let receipt = {
        let mut store = state.inbound_store.lock().await;
        store.receive(&ctx, id, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "receive",
        "M2",
        "receiving_order",
        id.to_string(),
    )
    .await;
    Ok(Json(receipt))
}

async fn reject_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<RejectReceivingOrderRequest>,
) -> Result<Json<ReceivingOrderReceipt>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "reject",
            "M2",
            "receiving_order",
            id.to_string(),
            None,
        );
        let outcome = repository
            .reject_receiving_order_with_audit(&ctx, id, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let receipt = {
        let mut store = state.inbound_store.lock().await;
        store.reject(&ctx, id, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "reject",
        "M2",
        "receiving_order",
        id.to_string(),
    )
    .await;
    Ok(Json(receipt))
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

async fn putaway_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<PutawayRequest>,
) -> Result<Json<PutawayRecord>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "putaway",
            "M2",
            "receiving_order",
            id.to_string(),
            None,
        );
        let outcome = repository
            .putaway_receiving_order_and_inventory_with_audit(
                &ctx,
                id,
                req,
                now,
                &idempotency_key,
                Some(audit),
            )
            .await?;
        return Ok(Json(outcome.value.putaway));
    }
    let putaway = {
        let mut store = state.inbound_store.lock().await;
        store.putaway(&ctx, id, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "putaway",
        "M2",
        "receiving_order",
        id.to_string(),
    )
    .await;
    Ok(Json(putaway))
}

async fn list_inventory_batches_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
) -> Result<Json<InventoryBatchListResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.read", "m3.write"])?;
    if let Some(config_center_state) = &state.config_center_state {
        if !config_center_state
            .is_feature_enabled(INVENTORY_BATCHES_SMOKE_FLAG)
            .await?
        {
            return Err(
                ConfigCenterError::DisabledFlag(INVENTORY_BATCHES_SMOKE_FLAG.to_string()).into(),
            );
        }
    }
    if let Some(repository) = &state.wave3_repository {
        let batches = repository.list_inventory_batches(&ctx).await?;
        let count = batches.len() as u32;
        return Ok(Json(InventoryBatchListResponse {
            data: batches,
            page: PageMeta {
                next_cursor: None,
                count,
            },
        }));
    }
    let batches = {
        let store = state.inventory_store.lock().await;
        store.list_batches(&ctx)
    };
    let count = batches.len() as u32;
    Ok(Json(InventoryBatchListResponse {
        data: batches,
        page: PageMeta {
            next_cursor: None,
            count,
        },
    }))
}

async fn putaway_inventory_batch_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<PutawayInventoryRequest>,
) -> Result<Json<InventoryBatch>, Wave3HandlerError> {
    ctx.require_permission("m3.write")?;
    let now = Utc::now();
    let source_id = req.source_receiving_order_id;
    let batch = {
        let mut store = state.inventory_store.lock().await;
        store.putaway_from_inbound(&ctx, req, now.date_naive(), now)?
    };
    append_audit(
        &state,
        &ctx,
        "putaway",
        "M3",
        "inventory_batch",
        source_id.to_string(),
    )
    .await;
    Ok(Json(batch))
}

async fn change_inventory_batch_status_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<ChangeInventoryStatusRequest>,
) -> Result<Json<InventoryBatch>, Wave3HandlerError> {
    ctx.require_permission("m3.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let batch_id = req.batch_id;
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "change_status",
            "M3",
            "inventory_batch",
            batch_id.to_string(),
            None,
        );
        let outcome = repository
            .change_inventory_status_with_audit(&ctx, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let batch = {
        let mut store = state.inventory_store.lock().await;
        store.change_status(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "change_status",
        "M3",
        "inventory_batch",
        batch_id.to_string(),
    )
    .await;
    Ok(Json(batch))
}

async fn create_cold_chain_device_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<CreateColdChainDeviceRequest>,
) -> Result<Json<ColdChainDevice>, Wave3HandlerError> {
    ctx.require_permission("m5.write")?;
    let now = Utc::now();
    let device = {
        let mut service = state.cold_chain_service.lock().await;
        service.create_device(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "create_device",
        "M5",
        "cold_chain_device",
        device.id.to_string(),
    )
    .await;
    Ok(Json(device))
}

async fn ingest_temperature_reading_handler(
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<IngestTemperatureReadingRequest>,
) -> Result<Json<TemperatureReading>, Wave3HandlerError> {
    let (ctx, idempotency_key) = cold_chain_external_context(&state, &headers)?;
    ctx.require_permission("m5.write")?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "ingest_reading",
            "M5",
            "temperature_reading",
            "",
            None,
        );
        let outcome = repository
            .ingest_temperature_reading_with_audit(&ctx, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let reading = {
        let mut service = state.cold_chain_service.lock().await;
        service.ingest_reading(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "ingest_reading",
        "M5",
        "temperature_reading",
        reading.id.to_string(),
    )
    .await;
    Ok(Json(reading))
}

async fn ingest_temperature_excursion_handler(
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<IngestTemperatureExcursionRequest>,
) -> Result<Json<TemperatureExcursionEvent>, Wave3HandlerError> {
    let (ctx, idempotency_key) = cold_chain_external_context(&state, &headers)?;
    ctx.require_permission("m5.write")?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "ingest_excursion",
            "M5",
            "temperature_excursion",
            "",
            None,
        );
        let outcome = repository
            .ingest_temperature_excursion_with_audit(&ctx, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let event = {
        let mut service = state.cold_chain_service.lock().await;
        service.ingest_excursion(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "ingest_excursion",
        "M5",
        "temperature_excursion",
        event.id.to_string(),
    )
    .await;
    Ok(Json(event))
}

async fn create_billing_account_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<CreateBillingAccountRequest>,
) -> Result<Json<BillingAccount>, Wave3HandlerError> {
    ctx.require_permission("m9.write")?;
    let now = Utc::now();
    let account = {
        let mut store = state.billing_store.lock().await;
        store.create_account(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "create_account",
        "M9",
        "billing_account",
        account.id.to_string(),
    )
    .await;
    Ok(Json(account))
}

async fn create_billing_contract_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<CreateBillingContractRequest>,
) -> Result<Json<BillingContract>, Wave3HandlerError> {
    ctx.require_permission("m9.write")?;
    let now = Utc::now();
    let contract = {
        let mut store = state.billing_store.lock().await;
        store.create_contract(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "create_contract",
        "M9",
        "billing_contract",
        contract.id.to_string(),
    )
    .await;
    Ok(Json(contract))
}

async fn create_billing_rule_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<CreateBillingRuleRequest>,
) -> Result<Json<BillingRule>, Wave3HandlerError> {
    ctx.require_permission("m9.write")?;
    let now = Utc::now();
    let rule = {
        let mut store = state.billing_store.lock().await;
        store.create_rule(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "create_rule",
        "M9",
        "billing_rule",
        rule.id.to_string(),
    )
    .await;
    Ok(Json(rule))
}

fn require_any_permission(ctx: &AuthContext, permissions: &[&str]) -> Result<(), AuthError> {
    if permissions
        .iter()
        .any(|permission| ctx.has_permission(permission))
    {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied(permissions.join("|")))
    }
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, Wave3HandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(Wave3HandlerError::MissingIdempotencyKey)
}

fn cold_chain_external_context(
    state: &Wave3AppState,
    headers: &HeaderMap,
) -> Result<(AuthContext, String), Wave3HandlerError> {
    let idempotency_key = idempotency_key_from_headers(headers)?;
    let config = state
        .cold_chain_api_key
        .as_ref()
        .ok_or(Wave3HandlerError::ExternalAuthConfigMissing)?;
    let configured_hash = config.key_sha256.trim();
    if configured_hash.len() != 64
        || !configured_hash
            .chars()
            .all(|value| value.is_ascii_hexdigit())
    {
        return Err(Wave3HandlerError::ExternalAuthConfigInvalid);
    }

    let api_key = headers
        .get(EXTERNAL_API_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(Wave3HandlerError::ExternalAuthMissing)?;
    let provided_hash = sha256_hex(api_key.as_bytes());
    if !constant_time_eq(
        provided_hash.as_bytes(),
        configured_hash.to_ascii_lowercase().as_bytes(),
    ) {
        return Err(Wave3HandlerError::ExternalAuthInvalid);
    }

    Ok((
        AuthContext {
            user_id: Uuid::nil(),
            owner_id: config.owner_id,
            actor_name: config.actor_name.clone(),
            permissions: vec!["m5.write".to_string()],
            jti: format!("m5-cold-chain:{idempotency_key}"),
        },
        idempotency_key,
    ))
}

fn sha256_hex(value: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    hex::encode(hasher.finalize())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut diff = left.len() ^ right.len();
    let max_len = left.len().max(right.len());
    for index in 0..max_len {
        let left_byte = left.get(index).copied().unwrap_or_default();
        let right_byte = right.get(index).copied().unwrap_or_default();
        diff |= (left_byte ^ right_byte) as usize;
    }
    diff == 0
}

async fn append_audit(
    state: &Wave3AppState,
    ctx: &AuthContext,
    action: &'static str,
    module: &'static str,
    resource_type: &'static str,
    resource_id: String,
) {
    let mut audit_log = state.audit_log.lock().await;
    audit_log.append_event(AuditWriteRequest::from_auth_context(
        ctx,
        action,
        module,
        resource_type,
        resource_id,
        None,
    ));
}

#[cfg(test)]
mod tests {
    use axum::{
        body::to_bytes,
        extract::{Path, State},
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        Json,
    };
    use chrono::{NaiveDate, TimeZone, Utc};
    use sqlx::PgPool;
    use uuid::Uuid;
    use wms_domain::{
        ChangeInventoryStatusRequest, CreateReceivingOrderRequest,
        IngestTemperatureExcursionRequest, IngestTemperatureReadingRequest,
        InspectReceivingOrderRequest, PutawayInventoryRequest, PutawayRequest,
        ReceiveReceivingOrderRequest, ReceivingOrderLine, SignInspectionRequest,
        UpdateReceivingOrderRequest,
    };

    use super::{
        change_inventory_batch_status_handler, ingest_temperature_excursion_handler,
        ingest_temperature_reading_handler, inspect_receiving_order_handler,
        list_inventory_batches_handler, putaway_inventory_batch_handler,
        putaway_receiving_order_handler, receive_receiving_order_handler, sha256_hex,
        sign_receiving_order_handler, wave3_router, ExternalApiKeyConfig, Wave3AppState,
        Wave3HandlerError, EXTERNAL_API_KEY_HEADER, IDEMPOTENCY_KEY_HEADER,
        INVENTORY_BATCHES_SMOKE_FLAG,
    };
    use crate::{
        auth::{AuthContext, AuthError},
        config_center::{
            ConfigCenterAppState, ConfigCenterError, FeatureFlagSource, CONFIG_FLAG_MISSING_CODE,
        },
        feature_flags::FeatureFlagRegistry,
        inventory::{InventoryError, STATUS_QUALIFIED, STATUS_QUARANTINED},
    };

    fn ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "tester".to_string(),
            permissions: permissions
                .iter()
                .map(|permission| permission.to_string())
                .collect(),
            jti: Uuid::new_v4().to_string(),
        }
    }

    fn receiving_line() -> ReceivingOrderLine {
        ReceivingOrderLine {
            line_no: 1,
            product_id: None,
            product_code: "P-001".to_string(),
            expected_qty: 10,
            batch_no: Some("B202606".to_string()),
            production_date: Some("2026-01-01".to_string()),
            expiry_date: Some("2028-01-01".to_string()),
        }
    }

    fn inventory_putaway_req() -> PutawayInventoryRequest {
        PutawayInventoryRequest {
            product_code: "P-001".to_string(),
            batch_no: "B202606".to_string(),
            production_date: "2026-01-01".to_string(),
            expiry_date: "2028-01-01".to_string(),
            qty: 10,
            quality_status: STATUS_QUALIFIED.to_string(),
            location_id: Uuid::new_v4(),
            location_code: "A-01-01".to_string(),
            source_receiving_order_id: Uuid::new_v4(),
        }
    }

    fn config_center_smoke_registry() -> FeatureFlagRegistry {
        FeatureFlagRegistry::from_toml_str(&format!(
            r#"
            [[flags]]
            key = "{INVENTORY_BATCHES_SMOKE_FLAG}"
            owner = "platform"
            created_at = 2026-06-07
            cleanup_by = 2026-09-05
            enabled = true
            "#
        ))
        .expect("valid smoke flag registry")
    }

    fn idempotency_headers(key: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(IDEMPOTENCY_KEY_HEADER, key.parse().expect("valid header"));
        headers
    }

    fn external_auth_headers(idempotency_key: &'static str, api_key: &'static str) -> HeaderMap {
        let mut headers = idempotency_headers(idempotency_key);
        headers.insert(
            EXTERNAL_API_KEY_HEADER,
            api_key.parse().expect("valid header"),
        );
        headers
    }

    fn external_api_key_config(owner_id: Uuid, api_key: &str) -> ExternalApiKeyConfig {
        ExternalApiKeyConfig {
            key_sha256: sha256_hex(api_key.as_bytes()),
            owner_id,
            actor_name: "external-cold-chain-test".to_string(),
        }
    }

    async fn error_response(error: Wave3HandlerError) -> (StatusCode, wms_domain::ErrorResponse) {
        let response = error.into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let payload = serde_json::from_slice(&body).expect("error response should be json");
        (status, payload)
    }

    async fn seed_cold_chain_device(pool: &PgPool, owner_id: Uuid, device_code: &str) {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 14, 0, 0)
            .single()
            .expect("valid time");
        sqlx::query(
            r#"
	            INSERT INTO cold_chain_devices (
	                id, owner_id, device_code, device_type,
	                installed_at_location_code, calibration_due_at, status, created_at, updated_at
	            )
	            VALUES ($1, $2, $3, 'thermometer', 'CC-01', NULL, 'active', $4, $4)
	            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(device_code)
        .bind(now)
        .execute(pool)
        .await
        .expect("seed cold-chain device");
    }

    #[test]
    fn wave3_router_registers_first_batch_handlers() {
        let _router = wave3_router(Wave3AppState::default());
    }

    #[tokio::test]
    async fn inbound_receive_handler_requires_permission_and_appends_audit() {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m2.write"]);
        let denied_ctx = ctx(owner_id, &[]);
        let state = Wave3AppState::default();
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
            .single()
            .expect("valid time");
        let order = {
            let mut store = state.inbound_store.lock().await;
            let created = store
                .create(
                    &authorized,
                    CreateReceivingOrderRequest {
                        receipt_no: "ASN-HANDLER-001".to_string(),
                        supplier_id: None,
                        warehouse_id: Uuid::new_v4(),
                        external_ref: None,
                        expected_arrival_at: None,
                        lines: vec![receiving_line()],
                    },
                    now,
                )
                .expect("create order");
            store
                .update(
                    &authorized,
                    created.id,
                    UpdateReceivingOrderRequest {
                        supplier_id: None,
                        warehouse_id: None,
                        external_ref: None,
                        status: Some("released".to_string()),
                        expected_arrival_at: None,
                        lines: None,
                    },
                    now,
                )
                .expect("release order")
        };

        let req = ReceiveReceivingOrderRequest {
            actual_qty: 8,
            shortage_qty: 2,
            rejected_qty: 0,
            arrival_temperature_celsius: None,
            exception_note: None,
        };
        let denied = receive_receiving_order_handler(
            denied_ctx,
            State(state.clone()),
            Path(order.id),
            HeaderMap::new(),
            Json(req.clone()),
        )
        .await
        .expect_err("permission should be required");
        assert!(matches!(
            denied,
            Wave3HandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == "m2.write"
        ));
        assert!(state.audit_log.lock().await.events().is_empty());

        let missing_idempotency_key = receive_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(order.id),
            HeaderMap::new(),
            Json(req.clone()),
        )
        .await
        .expect_err("idempotency key should be required for fallback writes");
        assert!(matches!(
            missing_idempotency_key,
            Wave3HandlerError::MissingIdempotencyKey
        ));
        assert!(state.audit_log.lock().await.events().is_empty());

        let Json(receipt) = receive_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(order.id),
            idempotency_headers("fallback-receive-1"),
            Json(req),
        )
        .await
        .expect("authorized receive should succeed");

        assert_eq!(receipt.actual_qty, 8);
        let audit_log = state.audit_log.lock().await;
        assert_eq!(audit_log.events().len(), 1);
        assert_eq!(audit_log.events()[0].action, "receive");
        assert_eq!(audit_log.events()[0].module, "M2");
        assert_eq!(audit_log.events()[0].resource_id, order.id.to_string());
        audit_log
            .verify_hash_chain()
            .expect("audit hash chain should verify");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn postgres_receive_handler_writes_business_idempotency_and_audit(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m2.write"]);
        let state = Wave3AppState::with_postgres(pool.clone());
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
            .single()
            .expect("valid time");
        let repository = state
            .wave3_repository
            .as_ref()
            .expect("postgres repository");
        let order = repository
            .create_receiving_order(
                &authorized,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-HANDLER-PG-001".to_string(),
                    supplier_id: None,
                    warehouse_id: Uuid::new_v4(),
                    external_ref: None,
                    expected_arrival_at: None,
                    lines: vec![receiving_line()],
                },
                now,
            )
            .await
            .expect("create order");
        repository
            .release_receiving_order(&authorized, order.id, now)
            .await
            .expect("release order");

        let req = ReceiveReceivingOrderRequest {
            actual_qty: 8,
            shortage_qty: 2,
            rejected_qty: 0,
            arrival_temperature_celsius: None,
            exception_note: None,
        };
        let Json(receipt) = receive_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(order.id),
            idempotency_headers("handler-receive-1"),
            Json(req.clone()),
        )
        .await
        .expect("postgres receive should succeed");
        let Json(replay) = receive_receiving_order_handler(
            authorized,
            State(state.clone()),
            Path(order.id),
            idempotency_headers("handler-receive-1"),
            Json(req),
        )
        .await
        .expect("same idempotency key should replay");

        assert_eq!(receipt.id, replay.id);
        let counts: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM receiving_order_receipts WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'receive')
            "#,
        )
        .bind(order.id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 1));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn postgres_putaway_handler_commits_inventory_and_audit(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m2.write"]);
        let state = Wave3AppState::with_postgres(pool.clone());
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 11, 0, 0)
            .single()
            .expect("valid time");
        let repository = state
            .wave3_repository
            .as_ref()
            .expect("postgres repository");
        let order = repository
            .create_receiving_order(
                &authorized,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-HANDLER-PG-002".to_string(),
                    supplier_id: None,
                    warehouse_id: Uuid::new_v4(),
                    external_ref: None,
                    expected_arrival_at: None,
                    lines: vec![receiving_line()],
                },
                now,
            )
            .await
            .expect("create order");
        sqlx::query("UPDATE receiving_orders SET status = 'putaway' WHERE id = $1")
            .bind(order.id)
            .execute(&pool)
            .await
            .expect("prepare putaway state");

        let Json(putaway) = putaway_receiving_order_handler(
            authorized,
            State(state),
            Path(order.id),
            idempotency_headers("handler-putaway-1"),
            Json(PutawayRequest {
                batch_no: "B202606".to_string(),
                product_code: "P-001".to_string(),
                qty: 10,
                location_id: Uuid::new_v4(),
                location_code: "A-01-01".to_string(),
                quality_status: crate::inventory::STATUS_QUALIFIED.to_string(),
            }),
        )
        .await
        .expect("postgres putaway should succeed");

        assert_eq!(putaway.receiving_order_id, order.id);
        let counts: (i64, i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM receiving_putaways WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM inventory_batches WHERE owner_id = $2),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'putaway'),
                (SELECT status FROM receiving_orders WHERE id = $1)
            "#,
        )
        .bind(order.id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 1, "completed".to_string()));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn postgres_inspect_and_sign_handlers_write_idempotency_and_audit(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m2.write"]);
        let state = Wave3AppState::with_postgres(pool.clone());
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
            .single()
            .expect("valid time");
        let repository = state
            .wave3_repository
            .as_ref()
            .expect("postgres repository");
        let order = repository
            .create_receiving_order(
                &authorized,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-HANDLER-PG-003".to_string(),
                    supplier_id: None,
                    warehouse_id: Uuid::new_v4(),
                    external_ref: None,
                    expected_arrival_at: None,
                    lines: vec![receiving_line()],
                },
                now,
            )
            .await
            .expect("create order");
        sqlx::query("UPDATE receiving_orders SET status = 'inspecting' WHERE id = $1")
            .bind(order.id)
            .execute(&pool)
            .await
            .expect("prepare inspecting state");

        let inspect_req = InspectReceivingOrderRequest {
            batch_no: "B202606".to_string(),
            accepted_qty: 10,
            rejected_qty: 0,
            production_date: "2026-01-01".to_string(),
            expiry_date: "2028-01-01".to_string(),
            quality_status: crate::inventory::STATUS_QUALIFIED.to_string(),
            trace_codes: vec!["TRACE-001".to_string()],
        };
        let Json(inspection) = inspect_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(order.id),
            idempotency_headers("handler-inspect-1"),
            Json(inspect_req.clone()),
        )
        .await
        .expect("postgres inspect should succeed");
        let Json(inspection_replay) = inspect_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(order.id),
            idempotency_headers("handler-inspect-1"),
            Json(inspect_req),
        )
        .await
        .expect("same inspect idempotency key should replay");
        assert_eq!(inspection.id, inspection_replay.id);

        let second_signer_id = Uuid::new_v4();
        let sign_req = SignInspectionRequest {
            first_signer_id: authorized.user_id,
            second_signer_id: Some(second_signer_id),
            dual_required: true,
        };
        let Json(signature) = sign_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(order.id),
            idempotency_headers("handler-sign-1"),
            Json(sign_req.clone()),
        )
        .await
        .expect("postgres sign should succeed");
        let Json(signature_replay) = sign_receiving_order_handler(
            authorized,
            State(state),
            Path(order.id),
            idempotency_headers("handler-sign-1"),
            Json(sign_req),
        )
        .await
        .expect("same sign idempotency key should replay");
        assert_eq!(signature.id, signature_replay.id);

        let counts: (i64, i64, i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM receiving_inspections WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM receiving_inspection_signatures WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action IN ('inspect', 'sign')),
                (SELECT status FROM receiving_orders WHERE id = $1)
            "#,
        )
        .bind(order.id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 2, 2, "putaway".to_string()));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn postgres_inventory_query_and_status_change_are_scoped_idempotent_and_audited(
        pool: PgPool,
    ) {
        let owner_id = Uuid::new_v4();
        let other_owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m3.read", "m3.write"]);
        let state = Wave3AppState::with_postgres(pool.clone());
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 13, 0, 0)
            .single()
            .expect("valid time");
        let batch_id = Uuid::new_v4();
        let other_batch_id = Uuid::new_v4();
        for (id, owner, code) in [
            (batch_id, owner_id, "P-001"),
            (other_batch_id, other_owner_id, "P-002"),
        ] {
            sqlx::query(
                r#"
                INSERT INTO inventory_batches (
                    id, owner_id, product_code, batch_no, production_date, expiry_date,
                    qty_on_hand, qty_locked, quality_status, location_id, location_code,
                    recall_flag, created_at, updated_at
                )
                VALUES ($1, $2, $3, 'B202606', $4, $5, 10, 0, $6, $7, 'A-01-01', FALSE, $8, $8)
                "#,
            )
            .bind(id)
            .bind(owner)
            .bind(code)
            .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
            .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid date"))
            .bind(STATUS_QUALIFIED)
            .bind(Uuid::new_v4())
            .bind(now)
            .execute(&pool)
            .await
            .expect("seed inventory batch");
        }

        let Json(list) = list_inventory_batches_handler(authorized.clone(), State(state.clone()))
            .await
            .expect("list should use postgres repository");
        assert_eq!(list.page.count, 1);
        assert_eq!(list.data[0].owner_id, owner_id);
        assert_eq!(list.data[0].id, batch_id);

        let missing_approval = change_inventory_batch_status_handler(
            authorized.clone(),
            State(state.clone()),
            idempotency_headers("m3-status-invalid"),
            Json(ChangeInventoryStatusRequest {
                batch_id,
                target_status: STATUS_QUARANTINED.to_string(),
                reason: "missing approval".to_string(),
                approval_source: "".to_string(),
                approval_id: "".to_string(),
            }),
        )
        .await
        .expect_err("approval source should be required");
        assert!(matches!(
            missing_approval,
            Wave3HandlerError::Repository(
                crate::wave3_repository::Wave3RepositoryError::MissingApprovalSource
            )
        ));

        let req = ChangeInventoryStatusRequest {
            batch_id,
            target_status: STATUS_QUARANTINED.to_string(),
            reason: "temperature exception".to_string(),
            approval_source: "温度超标事件".to_string(),
            approval_id: "TEMP-001".to_string(),
        };
        let Json(quarantined) = change_inventory_batch_status_handler(
            authorized.clone(),
            State(state.clone()),
            idempotency_headers("m3-status-1"),
            Json(req.clone()),
        )
        .await
        .expect("status change should succeed");
        let Json(replay) = change_inventory_batch_status_handler(
            authorized,
            State(state),
            idempotency_headers("m3-status-1"),
            Json(req),
        )
        .await
        .expect("same idempotency key should replay");

        assert_eq!(quarantined.id, batch_id);
        assert_eq!(quarantined.quality_status, STATUS_QUARANTINED);
        assert_eq!(replay.quality_status, STATUS_QUARANTINED);

        let counts: (i64, i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM inventory_status_changes WHERE batch_id = $1),
                (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'change_status'),
                (SELECT quality_status FROM inventory_batches WHERE id = $1)
            "#,
        )
        .bind(batch_id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 1, STATUS_QUARANTINED.to_string()));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn postgres_cold_chain_reading_uses_external_api_key_idempotency_and_audit(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let api_key = "test-cold-chain-key";
        let state = Wave3AppState::with_postgres_and_cold_chain_api_key(
            pool.clone(),
            external_api_key_config(owner_id, api_key),
        );
        seed_cold_chain_device(&pool, owner_id, "TEMP-DEVICE-01").await;

        let captured_at = Utc::now() - chrono::Duration::minutes(5);
        let req = IngestTemperatureReadingRequest {
            device_code: "TEMP-DEVICE-01".to_string(),
            temperature_celsius: 5.2,
            humidity_percent: Some(60.0),
            captured_at,
            external_report_url: Some("https://cold-chain.example.test/report/1".to_string()),
            out_of_range: false,
        };

        let missing_key = ingest_temperature_reading_handler(
            State(state.clone()),
            idempotency_headers("m5-reading-missing-key"),
            Json(req.clone()),
        )
        .await
        .expect_err("external API key should be required");
        assert!(matches!(
            missing_key,
            Wave3HandlerError::ExternalAuthMissing
        ));

        let bad_key = ingest_temperature_reading_handler(
            State(state.clone()),
            external_auth_headers("m5-reading-bad-key", "wrong-key"),
            Json(req.clone()),
        )
        .await
        .expect_err("invalid external API key should be rejected");
        assert!(matches!(bad_key, Wave3HandlerError::ExternalAuthInvalid));

        let Json(reading) = ingest_temperature_reading_handler(
            State(state.clone()),
            external_auth_headers("m5-reading-1", api_key),
            Json(req.clone()),
        )
        .await
        .expect("reading should be persisted");
        let Json(replay) = ingest_temperature_reading_handler(
            State(state),
            external_auth_headers("m5-reading-1", api_key),
            Json(req),
        )
        .await
        .expect("same idempotency key should replay");

        assert_eq!(reading.id, replay.id);
        let counts: (i64, i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM temperature_readings WHERE owner_id = $1),
                (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'ingest_reading'),
                (SELECT actor_name FROM audit_event WHERE owner_id = $1 AND action = 'ingest_reading')
            "#,
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 1, "external-cold-chain-test".to_string()));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn postgres_cold_chain_excursion_is_idempotent_and_audited(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let api_key = "test-cold-chain-key";
        let state = Wave3AppState::with_postgres_and_cold_chain_api_key(
            pool.clone(),
            external_api_key_config(owner_id, api_key),
        );
        seed_cold_chain_device(&pool, owner_id, "TEMP-DEVICE-02").await;

        let started_at = Utc::now() - chrono::Duration::minutes(30);
        let req = IngestTemperatureExcursionRequest {
            external_event_id: "EXT-EVENT-001".to_string(),
            device_code: "TEMP-DEVICE-02".to_string(),
            location_code: Some("CC-01".to_string()),
            started_at,
            ended_at: Some(started_at + chrono::Duration::minutes(15)),
            min_temperature_celsius: Some(1.0),
            max_temperature_celsius: Some(9.1),
            affected_batch_ids: vec![Uuid::new_v4()],
        };

        let Json(event) = ingest_temperature_excursion_handler(
            State(state.clone()),
            external_auth_headers("m5-excursion-1", api_key),
            Json(req.clone()),
        )
        .await
        .expect("excursion should be persisted");
        let Json(replay) = ingest_temperature_excursion_handler(
            State(state),
            external_auth_headers("m5-excursion-1", api_key),
            Json(req),
        )
        .await
        .expect("same idempotency key should replay");

        assert_eq!(event.id, replay.id);
        assert_eq!(event.status, "pending_disposition");
        let counts: (i64, i64, i64, String) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM temperature_excursion_events WHERE owner_id = $1),
                (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'ingest_excursion'),
                (SELECT resource_id FROM audit_event WHERE owner_id = $1 AND action = 'ingest_excursion')
            "#,
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (1, 1, 1, event.id.to_string()));
    }

    #[tokio::test]
    async fn inventory_handlers_audit_success_and_skip_failed_business_rule() {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m3.write"]);
        let state = Wave3AppState::default();

        let Json(batch) = putaway_inventory_batch_handler(
            authorized.clone(),
            State(state.clone()),
            Json(inventory_putaway_req()),
        )
        .await
        .expect("putaway should create batch");
        assert_eq!(batch.quality_status, STATUS_QUALIFIED);
        assert_eq!(state.audit_log.lock().await.events().len(), 1);

        let missing_approval = change_inventory_batch_status_handler(
            authorized.clone(),
            State(state.clone()),
            idempotency_headers("fallback-status-missing-approval"),
            Json(ChangeInventoryStatusRequest {
                batch_id: batch.id,
                target_status: STATUS_QUARANTINED.to_string(),
                reason: "temperature exception".to_string(),
                approval_source: "".to_string(),
                approval_id: "".to_string(),
            }),
        )
        .await
        .expect_err("approval source should be required");
        assert!(matches!(
            missing_approval,
            Wave3HandlerError::Inventory(InventoryError::MissingApprovalSource)
        ));
        assert_eq!(state.audit_log.lock().await.events().len(), 1);

        let missing_idempotency_key = change_inventory_batch_status_handler(
            authorized.clone(),
            State(state.clone()),
            HeaderMap::new(),
            Json(ChangeInventoryStatusRequest {
                batch_id: batch.id,
                target_status: STATUS_QUARANTINED.to_string(),
                reason: "temperature exception".to_string(),
                approval_source: "温度超标事件".to_string(),
                approval_id: "TEMP-001".to_string(),
            }),
        )
        .await
        .expect_err("idempotency key should be required for fallback writes");
        assert!(matches!(
            missing_idempotency_key,
            Wave3HandlerError::MissingIdempotencyKey
        ));
        assert_eq!(state.audit_log.lock().await.events().len(), 1);

        let Json(quarantined) = change_inventory_batch_status_handler(
            authorized,
            State(state.clone()),
            idempotency_headers("fallback-status-1"),
            Json(ChangeInventoryStatusRequest {
                batch_id: batch.id,
                target_status: STATUS_QUARANTINED.to_string(),
                reason: "temperature exception".to_string(),
                approval_source: "温度超标事件".to_string(),
                approval_id: "TEMP-001".to_string(),
            }),
        )
        .await
        .expect("approved transition should succeed");

        assert_eq!(quarantined.quality_status, STATUS_QUARANTINED);
        let audit_log = state.audit_log.lock().await;
        assert_eq!(audit_log.events().len(), 2);
        assert_eq!(audit_log.events()[1].action, "change_status");
        assert_eq!(audit_log.events()[1].resource_id, batch.id.to_string());
        audit_log
            .verify_hash_chain()
            .expect("audit hash chain should verify");
    }

    #[tokio::test]
    async fn inventory_batches_handler_reads_config_center_smoke_flag_and_fails_closed() {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m3.read"]);
        let config_center_state =
            ConfigCenterAppState::from_registry(config_center_smoke_registry());
        let state = Wave3AppState::default().with_config_center(config_center_state.clone());

        {
            config_center_state
                .switch_feature_flag_source(FeatureFlagSource::ConfigCenter)
                .await;
        }

        let missing_before_migration =
            list_inventory_batches_handler(authorized.clone(), State(state.clone()))
                .await
                .expect_err("config-center source should fail closed before migration");
        assert!(matches!(
            missing_before_migration,
            Wave3HandlerError::ConfigCenter(ConfigCenterError::MissingFlag(_))
        ));
        let (status, error) = error_response(missing_before_migration).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(error.code, CONFIG_FLAG_MISSING_CODE);

        {
            config_center_state.migrate_feature_flags_from_file().await;
        }

        let Json(list) = list_inventory_batches_handler(authorized, State(state))
            .await
            .expect("migrated config-center smoke flag should allow inventory list");

        assert_eq!(list.page.count, 0);
    }
}
