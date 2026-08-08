use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::Quantity;

use crate::common::PageMeta;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RelocateInventoryRequest {
    pub batch_id: Uuid,
    pub qty: Quantity,
    pub to_location_id: Uuid,
    pub to_location_code: String,
    #[serde(default)]
    pub relocation_mode: Option<String>,
    #[serde(default)]
    pub lpn_code: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryRelocation {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub batch_id: Uuid,
    pub product_code: String,
    pub batch_no: String,
    pub qty: Quantity,
    pub from_location_id: Uuid,
    pub from_location_code: String,
    pub to_location_id: Uuid,
    pub to_location_code: String,
    pub relocation_mode: String,
    pub lpn_code: Option<String>,
    pub quality_status: String,
    pub status: String,
    pub reason: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryRelocationListResponse {
    pub data: Vec<InventoryRelocation>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct InventoryAlertQuery {
    pub alert_type: Option<String>,
    pub lifecycle_status: Option<String>,
    pub product_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryAlertEvent {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub alert_type: String,
    pub product_code: Option<String>,
    pub batch_id: Option<Uuid>,
    pub batch_no: Option<String>,
    pub location_code: Option<String>,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub lifecycle_status: String,
    pub handled_by: Option<Uuid>,
    pub handled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryAlertListResponse {
    pub data: Vec<InventoryAlertEvent>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct HandleInventoryAlertRequest {
    pub lifecycle_status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct InventoryAbcQuery {
    pub abc_class: Option<String>,
    pub product_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryAbcClassification {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub product_code: String,
    pub abc_class: String,
    pub score: f64,
    pub outbound_qty: Quantity,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub source: String,
    pub override_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryAbcListResponse {
    pub data: Vec<InventoryAbcClassification>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RecomputeInventoryAbcRequest {
    pub period_days: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OverrideInventoryAbcRequest {
    pub product_code: String,
    pub abc_class: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ShippedCustomerHint {
    pub customer_id: Uuid,
    pub order_id: Uuid,
    pub wms_order_no: Option<String>,
    pub shipped_qty: Quantity,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryRecallImpact {
    pub batch_id: Uuid,
    pub batch_no: String,
    pub product_code: String,
    pub shipped_customers: Vec<ShippedCustomerHint>,
}
