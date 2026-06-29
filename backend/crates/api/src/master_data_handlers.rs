//! Runtime Axum handlers for M1 master data.

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, patch},
    Json, Router,
};
use sqlx::PgPool;
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
    master_data_postgres::PgMasterDataReadRepository,
};

#[derive(Clone, Debug)]
pub struct MasterDataAppState {
    store: Arc<RwLock<MasterDataStore>>,
    read_repository: Option<PgMasterDataReadRepository>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MasterDataHandlerError {
    MasterData(MasterDataError),
    PostgresReadNotImplemented,
    PostgresWriteNotImplemented,
    StoreUnavailable,
}

impl Default for MasterDataAppState {
    fn default() -> Self {
        Self {
            store: Arc::new(RwLock::new(MasterDataStore::default())),
            read_repository: None,
        }
    }
}

impl MasterDataAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            store: Arc::new(RwLock::new(MasterDataStore::default())),
            read_repository: Some(PgMasterDataReadRepository::new(pool)),
        }
    }

    fn read_store(&self) -> Result<RwLockReadGuard<'_, MasterDataStore>, MasterDataHandlerError> {
        if self.read_repository.is_some() {
            // ponytail: avoid fake 404/empty reads from the memory store in PG runtime.
            return Err(MasterDataHandlerError::PostgresReadNotImplemented);
        }
        self.store
            .read()
            .map_err(|_| MasterDataHandlerError::StoreUnavailable)
    }

    fn write_store(&self) -> Result<RwLockWriteGuard<'_, MasterDataStore>, MasterDataHandlerError> {
        if self.read_repository.is_some() {
            // ponytail: PG reads are real; fail writes until PG audit/idempotency writes are implemented.
            return Err(MasterDataHandlerError::PostgresWriteNotImplemented);
        }
        self.store
            .write()
            .map_err(|_| MasterDataHandlerError::StoreUnavailable)
    }

    async fn list_products(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Product>, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_products(ctx).await?);
        }
        Ok(self.read_store()?.list_products(ctx))
    }

    async fn list_suppliers(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Supplier>, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_suppliers(ctx).await?);
        }
        Ok(self.read_store()?.list_suppliers(ctx))
    }

    async fn list_customers(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Customer>, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_customers(ctx).await?);
        }
        Ok(self.read_store()?.list_customers(ctx))
    }

    async fn list_warehouses(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Warehouse>, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_warehouses(ctx).await?);
        }
        Ok(self.read_store()?.list_warehouses(ctx))
    }

    async fn list_locations(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Location>, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_locations(ctx).await?);
        }
        Ok(self.read_store()?.list_locations(ctx))
    }

    async fn list_special_drug_categories(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<SpecialDrugCategory>, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_special_drug_categories(ctx).await?);
        }
        Ok(self.read_store()?.list_special_drug_categories(ctx))
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
            MasterDataHandlerError::MasterData(MasterDataError::Database(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M1_MASTER_DATA_DATABASE_ERROR",
                "基础档案数据库读取失败",
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
    let data = state.list_products(&ctx).await?;
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
    let data = state.list_suppliers(&ctx).await?;
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
    let data = state.list_customers(&ctx).await?;
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
    let data = state.list_warehouses(&ctx).await?;
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
    let data = state.list_locations(&ctx).await?;
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
