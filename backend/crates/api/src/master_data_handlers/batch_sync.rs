use axum::{extract::State, http::HeaderMap, Json};
use wms_domain::{
    CreateCustomerRequest, CreateProductRequest, CreateSupplierRequest, Customer,
    CustomerListResponse, Product, ProductListResponse, Supplier, SupplierListResponse,
};

use crate::auth::AuthContext;

use super::{
    idempotency_key_from_headers, page, require_internal_product_write, MasterDataAppState,
    MasterDataHandlerError, MASTER_DATA_WRITE_PERMISSION,
};

impl MasterDataAppState {
    pub(super) async fn batch_create_products(
        &self,
        ctx: &AuthContext,
        requests: Vec<CreateProductRequest>,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<Vec<Product>, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .batch_create_products(ctx, requests, now, idempotency_key)
                .await?);
        }
        let mut store = self.write_store()?;
        let snapshot = store.clone();
        let mut products = Vec::with_capacity(requests.len());
        for request in requests {
            match store.create_product(ctx, request, now) {
                Ok(product) => products.push(product),
                Err(error) => {
                    *store = snapshot;
                    return Err(error.into());
                }
            }
        }
        Ok(products)
    }

    pub(super) async fn batch_create_suppliers(
        &self,
        ctx: &AuthContext,
        requests: Vec<CreateSupplierRequest>,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<Vec<Supplier>, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .batch_create_suppliers(ctx, requests, now, idempotency_key)
                .await?);
        }
        let mut store = self.write_store()?;
        let snapshot = store.clone();
        let mut suppliers = Vec::with_capacity(requests.len());
        for request in requests {
            match store.create_supplier(ctx, request, now) {
                Ok(supplier) => suppliers.push(supplier),
                Err(error) => {
                    *store = snapshot;
                    return Err(error.into());
                }
            }
        }
        Ok(suppliers)
    }

    pub(super) async fn batch_create_customers(
        &self,
        ctx: &AuthContext,
        requests: Vec<CreateCustomerRequest>,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<Vec<Customer>, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .batch_create_customers(ctx, requests, now, idempotency_key)
                .await?);
        }
        let mut store = self.write_store()?;
        let snapshot = store.clone();
        let mut customers = Vec::with_capacity(requests.len());
        for request in requests {
            match store.create_customer(ctx, request, now) {
                Ok(customer) => customers.push(customer),
                Err(error) => {
                    *store = snapshot;
                    return Err(error.into());
                }
            }
        }
        Ok(customers)
    }
}

pub(super) async fn batch_create_products_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    headers: HeaderMap,
    Json(requests): Json<Vec<CreateProductRequest>>,
) -> Result<Json<ProductListResponse>, MasterDataHandlerError> {
    require_internal_product_write(&ctx)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let data = state
        .batch_create_products(&ctx, requests, chrono::Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(ProductListResponse {
        page: page(data.len()),
        data,
    }))
}

pub(super) async fn batch_create_suppliers_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    headers: HeaderMap,
    Json(requests): Json<Vec<CreateSupplierRequest>>,
) -> Result<Json<SupplierListResponse>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let data = state
        .batch_create_suppliers(&ctx, requests, chrono::Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(SupplierListResponse {
        page: page(data.len()),
        data,
    }))
}

pub(super) async fn batch_create_customers_handler(
    ctx: AuthContext,
    State(state): State<MasterDataAppState>,
    headers: HeaderMap,
    Json(requests): Json<Vec<CreateCustomerRequest>>,
) -> Result<Json<CustomerListResponse>, MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let data = state
        .batch_create_customers(&ctx, requests, chrono::Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(CustomerListResponse {
        page: page(data.len()),
        data,
    }))
}
