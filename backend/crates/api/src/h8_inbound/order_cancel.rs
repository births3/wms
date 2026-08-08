use axum::{extract::State, http::HeaderMap, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    auth::AuthContext, wave3_repository::Wave3RepositoryError,
    wave4_repository::Wave4RepositoryError,
};

use super::{
    fail_message,
    lifecycle::{
        idempotency_key, prepare_message, record_convert_message, succeed_message,
        validate_payload_digest, InboundMetadata,
    },
    response, validate_envelope, H8InboundAppState, H8InboundError, H8InboundResponse,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8OrderCancelInboundRequest {
    pub schema_version: String,
    pub external_ref: String,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    pub payload_digest: String,
    pub source_version: Option<i64>,
    pub command_id: String,
    pub command_type: i32,
    pub erp_bill_code: String,
    pub revision: i32,
    pub order_type: i32,
    pub memo: Option<String>,
}

pub(super) async fn push_order_cancel(
    ctx: AuthContext,
    State(state): State<H8InboundAppState>,
    headers: HeaderMap,
    Json(body): Json<H8OrderCancelInboundRequest>,
) -> Result<Json<H8InboundResponse>, H8InboundError> {
    match body.order_type {
        1 => ctx.require_permission("m2.write")?,
        2 => ctx.require_permission("m4.write")?,
        _ => {
            return Err(H8InboundError::Unprocessable(
                "order_type must be 1 or 2".to_string(),
            ))
        }
    }
    validate_request(&ctx, &body)?;
    let key = idempotency_key(&headers)?;
    if key != body.command_id {
        return Err(H8InboundError::Unprocessable(
            "Idempotency-Key must equal command_id".to_string(),
        ));
    }
    let mut prepared = prepare_message(
        &state,
        &ctx,
        key,
        InboundMetadata {
            message_type: "order_cancel",
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
    let now = Utc::now();
    let outcome = match body.order_type {
        1 => state
            .wave3
            .cancel_erp_receiving_order(
                &ctx,
                &body.erp_bill_code,
                body.revision,
                &body.command_id,
                &body.correlation_id,
                body.memo.as_deref(),
                now,
            )
            .await
            .map(|outcome| (outcome.value, outcome.replayed))
            .map_err(map_wave3_cancel_error),
        2 => state
            .wave4
            .cancel_erp_outbound_order(
                &ctx,
                &body.erp_bill_code,
                body.revision,
                &body.command_id,
                &body.correlation_id,
                body.memo.as_deref(),
                now,
            )
            .await
            .map(|outcome| (outcome.value, outcome.replayed))
            .map_err(map_wave4_cancel_error),
        _ => unreachable!(),
    };
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => {
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    prepared.message =
        succeed_message(&state, &ctx, prepared.message.id, &outcome.0.to_string()).await?;
    Ok(Json(response(prepared.message, outcome.1)?))
}

fn validate_request(
    ctx: &AuthContext,
    body: &H8OrderCancelInboundRequest,
) -> Result<(), H8InboundError> {
    validate_envelope(
        ctx,
        &body.schema_version,
        &body.external_ref,
        &body.correlation_id,
        None,
    )?;
    if body.command_type != 99
        || body.command_id.trim().is_empty()
        || body.external_ref != body.command_id
        || body.erp_bill_code.trim().is_empty()
        || body.revision <= 0
    {
        return Err(H8InboundError::Unprocessable(
            "order cancel command is invalid".to_string(),
        ));
    }
    Ok(())
}

fn map_wave3_cancel_error(error: Wave3RepositoryError) -> H8InboundError {
    if matches!(error, Wave3RepositoryError::NotFound) {
        H8InboundError::OrderNotReady
    } else {
        super::map_wave3_error(error)
    }
}

fn map_wave4_cancel_error(error: Wave4RepositoryError) -> H8InboundError {
    if matches!(error, Wave4RepositoryError::NotFound) {
        H8InboundError::OrderNotReady
    } else {
        super::map_wave4_error(error)
    }
}
