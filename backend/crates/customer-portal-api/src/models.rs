use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
    pub user: PortalUserSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct PortalUserSummary {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub can_view_report_history: bool,
    pub address_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub role: String,
    #[serde(default)]
    pub can_view_report_history: bool,
    #[serde(default)]
    pub address_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: String,
    pub role: String,
    pub status: String,
    #[serde(default)]
    pub can_view_report_history: bool,
    #[serde(default)]
    pub address_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProjectionRequest {
    pub event_id: Uuid,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProjectionResponse {
    pub event_id: Uuid,
    pub duplicate: bool,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CustomerProjection {
    pub id: Uuid,
    pub customer_code: String,
    pub customer_name: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AddressProjection {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub address_code: String,
    pub address_name: String,
    pub address_snapshot: Value,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OrderProjection {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub order_no: String,
    pub status: String,
    pub delivery_address_id: Uuid,
    pub address_snapshot: Value,
    pub shipped_at: DateTime<Utc>,
    pub signed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub lines: Vec<OrderLineProjection>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OrderLineProjection {
    pub id: Uuid,
    pub product_id: Uuid,
    pub product_code: String,
    pub product_name: String,
    pub batch_no: String,
    pub quantity: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReportProjection {
    pub id: Uuid,
    pub report_id: Uuid,
    pub owner_id: Uuid,
    pub product_id: Uuid,
    pub batch_no: String,
    pub version_number: i32,
    pub report_no: String,
    pub status: String,
    pub is_current: bool,
    pub modification_reason: Option<String>,
    pub customer_copy_status: String,
    pub customer_copy_storage_key: Option<String>,
    pub customer_copy_file_name: Option<String>,
    pub customer_copy_size: Option<i64>,
    pub customer_copy_hash: Option<String>,
    #[serde(default)]
    pub digitally_signed_original: bool,
    pub confirmed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CustomerOrderSnapshotProjection {
    pub customer: CustomerProjection,
    pub address: AddressProjection,
    pub order: OrderProjection,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OrderQuery {
    pub address_id: Option<Uuid>,
    pub status: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OrderSummary {
    pub id: Uuid,
    pub order_no: String,
    pub status: String,
    pub customer_code: String,
    pub customer_name: String,
    pub delivery_address_id: Uuid,
    pub address_code: String,
    pub address_name: String,
    pub product_codes: Vec<String>,
    pub product_names: Vec<String>,
    pub batch_nos: Vec<String>,
    pub quantities: Vec<f64>,
    pub shipped_at: DateTime<Utc>,
    pub signed_at: Option<DateTime<Utc>>,
    pub line_count: i64,
    pub available_report_count: i64,
    pub pending_report_count: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct OrderDetail {
    pub id: Uuid,
    pub order_no: String,
    pub status: String,
    pub delivery_address_id: Uuid,
    pub address_snapshot: Value,
    pub shipped_at: DateTime<Utc>,
    pub signed_at: Option<DateTime<Utc>>,
    pub lines: Vec<OrderLineDetail>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OrderLineDetail {
    pub id: Uuid,
    pub product_id: Uuid,
    pub product_code: String,
    pub product_name: String,
    pub batch_no: String,
    pub quantity: f64,
    pub reports: Vec<ReportSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReportSummary {
    pub id: Uuid,
    pub report_id: Uuid,
    pub version_number: i32,
    pub report_no: String,
    pub status: String,
    pub is_current: bool,
    pub modification_reason: Option<String>,
    pub customer_copy_status: String,
    pub customer_copy_file_name: Option<String>,
    pub customer_copy_size: Option<i64>,
    pub digitally_signed_original: bool,
    pub confirmed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DownloadUrlResponse {
    pub url: String,
    pub file_name: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateExportRequest {
    pub order_ids: Vec<Uuid>,
    #[serde(default)]
    pub include_history: bool,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct ExportJob {
    pub id: Uuid,
    pub include_history: bool,
    pub status: String,
    pub requested_order_count: i32,
    pub report_file_count: i32,
    pub missing_count: i32,
    pub total_size: i64,
    pub result_file_name: Option<String>,
    pub last_error: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AddressSummary {
    pub id: Uuid,
    pub address_code: String,
    pub address_name: String,
}
