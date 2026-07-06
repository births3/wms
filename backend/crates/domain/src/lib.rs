//! 主仓 OpenAPI 契约使用的 domain schema。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{
    openapi::schema::{AdditionalProperties, Object, ObjectBuilder},
    ToSchema,
};
use uuid::Uuid;

fn audit_diff_schema() -> Object {
    ObjectBuilder::new()
        .description(Some("变更详情。"))
        .additional_properties(Some(AdditionalProperties::FreeForm(true)))
        .build()
}

fn error_details_schema() -> Object {
    ObjectBuilder::new()
        .description(Some("关联详情。"))
        .additional_properties(Some(AdditionalProperties::FreeForm(true)))
        .build()
}

fn free_form_json_schema() -> Object {
    ObjectBuilder::new()
        .description(Some("自由结构 JSON 对象。"))
        .additional_properties(Some(AdditionalProperties::FreeForm(true)))
        .build()
}

/// 分页信息。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PageMeta {
    /// 下一页游标；为空表示无更多数据。
    pub next_cursor: Option<String>,
    /// 本页数量。
    pub count: u32,
}

/// 健康检查响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct HealthzResponse {
    /// 服务状态。
    pub status: String,
    /// 契约版本。
    pub version: String,
    /// 文档生成时间。
    pub generated_at: DateTime<Utc>,
}

/// 登录请求。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LoginRequest {
    /// 货主编码。
    pub owner_code: String,
    /// 登录账号。
    pub username: String,
    /// 登录密码。
    pub password: String,
}

/// 当前登录用户摘要。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CurrentUser {
    /// 用户 ID。
    pub user_id: Uuid,
    /// 货主 ID。
    pub owner_id: Uuid,
    /// 货主编码。
    pub owner_code: String,
    /// 用户名。
    pub username: String,
    /// 展示名。
    pub display_name: String,
    /// 当前角色列表。
    pub roles: Vec<String>,
    /// 当前权限码列表。
    pub permissions: Vec<String>,
}

/// 登录成功响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LoginResponse {
    /// Bearer token。
    pub access_token: String,
    /// token 类型。
    pub token_type: String,
    /// 过期时间。
    pub expires_at: DateTime<Utc>,
    /// 当前用户。
    pub user: CurrentUser,
}

/// H1 菜单按钮权限点。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AdminMenuButtonPermission {
    pub action_key: String,
    pub action_label: String,
    pub action_kind: String,
    pub enabled: bool,
    pub sort_order: i32,
}

/// H1 管理端菜单节点。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AdminMenuNode {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub level: i32,
    pub code: String,
    pub path: String,
    pub title: String,
    pub view_id: Option<String>,
    pub icon_key: String,
    pub permission_key: String,
    pub sort_order: i32,
    pub enabled: bool,
    pub button_permissions: Vec<AdminMenuButtonPermission>,
    pub children: Vec<AdminMenuNode>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// H1 菜单树响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AdminMenuTreeResponse {
    pub data: Vec<AdminMenuNode>,
    pub version_no: Option<i64>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertAdminMenuButtonPermissionRequest {
    pub action_key: String,
    pub action_label: String,
    pub action_kind: String,
    pub enabled: bool,
    pub sort_order: i32,
}

/// 新增 H1 菜单节点请求。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateAdminMenuNodeRequest {
    pub parent_id: Option<Uuid>,
    pub code: String,
    pub title: String,
    pub view_id: Option<String>,
    pub icon_key: String,
    pub permission_key: String,
    pub sort_order: i32,
    pub enabled: bool,
    pub button_permissions: Vec<UpsertAdminMenuButtonPermissionRequest>,
}

