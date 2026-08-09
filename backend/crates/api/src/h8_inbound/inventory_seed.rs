use std::collections::HashSet;

use axum::{extract::State, http::HeaderMap, Json};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use wms_domain::Quantity;

use crate::{
    auth::AuthContext,
    wave3_repository::{ErpInventorySeedItem, ErpInventorySeedSnapshot},
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
pub struct H8InventorySeedSnapshotInboundRequest {
    pub schema_version: String,
    pub external_ref: String,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    pub payload_digest: String,
    pub source_version: Option<i64>,
    pub snapshot_id: String,
    pub depot_code: String,
    pub push_type: i32,
    pub push_time: DateTime<Utc>,
    pub items: Vec<H8InventorySeedItemInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8InventorySeedItemInput {
    pub row_no: i32,
    pub product_code: String,
    pub batch_no: String,
    pub expiry_date: Option<NaiveDate>,
    pub location_code: Option<String>,
    pub goods_status: Option<String>,
    #[schema(value_type = String, format = "decimal")]
    pub quantity: Quantity,
}

pub(super) async fn push_inventory_seed_snapshot(
    ctx: AuthContext,
    State(state): State<H8InboundAppState>,
    headers: HeaderMap,
    Json(body): Json<H8InventorySeedSnapshotInboundRequest>,
) -> Result<Json<H8InboundResponse>, H8InboundError> {
    ctx.require_permission("m3.write")?;
    let warehouse_id = super::resolve_warehouse(&state, &ctx, &body.depot_code).await?;
    validate_request(&ctx, &body, warehouse_id)?;
    let key = idempotency_key(&headers)?;
    let digest = validate_payload_digest(&body.payload_digest)?;
    let mut prepared = prepare_message(
        &state,
        &ctx,
        key,
        InboundMetadata {
            message_type: "inventory_seed_snapshot",
            schema_version: body.schema_version.clone(),
            external_ref: body.external_ref.clone(),
            correlation_id: body.correlation_id.clone(),
            warehouse_id: Some(warehouse_id),
            payload_digest: digest.clone(),
        },
    )
    .await?;
    if prepared.message.sync_status == "succeeded" {
        return Ok(Json(response(prepared.message, true)?));
    }
    prepared.message = record_convert_message(&state, &ctx, prepared.message).await?;
    let snapshot = ErpInventorySeedSnapshot {
        snapshot_id: body.snapshot_id.clone(),
        warehouse_id,
        push_type: body.push_type,
        push_time: body.push_time,
        payload_digest: digest,
        items: body
            .items
            .iter()
            .map(|item| ErpInventorySeedItem {
                row_no: item.row_no,
                product_code: item.product_code.clone(),
                batch_no: item.batch_no.clone(),
                expiry_date: item.expiry_date,
                location_code: item.location_code.clone(),
                goods_status: item.goods_status.clone(),
                quantity: item.quantity,
            })
            .collect(),
    };
    let outcome = match state
        .wave3
        .stage_erp_inventory_snapshot(&ctx, snapshot, Utc::now())
        .await
    {
        Ok(outcome) => outcome,
        Err(error) => {
            let error = super::map_wave3_error(error);
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    prepared.message = succeed_message(
        &state,
        &ctx,
        prepared.message.id,
        &outcome.value.to_string(),
    )
    .await?;
    Ok(Json(response(prepared.message, outcome.replayed)?))
}

fn validate_request(
    ctx: &AuthContext,
    body: &H8InventorySeedSnapshotInboundRequest,
    warehouse_id: uuid::Uuid,
) -> Result<(), H8InboundError> {
    validate_envelope(
        ctx,
        &body.schema_version,
        &body.external_ref,
        &body.correlation_id,
        Some(warehouse_id),
    )?;
    let mut rows = HashSet::with_capacity(body.items.len());
    let invalid_item = body.items.iter().any(|item| {
        item.row_no <= 0
            || !rows.insert(item.row_no)
            || item.product_code.trim().is_empty()
            || item.batch_no.trim().is_empty()
            || item.quantity < Quantity::ZERO
    });
    if body.snapshot_id.trim().is_empty()
        || body.external_ref != body.snapshot_id
        || !matches!(body.push_type, 1 | 2)
        || invalid_item
    {
        return Err(H8InboundError::Unprocessable(
            "inventory snapshot is invalid".to_string(),
        ));
    }
    Ok(())
}
