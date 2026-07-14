use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::{free_form_json_schema, PageMeta};

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

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CustomerQualification {
    pub certificate_type: String,
    pub certificate_no: String,
    pub expires_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CustomerProfile {
    pub customer_id: Uuid,
    pub owner_id: Uuid,
    pub customer_type: String,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub business_scope: Vec<String>,
    pub qualification_certificates: Vec<CustomerQualification>,
    pub chain_name: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertCustomerProfileRequest {
    pub customer_type: String,
    pub contact_name: String,
    pub contact_phone: String,
    #[serde(default)]
    pub business_scope: Vec<String>,
    #[serde(default)]
    pub qualification_certificates: Vec<CustomerQualification>,
    pub chain_name: Option<String>,
}

/// 客户收货地址。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CustomerAddress {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub customer_id: Uuid,
    pub province: String,
    pub city: String,
    pub district: String,
    pub detail_address: String,
    pub contact_name: String,
    pub contact_phone: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateCustomerAddressRequest {
    pub province: String,
    pub city: String,
    pub district: String,
    pub detail_address: String,
    pub contact_name: String,
    pub contact_phone: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateCustomerAddressRequest {
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub detail_address: Option<String>,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CustomerAddressListResponse {
    pub data: Vec<CustomerAddress>,
    pub page: PageMeta,
}

/// 仓库基础档案。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Warehouse {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub warehouse_code: String,
    pub warehouse_name: String,
    pub warehouse_type: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateWarehouseRequest {
    pub warehouse_code: String,
    pub warehouse_name: String,
    #[serde(default = "default_warehouse_type")]
    pub warehouse_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateWarehouseRequest {
    pub warehouse_name: Option<String>,
    pub warehouse_type: Option<String>,
    pub status: Option<String>,
}

fn default_warehouse_type() -> String {
    "physical".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct WarehouseListResponse {
    pub data: Vec<Warehouse>,
    pub page: PageMeta,
}

/// 库区基础档案。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct WarehouseZone {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub warehouse_id: Uuid,
    pub zone_code: String,
    pub zone_name: String,
    pub temperature_zone: String,
    pub quality_color: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateWarehouseZoneRequest {
    pub warehouse_id: Uuid,
    pub zone_code: String,
    pub zone_name: String,
    pub temperature_zone: String,
    pub quality_color: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateWarehouseZoneRequest {
    pub zone_name: Option<String>,
    pub temperature_zone: Option<String>,
    pub quality_color: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct WarehouseZoneListResponse {
    pub data: Vec<WarehouseZone>,
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

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StateMachineState {
    pub code: String,
    pub label: String,
    pub is_initial: bool,
    pub is_terminal: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StateMachineTransition {
    pub from_state: String,
    pub to_state: String,
    pub event_code: String,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StateMachineDefinition {
    pub machine_code: String,
    pub machine_name: String,
    pub business_module: String,
    pub version: String,
    pub states: Vec<StateMachineState>,
    pub transitions: Vec<StateMachineTransition>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StateMachineDefinitionListResponse {
    pub data: Vec<StateMachineDefinition>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StateTransitionValidationResponse {
    pub machine_code: String,
    pub from_state: String,
    pub to_state: String,
    pub event_code: Option<String>,
    pub allowed: bool,
    pub reason: Option<String>,
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
