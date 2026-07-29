use axum::{extract::State, http::HeaderMap, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;
use uuid::Uuid;
use wms_domain::H8CanonicalInboundCommand;

use super::{
    fail_message,
    lifecycle::{
        idempotency_key, payload_digest, prepare_message, record_convert_message, succeed_message,
        InboundMetadata,
    },
    map_document_type, map_wave3_error, receiving_request, response, validate_envelope,
    AuditWriteRequest, AuthContext, H8InboundAppState, H8InboundError, H8InboundResponse,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ReturnOrderInboundRequest {
    pub schema_version: String,
    pub external_ref: String,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    pub warehouse_id: Uuid,
    pub receipt_no: Option<String>,
    pub document_type: String,
    pub customer_id: Uuid,
    pub supplier_id: Option<Uuid>,
    pub product_code: String,
    pub expected_qty: i64,
    pub expected_arrival_at: DateTime<Utc>,
    pub batch_no: String,
}

pub(super) async fn push_return_order(
    ctx: AuthContext,
    State(state): State<H8InboundAppState>,
    headers: HeaderMap,
    Json(body): Json<H8ReturnOrderInboundRequest>,
) -> Result<Json<H8InboundResponse>, H8InboundError> {
    ctx.require_permission("m2.write")?;
    validate_request(&ctx, &body)?;
    let idempotency_key = idempotency_key(&headers)?;
    let mut prepared = prepare_message(
        &state,
        &ctx,
        idempotency_key,
        InboundMetadata {
            message_type: "return_order",
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
    let command = match canonical_command(
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
        .create_receiving_order_with_audit(&ctx, request, Utc::now(), idempotency_key, audit)
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

fn validate_request(
    ctx: &AuthContext,
    body: &H8ReturnOrderInboundRequest,
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
        || body.supplier_id.is_some_and(|id| id.is_nil())
        || body.product_code.trim().is_empty()
        || body.expected_qty <= 0
        || body.batch_no.trim().is_empty()
    {
        return Err(H8InboundError::Unprocessable(
            "required return order field is invalid".to_string(),
        ));
    }
    Ok(())
}

async fn canonical_command(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    body: &H8ReturnOrderInboundRequest,
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
        Value::String(
            body.receipt_no
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("ERP-RET-{}", body.external_ref)),
        ),
    );
    fields.insert("document_type".to_string(), Value::String(document_type));
    fields.insert(
        "supplier_id".to_string(),
        Value::String(body.supplier_id.unwrap_or(body.customer_id).to_string()),
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
    fields.insert("batch_no".to_string(), Value::String(body.batch_no.clone()));
    Ok(H8CanonicalInboundCommand {
        owner_id: ctx.owner_id,
        warehouse_id: Some(body.warehouse_id),
        message_type: "return_order".to_string(),
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