/// 更新 H1 菜单节点请求。
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct UpdateAdminMenuNodeRequest {
    pub parent_id: Option<Uuid>,
    pub title: Option<String>,
    pub view_id: Option<String>,
    pub icon_key: Option<String>,
    pub permission_key: Option<String>,
    pub sort_order: Option<i32>,
    pub enabled: Option<bool>,
    pub button_permissions: Option<Vec<UpsertAdminMenuButtonPermissionRequest>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BatchEnableAdminMenuRequest {
    pub ids: Vec<Uuid>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PublishAdminMenuRequest {
    pub note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RollbackAdminMenuRequest {
    pub target_version_no: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AdminMenuVersion {
    pub id: Uuid,
    pub version_no: i64,
    pub note: Option<String>,
    pub published_by: Uuid,
    pub published_at: DateTime<Utc>,
}

/// 审计事件操作者摘要。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuditActor {
    /// 操作者 ID。
    pub actor_id: Uuid,
    /// 操作者名称。
    pub actor_name: String,
    /// 操作者所属货主 ID。
    pub owner_id: Uuid,
    /// JWT jti，用于追溯登录态。
    pub jti: String,
}

/// 审计事件。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuditEvent {
    /// 审计事件 ID。
    pub id: i64,
    /// 被审计记录所属货主 ID。
    pub owner_id: Uuid,
    /// 资源类型。
    pub resource_type: String,
    /// 资源实例 ID。
    pub resource_id: String,
    /// 事件动作。
    pub action: String,
    /// 审计 trace ID。
    pub trace_id: String,
    /// 发生时间。
    pub occurred_at: DateTime<Utc>,
    /// 操作者摘要。
    pub actor: AuditActor,
    /// 变更详情。
    #[schema(schema_with = audit_diff_schema)]
    pub diff: serde_json::Value,
}

/// 审计事件分页响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AuditEventListResponse {
    /// 事件列表。
    pub data: Vec<AuditEvent>,
    /// 下一页游标；为空表示无更多数据。
    pub next_cursor: Option<String>,
}

/// 统一错误响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// 业务错误码。
    pub code: String,
    /// 中文错误消息。
    pub message: String,
    /// 严重度。
    pub severity: String,
    /// 关联详情。
    #[schema(schema_with = error_details_schema)]
    pub details: serde_json::Value,
    /// 链路追踪 ID。
    pub trace_id: String,
    /// 重试提示。
    pub retry_hint: Option<String>,
}

/// 商品基础档案。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Product {
    /// 商品 ID。
    pub id: Uuid,
    /// 货主 ID。
    pub owner_id: Uuid,
    /// 商品编码。
    pub product_code: String,
    /// 商品名称。
    pub product_name: String,
    /// 批准文号。
    pub approval_no: Option<String>,
    /// 规格。
    pub spec: Option<String>,
    /// 剂型。
    pub dosage_form: Option<String>,
    /// 生产企业。
    pub manufacturer: Option<String>,
    /// 特殊药品分类编码。
    pub special_drug_category_code: Option<String>,
    /// 启停状态。
    pub status: String,
    /// 扩展属性。
    #[schema(schema_with = free_form_json_schema)]
    pub attrs: serde_json::Value,
    /// 创建时间。
    pub created_at: DateTime<Utc>,
    /// 更新时间。
    pub updated_at: DateTime<Utc>,
}

/// 创建商品请求。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateProductRequest {
    pub product_code: String,
    pub product_name: String,
    pub approval_no: Option<String>,
    pub spec: Option<String>,
    pub dosage_form: Option<String>,
    pub manufacturer: Option<String>,
    pub special_drug_category_code: Option<String>,
    #[schema(schema_with = free_form_json_schema)]
    pub attrs: serde_json::Value,
}

/// 更新商品请求。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateProductRequest {
    pub product_name: Option<String>,
    pub approval_no: Option<String>,
    pub spec: Option<String>,
    pub dosage_form: Option<String>,
    pub manufacturer: Option<String>,
    pub special_drug_category_code: Option<String>,
    pub status: Option<String>,
    #[schema(schema_with = free_form_json_schema)]
    pub attrs: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ProductListResponse {
    pub data: Vec<Product>,
    pub page: PageMeta,
}

