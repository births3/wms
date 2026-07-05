//! 主仓 OpenAPI 契约。

pub mod audit;
pub mod auth;
pub mod auth_handlers;
pub mod auth_repository;
pub mod auth_service;
pub mod billing;
pub mod cold_chain;
pub mod config_center;
pub mod deploy_audit;
pub mod document_numbering;
pub mod document_numbering_handlers;
mod document_numbering_repository;
pub mod feature_flags;
pub mod inbound;
pub mod inventory;
pub mod master_data;
pub mod master_data_handlers;
pub mod master_data_postgres;
mod openapi_contract;
pub mod outbound;
pub mod packing_station;
pub mod parameter_mapping;
pub mod print_template;
pub mod reports;
pub mod retail_chain;
pub mod system_dictionary;
pub mod system_dictionary_handlers;
pub mod tms_plus;
pub mod traceability_code;
pub mod wave3_handlers;
pub mod wave3_repository;
pub mod wave4_handlers;
pub mod wave4_repository;
pub mod wave5_handlers;
pub mod wave5_repository;

use crate::document_numbering::{
    DocumentNumberRule, DocumentNumberRuleListResponse, SetDocumentNumberRuleEnabledRequest,
    UpsertDocumentNumberRuleRequest,
};
use crate::openapi_contract::ContractSecurityAddon;
use utoipa::OpenApi;
use wms_domain::{
    AuditActor, AuditEvent, AuditEventListResponse, BatchCreateLocationsRequest, BillingAccount,
    BillingChargeCalculation, BillingContract, BillingRule, BillingStatement,
    CalculateBillingChargesRequest, ChangeInventoryStatusRequest, ColdChainDevice,
    CompletePickTaskRequest, ConfigEntry, ConfirmBillingStatementRequest,
    ConfirmContainerRecoveryRequest, ContainerRecovery, CreateBillingAccountRequest,
    CreateBillingContractRequest, CreateBillingRuleRequest, CreateColdChainDeviceRequest,
    CreateCrossdockPlanRequest, CreateCustomerRequest, CreateLocationRequest,
    CreateOutboundOrderLineRequest, CreateOutboundOrderRequest, CreateOutboundWaveRequest,
    CreatePackJobRequest, CreatePackingStationRequest, CreateProductRequest,
    CreateReceivingOrderRequest, CreateRetailReplenishmentSuggestionRequest,
    CreateSpecialDrugCategoryRequest, CreateSupplierRequest, CreateWarehouseRequest, CrossdockPlan,
    CurrentUser, Customer, CustomerListResponse, DisableSystemDictionaryItemRequest,
    DisposeTemperatureExcursionRequest, DocumentNumberAllocation,
    DocumentNumberAllocationListResponse, DriverTask, DriverTaskListResponse, ErrorResponse,
    ExecuteMappingRequest, ExecuteMappingResponse, FeatureFlagArchiveRequest,
    FeatureFlagArchiveResult, FeatureFlagBatchImportRequest, FeatureFlagBatchImportResult,
    FeatureFlagConfig, FeatureFlagExportResponse, FeatureFlagMigrationResult,
    FeatureFlagReconcileReport, FeatureFlagSourceSwitchRequest, FeatureFlagSourceSwitchResponse,
    GenerateBillingStatementRequest, GspLedgerReport, GspLedgerRow, HealthzResponse,
    IngestTemperatureExcursionRequest, IngestTemperatureReadingRequest,
    IngestTransitTemperatureRequest, InspectReceivingOrderRequest, InspectionSignatureRecord,
    InventoryBatch, InventoryBatchListResponse, InventoryMovement, Location, LocationListResponse,
    LoginRequest, LoginResponse, MappingDictionary, MappingQueueItem, MappingRule,
    MappingTraceResponse, OutboundOrder, OutboundOrderLine, OutboundOrderListResponse,
    OutboundWave, PackJob, PackingStation, PageMeta, PrintWaybillRequest, Product,
    ProductListResponse, PutawayInventoryRequest, PutawayRecord, PutawayRequest,
    ReceiveReceivingOrderRequest, ReceiveTmsDispatchRequest, ReceivingInspectionRecord,
    ReceivingOrder, ReceivingOrderLine, ReceivingOrderListResponse, ReceivingOrderReceipt,
    RejectReceivingOrderRequest, ReportQueryRequest, ReportQueryResponse, ReportRow,
    RetailReplenishmentSuggestion, ReviewOutboundOrderRequest, ShipOutboundOrderRequest,
    SignInspectionRequest, SpecialDrugCategory, SpecialDrugCategoryListResponse,
    StoreDashboardResponse, Supplier, SupplierListResponse, SystemDictionaryCategory,
    SystemDictionaryImpactPreview, SystemDictionaryImpactReference, SystemDictionaryItem,
    SystemDictionaryItemListResponse, TemperatureExcursionDispositionResponse,
    TemperatureExcursionEvent, TemperatureExcursionEventListResponse, TemperatureReading,
    TmsDispatch, TraceabilityOutboundReport, TraceabilityOutboundReportRequest,
    TraceabilityStatusChangeEvent, TransitTemperatureReading, UpdateCustomerRequest,
    UpdateLocationRequest, UpdateProductRequest, UpdateReceivingOrderRequest,
    UpdateSpecialDrugCategoryRequest, UpdateSupplierRequest, UpdateWarehouseRequest,
    UpsertSystemDictionaryItemRequest, Warehouse, WarehouseListResponse, WeighPackJobRequest,
};

#[utoipa::path(
    get,
    path = "/api/v1/healthz",
    tag = "system",
    responses(
        (status = 200, description = "服务健康", body = HealthzResponse),
    ),
)]
#[allow(dead_code)]
fn healthz() {}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn login() {}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "当前登录用户", body = CurrentUser),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn me() {}

