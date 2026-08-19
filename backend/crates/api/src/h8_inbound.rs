//! US-H8-002：ERP REST 入站防腐层。

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;
use wms_domain::{
    resolve_active_connector, validate_schema_version, CreateOutboundOrderLineRequest,
    CreateOutboundOrderRequest, CreateReceivingOrderRequest, ErrorResponse,
    H8CanonicalInboundCommand, H8ErpMessage, H8ErrorClass, MapParameterRequest,
    ParameterMappingStatus, ReceivingOrderLine,
};

use crate::{
    audit::AuditWriteRequest,
    auth::{AuthContext, AuthError},
    h8_erp_connectors::H8ErpConnectorAppState,
    h8_erp_messages::H8ErpMessageAppState,
    master_data::MasterDataError,
    master_data_postgres::PgMasterDataReadRepository,
    parameter_mapping::{map_parameter, ParameterMappingAppState, ParameterMappingHandlerError},
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
    wave4_repository::{PgWave4Repository, Wave4RepositoryError},
};

mod inventory_seed;
mod lifecycle;
mod order_cancel;
mod partner_master;
mod product_change;
mod product_master;
mod return_order;

use inventory_seed::push_inventory_seed_snapshot;
pub use inventory_seed::{H8InventorySeedItemInput, H8InventorySeedSnapshotInboundRequest};
use lifecycle::{
    idempotency_key, prepare_message, record_convert_message, succeed_message,
    validate_payload_digest, InboundMetadata,
};
use order_cancel::push_order_cancel;
pub use order_cancel::H8OrderCancelInboundRequest;
use partner_master::{push_customer_master, push_supplier_master};
pub use partner_master::{H8CustomerMasterInboundRequest, H8SupplierMasterInboundRequest};
use product_change::push_product_change;
pub use product_change::{H8PhysicalDimensionsInput, H8ProductChangeInboundRequest};
use product_master::push_product_master;
pub use product_master::{H8ProductMasterInboundRequest, H8ProductPackagingLevelInput};
use return_order::push_return_order;
pub use return_order::H8ReturnOrderInboundRequest;

const ASN_PATH: &str = "/api/v1/integration/erp-messages/inbound/asn";
const OUTBOUND_ORDER_PATH: &str = "/api/v1/integration/erp-messages/inbound/outbound_order";
const PRODUCT_CHANGE_PATH: &str = "/api/v1/integration/erp-messages/inbound/product_change";
const PRODUCT_MASTER_PATH: &str = "/api/v1/integration/erp-messages/inbound/product_master";
const CUSTOMER_MASTER_PATH: &str = "/api/v1/integration/erp-messages/inbound/customer_master";
const SUPPLIER_MASTER_PATH: &str = "/api/v1/integration/erp-messages/inbound/supplier_master";
const INVENTORY_SEED_PATH: &str =
    "/api/v1/integration/erp-messages/inbound/inventory_seed_snapshot";
const ORDER_CANCEL_PATH: &str = "/api/v1/integration/erp-messages/inbound/order_cancel";
const RETURN_ORDER_PATH: &str = "/api/v1/integration/erp-messages/inbound/return_order";
const IDEMPOTENCY_HEADER: &str = "Idempotency-Key";

#[derive(Clone)]
pub struct H8InboundAppState {
    connectors: H8ErpConnectorAppState,
    messages: H8ErpMessageAppState,
    mappings: ParameterMappingAppState,
    master_data: PgMasterDataReadRepository,
    quality_liaison: crate::quality_liaison::PgQualityLiaisonRepository,
    wave3: PgWave3Repository,
    wave4: PgWave4Repository,
}

