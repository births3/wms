//! Wave 3 first-batch Axum handlers.
//!
//! These handlers wire the existing in-crate services to AuthContext and H2
//! audit logging. PostgreSQL repositories are intentionally deferred until the
//! Wave 3 table design is frozen.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use tokio::sync::Mutex;
use uuid::Uuid;
use wms_domain::{
    BillingAccount, BillingContract, BillingRule, ChangeInventoryStatusRequest, ColdChainDevice,
    CreateBillingAccountRequest, CreateBillingContractRequest, CreateBillingRuleRequest,
    CreateColdChainDeviceRequest, ErrorResponse, IngestTemperatureExcursionRequest,
    IngestTemperatureReadingRequest, InspectReceivingOrderRequest, InspectionSignatureRecord,
    InventoryBatch, InventoryBatchListResponse, PageMeta, PutawayInventoryRequest, PutawayRecord,
    PutawayRequest, ReceiveReceivingOrderRequest, ReceivingInspectionRecord, ReceivingOrderReceipt,
    TemperatureExcursionEvent, TemperatureReading,
};

use crate::{
    audit::{AuditLog, AuditWriteRequest},
    auth::{AuthContext, AuthError},
    billing::{BillingError, BillingStore},
    cold_chain::{ColdChainError, ColdChainService},
    inbound::{ReceivingOrderError, ReceivingOrderStore},
    inventory::{InventoryError, InventoryStore},
};

#[derive(Clone, Debug)]
pub struct Wave3AppState {
    pub inbound_store: Arc<Mutex<ReceivingOrderStore>>,
    pub inventory_store: Arc<Mutex<InventoryStore>>,
    pub cold_chain_service: Arc<Mutex<ColdChainService>>,
    pub billing_store: Arc<Mutex<BillingStore>>,
    pub audit_log: Arc<Mutex<AuditLog>>,
}

impl Default for Wave3AppState {
    fn default() -> Self {
        Self {
            inbound_store: Arc::new(Mutex::new(ReceivingOrderStore::default())),
            inventory_store: Arc::new(Mutex::new(InventoryStore::default())),
            cold_chain_service: Arc::new(Mutex::new(ColdChainService::default())),
            billing_store: Arc::new(Mutex::new(BillingStore::default())),
            audit_log: Arc::new(Mutex::new(AuditLog::default())),
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

impl IntoResponse for Wave3HandlerError {
    fn into_response(self) -> Response {
        if let Wave3HandlerError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            Wave3HandlerError::Receiving(ReceivingOrderError::NotFound)
            | Wave3HandlerError::Inventory(InventoryError::NotFound)
            | Wave3HandlerError::ColdChain(ColdChainError::DeviceNotFound(_))
            | Wave3HandlerError::Billing(BillingError::NotFound) => {
                (StatusCode::NOT_FOUND, "W3-404", "资源不存在")
            }
            Wave3HandlerError::Receiving(ReceivingOrderError::DuplicateReceiptNo(_))
            | Wave3HandlerError::ColdChain(ColdChainError::DuplicateDevice(_))
            | Wave3HandlerError::Billing(BillingError::DuplicateAccountCode(_))
            | Wave3HandlerError::Billing(BillingError::DuplicateContractNo(_)) => {
                (StatusCode::CONFLICT, "W3-409", "资源重复")
            }
            Wave3HandlerError::Receiving(ReceivingOrderError::EmptyLines)
            | Wave3HandlerError::Receiving(ReceivingOrderError::InvalidStatus { .. })
            | Wave3HandlerError::Receiving(ReceivingOrderError::QuantityClosureMismatch)
            | Wave3HandlerError::Receiving(ReceivingOrderError::OverReceiptNotAllowed)
            | Wave3HandlerError::Receiving(ReceivingOrderError::InvalidQuantity)
            | Wave3HandlerError::Receiving(ReceivingOrderError::BatchExpired)
            | Wave3HandlerError::Receiving(ReceivingOrderError::SameSigner)
            | Wave3HandlerError::Receiving(ReceivingOrderError::MissingSecondSigner)
            | Wave3HandlerError::Inventory(InventoryError::InvalidQuantity)
            | Wave3HandlerError::Inventory(InventoryError::ExpiredBatch)
            | Wave3HandlerError::Inventory(InventoryError::MissingApprovalSource)
            | Wave3HandlerError::Inventory(InventoryError::InvalidStateTransition { .. })
            | Wave3HandlerError::ColdChain(ColdChainError::FutureTimestamp)
            | Wave3HandlerError::Billing(BillingError::InvalidRate) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "W3-422",
                "业务规则校验失败",
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
    Json(req): Json<ReceiveReceivingOrderRequest>,
) -> Result<Json<ReceivingOrderReceipt>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let now = Utc::now();
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

async fn inspect_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<InspectReceivingOrderRequest>,
) -> Result<Json<ReceivingInspectionRecord>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let now = Utc::now();
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
    Json(req): Json<wms_domain::SignInspectionRequest>,
) -> Result<Json<InspectionSignatureRecord>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let now = Utc::now();
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
    Json(req): Json<PutawayRequest>,
) -> Result<Json<PutawayRecord>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let now = Utc::now();
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
    Json(req): Json<ChangeInventoryStatusRequest>,
) -> Result<Json<InventoryBatch>, Wave3HandlerError> {
    ctx.require_permission("m3.write")?;
    let now = Utc::now();
    let batch_id = req.batch_id;
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
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<IngestTemperatureReadingRequest>,
) -> Result<Json<TemperatureReading>, Wave3HandlerError> {
    ctx.require_permission("m5.write")?;
    let now = Utc::now();
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
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<IngestTemperatureExcursionRequest>,
) -> Result<Json<TemperatureExcursionEvent>, Wave3HandlerError> {
    ctx.require_permission("m5.write")?;
    let now = Utc::now();
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
        extract::{Path, State},
        Json,
    };
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;
    use wms_domain::{
        ChangeInventoryStatusRequest, CreateReceivingOrderRequest, PutawayInventoryRequest,
        ReceiveReceivingOrderRequest, ReceivingOrderLine, UpdateReceivingOrderRequest,
    };

    use super::{
        change_inventory_batch_status_handler, putaway_inventory_batch_handler,
        receive_receiving_order_handler, wave3_router, Wave3AppState, Wave3HandlerError,
    };
    use crate::{
        auth::{AuthContext, AuthError},
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

        let Json(receipt) = receive_receiving_order_handler(
            authorized.clone(),
            State(state.clone()),
            Path(order.id),
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

        let Json(quarantined) = change_inventory_batch_status_handler(
            authorized,
            State(state.clone()),
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
}
