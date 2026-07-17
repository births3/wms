impl IntoResponse for MasterDataHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            MasterDataHandlerError::Auth(error) => return error.into_response(),
            MasterDataHandlerError::MasterData(MasterDataError::NotFound) => (
                StatusCode::NOT_FOUND,
                "M1_MASTER_DATA_NOT_FOUND",
                "基础档案不存在",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::DuplicateCode(_)) => (
                StatusCode::CONFLICT,
                "M1_MASTER_DATA_DUPLICATE_CODE",
                "基础档案编码已存在",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::DuplicateLocationCode(_)) => (
                StatusCode::CONFLICT,
                "M1_LOCATION_DUPLICATE",
                "库位编码已存在",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::LocationHasStock) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LOCATION_HAS_STOCK",
                "库位仍有库存，不能停用",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::InvalidLocationCapacity) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LOCATION_CAPACITY_INVALID",
                "库位已用容积不能超过最大容积",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::InvalidLocationOwner) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LOCATION_OWNER_INVALID",
                "库位绑定货主不存在",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::InvalidWarehouseType) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_WAREHOUSE_TYPE_INVALID",
                "仓库类型必须是物理仓、逻辑仓或虚拟仓",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::InvalidLocationBatchRange) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LOCATION_BATCH_INVALID",
                "库位批量创建范围非法",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::InvalidStorageCondition) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_INVALID_STORAGE_CONDITION",
                "储存条件必须使用受控标准枚举",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::InvalidSpecialDrugCategory) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_INVALID_SPECIAL_DRUG_CATEGORY",
                "特殊药品分类必须使用已启用字典项",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::InvalidSupplierUscc) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_USCC_FORMAT_INVALID",
                "统一社会信用代码格式或校验码错误",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::InvalidCustomerAddress) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_CUSTOMER_ADDRESS_INVALID",
                "客户地址字段不能为空",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::InvalidCustomerProfile) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_CUSTOMER_PROFILE_INVALID",
                "客户联系方式、类型、门店经营范围或资质字段非法",
            ),
            MasterDataHandlerError::MasterData(MasterDataError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "M1_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            MasterDataHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "M1_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            MasterDataHandlerError::MasterData(
                MasterDataError::Audit(_)
                | MasterDataError::Database(_)
                | MasterDataError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M1_MASTER_DATA_DATABASE_ERROR",
                "基础档案数据库处理失败",
            ),
            MasterDataHandlerError::PostgresReadNotImplemented => (
                StatusCode::NOT_IMPLEMENTED,
                "M1_MASTER_DATA_READ_NOT_IMPLEMENTED",
                "该基础档案读取接口尚未接入 PostgreSQL",
            ),
            MasterDataHandlerError::PostgresWriteNotImplemented => (
                StatusCode::NOT_IMPLEMENTED,
                "M1_MASTER_DATA_WRITE_NOT_IMPLEMENTED",
                "基础档案写操作尚未接入 PostgreSQL 审计与幂等闭环",
            ),
            MasterDataHandlerError::StoreUnavailable => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M1_MASTER_DATA_STORE_UNAVAILABLE",
                "基础档案存储暂不可用",
            ),
        };

        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

