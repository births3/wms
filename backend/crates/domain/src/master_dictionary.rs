use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
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
    pub spec: String,
    /// 剂型。
    pub dosage_form: Option<String>,
    /// 生产企业。
    pub manufacturer: Option<String>,
    /// UDI 唯一码。
    pub udi_code: Option<String>,
    /// 电子监管码关联。
    pub electronic_regulatory_code: Option<String>,
    /// 69 码（中国商品条码）。
    pub barcode_69: Option<String>,
    /// 单品长度（毫米）。
    pub length_mm: Option<f64>,
    /// 单品宽度（毫米）。
    pub width_mm: Option<f64>,
    /// 单品高度（毫米）。
    pub height_mm: Option<f64>,
    /// 单品体积（立方厘米）。
    pub volume_cm3: Option<f64>,
    /// 单品重量（克）。
    pub weight_g: Option<f64>,
    /// 包装层级，转换比统一相对基础单位。
    pub packaging_levels: Vec<ProductPackagingLevel>,
    /// 经 M-PM 规整化字段的追加式溯源记录。
    pub mapping_traces: Vec<ProductMappingTrace>,
    /// 特殊药品分类编码。
    pub special_drug_category_code: Option<String>,
    /// 是否外用药专区属性。
    #[serde(default)]
    pub is_external_use: bool,
    /// 是否易串味专区属性。
    #[serde(default)]
    pub is_fragrant: bool,
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
    pub spec: String,
    pub dosage_form: Option<String>,
    pub manufacturer: Option<String>,
    pub special_drug_category_code: Option<String>,
    #[serde(default)]
    pub is_external_use: Option<bool>,
    #[serde(default)]
    pub is_fragrant: Option<bool>,
    pub udi_code: Option<String>,
    pub electronic_regulatory_code: Option<String>,
    pub barcode_69: Option<String>,
    pub length_mm: Option<f64>,
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub volume_cm3: Option<f64>,
    pub weight_g: Option<f64>,
    pub packaging_levels: Vec<ProductPackagingLevelInput>,
    #[schema(schema_with = free_form_json_schema)]
    pub attrs: serde_json::Value,
}

/// 更新商品请求。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateProductRequest {
    pub product_name: Option<String>,
    #[serde(default)]
    pub is_external_use: Option<bool>,
    #[serde(default)]
    pub is_fragrant: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub approval_no: Option<Option<String>>,
    pub spec: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub dosage_form: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub manufacturer: Option<Option<String>>,
    pub special_drug_category_code: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub udi_code: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub electronic_regulatory_code: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub barcode_69: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub length_mm: Option<Option<f64>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub width_mm: Option<Option<f64>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub height_mm: Option<Option<f64>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub volume_cm3: Option<Option<f64>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub weight_g: Option<Option<f64>>,
    pub packaging_levels: Option<Vec<ProductPackagingLevelInput>>,
    pub status: Option<String>,
    #[schema(schema_with = free_form_json_schema)]
    pub attrs: Option<serde_json::Value>,
}

fn deserialize_present_nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// 商品包装层级写入项。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ProductPackagingLevelInput {
    pub unit_code: String,
    pub unit_name: String,
    /// 相对基础单位的换算数量。
    pub ratio_to_base: i64,
    pub is_base: bool,
    pub is_default: bool,
    pub sort_order: i32,
}

/// 商品包装层级。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ProductPackagingLevel {
    pub id: Uuid,
    pub unit_code: String,
    pub unit_name: String,
    pub ratio_to_base: i64,
    pub is_base: bool,
    pub is_default: bool,
    pub sort_order: i32,
}

/// M-PM 规整化结果写入项，仅供受控防腐层调用。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ProductMappingTraceInput {
    pub field_name: String,
    pub rule_id: Option<Uuid>,
    pub source_system: String,
    pub source_value: String,
    pub target_value: Option<String>,
}