/// 供应商基础档案。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Supplier {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub supplier_code: String,
    pub supplier_name: String,
    pub license_no: Option<String>,
    pub contact_name: Option<String>,
    pub source: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateSupplierRequest {
    pub supplier_code: String,
    pub supplier_name: String,
    pub license_no: Option<String>,
    pub contact_name: Option<String>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateSupplierRequest {
    pub supplier_name: Option<String>,
    pub license_no: Option<String>,
    pub contact_name: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SupplierListResponse {
    pub data: Vec<Supplier>,
    pub page: PageMeta,
}

/// 客户基础档案。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Customer {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub customer_code: String,
    pub customer_name: String,
    pub license_no: Option<String>,
    pub source: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateCustomerRequest {
    pub customer_code: String,
    pub customer_name: String,
    pub license_no: Option<String>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateCustomerRequest {
    pub customer_name: Option<String>,
    pub license_no: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CustomerListResponse {
    pub data: Vec<Customer>,
    pub page: PageMeta,
}

/// 仓库基础档案。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Warehouse {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub warehouse_code: String,
    pub warehouse_name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateWarehouseRequest {
    pub warehouse_code: String,
    pub warehouse_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateWarehouseRequest {
    pub warehouse_name: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct WarehouseListResponse {
    pub data: Vec<Warehouse>,
    pub page: PageMeta,
}

/// 库位基础档案。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Location {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub warehouse_id: Uuid,
    pub zone_id: Uuid,
    pub location_code: String,
    pub row_no: i32,
    pub column_no: i32,
    pub layer_no: i32,
    pub max_volume_cm3: i64,
    pub used_volume_cm3: i64,
    pub max_sku_count: i32,
    pub location_type: String,
    pub bound_owner_id: Option<Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateLocationRequest {
    pub warehouse_id: Uuid,
    pub zone_id: Uuid,
    pub location_code: String,
    pub row_no: i32,
    pub column_no: i32,
    pub layer_no: i32,
    pub max_volume_cm3: i64,
    pub max_sku_count: i32,
    pub location_type: String,
    pub bound_owner_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BatchCreateLocationsRequest {
    pub warehouse_id: Uuid,
    pub zone_id: Uuid,
    pub area_code: String,
    pub row_start: i32,
    pub row_end: i32,
    pub column_start: i32,
    pub column_end: i32,
    pub layer_start: i32,
    pub layer_end: i32,
    pub max_volume_cm3: i64,
    pub max_sku_count: i32,
    pub location_type: String,
    pub bound_owner_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateLocationRequest {
    pub zone_id: Option<Uuid>,
    pub location_code: Option<String>,
    pub row_no: Option<i32>,
    pub column_no: Option<i32>,
    pub layer_no: Option<i32>,
    pub max_volume_cm3: Option<i64>,
    pub used_volume_cm3: Option<i64>,
    pub max_sku_count: Option<i32>,
    pub location_type: Option<String>,
    pub bound_owner_id: Option<Uuid>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LocationListResponse {
    pub data: Vec<Location>,
    pub page: PageMeta,
}

/// 特殊药品分类字典。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SpecialDrugCategory {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub category_code: String,
    pub category_name: String,
    pub requires_dual_sign: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateSpecialDrugCategoryRequest {
    pub category_code: String,
    pub category_name: String,
    pub requires_dual_sign: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateSpecialDrugCategoryRequest {
    pub category_name: Option<String>,
    pub requires_dual_sign: Option<bool>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SpecialDrugCategoryListResponse {
    pub data: Vec<SpecialDrugCategory>,
    pub page: PageMeta,
}

pub const SYSTEM_DICTIONARY_DOCUMENT_TYPE: &str = "document_type";
pub const DOCUMENT_TYPE_PURCHASE_INBOUND: &str = "purchase_inbound";
pub const DOCUMENT_TYPE_SALES_RETURN: &str = "sales_return";
pub const DOCUMENT_TYPE_PURCHASE_RETURN_OUTBOUND: &str = "purchase_return_outbound";
pub const DOCUMENT_TYPE_SALES_OUTBOUND: &str = "sales_outbound";
pub const SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE: &str = "print_template_type";
pub const PRINT_TEMPLATE_TYPE_ASN: &str = "asn";
pub const PRINT_TEMPLATE_TYPE_ACCEPTANCE_RECORD: &str = "acceptance_record";
pub const PRINT_TEMPLATE_TYPE_DELIVERY_NOTE: &str = "delivery_note";
pub const PRINT_TEMPLATE_TYPE_LOCATION_LABEL: &str = "location_label";
pub const PRINT_TEMPLATE_TYPE_LPN_LABEL: &str = "lpn_label";
pub const PRINT_TEMPLATE_TYPE_PRODUCT_LABEL: &str = "product_label";

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DocumentNumberAllocation {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub rule_id: Uuid,
    pub document_type: String,
    pub generated_no: String,
    pub sequence_value: i64,
    pub counter_key: String,
    pub source_module: String,
    pub source_document_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DocumentNumberAllocationListResponse {
    pub data: Vec<DocumentNumberAllocation>,
    pub page: PageMeta,
}

/// 系统字典分类。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SystemDictionaryCategory {
    pub dict_code: String,
    pub dict_name: String,
    pub enabled: bool,
    pub control_level: String,
    #[schema(schema_with = free_form_json_schema)]
    pub param_schema: serde_json::Value,
    pub scope_mode: String,
    #[schema(schema_with = free_form_json_schema)]
    pub override_policy: serde_json::Value,
    pub sort_order: i32,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 系统字典项。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SystemDictionaryItem {
    pub id: Uuid,
    pub dict_code: String,
    pub item_code: String,
    pub item_name: String,
    pub enabled: bool,
    pub owner_id: Option<Uuid>,
    #[schema(schema_with = free_form_json_schema)]
    pub params: serde_json::Value,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
    pub source: String,
    pub disabled_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SystemDictionaryItemListResponse {
    pub data: Vec<SystemDictionaryItem>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SystemDictionaryImpactReference {
    pub module_code: String,
    pub business_object: String,
    pub reference_count: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SystemDictionaryImpactPreview {
    pub dict_code: String,
    pub item_code: String,
    pub owner_id: Uuid,
    pub effective_at: DateTime<Utc>,
    pub total_references: i64,
    pub references: Vec<SystemDictionaryImpactReference>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertSystemDictionaryItemRequest {
    pub owner_id: Option<Uuid>,
    pub item_name: String,
    pub enabled: bool,
    #[schema(schema_with = free_form_json_schema)]
    pub params: serde_json::Value,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DisableSystemDictionaryItemRequest {
    pub owner_id: Option<Uuid>,
    pub disabled_reason: Option<String>,
}

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
    pub quantity_delta: Option<i64>,
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

/// M-PM 参数对照字典。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MappingDictionary {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub dictionary_code: String,
    pub dictionary_name: String,
    pub created_at: DateTime<Utc>,
}

/// M-PM 字段映射规则。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MappingRule {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub source_system: String,
    pub external_field: String,
    pub canonical_field: String,
    pub transform: String,
    pub created_at: DateTime<Utc>,
}

/// M-PM 待映射队列项。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MappingQueueItem {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub source_system: String,
    #[schema(schema_with = free_form_json_schema)]
    pub raw_payload: serde_json::Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ExecuteMappingRequest {
    pub source_system: String,
    #[schema(schema_with = free_form_json_schema)]
    pub raw_payload: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ExecuteMappingResponse {
    pub execution_id: Uuid,
    pub queue_item_id: Option<Uuid>,
    #[schema(schema_with = free_form_json_schema)]
    pub normalized_payload: serde_json::Value,
    pub unresolved_fields: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MappingTraceResponse {
    pub execution_id: Uuid,
    pub source_system: String,
    #[schema(schema_with = free_form_json_schema)]
    pub raw_payload: serde_json::Value,
    #[schema(schema_with = free_form_json_schema)]
    pub normalized_payload: serde_json::Value,
    pub applied_rule_ids: Vec<Uuid>,
    pub unresolved_fields: Vec<String>,
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

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ColdChainDevice {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub device_code: String,
    pub device_type: String,
    pub installed_at_location_code: Option<String>,
    pub calibration_due_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateColdChainDeviceRequest {
    pub device_code: String,
    pub device_type: String,
    pub installed_at_location_code: Option<String>,
    pub calibration_due_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TemperatureReading {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub device_code: String,
    pub temperature_celsius: f64,
    pub humidity_percent: Option<f64>,
    pub captured_at: DateTime<Utc>,
    pub external_report_url: Option<String>,
    pub out_of_range: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct IngestTemperatureReadingRequest {
    pub device_code: String,
    pub temperature_celsius: f64,
    pub humidity_percent: Option<f64>,
    pub captured_at: DateTime<Utc>,
    pub external_report_url: Option<String>,
    pub out_of_range: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TemperatureExcursionEvent {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub external_event_id: String,
    pub device_code: String,
    pub location_code: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub min_temperature_celsius: Option<f64>,
    pub max_temperature_celsius: Option<f64>,
    pub affected_batch_ids: Vec<Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TemperatureExcursionEventListResponse {
    pub data: Vec<TemperatureExcursionEvent>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct IngestTemperatureExcursionRequest {
    pub external_event_id: String,
    pub device_code: String,
    pub location_code: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub min_temperature_celsius: Option<f64>,
    pub max_temperature_celsius: Option<f64>,
    pub affected_batch_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DisposeTemperatureExcursionRequest {
    pub selected_batch_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TemperatureExcursionDispositionResponse {
    pub event: TemperatureExcursionEvent,
    pub quarantined_batches: Vec<InventoryBatch>,
    pub approval_source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BillingAccount {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub account_code: String,
    pub account_name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateBillingAccountRequest {
    pub account_code: String,
    pub account_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BillingContract {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub account_id: Uuid,
    pub contract_no: String,
    pub valid_from: String,
    pub valid_to: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateBillingContractRequest {
    pub account_id: Uuid,
    pub contract_no: String,
    pub valid_from: String,
    pub valid_to: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BillingRule {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub contract_id: Uuid,
    pub charge_item: String,
    pub unit: String,
    pub unit_price_cents: i64,
    pub billing_cycle: String,
    pub effective_from: String,
    pub effective_to: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateBillingRuleRequest {
    pub contract_id: Uuid,
    pub charge_item: String,
    pub unit: String,
    pub unit_price_cents: i64,
    pub billing_cycle: String,
    pub effective_from: String,
    pub effective_to: String,
}

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
pub struct BillingChargeCalculation {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub contract_id: Uuid,
    pub period_start: String,
    pub period_end: String,
    pub charge_item: String,
    pub quantity: i64,
    pub amount_cents: i64,
    pub source_refs: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CalculateBillingChargesRequest {
    pub contract_id: Uuid,
    pub period_start: String,
    pub period_end: String,
    pub charge_item: String,
    pub quantity: i64,
    pub source_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BillingStatement {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub contract_id: Uuid,
    pub period_start: String,
    pub period_end: String,
    pub status: String,
    pub total_amount_cents: i64,
    pub charge_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct GenerateBillingStatementRequest {
    pub contract_id: Uuid,
    pub period_start: String,
    pub period_end: String,
    pub charge_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ConfirmBillingStatementRequest {
    pub confirmation_note: Option<String>,
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

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TransitTemperatureReading {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub dispatch_id: Uuid,
    pub device_code: String,
    pub plate_no: String,
    pub measured_at: DateTime<Utc>,
    pub temperature_celsius: f64,
    pub humidity_percent: Option<f64>,
    pub is_exceeded: bool,
    pub external_trace_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct IngestTransitTemperatureRequest {
    pub dispatch_id: Uuid,
    pub device_code: String,
    pub plate_no: String,
    pub measured_at: DateTime<Utc>,
    pub temperature_celsius: f64,
    pub humidity_percent: Option<f64>,
    pub is_exceeded: bool,
    pub external_trace_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ContainerRecovery {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub container_lpn: String,
    pub dispatch_id: Option<Uuid>,
    pub customer_id: Uuid,
    pub delivery_provider_type: String,
    pub status: String,
    pub shipped_at: DateTime<Utc>,
    pub recovered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ConfirmContainerRecoveryRequest {
    pub container_lpn: String,
    pub dispatch_id: Option<Uuid>,
    pub customer_id: Uuid,
    pub delivery_provider_type: String,
    pub shipped_at: Option<DateTime<Utc>>,
}