#[utoipa::path(
    get,
    path = "/api/v1/audit/events",
    tag = "audit",
    params(
        ("resource_type" = Option<String>, Query, description = "按资源类型过滤"),
        ("actor_id" = Option<uuid::Uuid>, Query, description = "按操作者过滤"),
        ("from" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "开始时间（RFC3339）"),
        ("to" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "结束时间（RFC3339）"),
        ("limit" = Option<u32>, Query, description = "每页条数"),
        ("cursor" = Option<String>, Query, description = "分页游标"),
    ),
    responses(
        (status = 200, description = "审计事件列表", body = AuditEventListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn list_audit_events() {}

#[utoipa::path(
    get,
    path = "/api/v1/master-data/products",
    tag = "master-data",
    responses(
        (status = 200, description = "商品列表", body = ProductListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn list_products() {}

#[utoipa::path(
    post,
    path = "/api/v1/master-data/products",
    tag = "master-data",
    request_body = CreateProductRequest,
    responses(
        (status = 200, description = "创建商品", body = Product),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn create_product() {}

#[utoipa::path(
    get,
    path = "/api/v1/master-data/products/{id}",
    tag = "master-data",
    params(("id" = uuid::Uuid, Path, description = "商品 ID")),
    responses(
        (status = 200, description = "商品详情", body = Product),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn get_product() {}

#[utoipa::path(
    patch,
    path = "/api/v1/master-data/products/{id}",
    tag = "master-data",
    params(("id" = uuid::Uuid, Path, description = "商品 ID")),
    request_body = UpdateProductRequest,
    responses(
        (status = 200, description = "更新商品", body = Product),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn update_product() {}

#[utoipa::path(
    delete,
    path = "/api/v1/master-data/products/{id}",
    tag = "master-data",
    params(("id" = uuid::Uuid, Path, description = "商品 ID")),
    responses(
        (status = 200, description = "删除商品", body = Product),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn delete_product() {}

#[utoipa::path(get, path = "/api/v1/master-data/suppliers", tag = "master-data", responses((status = 200, description = "供应商列表", body = SupplierListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn list_suppliers() {}

#[utoipa::path(post, path = "/api/v1/master-data/suppliers", tag = "master-data", request_body = CreateSupplierRequest, responses((status = 200, description = "创建供应商", body = Supplier), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_supplier() {}

#[utoipa::path(patch, path = "/api/v1/master-data/suppliers/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "供应商 ID")), request_body = UpdateSupplierRequest, responses((status = 200, description = "更新供应商", body = Supplier), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn update_supplier() {}

#[utoipa::path(delete, path = "/api/v1/master-data/suppliers/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "供应商 ID")), responses((status = 200, description = "删除供应商", body = Supplier), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn delete_supplier() {}

#[utoipa::path(get, path = "/api/v1/master-data/customers", tag = "master-data", responses((status = 200, description = "客户列表", body = CustomerListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn list_customers() {}

#[utoipa::path(post, path = "/api/v1/master-data/customers", tag = "master-data", request_body = CreateCustomerRequest, responses((status = 200, description = "创建客户", body = Customer), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_customer() {}

#[utoipa::path(patch, path = "/api/v1/master-data/customers/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "客户 ID")), request_body = UpdateCustomerRequest, responses((status = 200, description = "更新客户", body = Customer), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn update_customer() {}

#[utoipa::path(delete, path = "/api/v1/master-data/customers/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "客户 ID")), responses((status = 200, description = "删除客户", body = Customer), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn delete_customer() {}

#[utoipa::path(get, path = "/api/v1/master-data/warehouses", tag = "master-data", responses((status = 200, description = "仓库列表", body = WarehouseListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn list_warehouses() {}

#[utoipa::path(post, path = "/api/v1/master-data/warehouses", tag = "master-data", request_body = CreateWarehouseRequest, responses((status = 200, description = "创建仓库", body = Warehouse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_warehouse() {}

#[utoipa::path(patch, path = "/api/v1/master-data/warehouses/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "仓库 ID")), request_body = UpdateWarehouseRequest, responses((status = 200, description = "更新仓库", body = Warehouse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn update_warehouse() {}

#[utoipa::path(delete, path = "/api/v1/master-data/warehouses/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "仓库 ID")), responses((status = 200, description = "删除仓库", body = Warehouse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn delete_warehouse() {}

#[utoipa::path(get, path = "/api/v1/master-data/locations", tag = "master-data", responses((status = 200, description = "库位列表", body = LocationListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn list_locations() {}

#[utoipa::path(post, path = "/api/v1/master-data/locations", tag = "master-data", request_body = CreateLocationRequest, responses((status = 200, description = "创建库位", body = Location), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_location() {}

#[utoipa::path(post, path = "/api/v1/master-data/locations/batch-create", tag = "master-data", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = BatchCreateLocationsRequest, responses((status = 200, description = "批量创建库位", body = LocationListResponse), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 409, description = "库位编码或幂等冲突", body = ErrorResponse), (status = 422, description = "库位批量创建范围非法", body = ErrorResponse)))]
#[allow(dead_code)]
fn batch_create_locations() {}

#[utoipa::path(patch, path = "/api/v1/master-data/locations/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "库位 ID")), request_body = UpdateLocationRequest, responses((status = 200, description = "更新库位", body = Location), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn update_location() {}

#[utoipa::path(delete, path = "/api/v1/master-data/locations/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "库位 ID")), responses((status = 200, description = "删除库位", body = Location), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn delete_location() {}

#[utoipa::path(get, path = "/api/v1/master-data/special-drug-categories", tag = "master-data", responses((status = 200, description = "特殊药品分类列表", body = SpecialDrugCategoryListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn list_special_drug_categories() {}

#[utoipa::path(post, path = "/api/v1/master-data/special-drug-categories", tag = "master-data", request_body = CreateSpecialDrugCategoryRequest, responses((status = 200, description = "创建特殊药品分类", body = SpecialDrugCategory), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_special_drug_category() {}

#[utoipa::path(patch, path = "/api/v1/master-data/special-drug-categories/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "特殊药品分类 ID")), request_body = UpdateSpecialDrugCategoryRequest, responses((status = 200, description = "更新特殊药品分类", body = SpecialDrugCategory), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn update_special_drug_category() {}

#[utoipa::path(delete, path = "/api/v1/master-data/special-drug-categories/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "特殊药品分类 ID")), responses((status = 200, description = "删除特殊药品分类", body = SpecialDrugCategory), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn delete_special_drug_category() {}

#[utoipa::path(
    get,
    path = "/api/v1/system-dictionaries/{dict_code}/items",
    tag = "system-dictionary",
    params(
        ("dict_code" = String, Path, description = "字典分类编码"),
        ("effective_at" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "按指定时间查询有效字典项"),
    ),
    responses(
        (status = 200, description = "按货主合并后的有效字典项", body = SystemDictionaryItemListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "字典分类不存在或停用", body = ErrorResponse),
        (status = 422, description = "运行时字典参数无效，fail closed", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn list_system_dictionary_items() {}

#[utoipa::path(
    get,
    path = "/api/v1/system-dictionaries/{dict_code}/items/{item_code}/impact-preview",
    tag = "system-dictionary",
    params(
        ("dict_code" = String, Path, description = "字典分类编码"),
        ("item_code" = String, Path, description = "字典项编码"),
        ("owner_id" = Option<uuid::Uuid>, Query, description = "预览指定货主影响；默认当前货主"),
        ("effective_at" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "按指定时间统计影响"),
    ),
    responses(
        (status = 200, description = "字典项引用影响预览", body = SystemDictionaryImpactPreview),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "跨货主预览越权", body = ErrorResponse),
        (status = 404, description = "字典分类或字典项不存在", body = ErrorResponse),
        (status = 422, description = "运行时字典参数无效，fail closed", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn preview_system_dictionary_item_impact() {}

#[utoipa::path(
    put,
    path = "/api/v1/system-dictionaries/{dict_code}/items/{item_code}",
    tag = "system-dictionary",
    params(
        ("dict_code" = String, Path, description = "字典分类编码"),
        ("item_code" = String, Path, description = "字典项编码"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = UpsertSystemDictionaryItemRequest,
    responses(
        (status = 200, description = "创建或更新字典项", body = SystemDictionaryItem),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "字典分类不存在", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "字典项参数或作用域非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn upsert_system_dictionary_item() {}

#[utoipa::path(
    patch,
    path = "/api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable",
    tag = "system-dictionary",
    params(
        ("dict_code" = String, Path, description = "字典分类编码"),
        ("item_code" = String, Path, description = "字典项编码"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = DisableSystemDictionaryItemRequest,
    responses(
        (status = 200, description = "停用字典项", body = SystemDictionaryItem),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "字典分类或字典项不存在", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "字典项作用域非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn disable_system_dictionary_item() {}

#[utoipa::path(
    get,
    path = "/api/v1/code-generator/document-number-rules",
    tag = "code-generator",
    params(
        ("document_type" = Option<String>, Query, description = "按单据类型过滤"),
    ),
    responses(
        (status = 200, description = "单据号规则列表", body = DocumentNumberRuleListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn list_document_number_rules() {}

#[utoipa::path(
    put,
    path = "/api/v1/code-generator/document-number-rules/{rule_code}",
    tag = "code-generator",
    params(
        ("rule_code" = String, Path, description = "规则编码"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = UpsertDocumentNumberRuleRequest,
    responses(
        (status = 200, description = "创建或更新单据号规则", body = DocumentNumberRule),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "规则非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn upsert_document_number_rule() {}

#[utoipa::path(
    patch,
    path = "/api/v1/code-generator/document-number-rules/{rule_code}/enabled",
    tag = "code-generator",
    params(
        ("rule_code" = String, Path, description = "规则编码"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = SetDocumentNumberRuleEnabledRequest,
    responses(
        (status = 200, description = "启用或停用单据号规则", body = DocumentNumberRule),
        (status = 400, description = "缺少或非法幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "规则不存在", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn set_document_number_rule_enabled() {}

#[utoipa::path(
    get,
    path = "/api/v1/code-generator/document-number-allocations",
    tag = "code-generator",
    params(
        ("document_type" = Option<String>, Query, description = "按单据类型过滤"),
        ("from" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "生成时间起点（RFC3339，含）"),
        ("to" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "生成时间终点（RFC3339，含）"),
        ("limit" = Option<u32>, Query, description = "返回条数，默认 50，最大 100"),
    ),
    responses(
        (status = 200, description = "单据号生成记录列表", body = DocumentNumberAllocationListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn list_document_number_allocations() {}

#[utoipa::path(get, path = "/api/v1/inbound/receiving-orders", tag = "inbound", responses((status = 200, description = "收货单列表", body = ReceivingOrderListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn list_receiving_orders() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders", tag = "inbound", request_body = CreateReceivingOrderRequest, responses((status = 200, description = "创建收货单", body = ReceivingOrder), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_receiving_order() {}

#[utoipa::path(get, path = "/api/v1/inbound/receiving-orders/{id}", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID")), responses((status = 200, description = "收货单详情", body = ReceivingOrder), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn get_receiving_order() {}

#[utoipa::path(patch, path = "/api/v1/inbound/receiving-orders/{id}", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID")), request_body = UpdateReceivingOrderRequest, responses((status = 200, description = "更新收货单", body = ReceivingOrder), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn update_receiving_order() {}

#[utoipa::path(delete, path = "/api/v1/inbound/receiving-orders/{id}", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID")), responses((status = 200, description = "删除收货单", body = ReceivingOrder), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn delete_receiving_order() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/receive", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ReceiveReceivingOrderRequest, responses((status = 200, description = "PDA 收货闭环记录", body = ReceivingOrderReceipt), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn receive_receiving_order() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/reject", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = RejectReceivingOrderRequest, responses((status = 200, description = "整单拒收闭环记录", body = ReceivingOrderReceipt), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn reject_receiving_order() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/inspect", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = InspectReceivingOrderRequest, responses((status = 200, description = "PDA 验收记录", body = ReceivingInspectionRecord), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn inspect_receiving_order() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/sign", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = SignInspectionRequest, responses((status = 200, description = "双人验收签字记录", body = InspectionSignatureRecord), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn sign_receiving_order_inspection() {}

#[utoipa::path(post, path = "/api/v1/inbound/receiving-orders/{id}/putaway", tag = "inbound", params(("id" = uuid::Uuid, Path, description = "收货单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = PutawayRequest, responses((status = 200, description = "PDA 上架记录", body = PutawayRecord), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn putaway_receiving_order() {}

#[utoipa::path(get, path = "/api/v1/inventory/batches", tag = "inventory", responses((status = 200, description = "库存批次列表", body = InventoryBatchListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn list_inventory_batches() {}

#[utoipa::path(post, path = "/api/v1/inventory/batches/putaway", tag = "inventory", request_body = PutawayInventoryRequest, responses((status = 200, description = "入库上架增加库存", body = InventoryBatch), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn putaway_inventory_batch() {}

#[utoipa::path(post, path = "/api/v1/inventory/batches/status", tag = "inventory", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ChangeInventoryStatusRequest, responses((status = 200, description = "库存状态变更", body = InventoryBatch), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn change_inventory_batch_status() {}

#[utoipa::path(post, path = "/api/v1/outbound/orders", tag = "outbound", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateOutboundOrderRequest, responses((status = 200, description = "创建出库订单", body = OutboundOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 409, description = "单号或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_outbound_order() {}

#[utoipa::path(
    get,
    path = "/api/v1/outbound/orders",
    tag = "outbound",
    params(
        ("status" = Option<String>, Query, description = "按出库订单状态过滤"),
        ("q" = Option<String>, Query, description = "按 WMS/ERP 单号模糊查询"),
        ("limit" = Option<u32>, Query, description = "返回条数，默认 50，最大 200"),
    ),
    responses(
        (status = 200, description = "出库订单列表", body = OutboundOrderListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn list_outbound_orders() {}

#[utoipa::path(
    get,
    path = "/api/v1/outbound/orders/{id}",
    tag = "outbound",
    params(("id" = uuid::Uuid, Path, description = "出库订单 ID")),
    responses(
        (status = 200, description = "出库订单详情", body = OutboundOrder),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 404, description = "出库订单不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn get_outbound_order() {}

#[utoipa::path(post, path = "/api/v1/outbound/waves", tag = "outbound", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateOutboundWaveRequest, responses((status = 200, description = "创建并下发出库波次", body = OutboundWave), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "订单状态不可入波次", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_outbound_wave() {}

#[utoipa::path(post, path = "/api/v1/outbound/pick-tasks/{id}/complete", tag = "outbound", params(("id" = uuid::Uuid, Path, description = "出库订单 ID；当前最小闭环按订单行完成拣选"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CompletePickTaskRequest, responses((status = 200, description = "完成拣选任务，短拣时订单进入待补齐状态", body = OutboundOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "数量或状态非法", body = ErrorResponse)))]
#[allow(dead_code)]
fn complete_outbound_pick_task() {}

#[utoipa::path(post, path = "/api/v1/outbound/orders/{id}/review", tag = "outbound", params(("id" = uuid::Uuid, Path, description = "出库订单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ReviewOutboundOrderRequest, responses((status = 200, description = "完成出库复核", body = OutboundOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "订单状态不可复核", body = ErrorResponse)))]
#[allow(dead_code)]
fn review_outbound_order() {}

#[utoipa::path(post, path = "/api/v1/outbound/orders/{id}/ship", tag = "outbound", params(("id" = uuid::Uuid, Path, description = "出库订单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ShipOutboundOrderRequest, responses((status = 200, description = "发货交接并扣减库存", body = OutboundOrder), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "短拣未补齐或库存不足", body = ErrorResponse)))]
#[allow(dead_code)]
fn ship_outbound_order() {}

#[utoipa::path(post, path = "/api/v1/reports/query", tag = "reports", request_body = ReportQueryRequest, responses((status = 200, description = "报表查询结果", body = ReportQueryResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn query_report() {}

#[utoipa::path(post, path = "/api/v1/reports/gsp/inbound-ledger", tag = "reports", request_body = ReportQueryRequest, responses((status = 200, description = "GSP 入库验收台账", body = GspLedgerReport), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn query_gsp_inbound_ledger() {}

#[utoipa::path(post, path = "/api/v1/reports/gsp/outbound-ledger", tag = "reports", request_body = ReportQueryRequest, responses((status = 200, description = "GSP 出库复核台账", body = GspLedgerReport), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn query_gsp_outbound_ledger() {}

#[utoipa::path(post, path = "/api/v1/reports/gsp/inventory-ledger", tag = "reports", request_body = ReportQueryRequest, responses((status = 200, description = "GSP 库存流水台账", body = GspLedgerReport), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn query_gsp_inventory_ledger() {}

#[utoipa::path(post, path = "/api/v1/traceability/outbound-reports", tag = "traceability", request_body = TraceabilityOutboundReportRequest, responses((status = 200, description = "追溯码出库核销待上报记录", body = TraceabilityOutboundReport), (status = 401, description = "未登录", body = ErrorResponse), (status = 422, description = "追溯码状态变更三元组不完整", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_traceability_outbound_report() {}

#[utoipa::path(get, path = "/api/v1/driver/tasks/today", tag = "driver", responses((status = 200, description = "司机今日配送任务", body = DriverTaskListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn list_driver_today_tasks() {}

#[utoipa::path(get, path = "/api/v1/store/dashboard", tag = "store", responses((status = 200, description = "门店首页业务概览", body = StoreDashboardResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn get_store_dashboard() {}

#[utoipa::path(post, path = "/api/v1/parameter-mapping/execute", tag = "parameter-mapping", request_body = ExecuteMappingRequest, responses((status = 200, description = "执行参数对照", body = ExecuteMappingResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn execute_mapping() {}

#[utoipa::path(get, path = "/api/v1/parameter-mapping/traces/{execution_id}", tag = "parameter-mapping", params(("execution_id" = uuid::Uuid, Path, description = "执行 ID")), responses((status = 200, description = "参数对照反向追溯", body = MappingTraceResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn trace_mapping() {}

#[utoipa::path(post, path = "/api/v1/config-center/feature-flags/migrate", tag = "config-center", responses((status = 200, description = "迁移文件版 Feature Flag", body = FeatureFlagMigrationResult), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn migrate_feature_flags() {}

#[utoipa::path(get, path = "/api/v1/config-center/feature-flags/reconcile", tag = "config-center", responses((status = 200, description = "Feature Flag 对账报告", body = FeatureFlagReconcileReport), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn reconcile_feature_flags() {}

#[utoipa::path(get, path = "/api/v1/config-center/feature-flags/export", tag = "config-center", responses((status = 200, description = "导出配置中心 Feature Flag", body = FeatureFlagExportResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn export_feature_flags() {}

#[utoipa::path(post, path = "/api/v1/config-center/feature-flags/import", tag = "config-center", request_body = FeatureFlagBatchImportRequest, responses((status = 200, description = "批量导入配置中心 Feature Flag", body = FeatureFlagBatchImportResult), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn import_feature_flags() {}

#[utoipa::path(post, path = "/api/v1/config-center/feature-flags/source", tag = "config-center", request_body = FeatureFlagSourceSwitchRequest, responses((status = 200, description = "切换 Feature Flag 读取源", body = FeatureFlagSourceSwitchResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn switch_feature_flag_source() {}

#[utoipa::path(post, path = "/api/v1/config-center/feature-flags/archive-file-source", tag = "config-center", request_body = FeatureFlagArchiveRequest, responses((status = 200, description = "归档 W1 文件版 Feature Flag", body = FeatureFlagArchiveResult), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn archive_feature_flag_file_source() {}

#[utoipa::path(post, path = "/api/v1/cold-chain/devices", tag = "cold-chain", request_body = CreateColdChainDeviceRequest, responses((status = 200, description = "创建冷链设备台账", body = ColdChainDevice), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_cold_chain_device() {}

#[utoipa::path(post, path = "/api/v1/cold-chain/readings", tag = "cold-chain", params(("Idempotency-Key" = String, Header, description = "外部系统生成的幂等键"), ("X-WMS-API-Key" = String, Header, description = "外部冷链系统 API Key")), request_body = IngestTemperatureReadingRequest, responses((status = 200, description = "接收外部温控数据", body = TemperatureReading), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "外部系统 API Key 缺失或无效", body = ErrorResponse)))]
#[allow(dead_code)]
fn ingest_temperature_reading() {}

#[utoipa::path(post, path = "/api/v1/cold-chain/excursions", tag = "cold-chain", params(("Idempotency-Key" = String, Header, description = "外部系统生成的幂等键"), ("X-WMS-API-Key" = String, Header, description = "外部冷链系统 API Key")), request_body = IngestTemperatureExcursionRequest, responses((status = 200, description = "接收外部温度超标事件", body = TemperatureExcursionEvent), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "外部系统 API Key 缺失或无效", body = ErrorResponse)))]
#[allow(dead_code)]
fn ingest_temperature_excursion() {}

#[utoipa::path(get, path = "/api/v1/cold-chain/excursions/pending-disposition", tag = "cold-chain", responses((status = 200, description = "温度超标待处置列表", body = TemperatureExcursionEventListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn list_pending_temperature_excursions() {}

#[utoipa::path(post, path = "/api/v1/cold-chain/excursions/{external_event_id}/dispose", tag = "cold-chain", params(("external_event_id" = String, Path, description = "外部冷链系统事件 ID")), request_body = DisposeTemperatureExcursionRequest, responses((status = 200, description = "温度超标处置并隔离批次", body = TemperatureExcursionDispositionResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "温度超标事件不存在", body = ErrorResponse), (status = 422, description = "批次不在影响范围或事件状态不可处置", body = ErrorResponse)))]
#[allow(dead_code)]
fn dispose_temperature_excursion() {}

#[utoipa::path(post, path = "/api/v1/billing/accounts", tag = "billing", request_body = CreateBillingAccountRequest, responses((status = 200, description = "创建计费账户", body = BillingAccount), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_billing_account() {}

#[utoipa::path(post, path = "/api/v1/billing/contracts", tag = "billing", request_body = CreateBillingContractRequest, responses((status = 200, description = "创建计费合同", body = BillingContract), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_billing_contract() {}

#[utoipa::path(post, path = "/api/v1/billing/rules", tag = "billing", request_body = CreateBillingRuleRequest, responses((status = 200, description = "创建计费规则", body = BillingRule), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_billing_rule() {}

#[utoipa::path(post, path = "/api/v1/packing/stations", tag = "packing", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreatePackingStationRequest, responses((status = 200, description = "创建包装工位", body = PackingStation), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 409, description = "工位或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_packing_station() {}

#[utoipa::path(post, path = "/api/v1/packing/jobs", tag = "packing", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreatePackJobRequest, responses((status = 200, description = "创建装箱任务", body = PackJob), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "出库订单或工位不存在", body = ErrorResponse), (status = 409, description = "装箱任务或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_pack_job() {}

#[utoipa::path(post, path = "/api/v1/packing/jobs/{id}/weigh", tag = "packing", params(("id" = uuid::Uuid, Path, description = "装箱任务 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = WeighPackJobRequest, responses((status = 200, description = "记录装箱称重", body = PackJob), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "装箱任务不存在", body = ErrorResponse), (status = 422, description = "称重数据非法", body = ErrorResponse)))]
#[allow(dead_code)]
fn weigh_pack_job() {}

#[utoipa::path(post, path = "/api/v1/packing/jobs/{id}/waybill", tag = "packing", params(("id" = uuid::Uuid, Path, description = "装箱任务 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = PrintWaybillRequest, responses((status = 200, description = "记录面单打印结果", body = PackJob), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "装箱任务不存在", body = ErrorResponse), (status = 422, description = "面单数据非法", body = ErrorResponse)))]
#[allow(dead_code)]
fn print_pack_job_waybill() {}

#[utoipa::path(post, path = "/api/v1/retail/replenishment-suggestions", tag = "retail", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateRetailReplenishmentSuggestionRequest, responses((status = 200, description = "生成门店补货建议", body = RetailReplenishmentSuggestion), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 409, description = "建议或幂等冲突", body = ErrorResponse), (status = 422, description = "补货水位非法", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_retail_replenishment_suggestion() {}

#[utoipa::path(post, path = "/api/v1/retail/crossdock-plans", tag = "retail", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateCrossdockPlanRequest, responses((status = 200, description = "创建门店越库计划", body = CrossdockPlan), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "出库订单不存在", body = ErrorResponse), (status = 422, description = "越库数量非法", body = ErrorResponse)))]
#[allow(dead_code)]
fn create_retail_crossdock_plan() {}

#[utoipa::path(post, path = "/api/v1/billing/charges/calculate", tag = "billing", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CalculateBillingChargesRequest, responses((status = 200, description = "计算周期计费明细", body = BillingChargeCalculation), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "计费合同不存在", body = ErrorResponse), (status = 409, description = "计费明细或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
fn calculate_billing_charges() {}

#[utoipa::path(post, path = "/api/v1/billing/statements", tag = "billing", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = GenerateBillingStatementRequest, responses((status = 200, description = "生成月结账单", body = BillingStatement), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "计费合同不存在", body = ErrorResponse), (status = 409, description = "账单或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
fn generate_billing_statement() {}

#[utoipa::path(post, path = "/api/v1/billing/statements/{id}/confirm", tag = "billing", params(("id" = uuid::Uuid, Path, description = "月结账单 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ConfirmBillingStatementRequest, responses((status = 200, description = "确认月结账单", body = BillingStatement), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "账单不存在", body = ErrorResponse), (status = 422, description = "账单状态不可确认", body = ErrorResponse)))]
#[allow(dead_code)]
fn confirm_billing_statement() {}

#[utoipa::path(post, path = "/api/v1/tms/dispatches", tag = "tms", params(("Idempotency-Key" = String, Header, description = "外部 TMS 生成的幂等键")), request_body = ReceiveTmsDispatchRequest, responses((status = 200, description = "接收 TMS 调度", body = TmsDispatch), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "出库订单不存在", body = ErrorResponse), (status = 409, description = "调度或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
fn receive_tms_dispatch() {}

#[utoipa::path(post, path = "/api/v1/tms/transit-temperature-readings", tag = "tms", params(("Idempotency-Key" = String, Header, description = "外部 TMS 生成的幂等键")), request_body = IngestTransitTemperatureRequest, responses((status = 200, description = "接收在途温控读数", body = TransitTemperatureReading), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "TMS 调度不存在", body = ErrorResponse), (status = 422, description = "温控数据非法", body = ErrorResponse)))]
#[allow(dead_code)]
fn ingest_transit_temperature() {}

#[utoipa::path(post, path = "/api/v1/tms/container-recoveries", tag = "tms", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = ConfirmContainerRecoveryRequest, responses((status = 200, description = "确认周转容器回收", body = ContainerRecovery), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 404, description = "TMS 调度不存在", body = ErrorResponse), (status = 409, description = "容器回收或幂等冲突", body = ErrorResponse)))]
#[allow(dead_code)]
fn confirm_container_recovery() {}

#[derive(OpenApi)]
#[openapi(
    modifiers(&ContractSecurityAddon),
    info(
        title = "WMS API",
        version = "0.0.5-wave-5-value-added",
        description = "Wave 5 增值模块与横向能力契约",
    ),
    paths(
        healthz,
        login,
        me,
        list_audit_events,
        list_products,
        create_product,
        get_product,
        update_product,
        delete_product,
        list_suppliers,
        create_supplier,
        update_supplier,
        delete_supplier,
        list_customers,
        create_customer,
        update_customer,
        delete_customer,
        list_warehouses,
        create_warehouse,
        update_warehouse,
        delete_warehouse,
        list_locations,
        create_location,
        batch_create_locations,
        update_location,
        delete_location,
        list_special_drug_categories,
        create_special_drug_category,
        update_special_drug_category,
        delete_special_drug_category,
        list_system_dictionary_items,
        preview_system_dictionary_item_impact,
        upsert_system_dictionary_item,
        disable_system_dictionary_item,
        list_document_number_rules,
        upsert_document_number_rule,
        set_document_number_rule_enabled,
        list_document_number_allocations,
        list_receiving_orders,
        create_receiving_order,
        get_receiving_order,
        update_receiving_order,
        delete_receiving_order,
        receive_receiving_order,
        reject_receiving_order,
        inspect_receiving_order,
        sign_receiving_order_inspection,
        putaway_receiving_order,
        list_inventory_batches,
        putaway_inventory_batch,
        change_inventory_batch_status,
        create_outbound_order,
        list_outbound_orders,
        get_outbound_order,
        create_outbound_wave,
        complete_outbound_pick_task,
        review_outbound_order,
        ship_outbound_order,
        query_report,
        query_gsp_inbound_ledger,
        query_gsp_outbound_ledger,
        query_gsp_inventory_ledger,
        create_traceability_outbound_report,
        list_driver_today_tasks,
        get_store_dashboard,
        execute_mapping,
        trace_mapping,
        migrate_feature_flags,
        reconcile_feature_flags,
        export_feature_flags,
        import_feature_flags,
        switch_feature_flag_source,
        archive_feature_flag_file_source,
        create_cold_chain_device,
        ingest_temperature_reading,
        ingest_temperature_excursion,
        list_pending_temperature_excursions,
        dispose_temperature_excursion,
        create_billing_account,
        create_billing_contract,
        create_billing_rule,
        create_packing_station,
        create_pack_job,
        weigh_pack_job,
        print_pack_job_waybill,
        create_retail_replenishment_suggestion,
        create_retail_crossdock_plan,
        calculate_billing_charges,
        generate_billing_statement,
        confirm_billing_statement,
        receive_tms_dispatch,
        ingest_transit_temperature,
        confirm_container_recovery,
    ),
    components(schemas(
        AuditActor,
        AuditEvent,
        AuditEventListResponse,
        BatchCreateLocationsRequest,
        BillingAccount,
        BillingChargeCalculation,
        BillingContract,
        BillingRule,
        BillingStatement,
        CalculateBillingChargesRequest,
        ChangeInventoryStatusRequest,
        ColdChainDevice,
        CompletePickTaskRequest,
        ConfirmBillingStatementRequest,
        ConfirmContainerRecoveryRequest,
        ConfigEntry,
        ContainerRecovery,
        CreateBillingAccountRequest,
        CreateBillingContractRequest,
        CreateBillingRuleRequest,
        CreateColdChainDeviceRequest,
        CreateCrossdockPlanRequest,
        CreateCustomerRequest,
        CreateLocationRequest,
        CreateOutboundOrderLineRequest,
        CreateOutboundOrderRequest,
        CreateOutboundWaveRequest,
        CreatePackJobRequest,
        CreatePackingStationRequest,
        CreateProductRequest,
        CreateReceivingOrderRequest,
        CreateRetailReplenishmentSuggestionRequest,
        CreateSpecialDrugCategoryRequest,
        CreateSupplierRequest,
        CreateWarehouseRequest,
        CrossdockPlan,
        CurrentUser,
        Customer,
        CustomerListResponse,
        DocumentNumberRule,
        DocumentNumberRuleListResponse,
        DriverTask,
        DriverTaskListResponse,
        DocumentNumberAllocation,
        DocumentNumberAllocationListResponse,
        DisposeTemperatureExcursionRequest,
        ErrorResponse,
        ExecuteMappingRequest,
        ExecuteMappingResponse,
        FeatureFlagArchiveRequest,
        FeatureFlagArchiveResult,
        FeatureFlagBatchImportRequest,
        FeatureFlagBatchImportResult,
        FeatureFlagConfig,
        FeatureFlagExportResponse,
        FeatureFlagMigrationResult,
        FeatureFlagReconcileReport,
        FeatureFlagSourceSwitchRequest,
        FeatureFlagSourceSwitchResponse,
        GenerateBillingStatementRequest,
        GspLedgerReport,
        GspLedgerRow,
        HealthzResponse,
        IngestTemperatureExcursionRequest,
        IngestTemperatureReadingRequest,
        IngestTransitTemperatureRequest,
        InspectReceivingOrderRequest,
        InspectionSignatureRecord,
        InventoryBatch,
        InventoryBatchListResponse,
        InventoryMovement,
        Location,
        LocationListResponse,
        LoginRequest,
        LoginResponse,
        MappingDictionary,
        MappingQueueItem,
        MappingRule,
        MappingTraceResponse,
        OutboundOrder,
        OutboundOrderLine,
        OutboundOrderListResponse,
        OutboundWave,
        PackJob,
        PackingStation,
        PageMeta,
        PrintWaybillRequest,
        Product,
        ProductListResponse,
        PutawayInventoryRequest,
        PutawayRecord,
        PutawayRequest,
        ReceiveReceivingOrderRequest,
        ReceiveTmsDispatchRequest,
        ReceivingInspectionRecord,
        ReceivingOrder,
        ReceivingOrderLine,
        ReceivingOrderListResponse,
        ReceivingOrderReceipt,
        RejectReceivingOrderRequest,
        ReportQueryRequest,
        ReportQueryResponse,
        ReportRow,
        RetailReplenishmentSuggestion,
        ReviewOutboundOrderRequest,
        ShipOutboundOrderRequest,
        SignInspectionRequest,
        SpecialDrugCategory,
        SpecialDrugCategoryListResponse,
        StoreDashboardResponse,
        Supplier,
        SupplierListResponse,
        SystemDictionaryCategory,
        SystemDictionaryImpactPreview,
        SystemDictionaryImpactReference,
        SystemDictionaryItem,
        SystemDictionaryItemListResponse,
        TmsDispatch,
        TemperatureExcursionDispositionResponse,
        TemperatureExcursionEvent,
        TemperatureExcursionEventListResponse,
        TemperatureReading,
        TraceabilityOutboundReport,
        TraceabilityOutboundReportRequest,
        TraceabilityStatusChangeEvent,
        TransitTemperatureReading,
        UpdateCustomerRequest,
        UpdateLocationRequest,
        UpdateProductRequest,
        UpdateReceivingOrderRequest,
        UpdateSpecialDrugCategoryRequest,
        UpdateSupplierRequest,
        UpdateWarehouseRequest,
        SetDocumentNumberRuleEnabledRequest,
        UpsertSystemDictionaryItemRequest,
        UpsertDocumentNumberRuleRequest,
        WeighPackJobRequest,
        Warehouse,
        WarehouseListResponse,
        DisableSystemDictionaryItemRequest,
    )),
    tags(
        (name = "system", description = "系统探针"),
        (name = "auth", description = "鉴权与会话"),
        (name = "audit", description = "审计追踪"),
        (name = "master-data", description = "M1 基础档案"),
        (name = "system-dictionary", description = "US-M1-011 系统字典中心"),
        (name = "code-generator", description = "M-CG 单据号生成"),
        (name = "inbound", description = "M2 入库业务规则"),
        (name = "inventory", description = "M3 库存批次与状态"),
        (name = "outbound", description = "M4 出库闭环"),
        (name = "reports", description = "M6 报表查询"),
        (name = "traceability", description = "M-TC 追溯码"),
        (name = "driver", description = "H-Driver 司机端"),
        (name = "store", description = "H-Store 门店端"),
        (name = "parameter-mapping", description = "M-PM 参数对照"),
        (name = "config-center", description = "M1-008 配置中心"),
        (name = "cold-chain", description = "M5 外部冷链数据接入"),
        (name = "billing", description = "M9 计费账户、规则与月结"),
        (name = "packing", description = "M-PK 包装站"),
        (name = "retail", description = "M8 连锁门店"),
        (name = "tms", description = "M10 TMS+"),
    ),
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::ApiDoc;
    use crate::openapi_contract::{
        AUTH_EXEMPT_REASON, BEARER_AUTH_SCHEME, COLD_CHAIN_API_KEY_SCHEME,
        IDEMPOTENCY_EXEMPT_REASON,
    };
    use utoipa::OpenApi;

    #[test]
    fn openapi_contains_wave1_contract_paths() {
        let json = ApiDoc::openapi()
            .to_pretty_json()
            .expect("openapi json should serialize");

        for required_path in [
            "/api/v1/healthz",
            "/api/v1/auth/login",
            "/api/v1/auth/me",
            "/api/v1/audit/events",
            "/api/v1/master-data/products",
            "/api/v1/master-data/products/{id}",
            "/api/v1/master-data/suppliers",
            "/api/v1/master-data/suppliers/{id}",
            "/api/v1/master-data/customers",
            "/api/v1/master-data/customers/{id}",
            "/api/v1/master-data/warehouses",
            "/api/v1/master-data/warehouses/{id}",
            "/api/v1/master-data/locations",
            "/api/v1/master-data/locations/batch-create",
            "/api/v1/master-data/locations/{id}",
            "/api/v1/master-data/special-drug-categories",
            "/api/v1/master-data/special-drug-categories/{id}",
            "/api/v1/system-dictionaries/{dict_code}/items",
            "/api/v1/system-dictionaries/{dict_code}/items/{item_code}",
            "/api/v1/system-dictionaries/{dict_code}/items/{item_code}/impact-preview",
            "/api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable",
            "/api/v1/code-generator/document-number-rules",
            "/api/v1/code-generator/document-number-rules/{rule_code}",
            "/api/v1/code-generator/document-number-rules/{rule_code}/enabled",
            "/api/v1/code-generator/document-number-allocations",
            "/api/v1/inbound/receiving-orders",
            "/api/v1/inbound/receiving-orders/{id}",
            "/api/v1/inbound/receiving-orders/{id}/receive",
            "/api/v1/inbound/receiving-orders/{id}/reject",
            "/api/v1/inbound/receiving-orders/{id}/inspect",
            "/api/v1/inbound/receiving-orders/{id}/sign",
            "/api/v1/inbound/receiving-orders/{id}/putaway",
            "/api/v1/inventory/batches",
            "/api/v1/inventory/batches/putaway",
            "/api/v1/inventory/batches/status",
            "/api/v1/outbound/orders",
            "/api/v1/outbound/orders/{id}",
            "/api/v1/outbound/waves",
            "/api/v1/outbound/pick-tasks/{id}/complete",
            "/api/v1/outbound/orders/{id}/review",
            "/api/v1/outbound/orders/{id}/ship",
            "/api/v1/reports/query",
            "/api/v1/reports/gsp/inbound-ledger",
            "/api/v1/reports/gsp/outbound-ledger",
            "/api/v1/reports/gsp/inventory-ledger",
            "/api/v1/traceability/outbound-reports",
            "/api/v1/driver/tasks/today",
            "/api/v1/store/dashboard",
            "/api/v1/parameter-mapping/execute",
            "/api/v1/parameter-mapping/traces/{execution_id}",
            "/api/v1/config-center/feature-flags/migrate",
            "/api/v1/config-center/feature-flags/reconcile",
            "/api/v1/config-center/feature-flags/export",
            "/api/v1/config-center/feature-flags/import",
            "/api/v1/config-center/feature-flags/source",
            "/api/v1/config-center/feature-flags/archive-file-source",
            "/api/v1/cold-chain/devices",
            "/api/v1/cold-chain/readings",
            "/api/v1/cold-chain/excursions",
            "/api/v1/cold-chain/excursions/pending-disposition",
            "/api/v1/cold-chain/excursions/{external_event_id}/dispose",
            "/api/v1/billing/accounts",
            "/api/v1/billing/contracts",
            "/api/v1/billing/rules",
            "/api/v1/packing/stations",
            "/api/v1/packing/jobs",
            "/api/v1/packing/jobs/{id}/weigh",
            "/api/v1/packing/jobs/{id}/waybill",
            "/api/v1/retail/replenishment-suggestions",
            "/api/v1/retail/crossdock-plans",
            "/api/v1/billing/charges/calculate",
            "/api/v1/billing/statements",
            "/api/v1/billing/statements/{id}/confirm",
            "/api/v1/tms/dispatches",
            "/api/v1/tms/transit-temperature-readings",
            "/api/v1/tms/container-recoveries",
        ] {
            assert!(
                json.contains(required_path),
                "missing required path: {required_path}"
            );
        }

        for required_schema in [
            "\"ErrorResponse\"",
            "\"LoginRequest\"",
            "\"LoginResponse\"",
            "\"CurrentUser\"",
            "\"AuditEvent\"",
            "\"BatchCreateLocationsRequest\"",
            "\"Product\"",
            "\"Supplier\"",
            "\"ReceivingOrder\"",
            "\"ReceiveReceivingOrderRequest\"",
            "\"InventoryBatch\"",
            "\"ColdChainDevice\"",
            "\"BillingContract\"",
            "\"ExecuteMappingRequest\"",
            "\"FeatureFlagBatchImportRequest\"",
            "\"FeatureFlagReconcileReport\"",
            "\"FeatureFlagArchiveResult\"",
            "\"CreateOutboundOrderRequest\"",
            "\"OutboundOrder\"",
            "\"CreateOutboundWaveRequest\"",
            "\"OutboundWave\"",
            "\"CompletePickTaskRequest\"",
            "\"ReviewOutboundOrderRequest\"",
            "\"ShipOutboundOrderRequest\"",
            "\"DisposeTemperatureExcursionRequest\"",
            "\"TemperatureExcursionDispositionResponse\"",
            "\"TemperatureExcursionEventListResponse\"",
            "\"SystemDictionaryItem\"",
            "\"SystemDictionaryItemListResponse\"",
            "\"SystemDictionaryImpactPreview\"",
            "\"SystemDictionaryImpactReference\"",
            "\"UpsertSystemDictionaryItemRequest\"",
            "\"DisableSystemDictionaryItemRequest\"",
            "\"DocumentNumberRule\"",
            "\"DocumentNumberRuleListResponse\"",
            "\"UpsertDocumentNumberRuleRequest\"",
            "\"SetDocumentNumberRuleEnabledRequest\"",
            "\"DocumentNumberAllocation\"",
            "\"DocumentNumberAllocationListResponse\"",
            "\"GspLedgerReport\"",
            "\"GspLedgerRow\"",
            "\"TraceabilityOutboundReport\"",
            "\"TraceabilityOutboundReportRequest\"",
            "\"TraceabilityStatusChangeEvent\"",
            "\"DriverTask\"",
            "\"DriverTaskListResponse\"",
            "\"StoreDashboardResponse\"",
            "\"PackingStation\"",
            "\"CreatePackingStationRequest\"",
            "\"PackJob\"",
            "\"CreatePackJobRequest\"",
            "\"WeighPackJobRequest\"",
            "\"PrintWaybillRequest\"",
            "\"RetailReplenishmentSuggestion\"",
            "\"CreateRetailReplenishmentSuggestionRequest\"",
            "\"CrossdockPlan\"",
            "\"CreateCrossdockPlanRequest\"",
            "\"BillingChargeCalculation\"",
            "\"CalculateBillingChargesRequest\"",
            "\"BillingStatement\"",
            "\"GenerateBillingStatementRequest\"",
            "\"ConfirmBillingStatementRequest\"",
            "\"TmsDispatch\"",
            "\"ReceiveTmsDispatchRequest\"",
            "\"TransitTemperatureReading\"",
            "\"IngestTransitTemperatureRequest\"",
            "\"ContainerRecovery\"",
            "\"ConfirmContainerRecoveryRequest\"",
        ] {
            assert!(
                json.contains(required_schema),
                "missing required schema: {required_schema}"
            );
        }
    }

    #[test]
    fn openapi_declares_h3_security_and_idempotency_contract() {
        let doc: serde_json::Value = serde_json::from_str(
            &ApiDoc::openapi()
                .to_pretty_json()
                .expect("openapi json should serialize"),
        )
        .expect("openapi json should parse as value");

        assert_eq!(
            doc.pointer("/components/securitySchemes/BearerAuth/type"),
            Some(&serde_json::json!("http")),
        );
        assert_eq!(
            doc.pointer("/components/securitySchemes/BearerAuth/scheme"),
            Some(&serde_json::json!("bearer")),
        );
        assert_eq!(
            doc.pointer("/components/securitySchemes/BearerAuth/bearerFormat"),
            Some(&serde_json::json!("JWT")),
        );
        assert_eq!(
            doc.pointer("/components/securitySchemes/ColdChainApiKeyAuth/type"),
            Some(&serde_json::json!("apiKey")),
        );
        assert_eq!(
            doc.pointer("/components/securitySchemes/ColdChainApiKeyAuth/name"),
            Some(&serde_json::json!("X-WMS-API-Key")),
        );

        let global_security = doc
            .get("security")
            .and_then(serde_json::Value::as_array)
            .expect("openapi should declare global security");
        assert!(
            global_security
                .iter()
                .any(|requirement| requirement.get(BEARER_AUTH_SCHEME).is_some()),
            "global security should require BearerAuth",
        );

        for public_operation in [
            "/paths/~1api~1v1~1healthz/get",
            "/paths/~1api~1v1~1auth~1login/post",
        ] {
            let security = doc
                .pointer(&format!("{public_operation}/security"))
                .and_then(serde_json::Value::as_array)
                .expect("public operation should override security");
            assert!(
                security.is_empty(),
                "public operation should be unauthenticated"
            );
            assert!(
                doc.pointer(&format!("{public_operation}/{AUTH_EXEMPT_REASON}"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|reason| !reason.is_empty()),
                "public operation should declare auth exemption reason",
            );
        }

        for cold_chain_operation in [
            "/paths/~1api~1v1~1cold-chain~1readings/post",
            "/paths/~1api~1v1~1cold-chain~1excursions/post",
        ] {
            let security = doc
                .pointer(&format!("{cold_chain_operation}/security"))
                .and_then(serde_json::Value::as_array)
                .expect("cold-chain external operation should declare security");
            assert!(
                security
                    .iter()
                    .any(|requirement| requirement.get(COLD_CHAIN_API_KEY_SCHEME).is_some()),
                "cold-chain external operation should require API key",
            );
        }

        let login_idempotency_pointer =
            format!("/paths/~1api~1v1~1auth~1login/post/{IDEMPOTENCY_EXEMPT_REASON}");
        assert!(
            doc.pointer(&login_idempotency_pointer)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|reason| !reason.is_empty()),
            "login should document idempotency exemption",
        );
        let master_data_idempotency_pointer =
            format!("/paths/~1api~1v1~1master-data~1products/post/{IDEMPOTENCY_EXEMPT_REASON}");
        assert!(
            doc.pointer(&master_data_idempotency_pointer)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|reason| !reason.is_empty()),
            "master-data legacy write should document idempotency exemption",
        );
        let outbound_parameters = doc
            .pointer("/paths/~1api~1v1~1outbound~1orders/post/parameters")
            .and_then(serde_json::Value::as_array)
            .expect("outbound order creation should declare parameters");
        assert!(
            outbound_parameters.iter().any(|parameter| {
                parameter.get("name") == Some(&serde_json::json!("Idempotency-Key"))
                    && parameter.get("in") == Some(&serde_json::json!("header"))
            }),
            "newer write contracts should keep Idempotency-Key header",
        );
    }
}
