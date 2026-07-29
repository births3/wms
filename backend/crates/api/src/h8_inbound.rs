//! US-H8-002：ERP REST 入站防腐层。

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
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

mod lifecycle;
mod product_change;
mod product_master;
mod return_order;

use lifecycle::{
    idempotency_key, payload_digest, prepare_message, record_convert_message, succeed_message,
    InboundMetadata,
};
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
    pub warehouse_id: Uuid,
    pub receipt_no: String,
    pub document_type: String,
    pub supplier_id: Uuid,
    pub product_code: String,
    pub expected_qty: i64,
    pub expected_arrival_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8OutboundOrderInboundRequest {
    pub schema_version: String,
    pub external_ref: String,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    pub warehouse_id: Uuid,
    pub wms_order_no: Option<String>,
    pub document_type: String,
    pub erp_order_no: Option<String>,
    pub customer_id: Uuid,
    pub delivery_address_id: Uuid,
    pub product_code: String,
    pub batch_no: String,
    pub planned_qty: i64,
    pub required_ship_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct H8InboundResponse {
    pub message_id: Uuid,
    pub wms_resource_id: String,
    pub status: String,
    pub replayed: bool,
}

#[derive(Debug)]
enum H8InboundError {
    Auth(AuthError),
    BadRequest(&'static str),
    Unprocessable(String),
    Conflict(&'static str),
    Internal(String),
}

impl H8InboundError {
    fn error_class(&self) -> H8ErrorClass {
        match self {
            Self::Internal(_) => H8ErrorClass::Retryable,
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
    validate_asn_request(&ctx, &body)?;
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
            warehouse_id: Some(body.warehouse_id),
            payload_digest: payload_digest(&body)?,
        },
    )
    .await?;
    if prepared.message.sync_status == "succeeded" {
        return Ok(Json(response(prepared.message, true)?));
    }
    let now = Utc::now();
    let command = match canonical_asn(
        &state,
        &ctx,
        &body,
        idempotency_key,
        prepared.connector_id,
        prepared.config_version,
        &prepared.connector_code,
    )
    .await
    {
        Ok(command) => command,
        Err(error) => {
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    let request = match receiving_request(&command) {
        Ok(request) => request,
        Err(error) => {
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    prepared.message = record_convert_message(&state, &ctx, prepared.message).await?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create",
        "M2",
        "receiving_order",
        "pending",
        None,
    );
    let outcome = match state
        .wave3
        .create_receiving_order_with_audit(&ctx, request, now, idempotency_key, audit)
        .await
    {
        Ok(outcome) => outcome,
        Err(error) => {
            let error = map_wave3_error(error);
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    prepared.message = succeed_message(
        &state,
        &ctx,
        prepared.message.id,
        &outcome.value.id.to_string(),
    )
    .await?;
    Ok(Json(response(prepared.message, outcome.replayed)?))
}

async fn push_outbound_order(
    ctx: AuthContext,
    State(state): State<H8InboundAppState>,
    headers: HeaderMap,
    Json(body): Json<H8OutboundOrderInboundRequest>,
) -> Result<Json<H8InboundResponse>, H8InboundError> {
    ctx.require_permission("m4.write")?;
    validate_outbound_order_request(&ctx, &body)?;
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
            warehouse_id: Some(body.warehouse_id),
            payload_digest: payload_digest(&body)?,
        },
    )
    .await?;
    if prepared.message.sync_status == "succeeded" {
        return Ok(Json(response(prepared.message, true)?));
    }
    let command = match canonical_outbound_order(
        &state,
        &ctx,
        &body,
        idempotency_key,
        prepared.connector_id,
        prepared.config_version,
        &prepared.connector_code,
    )
    .await
    {
        Ok(command) => command,
        Err(error) => {
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    let request = match outbound_order_request(&command) {
        Ok(request) => request,
        Err(error) => {
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
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
) -> Result<(), H8InboundError> {
    validate_envelope(
        ctx,
        &body.schema_version,
        &body.external_ref,
        &body.correlation_id,
        Some(body.warehouse_id),
    )?;
    if body.external_ref.trim().is_empty()
        || body.receipt_no.trim().is_empty()
        || body.document_type.trim().is_empty()
        || body.product_code.trim().is_empty()
        || body.expected_qty <= 0
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
) -> Result<(), H8InboundError> {
    validate_envelope(
        ctx,
        &body.schema_version,
        &body.external_ref,
        &body.correlation_id,
        Some(body.warehouse_id),
    )?;
    if body.document_type.trim().is_empty()
        || body.customer_id.is_nil()
        || body.delivery_address_id.is_nil()
        || body.product_code.trim().is_empty()
        || body.batch_no.trim().is_empty()
        || body.planned_qty <= 0
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

async fn canonical_asn(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    body: &H8AsnInboundRequest,
    idempotency_key: &str,
    connector_id: Uuid,
    config_version: i64,
    connector_code: &str,
) -> Result<H8CanonicalInboundCommand, H8InboundError> {
    let document_type = map_document_type(
        state,
        ctx,
        &body.document_type,
        &body.external_ref,
        connector_code,
        idempotency_key,
    )
    .await?;
    let mut fields = Map::new();
    fields.insert(
        "receipt_no".to_string(),
        Value::String(body.receipt_no.clone()),
    );
    fields.insert("document_type".to_string(), Value::String(document_type));
    fields.insert(
        "supplier_id".to_string(),
        Value::String(body.supplier_id.to_string()),
    );
    fields.insert(
        "product_code".to_string(),
        Value::String(body.product_code.clone()),
    );
    fields.insert("expected_qty".to_string(), Value::from(body.expected_qty));
    fields.insert(
        "expected_arrival_at".to_string(),
        Value::String(body.expected_arrival_at.to_rfc3339()),
    );
    Ok(H8CanonicalInboundCommand {
        owner_id: ctx.owner_id,
        warehouse_id: Some(body.warehouse_id),
        message_type: "asn".to_string(),
        external_ref: body.external_ref.clone(),
        idempotency_key: idempotency_key.to_string(),
        correlation_id: body.correlation_id.clone(),
        connector_id,
        config_version,
        channel: "rest".to_string(),
        fields,
        occurred_at: body.occurred_at,
    })
}

async fn canonical_outbound_order(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    body: &H8OutboundOrderInboundRequest,
    idempotency_key: &str,
    connector_id: Uuid,
    config_version: i64,
    connector_code: &str,
) -> Result<H8CanonicalInboundCommand, H8InboundError> {
    let document_type = map_document_type(
        state,
        ctx,
        &body.document_type,
        &body.external_ref,
        connector_code,
        idempotency_key,
    )
    .await?;
    let mut fields = Map::new();
    fields.insert(
        "wms_order_no".to_string(),
        Value::String(
            body.wms_order_no
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("WMS-{}", body.external_ref)),
        ),
    );
    fields.insert("document_type".to_string(), Value::String(document_type));
    fields.insert(
        "erp_order_no".to_string(),
        Value::String(
            body.erp_order_no
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| body.external_ref.clone()),
        ),
    );
    fields.insert(
        "customer_id".to_string(),
        Value::String(body.customer_id.to_string()),
    );
    fields.insert(
        "delivery_address_id".to_string(),
        Value::String(body.delivery_address_id.to_string()),
    );
    fields.insert(
        "product_code".to_string(),
        Value::String(body.product_code.clone()),
    );
    fields.insert("batch_no".to_string(), Value::String(body.batch_no.clone()));
    fields.insert("planned_qty".to_string(), Value::from(body.planned_qty));
    fields.insert(
        "required_ship_at".to_string(),
        body.required_ship_at
            .map(|value| Value::String(value.to_rfc3339()))
            .unwrap_or(Value::Null),
    );
    Ok(H8CanonicalInboundCommand {
        owner_id: ctx.owner_id,
        warehouse_id: Some(body.warehouse_id),
        message_type: "outbound_order".to_string(),
        external_ref: body.external_ref.clone(),
        idempotency_key: idempotency_key.to_string(),
        correlation_id: body.correlation_id.clone(),
        connector_id,
        config_version,
        channel: "rest".to_string(),
        fields,
        occurred_at: body.occurred_at,
    })
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
        supplier_id: Some(
            string("supplier_id")?
                .parse()
                .map_err(|_| H8InboundError::Unprocessable("supplier_id is invalid".to_string()))?,
        ),
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
            expected_qty: command
                .fields
                .get("expected_qty")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
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

fn outbound_order_request(
    command: &H8CanonicalInboundCommand,
) -> Result<CreateOutboundOrderRequest, H8InboundError> {
    let string = |field: &'static str| {
        command
            .fields
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| H8InboundError::Unprocessable(format!("{field} is invalid")))
    };
    let optional_string = |field: &'static str| {
        command
            .fields
            .get(field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    Ok(CreateOutboundOrderRequest {
        document_type: string("document_type")?,
        wms_order_no: string("wms_order_no")?,
        erp_order_no: Some(string("erp_order_no")?),
        invoice_no: optional_string("invoice_no"),
        transport_mode_code: optional_string("transport_mode_code"),
        department_code: optional_string("department_code"),
        sales_group_code: optional_string("sales_group_code"),
        order_group_no: optional_string("order_group_no"),
        business_type_code: optional_string("business_type_code"),
        customer_id: string("customer_id")?
            .parse()
            .map_err(|_| H8InboundError::Unprocessable("customer_id is invalid".to_string()))?,
        warehouse_id: command
            .warehouse_id
            .ok_or_else(|| H8InboundError::Unprocessable("warehouse_id is required".to_string()))?,
        delivery_address_id: string("delivery_address_id")?.parse().map_err(|_| {
            H8InboundError::Unprocessable("delivery_address_id is invalid".to_string())
        })?,
        required_ship_at: command
            .fields
            .get("required_ship_at")
            .and_then(Value::as_str)
            .map(str::parse)
            .transpose()
            .map_err(|_| {
                H8InboundError::Unprocessable("required_ship_at is invalid".to_string())
            })?,
        lines: vec![CreateOutboundOrderLineRequest {
            line_no: 1,
            product_code: string("product_code")?,
            batch_no: string("batch_no")?,
            planned_qty: command
                .fields
                .get("planned_qty")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    H8InboundError::Unprocessable("planned_qty is invalid".to_string())
                })?,
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
