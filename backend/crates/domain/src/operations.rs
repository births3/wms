use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;
use crate::receiving_outbound::ReceivingOrder;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceiveReceivingOrderRequest {
    pub actual_qty: i64,
    pub shortage_qty: i64,
    pub rejected_qty: i64,
    #[serde(default)]
    pub arrival_temperature_celsius: Option<f64>,
    #[serde(default)]
    pub exception_note: Option<String>,
    #[serde(default)]
    pub details: Option<ReceivingReceiptDetails>,
}

/// 收货现场信息。固定字段使用类型化结构，避免打印或审计依赖前端展示字符串。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingReceiptDetails {
    #[serde(default)]
    pub temperature_control_method: Option<String>,
    #[serde(default)]
    pub vehicle_no: Option<String>,
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub departure_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub arrival_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub storage_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub transport_mode: Option<String>,
    #[serde(default)]
    pub carrier: Option<String>,
    #[serde(default)]
    pub contact_name: Option<String>,
    #[serde(default)]
    pub contact_phone: Option<String>,
    #[serde(default)]
    pub contact_id_no: Option<String>,
    #[serde(default)]
    pub seal_checked: Option<String>,
    #[serde(default)]
    pub filing_checked: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RejectReceivingOrderRequest {
    pub reason: String,
}

/// 待收货/草稿 ASN 审批作废（软作废，状态变为 cancelled）。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CancelReceivingOrderRequest {
    pub reason: String,
    /// H4/企业微信审批单号或审批记录 ID；待收货状态必填。
    #[serde(default)]
    pub approval_id: Option<String>,
}

/// 验收环节短少强制关闭。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ForceCloseShortageRequest {
    pub reason: String,
}

/// 上架策略方案（可配置规则优先级、启停、仓库/品类绑定与无库位通知）。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayStrategyProfile {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub profile_code: String,
    pub profile_name: String,
    pub is_default: bool,
    pub top_n: i32,
    pub enabled_rules: serde_json::Value,
    pub rule_priority: serde_json::Value,
    #[serde(default)]
    pub warehouse_id: Option<Uuid>,
    #[serde(default)]
    pub product_category: Option<String>,
    #[serde(default = "default_notify_on_no_location")]
    pub notify_on_no_location: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertPutawayStrategyProfileRequest {
    pub profile_code: String,
    pub profile_name: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "default_putaway_top_n")]
    pub top_n: i32,
    #[serde(default)]
    pub enabled_rules: Option<serde_json::Value>,
    #[serde(default)]
    pub rule_priority: Option<serde_json::Value>,
    #[serde(default)]
    pub warehouse_id: Option<Uuid>,
    #[serde(default)]
    pub product_category: Option<String>,
    #[serde(default = "default_notify_on_no_location")]
    pub notify_on_no_location: bool,
    #[serde(default = "default_active_status")]
    pub status: String,
}

fn default_notify_on_no_location() -> bool {
    true
}

fn default_putaway_top_n() -> i32 {
    3
}

