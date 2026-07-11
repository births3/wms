use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
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
#[serde(deny_unknown_fields)]
pub struct UpdateReceivingOrderRequest {
    pub supplier_id: Option<Uuid>,
    pub warehouse_id: Option<Uuid>,
    #[serde(
        default,
        deserialize_with = "deserialize_nullable_patch",
        skip_serializing_if = "Option::is_none"
    )]
    pub external_ref: Option<Option<String>>,
    pub expected_arrival_at: Option<DateTime<Utc>>,
    pub lines: Option<Vec<ReceivingOrderLine>>,
}

fn deserialize_nullable_patch<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
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

#[cfg(test)]
mod tests {
    use super::UpdateReceivingOrderRequest;

    #[test]
    fn nullable_patch_distinguishes_missing_null_and_value() {
        let missing: UpdateReceivingOrderRequest =
            serde_json::from_str("{}").expect("missing fields should deserialize");
        let clear: UpdateReceivingOrderRequest = serde_json::from_str(r#"{"external_ref":null}"#)
            .expect("null field should deserialize");
        let value: UpdateReceivingOrderRequest = serde_json::from_str(
            r#"{"external_ref":"ERP-001","expected_arrival_at":"2026-07-11T00:00:00Z"}"#,
        )
        .expect("values should deserialize");

        assert_eq!(missing.external_ref, None);
        assert_eq!(clear.external_ref, Some(None));
        assert_eq!(value.external_ref, Some(Some("ERP-001".to_string())));
        assert!(value.expected_arrival_at.is_some());
    }

    #[test]
    fn receiving_order_patch_rejects_workflow_status_field() {
        let result =
            serde_json::from_str::<UpdateReceivingOrderRequest>(r#"{"status":"completed"}"#);

        assert!(result.is_err());
    }
}