pub fn master_data_router(state: MasterDataAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/master-data/products",
            get(list_products_handler).post(create_product_handler),
        )
        .route(
            "/api/v1/master-data/products/batch-sync",
            post(batch_sync::batch_create_products_handler),
        )
        .route(
            "/api/v1/master-data/products/:id",
            get(get_product_handler)
                .patch(update_product_handler)
                .delete(delete_product_handler),
        )
        .route(
            "/api/v1/master-data/suppliers",
            get(list_suppliers_handler).post(create_supplier_handler),
        )
        .route(
            "/api/v1/master-data/suppliers/batch-sync",
            post(batch_sync::batch_create_suppliers_handler),
        )
        .route(
            "/api/v1/master-data/suppliers/:id",
            patch(update_supplier_handler).delete(delete_supplier_handler),
        )
        .route(
            "/api/v1/master-data/customers",
            get(list_customers_handler).post(create_customer_handler),
        )
        .route(
            "/api/v1/master-data/customers/batch-sync",
            post(batch_sync::batch_create_customers_handler),
        )
        .route(
            "/api/v1/master-data/customers/:id",
            patch(update_customer_handler).delete(delete_customer_handler),
        )
        .route(
            "/api/v1/master-data/customers/:customer_id/addresses",
            get(list_customer_addresses_handler).post(create_customer_address_handler),
        )
        .route(
            "/api/v1/master-data/customers/:customer_id/addresses/:address_id",
            patch(update_customer_address_handler),
        )
        .route(
            "/api/v1/master-data/customers/:customer_id/profile",
            get(get_customer_profile_handler).patch(upsert_customer_profile_handler),
        )
        .route(
            "/api/v1/master-data/warehouses",
            get(list_warehouses_handler).post(create_warehouse_handler),
        )
        .route(
            "/api/v1/master-data/warehouses/:id",
            patch(update_warehouse_handler).delete(delete_warehouse_handler),
        )
        .route(
            "/api/v1/master-data/warehouse-zones",
            get(list_warehouse_zones_handler).post(create_warehouse_zone_handler),
        )
        .route(
            "/api/v1/master-data/warehouse-zones/:id",
            patch(update_warehouse_zone_handler).delete(delete_warehouse_zone_handler),
        )
        .route(
            "/api/v1/master-data/locations",
            get(list_locations_handler).post(create_location_handler),
        )
        .route(
            "/api/v1/master-data/locations/batch-create",
            post(batch_create_locations_handler),
        )
        .route(
            "/api/v1/master-data/locations/:id",
            patch(update_location_handler).delete(delete_location_handler),
        )
        .route(
            "/api/v1/master-data/special-drug-categories",
            get(list_special_drug_categories_handler).post(create_special_drug_category_handler),
        )
        .route(
            "/api/v1/master-data/special-drug-categories/:id",
            patch(update_special_drug_category_handler)
                .delete(delete_special_drug_category_handler),
        )
        .with_state(state)
}

async fn list_products_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<ProductListResponse>, MasterDataHandlerError> {
    let data = state.list_products(&ctx).await?;
    Ok(Json(ProductListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_product_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateProductRequest>,
) -> Result<Json<Product>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .create_product(&ctx, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn get_product_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Product>, MasterDataHandlerError> {
    Ok(Json(state.get_product(&ctx, id).await?))
}

async fn update_product_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<Product>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_product(&ctx, id, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn delete_product_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Product>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_product(
                &ctx,
                id,
                UpdateProductRequest {
                    product_name: None,
                    approval_no: None,
                    spec: None,
                    dosage_form: None,
                    manufacturer: None,
                    special_drug_category_code: None,
                    status: Some("disabled".into()),
                    attrs: None,
                },
                chrono::Utc::now(),
                &idempotency_key,
            )
            .await?,
    ))
}

async fn list_suppliers_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<SupplierListResponse>, MasterDataHandlerError> {
    let data = state.list_suppliers(&ctx).await?;
    Ok(Json(SupplierListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_supplier_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateSupplierRequest>,
) -> Result<Json<Supplier>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .create_supplier(&ctx, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn update_supplier_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpdateSupplierRequest>,
) -> Result<Json<Supplier>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_supplier(&ctx, id, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn delete_supplier_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Supplier>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_supplier(
                &ctx,
                id,
                UpdateSupplierRequest {
                    supplier_name: None,
                    license_no: None,
                    contact_name: None,
                    status: Some("disabled".into()),
                },
                chrono::Utc::now(),
                &idempotency_key,
            )
            .await?,
    ))
}

async fn list_customers_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<CustomerListResponse>, MasterDataHandlerError> {
    let data = state.list_customers(&ctx).await?;
    Ok(Json(CustomerListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_customer_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateCustomerRequest>,
) -> Result<Json<Customer>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .create_customer(&ctx, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn update_customer_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpdateCustomerRequest>,
) -> Result<Json<Customer>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_customer(&ctx, id, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn delete_customer_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Customer>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_customer(
                &ctx,
                id,
                UpdateCustomerRequest {
                    customer_name: None,
                    license_no: None,
                    status: Some("disabled".into()),
                },
                chrono::Utc::now(),
                &idempotency_key,
            )
            .await?,
    ))
}

async fn list_warehouses_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<WarehouseListResponse>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_READ_PERMISSION)?;
    let data = state.list_warehouses(&ctx).await?;
    Ok(Json(WarehouseListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_warehouse_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateWarehouseRequest>,
) -> Result<Json<Warehouse>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .create_warehouse(&ctx, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn update_warehouse_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpdateWarehouseRequest>,
) -> Result<Json<Warehouse>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_warehouse(&ctx, id, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn delete_warehouse_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Warehouse>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_warehouse(
                &ctx,
                id,
                UpdateWarehouseRequest {
                    warehouse_name: None,
                    warehouse_type: None,
                    status: Some("disabled".into()),
                },
                chrono::Utc::now(),
                &idempotency_key,
            )
            .await?,
    ))
}

