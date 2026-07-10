use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PackingStation {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub station_code: String,
    pub station_name: String,
    pub printer_code: Option<String>,
    pub scale_code: Option<String>,
    pub temperature_zone: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreatePackingStationRequest {
    pub station_code: String,
    pub station_name: String,
    pub printer_code: Option<String>,
    pub scale_code: Option<String>,
    pub temperature_zone: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PackJob {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub outbound_order_id: Uuid,
    pub station_id: Option<Uuid>,
    pub job_no: String,
    pub pack_mode: String,
    pub recommended_box_type: String,
    pub actual_box_type: String,
    pub adjustment_reason: Option<String>,
    pub outbound_lpn: String,
    pub trace_codes: Vec<String>,
    pub status: String,
    pub weight_grams: Option<i64>,
    pub waybill_no: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreatePackJobRequest {
    pub outbound_order_id: Uuid,
    pub station_id: Option<Uuid>,
    pub job_no: String,
    pub pack_mode: String,
    pub recommended_box_type: String,
    pub actual_box_type: String,
    pub adjustment_reason: Option<String>,
    pub outbound_lpn: String,
    pub trace_codes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct WeighPackJobRequest {
    pub actual_weight_grams: i64,
    pub theoretical_weight_grams: i64,
    pub tolerance_percent: i32,
    pub override_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PrintWaybillRequest {
    pub carrier_code: String,
    pub waybill_no: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ExpressCarrier {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub carrier_code: String,
    pub carrier_name: String,
    pub api_url: String,
    pub api_key_alias: Option<String>,
    pub api_secret_alias: Option<String>,
    pub account_no: Option<String>,
    pub enabled: bool,
    pub priority: i32,
    pub conditions: Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ExpressCarrierListResponse {
    pub data: Vec<ExpressCarrier>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertExpressCarrierRequest {
    pub carrier_code: String,
    pub carrier_name: String,
    pub api_url: String,
    pub api_key_alias: Option<String>,
    pub api_secret_alias: Option<String>,
    pub account_no: Option<String>,
    pub enabled: bool,
    pub priority: i32,
    pub conditions: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ExpressRoutingRule {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub rule_code: String,
    pub rule_name: String,
    pub delivery_provider_type: String,
    pub carrier_code: Option<String>,
    pub priority: i32,
    pub conditions: Value,
    pub fallback_strategy: Option<String>,
    pub enabled: bool,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ExpressRoutingRuleListResponse {
    pub data: Vec<ExpressRoutingRule>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertExpressRoutingRuleRequest {
    pub rule_code: String,
    pub rule_name: String,
    pub delivery_provider_type: String,
    pub carrier_code: Option<String>,
    pub priority: i32,
    pub conditions: Value,
    pub fallback_strategy: Option<String>,
    pub enabled: bool,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ExpressWaybill {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub outbound_order_id: Option<Uuid>,
    pub package_no: String,
    pub carrier_code: String,
    pub waybill_no: String,
    pub status: String,
    pub sender_name: String,
    pub sender_mobile: String,
    pub sender_address: String,
    pub receiver_name: String,
    pub receiver_mobile: String,
    pub receiver_address: String,
    pub weight_grams: i64,
    pub volume_cm3: i64,
    pub package_count: i32,
    pub eta_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateExpressWaybillRequest {
    pub outbound_order_id: Option<Uuid>,
    pub package_no: String,
    pub carrier_code: String,
    pub requested_waybill_no: Option<String>,
    pub sender_name: String,
    pub sender_mobile: String,
    pub sender_address: String,
    pub receiver_name: String,
    pub receiver_mobile: String,
    pub receiver_address: String,
    pub weight_grams: i64,
    pub volume_cm3: i64,
    pub package_count: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CancelExpressWaybillRequest {
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ExpressTrackingEvent {
    pub id: Uuid,
    pub waybill_no: String,
    pub event_time: DateTime<Utc>,
    pub status: String,
    pub location: Option<String>,
    pub description: String,
    pub source: String,
    pub cached_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ExpressTrackingResponse {
    pub waybill: ExpressWaybill,
    pub events: Vec<ExpressTrackingEvent>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RetailReplenishmentSuggestion {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub store_id: Uuid,
    pub product_code: String,
    pub period_key: String,
    pub min_qty: i64,
    pub max_qty: i64,
    pub current_qty: i64,
    pub in_transit_qty: i64,
    pub daily_sales_avg: i64,
    pub suggested_qty: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateRetailReplenishmentSuggestionRequest {
    pub store_id: Uuid,
    pub product_code: String,
    pub period_key: String,
    pub min_qty: i64,
    pub max_qty: i64,
    pub current_qty: i64,
    pub in_transit_qty: i64,
    pub daily_sales_avg: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CrossdockPlan {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub asn_id: Uuid,
    pub outbound_order_id: Uuid,
    pub store_id: Uuid,
    pub product_code: String,
    pub qty: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateCrossdockPlanRequest {
    pub asn_id: Uuid,
    pub outbound_order_id: Uuid,
    pub store_id: Uuid,
    pub product_code: String,
    pub qty: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TmsDispatch {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub dispatch_no: String,
    pub outbound_order_id: Uuid,
    pub delivery_provider_type: String,
    pub vehicle_no: Option<String>,
    pub plate_no: Option<String>,
    pub driver_user_id: Option<Uuid>,
    pub carrier_code: Option<String>,
    pub waybill_no: Option<String>,
    pub status: String,
    pub version: i32,
    pub scheduled_load_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceiveTmsDispatchRequest {
    pub dispatch_no: String,
    pub outbound_order_id: Uuid,
    pub delivery_provider_type: String,
    pub vehicle_no: Option<String>,
    pub plate_no: Option<String>,
    pub driver_user_id: Option<Uuid>,
    pub carrier_code: Option<String>,
    pub waybill_no: Option<String>,
    pub version: i32,
    pub scheduled_load_at: Option<DateTime<Utc>>,
}
