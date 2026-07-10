#[allow(unused_imports)]
use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/healthz",
    tag = "system",
    responses(
        (status = 200, description = "服务健康", body = HealthzResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn healthz() {}

#[utoipa::path(
    get,
    path = "/openapi.json",
    tag = "system",
    responses(
        (status = 200, description = "OpenAPI JSON 文档"),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "非内网访问被拒绝", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn openapi_json() {}

#[utoipa::path(
    get,
    path = "/api-docs",
    tag = "system",
    responses(
        (status = 200, description = "API 文档浏览页"),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn api_docs() {}

#[utoipa::path(
    get,
    path = "/redoc",
    tag = "system",
    responses(
        (status = 200, description = "生产只读 API 文档 ReDoc"),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "非内网访问被拒绝", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn redoc_docs() {}

#[utoipa::path(
    get,
    path = "/api/v1/resilience/status",
    tag = "system",
    responses(
        (status = 200, description = "H3 API 韧性保护状态", body = ResilienceStatus),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_resilience_status() {}

#[utoipa::path(
    get,
    path = "/metrics",
    tag = "system",
    responses(
        (status = 200, description = "Prometheus 文本指标"),
        (status = 401, description = "未登录", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn metrics() {}

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
pub(crate) fn login() {}

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
pub(crate) fn me() {}

#[utoipa::path(get, path = "/api/v1/admin/menus/published", tag = "admin-menu", responses((status = 200, description = "已发布三层菜单树", body = AdminMenuTreeResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_published_admin_menu() {}

#[utoipa::path(get, path = "/api/v1/admin/menus/draft", tag = "admin-menu", responses((status = 200, description = "草稿三层菜单树", body = AdminMenuTreeResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "无菜单维护权限", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_draft_admin_menu() {}

#[utoipa::path(post, path = "/api/v1/admin/menus/draft/nodes", tag = "admin-menu", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = CreateAdminMenuNodeRequest, responses((status = 200, description = "新增草稿菜单节点", body = AdminMenuNode), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "无菜单维护权限", body = ErrorResponse), (status = 422, description = "菜单节点非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_admin_menu_node() {}

#[utoipa::path(patch, path = "/api/v1/admin/menus/draft/nodes/{id}", tag = "admin-menu", params(("id" = uuid::Uuid, Path, description = "菜单节点 ID"), ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = UpdateAdminMenuNodeRequest, responses((status = 200, description = "更新草稿菜单节点", body = AdminMenuNode), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "无菜单维护权限", body = ErrorResponse), (status = 404, description = "菜单节点不存在", body = ErrorResponse), (status = 422, description = "菜单节点非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn update_admin_menu_node() {}

#[utoipa::path(post, path = "/api/v1/admin/menus/draft/batch-enable", tag = "admin-menu", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = BatchEnableAdminMenuRequest, responses((status = 200, description = "批量启停草稿菜单节点", body = [AdminMenuNode]), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "无菜单维护权限", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn batch_enable_admin_menu_nodes() {}

#[utoipa::path(post, path = "/api/v1/admin/menus/publish", tag = "admin-menu", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = PublishAdminMenuRequest, responses((status = 200, description = "发布菜单版本", body = AdminMenuVersion), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "无菜单发布权限", body = ErrorResponse), (status = 422, description = "草稿菜单非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn publish_admin_menu() {}

#[utoipa::path(post, path = "/api/v1/admin/menus/rollback", tag = "admin-menu", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = RollbackAdminMenuRequest, responses((status = 200, description = "回滚并生成新菜单版本", body = AdminMenuVersion), (status = 400, description = "缺少幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "无菜单发布权限", body = ErrorResponse), (status = 404, description = "菜单版本不存在", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn rollback_admin_menu() {}

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
pub(crate) fn list_audit_events() {}

#[utoipa::path(
    get,
    path = "/api/v1/audit/archive/partitions",
    tag = "audit",
    responses(
        (status = 200, description = "审计归档分区状态", body = AuditArchivePartitionStateListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_audit_archive_partitions() {}

#[utoipa::path(
    post,
    path = "/api/v1/audit/archive/runs",
    tag = "audit",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = AuditArchiveRunRequest,
    responses(
        (status = 200, description = "执行审计归档周期", body = AuditArchiveRunResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 422, description = "归档参数非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn run_audit_archive() {}

#[utoipa::path(
    get,
    path = "/api/v1/event-bus/deliveries/pending",
    tag = "event-bus",
    params(("limit" = Option<i64>, Query, description = "最多返回条数")),
    responses(
        (status = 200, description = "待投递事件列表", body = EventDeliveryListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_pending_event_deliveries() {}

#[utoipa::path(
    post,
    path = "/api/v1/event-bus/deliveries/{delivery_id}/ack",
    tag = "event-bus",
    params(("delivery_id" = uuid::Uuid, Path, description = "事件投递 ID")),
    responses(
        (status = 200, description = "确认事件投递成功", body = EventDelivery),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "事件投递不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn ack_event_delivery() {}

#[utoipa::path(
    post,
    path = "/api/v1/event-bus/deliveries/{delivery_id}/nack",
    tag = "event-bus",
    params(("delivery_id" = uuid::Uuid, Path, description = "事件投递 ID")),
    request_body = EventDeliveryNackRequest,
    responses(
        (status = 200, description = "记录事件投递失败", body = EventDelivery),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "事件投递不存在", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn nack_event_delivery() {}

#[utoipa::path(
    get,
    path = "/api/v1/business-retention/policies",
    tag = "business-retention",
    responses(
        (status = 200, description = "业务数据留存策略列表", body = BusinessRetentionPolicyListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_business_retention_policies() {}

#[utoipa::path(
    post,
    path = "/api/v1/business-retention/jobs",
    tag = "business-retention",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = PlanBusinessArchiveJobRequest,
    responses(
        (status = 200, description = "生成业务数据归档计划", body = BusinessArchiveJob),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "留存策略不存在", body = ErrorResponse),
        (status = 422, description = "归档计划参数非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn plan_business_retention_archive_job() {}

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
pub(crate) fn list_products() {}

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
pub(crate) fn create_product() {}

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
pub(crate) fn get_product() {}

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
pub(crate) fn update_product() {}

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
pub(crate) fn delete_product() {}

#[utoipa::path(get, path = "/api/v1/master-data/suppliers", tag = "master-data", responses((status = 200, description = "供应商列表", body = SupplierListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_suppliers() {}

#[utoipa::path(post, path = "/api/v1/master-data/suppliers", tag = "master-data", request_body = CreateSupplierRequest, responses((status = 200, description = "创建供应商", body = Supplier), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_supplier() {}

#[utoipa::path(patch, path = "/api/v1/master-data/suppliers/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "供应商 ID")), request_body = UpdateSupplierRequest, responses((status = 200, description = "更新供应商", body = Supplier), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn update_supplier() {}

#[utoipa::path(delete, path = "/api/v1/master-data/suppliers/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "供应商 ID")), responses((status = 200, description = "删除供应商", body = Supplier), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn delete_supplier() {}

#[utoipa::path(get, path = "/api/v1/master-data/customers", tag = "master-data", responses((status = 200, description = "客户列表", body = CustomerListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_customers() {}

#[utoipa::path(post, path = "/api/v1/master-data/customers", tag = "master-data", request_body = CreateCustomerRequest, responses((status = 200, description = "创建客户", body = Customer), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_customer() {}

#[utoipa::path(patch, path = "/api/v1/master-data/customers/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "客户 ID")), request_body = UpdateCustomerRequest, responses((status = 200, description = "更新客户", body = Customer), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn update_customer() {}

#[utoipa::path(delete, path = "/api/v1/master-data/customers/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "客户 ID")), responses((status = 200, description = "删除客户", body = Customer), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn delete_customer() {}

#[utoipa::path(get, path = "/api/v1/master-data/warehouses", tag = "master-data", responses((status = 200, description = "仓库列表", body = WarehouseListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_warehouses() {}

#[utoipa::path(post, path = "/api/v1/master-data/warehouses", tag = "master-data", request_body = CreateWarehouseRequest, responses((status = 200, description = "创建仓库", body = Warehouse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_warehouse() {}

#[utoipa::path(patch, path = "/api/v1/master-data/warehouses/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "仓库 ID")), request_body = UpdateWarehouseRequest, responses((status = 200, description = "更新仓库", body = Warehouse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn update_warehouse() {}

#[utoipa::path(delete, path = "/api/v1/master-data/warehouses/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "仓库 ID")), responses((status = 200, description = "删除仓库", body = Warehouse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn delete_warehouse() {}

#[utoipa::path(get, path = "/api/v1/master-data/locations", tag = "master-data", responses((status = 200, description = "库位列表", body = LocationListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_locations() {}

#[utoipa::path(post, path = "/api/v1/master-data/locations", tag = "master-data", request_body = CreateLocationRequest, responses((status = 200, description = "创建库位", body = Location), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_location() {}

#[utoipa::path(post, path = "/api/v1/master-data/locations/batch-create", tag = "master-data", params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")), request_body = BatchCreateLocationsRequest, responses((status = 200, description = "批量创建库位", body = LocationListResponse), (status = 400, description = "缺少或非法幂等键", body = ErrorResponse), (status = 401, description = "未登录", body = ErrorResponse), (status = 403, description = "权限不足", body = ErrorResponse), (status = 409, description = "库位编码或幂等冲突", body = ErrorResponse), (status = 422, description = "库位批量创建范围非法", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn batch_create_locations() {}

#[utoipa::path(patch, path = "/api/v1/master-data/locations/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "库位 ID")), request_body = UpdateLocationRequest, responses((status = 200, description = "更新库位", body = Location), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn update_location() {}

#[utoipa::path(delete, path = "/api/v1/master-data/locations/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "库位 ID")), responses((status = 200, description = "删除库位", body = Location), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn delete_location() {}

#[utoipa::path(get, path = "/api/v1/master-data/special-drug-categories", tag = "master-data", responses((status = 200, description = "特殊药品分类列表", body = SpecialDrugCategoryListResponse), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn list_special_drug_categories() {}

#[utoipa::path(post, path = "/api/v1/master-data/special-drug-categories", tag = "master-data", request_body = CreateSpecialDrugCategoryRequest, responses((status = 200, description = "创建特殊药品分类", body = SpecialDrugCategory), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn create_special_drug_category() {}

#[utoipa::path(patch, path = "/api/v1/master-data/special-drug-categories/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "特殊药品分类 ID")), request_body = UpdateSpecialDrugCategoryRequest, responses((status = 200, description = "更新特殊药品分类", body = SpecialDrugCategory), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn update_special_drug_category() {}

#[utoipa::path(delete, path = "/api/v1/master-data/special-drug-categories/{id}", tag = "master-data", params(("id" = uuid::Uuid, Path, description = "特殊药品分类 ID")), responses((status = 200, description = "删除特殊药品分类", body = SpecialDrugCategory), (status = 401, description = "未登录", body = ErrorResponse)))]
#[allow(dead_code)]
pub(crate) fn delete_special_drug_category() {}

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
pub(crate) fn list_system_dictionary_items() {}

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
pub(crate) fn preview_system_dictionary_item_impact() {}

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
pub(crate) fn upsert_system_dictionary_item() {}

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
pub(crate) fn disable_system_dictionary_item() {}

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
pub(crate) fn list_document_number_rules() {}

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
pub(crate) fn upsert_document_number_rule() {}

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
pub(crate) fn set_document_number_rule_enabled() {}

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
pub(crate) fn list_document_number_allocations() {}