/// 商品字段映射溯源。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ProductMappingTrace {
    pub id: Uuid,
    pub field_name: String,
    pub rule_id: Option<Uuid>,
    pub source_system: String,
    pub source_value: String,
    pub target_value: Option<String>,
    pub created_at: DateTime<Utc>,
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
    #[schema(schema_with = free_form_json_schema)]
    #[serde(default = "default_empty_json_array")]
    pub allowed_categories: serde_json::Value,
    #[serde(default)]
    pub is_external_use_zone: bool,
    #[serde(default)]
    pub is_fragrant_zone: bool,
    #[serde(default)]
    pub is_special_drug_zone: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_empty_json_array() -> serde_json::Value {
    serde_json::json!([])
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateWarehouseZoneRequest {
    pub warehouse_id: Uuid,
    pub zone_code: String,
    pub zone_name: String,
    pub temperature_zone: String,
    pub quality_color: String,
    #[schema(schema_with = free_form_json_schema)]
    #[serde(default)]
    pub allowed_categories: Option<serde_json::Value>,
    #[serde(default)]
    pub is_external_use_zone: Option<bool>,
    #[serde(default)]
    pub is_fragrant_zone: Option<bool>,
    #[serde(default)]
    pub is_special_drug_zone: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateWarehouseZoneRequest {
    pub zone_name: Option<String>,
    pub temperature_zone: Option<String>,
    pub quality_color: Option<String>,
    #[schema(schema_with = free_form_json_schema)]
    #[serde(default)]
    pub allowed_categories: Option<serde_json::Value>,
    #[serde(default)]
    pub is_external_use_zone: Option<bool>,
    #[serde(default)]
    pub is_fragrant_zone: Option<bool>,
    #[serde(default)]
    pub is_special_drug_zone: Option<bool>,
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
    pub current_owner_id: Option<Uuid>,
    #[serde(default = "default_true")]
    pub allows_container: bool,
    #[serde(default = "default_single_product_only")]
    pub mix_product_policy: String,
    #[serde(default = "default_single_batch")]
    pub mix_batch_policy: String,
    #[serde(default = "default_lock_normal")]
    pub lock_status: String,
    pub pick_zone_level: Option<String>,
    pub pick_sequence_no: Option<i32>,
    pub putaway_sequence_no: Option<i32>,
    #[serde(default)]
    pub is_agv_managed: bool,
    pub agv_pod_code: Option<String>,
    pub agv_unreachable_at: Option<DateTime<Utc>>,
    pub replenish_strategy_id: Option<Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// PDA 扫码查单库位轻量响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PdaLocationInfo {
    pub location_id: Uuid,
    pub location_code: String,
    pub zone_code: String,
    pub temperature_zone: String,
    pub status: String,
    pub mix_product_policy: String,
    pub mix_batch_policy: String,
    pub max_volume_cm3: i64,
    pub used_volume_cm3: i64,
    pub remaining_volume_cm3: i64,
}

fn default_true() -> bool {
    true
}

fn default_single_product_only() -> String {
    "single_product_only".to_string()
}

fn default_single_batch() -> String {
    "single_batch".to_string()
}

fn default_lock_normal() -> String {
    "normal".to_string()
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
    pub current_owner_id: Option<Uuid>,
    #[serde(default)]
    pub allows_container: Option<bool>,
    pub mix_product_policy: Option<String>,
    pub mix_batch_policy: Option<String>,
    pub lock_status: Option<String>,
    pub pick_zone_level: Option<String>,
    pub pick_sequence_no: Option<i32>,
    pub putaway_sequence_no: Option<i32>,
    #[serde(default)]
    pub is_agv_managed: Option<bool>,
    pub agv_pod_code: Option<String>,
    pub replenish_strategy_id: Option<Uuid>,
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
    pub current_owner_id: Option<Uuid>,
    #[serde(default)]
    pub allows_container: Option<bool>,
    pub mix_product_policy: Option<String>,
    pub mix_batch_policy: Option<String>,
    pub lock_status: Option<String>,
    pub pick_zone_level: Option<String>,
    pub pick_sequence_no: Option<i32>,
    pub putaway_sequence_no: Option<i32>,
    #[serde(default)]
    pub is_agv_managed: Option<bool>,
    pub agv_pod_code: Option<String>,
    pub replenish_strategy_id: Option<Uuid>,
}

fn default_high_rack_mode() -> String {
    "high_rack".to_string()
}

fn default_location_volume() -> i64 {
    100000
}

fn default_location_sku() -> i32 {
    3
}

/// 库位批量生成向导请求。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BatchGenerateLocationsRequest {
    pub warehouse_id: Uuid,
    pub zone_id: Uuid,
    #[serde(default = "default_high_rack_mode")]
    pub rule_type: String,

    // 高架/静态货架参数
    pub prefix: Option<String>,
    pub aisle_start: Option<i32>,
    pub aisle_end: Option<i32>,
    pub row_start: Option<i32>,
    pub row_end: Option<i32>,
    pub column_start: Option<i32>,
    pub column_end: Option<i32>,
    pub layer_start: Option<i32>,
    pub layer_end: Option<i32>,
    pub grid_start: Option<i32>,
    pub grid_end: Option<i32>,

    // AGV 移动货架参数
    pub pod_prefix: Option<String>,
    pub pod_start: Option<i32>,
    pub pod_end: Option<i32>,

    // 容量参数
    #[serde(default = "default_location_volume")]
    pub max_volume_cm3: i64,
    #[serde(default = "default_location_sku")]
    pub max_sku_count: i32,

    // 作业形态与管控策略
    pub location_type: Option<String>,
    pub current_owner_id: Option<Uuid>,
    #[serde(default)]
    pub allows_container: Option<bool>,
    pub mix_product_policy: Option<String>,
    pub mix_batch_policy: Option<String>,
    pub lock_status: Option<String>,
    pub pick_zone_level: Option<String>,
    #[serde(default)]
    pub is_agv_managed: Option<bool>,
    pub agv_pod_code: Option<String>,
    pub replenish_strategy_id: Option<Uuid>,

    // 动线顺序计算参数
    pub initial_pick_sequence: Option<i32>,
    pub pick_sequence_step: Option<i32>,
    pub initial_putaway_sequence: Option<i32>,
    pub putaway_sequence_step: Option<i32>,
    pub reverse_aisle_direction: Option<bool>,
}

/// 库位批量生成向导响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BatchGenerateLocationsResponse {
    pub total_generated: u32,
    pub locations: Vec<Location>,
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
    pub current_owner_id: Option<Uuid>,
    pub allows_container: Option<bool>,
    pub mix_product_policy: Option<String>,
    pub mix_batch_policy: Option<String>,
    pub lock_status: Option<String>,
    pub pick_zone_level: Option<String>,
    pub pick_sequence_no: Option<i32>,
    pub putaway_sequence_no: Option<i32>,
    pub is_agv_managed: Option<bool>,
    pub agv_pod_code: Option<String>,
    pub replenish_strategy_id: Option<Uuid>,
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