async fn list_warehouse_zones_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<WarehouseZoneListResponse>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_READ_PERMISSION)?;
    let data = state.list_warehouse_zones(&ctx).await?;
    Ok(Json(WarehouseZoneListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_warehouse_zone_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateWarehouseZoneRequest>,
) -> Result<Json<WarehouseZone>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .create_warehouse_zone(&ctx, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn update_warehouse_zone_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpdateWarehouseZoneRequest>,
) -> Result<Json<WarehouseZone>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_warehouse_zone(&ctx, id, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn delete_warehouse_zone_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<WarehouseZone>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_warehouse_zone(
                &ctx,
                id,
                UpdateWarehouseZoneRequest {
                    zone_name: None,
                    temperature_zone: None,
                    quality_color: None,
                    status: Some("disabled".into()),
                },
                chrono::Utc::now(),
                &idempotency_key,
            )
            .await?,
    ))
}

async fn list_locations_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<LocationListResponse>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_READ_PERMISSION)?;
    let data = state.list_locations(&ctx).await?;
    Ok(Json(LocationListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_location_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateLocationRequest>,
) -> Result<Json<Location>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .create_location(&ctx, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn batch_create_locations_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    headers: HeaderMap,
    Json(req): Json<BatchCreateLocationsRequest>,
) -> Result<Json<LocationListResponse>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .batch_create_locations(&ctx, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn update_location_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpdateLocationRequest>,
) -> Result<Json<Location>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_location(&ctx, id, req, chrono::Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn delete_location_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Location>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .update_location(
                &ctx,
                id,
                UpdateLocationRequest {
                    zone_id: None,
                    location_code: None,
                    row_no: None,
                    column_no: None,
                    layer_no: None,
                    max_volume_cm3: None,
                    used_volume_cm3: None,
                    max_sku_count: None,
                    location_type: None,
                    bound_owner_id: None,
                    status: Some("disabled".into()),
                },
                chrono::Utc::now(),
                &idempotency_key,
            )
            .await?,
    ))
}

async fn list_special_drug_categories_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<SpecialDrugCategoryListResponse>, MasterDataHandlerError> {
    let data = state.list_special_drug_categories(&ctx).await?;
    Ok(Json(SpecialDrugCategoryListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_special_drug_category_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Json(req): Json<CreateSpecialDrugCategoryRequest>,
) -> Result<Json<SpecialDrugCategory>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.create_special_drug_category(
        &ctx,
        req,
        chrono::Utc::now(),
    )?))
}

async fn update_special_drug_category_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateSpecialDrugCategoryRequest>,
) -> Result<Json<SpecialDrugCategory>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.update_special_drug_category(
        &ctx,
        id,
        req,
        chrono::Utc::now(),
    )?))
}

async fn delete_special_drug_category_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SpecialDrugCategory>, MasterDataHandlerError> {
    Ok(Json(
        state
            .write_store()?
            .delete_special_drug_category(&ctx, id)?,
    ))
}

fn page(count: usize) -> PageMeta {
    PageMeta {
        next_cursor: None,
        count: count as u32,
    }
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, MasterDataHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(MasterDataHandlerError::MissingIdempotencyKey)
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::{MasterDataAppState, MasterDataHandlerError};

    #[tokio::test]
    async fn postgres_state_rejects_memory_backed_writes() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during write guard test");
        let state = MasterDataAppState::with_postgres(pool);

        assert_eq!(
            state
                .read_store()
                .expect_err("PostgreSQL state must not read from memory store"),
            MasterDataHandlerError::PostgresReadNotImplemented
        );
        assert_eq!(
            state
                .write_store()
                .expect_err("PostgreSQL state must not write to memory store"),
            MasterDataHandlerError::PostgresWriteNotImplemented
        );
    }
}
