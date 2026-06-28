//! Runtime Axum handlers for M1 master data.

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, patch},
    Json, Router,
};
use uuid::Uuid;
use wms_domain::{
    CreateCustomerRequest, CreateLocationRequest, CreateProductRequest,
    CreateSpecialDrugCategoryRequest, CreateSupplierRequest, CreateWarehouseRequest, Customer,
    CustomerListResponse, ErrorResponse, Location, LocationListResponse, PageMeta, Product,
    ProductListResponse, SpecialDrugCategory, SpecialDrugCategoryListResponse, Supplier,
    SupplierListResponse, UpdateCustomerRequest, UpdateLocationRequest, UpdateProductRequest,
    UpdateSpecialDrugCategoryRequest, UpdateSupplierRequest, UpdateWarehouseRequest, Warehouse,
    WarehouseListResponse,
};

use crate::{
    auth::AuthContext,
    master_data::{MasterDataError, MasterDataStore},
};

#[derive(Clone, Debug)]
pub struct MasterDataAppState {
    store: Arc<RwLock<MasterDataStore>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MasterDataHandlerError {
    MasterData(MasterDataError),
    StoreUnavailable,
}

impl Default for MasterDataAppState {
    fn default() -> Self {
        Self {
            store: Arc::new(RwLock::new(MasterDataStore::default())),
        }
    }
}

impl MasterDataAppState {
    fn read_store(&self) -> Result<RwLockReadGuard<'_, MasterDataStore>, MasterDataHandlerError> {
        self.store
            .read()
            .map_err(|_| MasterDataHandlerError::StoreUnavailable)
    }

    fn write_store(&self) -> Result<RwLockWriteGuard<'_, MasterDataStore>, MasterDataHandlerError> {
        self.store
            .write()
            .map_err(|_| MasterDataHandlerError::StoreUnavailable)
    }
}

impl From<MasterDataError> for MasterDataHandlerError {
    fn from(value: MasterDataError) -> Self {
        Self::MasterData(value)
    }
}

impl IntoResponse for MasterDataHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
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
            "/api/v1/master-data/suppliers/:id",
            patch(update_supplier_handler).delete(delete_supplier_handler),
        )
        .route(
            "/api/v1/master-data/customers",
            get(list_customers_handler).post(create_customer_handler),
        )
        .route(
            "/api/v1/master-data/customers/:id",
            patch(update_customer_handler).delete(delete_customer_handler),
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
            "/api/v1/master-data/locations",
            get(list_locations_handler).post(create_location_handler),
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
    let data = state.read_store()?.list_products(&ctx);
    Ok(Json(ProductListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_product_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Json(req): Json<CreateProductRequest>,
) -> Result<Json<Product>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.create_product(
        &ctx,
        req,
        chrono::Utc::now(),
    )?))
}

async fn get_product_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Product>, MasterDataHandlerError> {
    Ok(Json(state.read_store()?.get_product(&ctx, id)?))
}

async fn update_product_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<Product>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.update_product(
        &ctx,
        id,
        req,
        chrono::Utc::now(),
    )?))
}

async fn delete_product_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Product>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.delete_product(&ctx, id)?))
}

async fn list_suppliers_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<SupplierListResponse>, MasterDataHandlerError> {
    let data = state.read_store()?.list_suppliers(&ctx);
    Ok(Json(SupplierListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_supplier_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Json(req): Json<CreateSupplierRequest>,
) -> Result<Json<Supplier>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.create_supplier(
        &ctx,
        req,
        chrono::Utc::now(),
    )?))
}

async fn update_supplier_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateSupplierRequest>,
) -> Result<Json<Supplier>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.update_supplier(
        &ctx,
        id,
        req,
        chrono::Utc::now(),
    )?))
}

async fn delete_supplier_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Supplier>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.delete_supplier(&ctx, id)?))
}

async fn list_customers_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<CustomerListResponse>, MasterDataHandlerError> {
    let data = state.read_store()?.list_customers(&ctx);
    Ok(Json(CustomerListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_customer_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Json(req): Json<CreateCustomerRequest>,
) -> Result<Json<Customer>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.create_customer(
        &ctx,
        req,
        chrono::Utc::now(),
    )?))
}

async fn update_customer_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCustomerRequest>,
) -> Result<Json<Customer>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.update_customer(
        &ctx,
        id,
        req,
        chrono::Utc::now(),
    )?))
}

async fn delete_customer_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Customer>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.delete_customer(&ctx, id)?))
}

async fn list_warehouses_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<WarehouseListResponse>, MasterDataHandlerError> {
    let data = state.read_store()?.list_warehouses(&ctx);
    Ok(Json(WarehouseListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_warehouse_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Json(req): Json<CreateWarehouseRequest>,
) -> Result<Json<Warehouse>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.create_warehouse(
        &ctx,
        req,
        chrono::Utc::now(),
    )?))
}

async fn update_warehouse_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWarehouseRequest>,
) -> Result<Json<Warehouse>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.update_warehouse(
        &ctx,
        id,
        req,
        chrono::Utc::now(),
    )?))
}

async fn delete_warehouse_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Warehouse>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.delete_warehouse(&ctx, id)?))
}

async fn list_locations_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<LocationListResponse>, MasterDataHandlerError> {
    let data = state.read_store()?.list_locations(&ctx);
    Ok(Json(LocationListResponse {
        page: page(data.len()),
        data,
    }))
}

async fn create_location_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Json(req): Json<CreateLocationRequest>,
) -> Result<Json<Location>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.create_location(
        &ctx,
        req,
        chrono::Utc::now(),
    )?))
}

async fn update_location_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateLocationRequest>,
) -> Result<Json<Location>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.update_location(
        &ctx,
        id,
        req,
        chrono::Utc::now(),
    )?))
}

async fn delete_location_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Location>, MasterDataHandlerError> {
    Ok(Json(state.write_store()?.delete_location(&ctx, id)?))
}

async fn list_special_drug_categories_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
) -> Result<Json<SpecialDrugCategoryListResponse>, MasterDataHandlerError> {
    let data = state.read_store()?.list_special_drug_categories(&ctx);
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
