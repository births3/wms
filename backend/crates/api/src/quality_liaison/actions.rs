use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::StockLossReason;

use crate::{
    auth::AuthContext,
    stock_adjustment::quality_liaison::{
        create_approved_stock_loss_order_in_tx, ApprovedStockLossRequest,
    },
    stock_adjustment::StockAdjustmentError,
};

use super::{QualityLiaisonError, QualityLiaisonOrderRow};

pub(super) async fn apply_approved_action_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    liaison: &QualityLiaisonOrderRow,
    now: DateTime<Utc>,
) -> Result<(), QualityLiaisonError> {
    if liaison
        .business_payload
        .get("action")
        .and_then(serde_json::Value::as_str)
        != Some("create_stock_loss")
    {
        return Ok(());
    }
    let warehouse_id = payload_uuid(&liaison.business_payload, "warehouse_id")?;
    let batch_id = payload_uuid(&liaison.business_payload, "batch_id")?;
    let quantity = liaison
        .business_payload
        .get("quantity")
        .and_then(serde_json::Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or(QualityLiaisonError::BusinessActionInvalid)?;
    let reason = liaison
        .business_payload
        .get("reason_code")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| StockLossReason::try_from(value).ok())
        .filter(|value| value.is_destruction())
        .ok_or(QualityLiaisonError::BusinessActionInvalid)?;
    let recall_id = liaison
        .business_payload
        .get("recall_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    create_approved_stock_loss_order_in_tx(
        tx,
        ctx,
        ApprovedStockLossRequest {
            warehouse_id,
            batch_id,
            quantity,
            reason,
            recall_id,
            quality_liaison_id: liaison.id,
        },
        now,
    )
    .await
    .map(|_| ())
    .map_err(|error| match error {
        StockAdjustmentError::InvalidRequest
        | StockAdjustmentError::NotFound
        | StockAdjustmentError::QuantityExceeded => QualityLiaisonError::BusinessActionInvalid,
        _ => QualityLiaisonError::BusinessAction(format!("{error:?}")),
    })
}

fn payload_uuid(payload: &serde_json::Value, key: &str) -> Result<Uuid, QualityLiaisonError> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or(QualityLiaisonError::BusinessActionInvalid)
}
