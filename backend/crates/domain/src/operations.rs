use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceiveReceivingOrderRequest {
    pub actual_qty: i64,
    pub shortage_qty: i64,
    pub rejected_qty: i64,
    pub arrival_temperature_celsius: Option<f64>,
    pub exception_note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RejectReceivingOrderRequest {
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingOrderReceipt {
    pub id: Uuid,
    pub receiving_order_id: Uuid,
    pub owner_id: Uuid,
    pub actual_qty: i64,
    pub shortage_qty: i64,
    pub rejected_qty: i64,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InspectReceivingOrderRequest {
    pub batch_no: String,
    pub accepted_qty: i64,
    pub rejected_qty: i64,
    pub production_date: String,
    pub expiry_date: String,
    pub quality_status: String,
    pub trace_codes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingInspectionRecord {
    pub id: Uuid,
    pub receiving_order_id: Uuid,
    pub owner_id: Uuid,
    pub batch_no: String,
    pub accepted_qty: i64,
    pub rejected_qty: i64,
    pub quality_status: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SignInspectionRequest {
    pub first_signer_id: Uuid,
    pub second_signer_id: Option<Uuid>,
    pub dual_required: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InspectionSignatureRecord {
    pub id: Uuid,
    pub receiving_order_id: Uuid,
    pub owner_id: Uuid,
    pub first_signer_id: Uuid,
    pub second_signer_id: Option<Uuid>,
    pub signed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayRequest {
    pub batch_no: String,
    pub product_code: String,
    pub qty: i64,
    pub location_id: Uuid,
    pub location_code: String,
    pub quality_status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayRecord {
    pub id: Uuid,
    pub receiving_order_id: Uuid,
    pub owner_id: Uuid,
    pub batch_no: String,
    pub product_code: String,
    pub qty: i64,
    pub location_id: Uuid,
    pub location_code: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryBatch {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub product_code: String,
    pub batch_no: String,
    pub production_date: String,
    pub expiry_date: String,
    pub qty_on_hand: i64,
    pub qty_locked: i64,
    pub quality_status: String,
    pub location_id: Uuid,
    pub location_code: String,
    pub recall_flag: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryBatchListResponse {
    pub data: Vec<InventoryBatch>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayInventoryRequest {
    pub product_code: String,
    pub batch_no: String,
    pub production_date: String,
    pub expiry_date: String,
    pub qty: i64,
    pub quality_status: String,
    pub location_id: Uuid,
    pub location_code: String,
    pub source_receiving_order_id: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ChangeInventoryStatusRequest {
    pub batch_id: Uuid,
    pub target_status: String,
    pub reason: String,
    pub approval_source: String,
    pub approval_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryMovement {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub batch_id: Uuid,
    pub movement_type: String,
    pub qty_delta: i64,
    pub source_document_type: String,
    pub source_document_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}