fn default_active_status() -> String {
    "active".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayStrategyProfileListResponse {
    pub data: Vec<PutawayStrategyProfile>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingOrderReceipt {
    pub id: Uuid,
    pub receiving_order_id: Uuid,
    pub owner_id: Uuid,
    pub actual_qty: i64,
    pub shortage_qty: i64,
    pub rejected_qty: i64,
    pub arrival_temperature_celsius: Option<f64>,
    pub exception_note: Option<String>,
    pub details: Option<ReceivingReceiptDetails>,
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
    /// GSP 外观/包装/说明书/标签核对（必填）。
    #[serde(default)]
    pub appearance_check: Option<String>,
    #[serde(default)]
    pub package_check: Option<String>,
    #[serde(default)]
    pub instruction_check: Option<String>,
    #[serde(default)]
    pub label_check: Option<String>,
    /// 抽验数量。
    #[serde(default)]
    pub sampling_qty: Option<i64>,
    /// 批准文号核对值。
    #[serde(default)]
    pub approval_no: Option<String>,
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
    #[serde(default)]
    pub quality_checks: Option<serde_json::Value>,
    #[serde(default)]
    pub sampling_qty: Option<i64>,
    #[serde(default)]
    pub approval_no: Option<String>,
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
    pub strategy_rule_id: Option<Uuid>,
    pub approval_record_id: Option<Uuid>,
    pub signed_at: DateTime<Utc>,
}

/// 收货单打印所需的业务事实，按货主范围聚合，不包含模板或渲染逻辑。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingOrderPrintData {
    pub order: ReceivingOrder,
    pub receipts: Vec<ReceivingOrderReceipt>,
    pub inspections: Vec<ReceivingInspectionRecord>,
    pub signatures: Vec<InspectionSignatureRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayRequest {
    pub batch_no: String,
    pub product_code: String,
    pub qty: i64,
    pub location_id: Uuid,
    pub location_code: String,
    pub quality_status: String,
    /// 可选容器/托盘 LPN（整托上架）。
    #[serde(default)]
    pub lpn_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayRecommendationQuery {
    pub product_code: String,
    pub batch_no: String,
    pub qty: i64,
    pub quality_status: String,
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayLocationRecommendation {
    pub location_id: Uuid,
    pub location_code: String,
    pub temperature_zone: String,
    pub quality_color: String,
    pub available_volume_cm3: i64,
    pub required_volume_cm3: i64,
    pub same_product: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayRecommendationResponse {
    pub receiving_order_id: Uuid,
    pub owner_id: Uuid,
    pub product_code: String,
    pub batch_no: String,
    pub qty: i64,
    pub quality_status: String,
    pub data: Vec<PutawayLocationRecommendation>,
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
    #[serde(default)]
    pub lpn_code: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryBatch {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub product_code: String,
    pub product_name: Option<String>,
    pub specification: Option<String>,
    pub manufacturer: Option<String>,
    pub batch_no: String,
    pub production_date: String,
    pub expiry_date: String,
    pub qty_on_hand: i64,
    pub qty_locked: i64,
    pub quality_status: String,
    pub location_id: Uuid,
    pub location_code: String,
    pub row_no: Option<i32>,
    pub column_no: Option<i32>,
    pub layer_no: Option<i32>,
    pub zone_code: Option<String>,
    pub temperature_zone: Option<String>,
    pub quality_color: Option<String>,
    pub max_volume_cm3: Option<i64>,
    pub used_volume_cm3: Option<i64>,
    pub remaining_volume_cm3: Option<i64>,
    pub max_sku_count: Option<i32>,
    pub current_sku_count: Option<i64>,
    pub container_lpn: Option<String>,
    pub recall_flag: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct InventoryBatchQuery {
    pub q: Option<String>,
    pub product_code: Option<String>,
    pub batch_no: Option<String>,
    pub location_code: Option<String>,
    pub location_type: Option<String>,
    pub zone_code: Option<String>,
    pub temperature_zone: Option<String>,
    pub quality_status: Option<String>,
    pub production_from: Option<String>,
    pub production_to: Option<String>,
    pub expiry_from: Option<String>,
    pub expiry_to: Option<String>,
    pub created_from: Option<String>,
    pub created_to: Option<String>,
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct ReceivingDashboardQuery {
    pub supplier_id: Option<Uuid>,
    pub product_code: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingDashboardRow {
    pub created_at: DateTime<Utc>,
    pub status: String,
    pub order_count: i64,
    pub expected_qty: i64,
    pub abnormal: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingDashboardResponse {
    pub data: Vec<ReceivingDashboardRow>,
    pub refreshed_at: DateTime<Utc>,
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
pub struct InventoryStatusTransition {
    pub id: Uuid,
    pub owner_id: Option<Uuid>,
    pub from_status: String,
    pub to_status: String,
    pub approval_sources: Vec<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryStatusTransitionListResponse {
    pub data: Vec<InventoryStatusTransition>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertInventoryStatusTransitionRequest {
    pub owner_id: Option<Uuid>,
    pub approval_sources: Vec<String>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MarkInventoryRecallRequest {
    pub batch_id: Uuid,
    pub reason: String,
    pub approval_source: String,
    pub approval_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CancelInventoryRecallRequest {
    pub batch_id: Uuid,
    pub reason: String,
    pub approval_id: String,
    pub second_approver_id: Uuid,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct ExpireInventoryBatchesRequest {
    pub as_of: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_location_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_location_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lpn_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_user_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_delta_cm3: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_no: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryStatusChange {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub batch_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub reason: String,
    pub approval_source: String,
    pub approval_id: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryBatchTrace {
    pub batch: InventoryBatch,
    pub movements: Vec<InventoryMovement>,
    pub status_changes: Vec<InventoryStatusChange>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct LocationHistoryQuery {
    pub location_code: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub movement_type: Option<String>,
    pub product_code: Option<String>,
    pub batch_no: Option<String>,
    pub days: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LocationHistoryRisk {
    pub risk_code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LocationHistoryProductShare {
    pub product_code: String,
    pub product_name: Option<String>,
    pub event_count: i64,
    pub total_qty_delta: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LocationHistoryResponse {
    pub location_code: String,
    pub data: Vec<InventoryMovement>,
    pub risks: Vec<LocationHistoryRisk>,
    pub product_shares: Vec<LocationHistoryProductShare>,
    pub page: PageMeta,
}
