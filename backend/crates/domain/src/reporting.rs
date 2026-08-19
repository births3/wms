use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::{free_form_json_schema, PageMeta};

/// M6 报表行。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReportRow {
    #[schema(schema_with = free_form_json_schema)]
    pub values: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReportQueryRequest {
    pub report_code: String,
    #[schema(schema_with = free_form_json_schema)]
    pub filters: serde_json::Value,
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReportQueryResponse {
    pub report_code: String,
    pub generated_at: DateTime<Utc>,
    pub rows: Vec<ReportRow>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct GspLedgerRow {
    pub ledger_type: String,
    pub occurred_at: Option<DateTime<Utc>>,
    pub product_code: Option<String>,
    pub batch_no: Option<String>,
    #[schema(value_type = String, format = "decimal", nullable = true)]
    pub quantity_delta: Option<crate::Quantity>,
    pub document_type: Option<String>,
    pub document_no: Option<String>,
    pub approval_source: Option<String>,
    pub approval_id: Option<String>,
    pub operator_id: Option<Uuid>,
    pub operator_name: Option<String>,
    #[schema(schema_with = free_form_json_schema)]
    pub values: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct GspLedgerReport {
    pub ledger_type: String,
    pub generated_at: DateTime<Utc>,
    pub rows: Vec<GspLedgerRow>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TraceabilityStatusChangeEvent {
    pub event_id: Uuid,
    pub trace_code: String,
    pub status_change_type: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TraceabilityOutboundReportRequest {
    pub events: Vec<TraceabilityStatusChangeEvent>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TraceabilityOutboundReport {
    pub report_id: Uuid,
    pub platform: String,
    pub status: String,
    pub queued_count: u32,
    pub generated_at: DateTime<Utc>,
    pub events: Vec<TraceabilityStatusChangeEvent>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DriverTask {
    pub order_no: String,
    pub customer_name: String,
    pub delivery_address: String,
    pub planned_arrival_at: Option<DateTime<Utc>>,
    pub cold_chain: bool,
    pub status: String,
    pub owner_id: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DriverTaskListResponse {
    pub data: Vec<DriverTask>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StoreDashboardResponse {
    pub store_id: Option<Uuid>,
    pub pending_receipt_orders: u32,
    pub in_transit_orders: u32,
    pub signed_orders_last_7_days: u32,
    pub inventory_alert_count: u32,
    pub returns_this_month: u32,
    pub exceptions_this_month: u32,
    pub generated_at: DateTime<Utc>,
}

/// M-PM 单值映射请求；外部自由文本不得直接进入业务模块。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MapParameterRequest {
    pub dict_code: String,
    pub source_value: String,
    pub source_system: Option<String>,
    pub source_record_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParameterMappingStatus {
    Matched,
    Unmatched,
    Ambiguous,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MapParameterResponse {
    pub status: ParameterMappingStatus,
    pub target_value: Option<String>,
    pub rule_id: Option<Uuid>,
    pub confidence: i32,
    pub fallback_used: bool,
    pub queued: bool,
}

/// M1-008 配置中心条目。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ConfigEntry {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub config_key: String,
    #[schema(schema_with = free_form_json_schema)]
    pub config_value: serde_json::Value,
    pub version: i64,
    pub updated_at: DateTime<Utc>,
}

/// 配置中心版 Feature Flag。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FeatureFlagConfig {
    pub key: String,
    pub owner: String,
    pub created_at: String,
    pub cleanup_by: String,
    pub enabled: bool,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FeatureFlagExportResponse {
    pub source: String,
    pub flags: Vec<FeatureFlagConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FeatureFlagBatchImportRequest {
    pub flags: Vec<FeatureFlagConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FeatureFlagBatchImportResult {
    pub imported_count: u32,
    pub target: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FeatureFlagMigrationResult {
    pub migrated_count: u32,
    pub source: String,
    pub target: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FeatureFlagReconcileReport {
    pub matched: u32,
    pub missing_in_config_center: Vec<String>,
    pub mismatched: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FeatureFlagSourceSwitchRequest {
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FeatureFlagSourceSwitchResponse {
    pub active_source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FeatureFlagArchiveRequest {
    pub archive_ref: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FeatureFlagArchiveResult {
    pub archived_source: String,
    pub archive_ref: String,
    pub archived_at: DateTime<Utc>,
}
