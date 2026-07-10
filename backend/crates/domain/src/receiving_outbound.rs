use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

/// 收货单明细。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingOrderLine {
    pub line_no: u32,
    pub product_id: Option<Uuid>,
    pub product_code: String,
    pub expected_qty: i64,
    pub batch_no: Option<String>,
    pub production_date: Option<String>,
    pub expiry_date: Option<String>,
}

pub const RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND: &str = "purchase_inbound";
pub const RECEIVING_DOCUMENT_TYPE_SALES_RETURN: &str = "sales_return";

/// 收货单。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingOrder {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub receipt_no: String,
    pub document_type: String,
    pub supplier_id: Option<Uuid>,
    pub warehouse_id: Uuid,
    pub external_ref: Option<String>,
    pub status: String,
    pub expected_arrival_at: Option<DateTime<Utc>>,
    pub lines: Vec<ReceivingOrderLine>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateReceivingOrderRequest {
    pub receipt_no: String,
    pub document_type: String,
    pub supplier_id: Option<Uuid>,
    pub warehouse_id: Uuid,
    pub external_ref: Option<String>,
    pub expected_arrival_at: Option<DateTime<Utc>>,
    pub lines: Vec<ReceivingOrderLine>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateReceivingOrderRequest {
    pub supplier_id: Option<Uuid>,
    pub warehouse_id: Option<Uuid>,
    pub external_ref: Option<String>,
    pub status: Option<String>,
    pub expected_arrival_at: Option<DateTime<Utc>>,
    pub lines: Option<Vec<ReceivingOrderLine>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingOrderListResponse {
    pub data: Vec<ReceivingOrder>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateOutboundOrderLineRequest {
    pub line_no: u32,
    pub product_code: String,
    pub batch_no: String,
    pub planned_qty: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OutboundOrderLine {
    pub line_no: u32,
    pub product_code: String,
    pub batch_no: String,
    pub planned_qty: i64,
    pub picked_qty: i64,
    pub reviewed_qty: i64,
    pub shipped_qty: i64,
    pub short_pick_qty: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateOutboundOrderRequest {
    pub wms_order_no: String,
    pub erp_order_no: Option<String>,
    pub customer_id: Uuid,
    pub warehouse_id: Uuid,
    pub required_ship_at: Option<DateTime<Utc>>,
    pub lines: Vec<CreateOutboundOrderLineRequest>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OutboundOrder {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub wms_order_no: String,
    pub erp_order_no: Option<String>,
    pub customer_id: Uuid,
    pub warehouse_id: Uuid,
    pub required_ship_at: Option<DateTime<Utc>>,
    pub status: String,
    pub short_pick: bool,
    pub lines: Vec<OutboundOrderLine>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OutboundOrderListResponse {
    pub data: Vec<OutboundOrder>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateOutboundWaveRequest {
    pub wave_no: String,
    pub order_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OutboundWave {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub wave_no: String,
    pub status: String,
    pub order_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CompletePickTaskRequest {
    pub line_no: u32,
    pub picked_qty: i64,
    pub exception_code: Option<String>,
    pub exception_note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReviewOutboundOrderRequest {
    pub reviewer_id: Uuid,
    pub review_mode: String,
    pub second_reviewer_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ShipOutboundOrderRequest {
    pub carrier_type: String,
    pub handover_to: String,
    pub package_count: u32,
    pub shipped_at: Option<DateTime<Utc>>,
}