impl H8InboundAppState {
    pub fn with_postgres(pool: sqlx::PgPool) -> Self {
        Self {
            connectors: H8ErpConnectorAppState::with_postgres(pool.clone()),
            messages: H8ErpMessageAppState::with_postgres(pool.clone()),
            mappings: ParameterMappingAppState::with_postgres(pool.clone()),
            master_data: PgMasterDataReadRepository::new(pool.clone()),
            quality_liaison: crate::quality_liaison::PgQualityLiaisonRepository::new(pool.clone()),
            wave3: PgWave3Repository::new(pool.clone()),
            wave4: PgWave4Repository::new(pool),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8AsnInboundRequest {
    pub schema_version: String,
    pub external_ref: String,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    pub payload_digest: String,
    pub source_version: Option<i64>,
    pub erp_bill_id: i64,
    pub erp_bill_code: String,
    pub revision: i32,
    pub order_type: i32,
    pub partner_type: Option<String>,
    pub partner_code: Option<String>,
    pub depot_code: String,
    pub business_date: NaiveDate,
    pub note_code: Option<String>,
    pub lines: Vec<H8AsnLineInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8AsnLineInput {
    pub line_no: u32,
    pub product_code: String,
    #[schema(value_type = String, format = "decimal")]
    pub expected_qty: wms_domain::Quantity,
    pub batch_no: Option<String>,
    pub production_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8OutboundOrderInboundRequest {
    pub schema_version: String,
    pub external_ref: String,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    pub payload_digest: String,
    pub source_version: Option<i64>,
    pub erp_bill_id: i64,
    pub erp_bill_code: String,
    pub revision: i32,
    pub order_type: i32,
    pub customer_code: String,
    pub depot_code: String,
    pub required_ship_at: DateTime<Utc>,
    pub send_mode: Option<i32>,
    pub erp_address_id: i64,
    pub address_code: String,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub address: String,
    pub lines: Vec<H8OutboundLineInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8OutboundLineInput {
    pub line_no: u32,
    pub product_code: String,
    pub batch_no: String,
    #[schema(value_type = String, format = "decimal")]
    pub planned_qty: wms_domain::Quantity,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct H8InboundResponse {
    pub message_id: Uuid,
    pub wms_resource_id: String,
    pub status: String,
    pub replayed: bool,
    pub ignored_old_version: bool,
}

#[derive(Debug)]
enum H8InboundError {
    Auth(AuthError),
    BadRequest(&'static str),
    Unprocessable(String),
    Conflict(&'static str),
    OrderNotReady,
    Internal(String),
}

impl H8InboundError {
    fn error_class(&self) -> H8ErrorClass {
        match self {
            Self::Internal(_) | Self::OrderNotReady => H8ErrorClass::Retryable,
            Self::Auth(_) | Self::BadRequest(_) | Self::Unprocessable(_) | Self::Conflict(_) => {
                H8ErrorClass::NonRetryable
            }
        }
    }
}

impl From<AuthError> for H8InboundError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl IntoResponse for H8InboundError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "H8-400", message.to_string()),
            Self::Unprocessable(message) => (StatusCode::UNPROCESSABLE_ENTITY, "H8-422", message),
            Self::Conflict(message) => (StatusCode::CONFLICT, "H8-409", message.to_string()),
            Self::OrderNotReady => (
                StatusCode::TOO_EARLY,
                "ORDER_NOT_READY",
                "ERP order is not ready in WMS".to_string(),
            ),
            Self::Internal(message) => {
                tracing::error!(target: "h8.inbound", error = %message, "H8 inbound failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "H8-500",
                    "H8 inbound persistence failed".to_string(),
                )
            }
            Self::Auth(_) => unreachable!(),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message,
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: Uuid::new_v4().to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

pub fn h8_inbound_router(state: H8InboundAppState) -> Router {
    Router::new()
        .route(ASN_PATH, post(push_asn))
        .route(OUTBOUND_ORDER_PATH, post(push_outbound_order))
        .route(PRODUCT_CHANGE_PATH, post(push_product_change))
        .route(PRODUCT_MASTER_PATH, post(push_product_master))
        .route(CUSTOMER_MASTER_PATH, post(push_customer_master))
        .route(SUPPLIER_MASTER_PATH, post(push_supplier_master))
        .route(INVENTORY_SEED_PATH, post(push_inventory_seed_snapshot))
        .route(ORDER_CANCEL_PATH, post(push_order_cancel))
        .route(RETURN_ORDER_PATH, post(push_return_order))
        .with_state(state)
}

async fn push_asn(
    ctx: AuthContext,
    State(state): State<H8InboundAppState>,
    headers: HeaderMap,
    Json(body): Json<H8AsnInboundRequest>,
) -> Result<Json<H8InboundResponse>, H8InboundError> {
    ctx.require_permission("m2.write")?;
    let warehouse_id = resolve_warehouse(&state, &ctx, &body.depot_code).await?;
    validate_asn_request(&ctx, &body, warehouse_id)?;
    let (document_type, supplier_id, partner_id) = resolve_asn_partner(&state, &ctx, &body).await?;
    let idempotency_key = idempotency_key(&headers)?;
    let mut prepared = prepare_message(
        &state,
        &ctx,
        idempotency_key,
        InboundMetadata {
            message_type: "asn",
            schema_version: body.schema_version.clone(),
            external_ref: body.external_ref.clone(),
            correlation_id: body.correlation_id.clone(),
            warehouse_id: Some(warehouse_id),
            payload_digest: validate_payload_digest(&body.payload_digest)?,
        },
    )
    .await?;
    if prepared.message.sync_status == "succeeded" {
        return Ok(Json(response(prepared.message, true)?));
    }
    prepared.message = record_convert_message(&state, &ctx, prepared.message).await?;
    let now = Utc::now();
    let mut resource_ids = Vec::with_capacity(body.lines.len());
    let mut business_replayed = true;
    for line in &body.lines {
        let request = CreateReceivingOrderRequest {
            receipt_no: format!("{}-{}", body.erp_bill_code, line.line_no),
            document_type: document_type.to_string(),
            supplier_id,
            warehouse_id,
            external_ref: Some(format!(
                "{}:r{}:l{}",
                body.erp_bill_code, body.revision, line.line_no
            )),
            expected_arrival_at: Some(now),
            lines: vec![ReceivingOrderLine {
                line_no: line.line_no,
                product_id: None,
                product_code: line.product_code.clone(),
                expected_qty: line.expected_qty,
                batch_no: line.batch_no.clone(),
                production_date: line.production_date.map(|value| value.to_string()),
                expiry_date: line.expiry_date.map(|value| value.to_string()),
            }],
        };
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "create",
            "M2",
            "receiving_order",
            "pending",
            None,
        );
        let line_key = format!("{idempotency_key}:line:{}", line.line_no);
        let outcome = match state
            .wave3
            .create_receiving_order_with_audit(&ctx, request, now, &line_key, audit)
            .await
        {
            Ok(outcome) => outcome,
            Err(error) => {
                let error = map_wave3_error(error);
                fail_message(&state, &ctx, prepared.message.id, &error).await?;
                return Err(error);
            }
        };
        if let Err(error) = state
            .wave3
            .attach_erp_receiving_identity(
                ctx.owner_id,
                outcome.value.id,
                body.erp_bill_id,
                &body.erp_bill_code,
                body.revision,
                i32::try_from(line.line_no)
                    .map_err(|_| H8InboundError::Unprocessable("line_no is invalid".to_string()))?,
                body.partner_type.as_deref(),
                partner_id,
                body.partner_code.as_deref(),
                &body.correlation_id,
            )
            .await
        {
            let error = map_wave3_error(error);
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
        business_replayed &= outcome.replayed;
        resource_ids.push(outcome.value.id.to_string());
    }
    let resource_id = resource_ids.join(",");
    prepared.message = succeed_message(&state, &ctx, prepared.message.id, &resource_id).await?;
    Ok(Json(response(prepared.message, business_replayed)?))
}

async fn push_outbound_order(
    ctx: AuthContext,
    State(state): State<H8InboundAppState>,
    headers: HeaderMap,
    Json(body): Json<H8OutboundOrderInboundRequest>,
) -> Result<Json<H8InboundResponse>, H8InboundError> {
    ctx.require_permission("m4.write")?;
    let warehouse_id = resolve_warehouse(&state, &ctx, &body.depot_code).await?;
    validate_outbound_order_request(&ctx, &body, warehouse_id)?;
    let customer_id = state
        .master_data
        .resolve_active_customer_id(ctx.owner_id, &body.customer_code)
        .await
        .map_err(map_master_data_error)?
        .ok_or_else(|| H8InboundError::Unprocessable("customer_code is unknown".to_string()))?;
    let idempotency_key = idempotency_key(&headers)?;
    let mut prepared = prepare_message(
        &state,
        &ctx,
        idempotency_key,
        InboundMetadata {
            message_type: "outbound_order",
            schema_version: body.schema_version.clone(),
            external_ref: body.external_ref.clone(),
            correlation_id: body.correlation_id.clone(),
            warehouse_id: Some(warehouse_id),
            payload_digest: validate_payload_digest(&body.payload_digest)?,
        },
    )
    .await?;
    if prepared.message.sync_status == "succeeded" {
        return Ok(Json(response(prepared.message, true)?));
    }
    let delivery_address_id = state
        .master_data
        .upsert_erp_customer_address(
            ctx.owner_id,
            customer_id,
            body.erp_address_id,
            &body.address_code,
            &body.address,
            body.contact_name.as_deref(),
            body.contact_phone.as_deref(),
            Utc::now(),
        )
        .await
        .map_err(map_master_data_error)?;
    let request = CreateOutboundOrderRequest {
        document_type: outbound_document_type(body.order_type)?.to_string(),
        wms_order_no: format!("{}-R{}", body.erp_bill_code, body.revision),
        erp_order_no: Some(body.erp_bill_code.clone()),
        invoice_no: None,
        transport_mode_code: body.send_mode.map(|value| value.to_string()),
        department_code: None,
        sales_group_code: None,
        order_group_no: None,
        business_type_code: None,
        customer_id,
        delivery_address_id,
        warehouse_id,
        required_ship_at: Some(body.required_ship_at),
        lines: body
            .lines
            .iter()
            .map(|line| CreateOutboundOrderLineRequest {
                line_no: line.line_no,
                product_code: line.product_code.clone(),
                batch_no: line.batch_no.clone(),
                planned_qty: line.planned_qty,
            })
            .collect(),
    };
    prepared.message = record_convert_message(&state, &ctx, prepared.message).await?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create_outbound_order",
        "M4",
        "outbound_order",
        request.wms_order_no.clone(),
        None,
    );
    let outcome = match state
        .wave4
        .create_outbound_order(&ctx, request, Utc::now(), idempotency_key, Some(audit))
        .await
    {
        Ok(outcome) => outcome,
        Err(error) => {
            let error = map_wave4_error(error);
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    if let Err(error) = state
        .wave4
        .attach_erp_outbound_identity(
            ctx.owner_id,
            outcome.value.id,
            body.erp_bill_id,
            &body.erp_bill_code,
            body.revision,
            body.order_type,
            body.send_mode,
            &body.correlation_id,
        )
        .await
    {
        let error = map_wave4_error(error);
        fail_message(&state, &ctx, prepared.message.id, &error).await?;
        return Err(error);
    }
    prepared.message = succeed_message(
        &state,
        &ctx,
        prepared.message.id,
        &outcome.value.id.to_string(),
    )
    .await?;
    Ok(Json(response(prepared.message, outcome.replayed)?))
}

fn validate_asn_request(
    ctx: &AuthContext,
    body: &H8AsnInboundRequest,
    warehouse_id: Uuid,
) -> Result<(), H8InboundError> {
    validate_envelope(
        ctx,
        &body.schema_version,
        &body.external_ref,
        &body.correlation_id,
        Some(warehouse_id),
    )?;
    if body.erp_bill_id <= 0
        || body.erp_bill_code.trim().is_empty()
        || body.revision <= 0
        || body.depot_code.trim().is_empty()
        || body.lines.is_empty()
        || body.lines.iter().any(|line| {
            line.line_no == 0
                || line.product_code.trim().is_empty()
                || line.expected_qty <= wms_domain::Quantity::ZERO
        })
    {
        return Err(H8InboundError::Unprocessable(
            "required ASN field is invalid".to_string(),
        ));
    }
    Ok(())
}

fn validate_outbound_order_request(
    ctx: &AuthContext,
    body: &H8OutboundOrderInboundRequest,
    warehouse_id: Uuid,
) -> Result<(), H8InboundError> {
    validate_envelope(
        ctx,
        &body.schema_version,
        &body.external_ref,
        &body.correlation_id,
        Some(warehouse_id),
    )?;
    if body.erp_bill_id <= 0
        || body.erp_bill_code.trim().is_empty()
        || body.revision <= 0
        || body.customer_code.trim().is_empty()
        || body.depot_code.trim().is_empty()
        || body.erp_address_id <= 0
        || body.address_code.trim().is_empty()
        || body.address.trim().is_empty()
        || body.lines.is_empty()
        || body.lines.iter().any(|line| {
            line.line_no == 0
                || line.product_code.trim().is_empty()
                || line.batch_no.trim().is_empty()
                || line.planned_qty <= wms_domain::Quantity::ZERO
        })
    {
        return Err(H8InboundError::Unprocessable(
            "required outbound order field is invalid".to_string(),
        ));
    }
    Ok(())
}

fn validate_envelope(
    ctx: &AuthContext,
    schema_version: &str,
    external_ref: &str,
    correlation_id: &str,
    warehouse_id: Option<Uuid>,
) -> Result<(), H8InboundError> {
    validate_schema_version(schema_version.trim())
        .map_err(|error| H8InboundError::Unprocessable(format!("{error:?}")))?;
    if matches!(
        (ctx.warehouse_scope, warehouse_id),
        (Some(_), None) | (Some(_), Some(_))
    ) && ctx.warehouse_scope != warehouse_id
    {
        return Err(AuthError::PermissionDenied("warehouse scope".to_string()).into());
    }
    if external_ref.trim().is_empty() || correlation_id.trim().is_empty() {
        return Err(H8InboundError::Unprocessable(
            "required H8 envelope field is invalid".to_string(),
        ));
    }
    Ok(())
}

async fn resolve_warehouse(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    depot_code: &str,
) -> Result<Uuid, H8InboundError> {
    state
        .master_data
        .resolve_active_warehouse_id(ctx.owner_id, depot_code)
        .await
        .map_err(map_master_data_error)?
        .ok_or_else(|| H8InboundError::Unprocessable("depot_code is unknown".to_string()))
}

async fn resolve_asn_partner(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    body: &H8AsnInboundRequest,
) -> Result<(&'static str, Option<Uuid>, Option<Uuid>), H8InboundError> {
    match body.order_type {
        1 => {
            if body.partner_type.as_deref() != Some("supplier") {
                return Err(H8InboundError::Unprocessable(
                    "purchase inbound requires supplier".to_string(),
                ));
            }
            let code = body.partner_code.as_deref().ok_or_else(|| {
                H8InboundError::Unprocessable("partner_code is required".to_string())
            })?;
            let id = state
                .master_data
                .resolve_active_supplier_id(ctx.owner_id, code)
                .await
                .map_err(map_master_data_error)?
                .ok_or_else(|| {
                    H8InboundError::Unprocessable("supplier code is unknown".to_string())
                })?;
            Ok(("purchase_inbound", Some(id), Some(id)))
        }
        2 => {
            if body.partner_type.as_deref() != Some("customer") {
                return Err(H8InboundError::Unprocessable(
                    "sales return requires customer".to_string(),
                ));
            }
            let code = body.partner_code.as_deref().ok_or_else(|| {
                H8InboundError::Unprocessable("partner_code is required".to_string())
            })?;
            let id = state
                .master_data
                .resolve_active_customer_id(ctx.owner_id, code)
                .await
                .map_err(map_master_data_error)?
                .ok_or_else(|| {
                    H8InboundError::Unprocessable("customer code is unknown".to_string())
                })?;
            Ok(("sales_return", None, Some(id)))
        }
        3 if body.partner_type.is_none() && body.partner_code.is_none() => {
            Ok(("other_inbound", None, None))
        }
        _ => Err(H8InboundError::Unprocessable(
            "order_type or partner is invalid".to_string(),
        )),
    }
}

fn outbound_document_type(order_type: i32) -> Result<&'static str, H8InboundError> {
    match order_type {
        1 => Ok("sales_outbound"),
        2 => Ok("sample_outbound"),
        3 => Ok("other_outbound"),
        _ => Err(H8InboundError::Unprocessable(
            "order_type is invalid".to_string(),
        )),
    }
}

async fn map_document_type(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    source_value: &str,
    external_ref: &str,
    connector_code: &str,
    idempotency_key: &str,
) -> Result<String, H8InboundError> {
    let mapped = map_parameter(
        &state.mappings,
        ctx,
        &MapParameterRequest {
            dict_code: "document_type".to_string(),
            source_value: source_value.to_string(),
            source_system: Some(connector_code.to_string()),
            source_record_id: Some(external_ref.to_string()),
        },
        &format!("{idempotency_key}:mpm:document_type"),
    )
    .await
    .map_err(map_parameter_error)?;
    if mapped.status != ParameterMappingStatus::Matched {
        return Err(H8InboundError::Unprocessable(
            "document_type mapping is unresolved".to_string(),
        ));
    }
    mapped.target_value.ok_or_else(|| {
        H8InboundError::Unprocessable("document_type mapping has no target".to_string())
    })
}

fn receiving_request(
    command: &H8CanonicalInboundCommand,
) -> Result<CreateReceivingOrderRequest, H8InboundError> {
    let string = |field: &'static str| {
        command
            .fields
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| H8InboundError::Unprocessable(format!("{field} is invalid")))
    };
    Ok(CreateReceivingOrderRequest {
        receipt_no: string("receipt_no")?,
        document_type: string("document_type")?,
        supplier_id: command
            .fields
            .get("supplier_id")
            .and_then(Value::as_str)
            .map(str::parse)
            .transpose()
            .map_err(|_| H8InboundError::Unprocessable("supplier_id is invalid".to_string()))?,
        warehouse_id: command
            .warehouse_id
            .ok_or_else(|| H8InboundError::Unprocessable("warehouse_id is required".to_string()))?,
        external_ref: Some(command.external_ref.clone()),
        expected_arrival_at: Some(string("expected_arrival_at")?.parse().map_err(|_| {
            H8InboundError::Unprocessable("expected_arrival_at is invalid".to_string())
        })?),
        lines: vec![ReceivingOrderLine {
            line_no: 1,
            product_id: None,
            product_code: string("product_code")?,
            expected_qty: string("expected_qty")?.parse().map_err(|_| {
                H8InboundError::Unprocessable("expected_qty is invalid".to_string())
            })?,
            batch_no: command
                .fields
                .get("batch_no")
                .and_then(Value::as_str)
                .map(str::to_string),
            production_date: None,
            expiry_date: None,
        }],
    })
}

async fn fail_message(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    message_id: Uuid,
    error: &H8InboundError,
) -> Result<(), H8InboundError> {
    let message = state
        .messages
        .repository
        .get(ctx.owner_id, message_id)
        .await
        .map_err(|error| H8InboundError::Internal(format!("{error:?}")))?;
    crate::h8_erp_messages::apply_lifecycle_failure(
        &state.messages,
        ctx,
        message,
        &format!("{error:?}"),
        error.error_class(),
        Utc::now(),
    )
    .await
    .map_err(|error| H8InboundError::Internal(format!("{error:?}")))?;
    Ok(())
}

fn response(message: H8ErpMessage, replayed: bool) -> Result<H8InboundResponse, H8InboundError> {
    Ok(H8InboundResponse {
        message_id: message.id,
        wms_resource_id: message
            .wms_resource_id
            .ok_or_else(|| H8InboundError::Internal("missing WMS resource id".to_string()))?,
        status: message.sync_status,
        replayed,
        ignored_old_version: false,
    })
}

fn map_parameter_error(error: ParameterMappingHandlerError) -> H8InboundError {
    match error {
        ParameterMappingHandlerError::IdempotencyConflict => {
            H8InboundError::Conflict("M-PM idempotency conflict")
        }
        ParameterMappingHandlerError::Persistence(message) => H8InboundError::Internal(message),
        other => H8InboundError::Unprocessable(format!("{other:?}")),
    }
}

fn map_wave3_error(error: Wave3RepositoryError) -> H8InboundError {
    match error {
        Wave3RepositoryError::Database(message)
        | Wave3RepositoryError::Audit(message)
        | Wave3RepositoryError::Serialize(message) => H8InboundError::Internal(message),
        Wave3RepositoryError::IdempotencyConflict => {
            H8InboundError::Conflict("business idempotency conflict")
        }
        other => H8InboundError::Unprocessable(format!("{other:?}")),
    }
}

fn map_wave4_error(error: Wave4RepositoryError) -> H8InboundError {
    match error {
        Wave4RepositoryError::Database(message)
        | Wave4RepositoryError::Audit(message)
        | Wave4RepositoryError::Serialize(message) => H8InboundError::Internal(message),
        Wave4RepositoryError::IdempotencyConflict => {
            H8InboundError::Conflict("business idempotency conflict")
        }
        other => H8InboundError::Unprocessable(format!("{other:?}")),
    }
}

fn map_master_data_error(error: MasterDataError) -> H8InboundError {
    match error {
        MasterDataError::Database(message)
        | MasterDataError::Audit(message)
        | MasterDataError::Serialize(message) => H8InboundError::Internal(message),
        MasterDataError::IdempotencyConflict => {
            H8InboundError::Conflict("business idempotency conflict")
        }
        other => H8InboundError::Unprocessable(format!("{other:?}")),
    }
}
