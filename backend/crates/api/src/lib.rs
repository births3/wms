//! 主仓 OpenAPI 契约。

pub mod audit;
pub mod auth;
pub mod config_center;
pub mod feature_flags;
pub mod inbound;
pub mod master_data;
pub mod parameter_mapping;
pub mod reports;

use utoipa::OpenApi;
use wms_domain::{
    AuditActor, AuditEvent, AuditEventListResponse, ConfigEntry, CreateCustomerRequest,
    CreateLocationRequest, CreateProductRequest, CreateReceivingOrderRequest,
    CreateSpecialDrugCategoryRequest, CreateSupplierRequest, CreateWarehouseRequest, CurrentUser,
    Customer, CustomerListResponse, ErrorResponse, ExecuteMappingRequest, ExecuteMappingResponse,
    FeatureFlagArchiveRequest, FeatureFlagArchiveResult, FeatureFlagBatchImportRequest,
    FeatureFlagBatchImportResult, FeatureFlagConfig, FeatureFlagExportResponse,
    FeatureFlagMigrationResult, FeatureFlagReconcileReport, FeatureFlagSourceSwitchRequest,
    FeatureFlagSourceSwitchResponse, HealthzResponse, Location, LocationListResponse, LoginRequest,
    LoginResponse, MappingDictionary, MappingQueueItem, MappingRule, MappingTraceResponse,
    PageMeta, Product, ProductListResponse, ReceivingOrder, ReceivingOrderLine,
    ReceivingOrderListResponse, ReportQueryRequest, ReportQueryResponse, ReportRow,
    SpecialDrugCategory, SpecialDrugCategoryListResponse, Supplier, SupplierListResponse,
    UpdateCustomerRequest, UpdateLocationRequest, UpdateProductRequest,
    UpdateReceivingOrderRequest, UpdateSpecialDrugCategoryRequest, UpdateSupplierRequest,
    UpdateWarehouseRequest, Warehouse, WarehouseListResponse,
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

#[utoipa::path(post, path = "/api/v1/reports/query", tag = "reports", request_body = ReportQueryRequest, responses((status = 200, description = "报表查询结果", body = ReportQueryResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
fn query_report() {}

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

#[derive(OpenApi)]
#[openapi(
    info(
        title = "WMS API",
        version = "0.0.2-wave-2-schema",
        description = "Wave 2 业务底座与 schema 先行契约",
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
        update_location,
        delete_location,
        list_special_drug_categories,
        create_special_drug_category,
        update_special_drug_category,
        delete_special_drug_category,
        list_receiving_orders,
        create_receiving_order,
        get_receiving_order,
        update_receiving_order,
        delete_receiving_order,
        query_report,
        execute_mapping,
        trace_mapping,
        migrate_feature_flags,
        reconcile_feature_flags,
        export_feature_flags,
        import_feature_flags,
        switch_feature_flag_source,
        archive_feature_flag_file_source,
    ),
    components(schemas(
        AuditActor,
        AuditEvent,
        AuditEventListResponse,
        ConfigEntry,
        CreateCustomerRequest,
        CreateLocationRequest,
        CreateProductRequest,
        CreateReceivingOrderRequest,
        CreateSpecialDrugCategoryRequest,
        CreateSupplierRequest,
        CreateWarehouseRequest,
        CurrentUser,
        Customer,
        CustomerListResponse,
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
        HealthzResponse,
        Location,
        LocationListResponse,
        LoginRequest,
        LoginResponse,
        MappingDictionary,
        MappingQueueItem,
        MappingRule,
        MappingTraceResponse,
        PageMeta,
        Product,
        ProductListResponse,
        ReceivingOrder,
        ReceivingOrderLine,
        ReceivingOrderListResponse,
        ReportQueryRequest,
        ReportQueryResponse,
        ReportRow,
        SpecialDrugCategory,
        SpecialDrugCategoryListResponse,
        Supplier,
        SupplierListResponse,
        UpdateCustomerRequest,
        UpdateLocationRequest,
        UpdateProductRequest,
        UpdateReceivingOrderRequest,
        UpdateSpecialDrugCategoryRequest,
        UpdateSupplierRequest,
        UpdateWarehouseRequest,
        Warehouse,
        WarehouseListResponse,
    )),
    tags(
        (name = "system", description = "系统探针"),
        (name = "auth", description = "鉴权与会话"),
        (name = "audit", description = "审计追踪"),
        (name = "master-data", description = "M1 基础档案"),
        (name = "inbound", description = "M2 入库 schema"),
        (name = "reports", description = "M6 报表查询"),
        (name = "parameter-mapping", description = "M-PM 参数对照"),
        (name = "config-center", description = "M1-008 配置中心"),
    ),
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::ApiDoc;
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
            "/api/v1/master-data/locations/{id}",
            "/api/v1/master-data/special-drug-categories",
            "/api/v1/master-data/special-drug-categories/{id}",
            "/api/v1/inbound/receiving-orders",
            "/api/v1/inbound/receiving-orders/{id}",
            "/api/v1/reports/query",
            "/api/v1/parameter-mapping/execute",
            "/api/v1/parameter-mapping/traces/{execution_id}",
            "/api/v1/config-center/feature-flags/migrate",
            "/api/v1/config-center/feature-flags/reconcile",
            "/api/v1/config-center/feature-flags/export",
            "/api/v1/config-center/feature-flags/import",
            "/api/v1/config-center/feature-flags/source",
            "/api/v1/config-center/feature-flags/archive-file-source",
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
            "\"Product\"",
            "\"Supplier\"",
            "\"ReceivingOrder\"",
            "\"ExecuteMappingRequest\"",
            "\"FeatureFlagBatchImportRequest\"",
            "\"FeatureFlagReconcileReport\"",
            "\"FeatureFlagArchiveResult\"",
        ] {
            assert!(
                json.contains(required_schema),
                "missing required schema: {required_schema}"
            );
        }
    }
}
