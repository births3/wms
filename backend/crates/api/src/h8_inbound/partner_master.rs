use axum::{extract::State, http::HeaderMap, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::to_value;
use utoipa::ToSchema;

use crate::{auth::AuthContext, master_data_postgres::ErpPartnerSnapshot};

use super::{
    fail_message,
    lifecycle::{
        idempotency_key, prepare_message, record_convert_message, succeed_message,
        validate_payload_digest, InboundMetadata,
    },
    response, validate_envelope, H8InboundAppState, H8InboundError, H8InboundResponse,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8CustomerMasterInboundRequest {
    pub schema_version: String,
    pub external_ref: String,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    pub payload_digest: String,
    pub source_version: i64,
    pub entity_id: i64,
    pub op_type: String,
    pub customer_code: Option<String>,
    pub customer_name: Option<String>,
    pub customer_type: Option<String>,
    pub address: Option<String>,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub delivery_address: Option<String>,
    pub delivery_contact: Option<String>,
    pub delivery_phone: Option<String>,
    pub delivery_mode: Option<i32>,
    pub stop_send: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8SupplierMasterInboundRequest {
    pub schema_version: String,
    pub external_ref: String,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    pub payload_digest: String,
    pub source_version: i64,
    pub entity_id: i64,
    pub op_type: String,
    pub supplier_code: Option<String>,
    pub supplier_name: Option<String>,
    pub address: Option<String>,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
}

pub(super) async fn push_customer_master(
    ctx: AuthContext,
    State(state): State<H8InboundAppState>,
    headers: HeaderMap,
    Json(body): Json<H8CustomerMasterInboundRequest>,
) -> Result<Json<H8InboundResponse>, H8InboundError> {
    ctx.require_permission("m1.master_data.write")?;
    validate_partner(
        &ctx,
        &body.schema_version,
        &body.external_ref,
        &body.correlation_id,
        body.entity_id,
        body.source_version,
        &body.op_type,
        body.customer_code.as_deref(),
        body.customer_name.as_deref(),
    )?;
    let key = idempotency_key(&headers)?;
    let mut prepared = prepare_message(
        &state,
        &ctx,
        key,
        InboundMetadata {
            message_type: "customer_master",
            schema_version: body.schema_version.clone(),
            external_ref: body.external_ref.clone(),
            correlation_id: body.correlation_id.clone(),
            warehouse_id: None,
            payload_digest: validate_payload_digest(&body.payload_digest)?,
        },
    )
    .await?;
    if prepared.message.sync_status == "succeeded" {
        return Ok(Json(response(prepared.message, true)?));
    }
    prepared.message = record_convert_message(&state, &ctx, prepared.message).await?;
    let snapshot = ErpPartnerSnapshot {
        entity_id: body.entity_id,
        source_version: body.source_version,
        code: body.customer_code.clone(),
        name: body.customer_name.clone(),
        contact_name: body.contact_name.clone(),
        contact_phone: body.contact_phone.clone(),
        payload: to_value(&body).map_err(|error| H8InboundError::Internal(error.to_string()))?,
    };
    let outcome = match state
        .master_data
        .apply_erp_customer_snapshot(&ctx, &body.op_type, snapshot, Utc::now())
        .await
    {
        Ok(outcome) => outcome,
        Err(error) => {
            let error = super::map_master_data_error(error);
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    let replayed = prepared.replayed;
    prepared.message =
        succeed_message(&state, &ctx, prepared.message.id, &outcome.id.to_string()).await?;
    let mut response = response(prepared.message, replayed)?;
    response.ignored_old_version = outcome.ignored_old_version;
    Ok(Json(response))
}

pub(super) async fn push_supplier_master(
    ctx: AuthContext,
    State(state): State<H8InboundAppState>,
    headers: HeaderMap,
    Json(body): Json<H8SupplierMasterInboundRequest>,
) -> Result<Json<H8InboundResponse>, H8InboundError> {
    ctx.require_permission("m1.master_data.write")?;
    validate_partner(
        &ctx,
        &body.schema_version,
        &body.external_ref,
        &body.correlation_id,
        body.entity_id,
        body.source_version,
        &body.op_type,
        body.supplier_code.as_deref(),
        body.supplier_name.as_deref(),
    )?;
    let key = idempotency_key(&headers)?;
    let mut prepared = prepare_message(
        &state,
        &ctx,
        key,
        InboundMetadata {
            message_type: "supplier_master",
            schema_version: body.schema_version.clone(),
            external_ref: body.external_ref.clone(),
            correlation_id: body.correlation_id.clone(),
            warehouse_id: None,
            payload_digest: validate_payload_digest(&body.payload_digest)?,
        },
    )
    .await?;
    if prepared.message.sync_status == "succeeded" {
        return Ok(Json(response(prepared.message, true)?));
    }
    prepared.message = record_convert_message(&state, &ctx, prepared.message).await?;
    let snapshot = ErpPartnerSnapshot {
        entity_id: body.entity_id,
        source_version: body.source_version,
        code: body.supplier_code.clone(),
        name: body.supplier_name.clone(),
        contact_name: body.contact_name.clone(),
        contact_phone: body.contact_phone.clone(),
        payload: to_value(&body).map_err(|error| H8InboundError::Internal(error.to_string()))?,
    };
    let outcome = match state
        .master_data
        .apply_erp_supplier_snapshot(&ctx, &body.op_type, snapshot, Utc::now())
        .await
    {
        Ok(outcome) => outcome,
        Err(error) => {
            let error = super::map_master_data_error(error);
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    let replayed = prepared.replayed;
    prepared.message =
        succeed_message(&state, &ctx, prepared.message.id, &outcome.id.to_string()).await?;
    let mut response = response(prepared.message, replayed)?;
    response.ignored_old_version = outcome.ignored_old_version;
    Ok(Json(response))
}

#[allow(clippy::too_many_arguments)]
fn validate_partner(
    ctx: &AuthContext,
    schema_version: &str,
    external_ref: &str,
    correlation_id: &str,
    entity_id: i64,
    source_version: i64,
    op_type: &str,
    code: Option<&str>,
    name: Option<&str>,
) -> Result<(), H8InboundError> {
    validate_envelope(ctx, schema_version, external_ref, correlation_id, None)?;
    let snapshot_missing = op_type != "D"
        && (code.map_or(true, |value| value.trim().is_empty())
            || name.map_or(true, |value| value.trim().is_empty()));
    if entity_id <= 0
        || source_version <= 0
        || !matches!(op_type, "I" | "U" | "D")
        || snapshot_missing
    {
        return Err(H8InboundError::Unprocessable(
            "required partner master field is invalid".to_string(),
        ));
    }
    Ok(())
}
