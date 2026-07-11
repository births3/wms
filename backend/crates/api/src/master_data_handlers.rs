//! Runtime Axum handlers for M1 master data.

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    BatchCreateLocationsRequest, CreateCustomerRequest, CreateLocationRequest,
    CreateProductRequest, CreateSpecialDrugCategoryRequest, CreateSupplierRequest,
    CreateWarehouseRequest, CreateWarehouseZoneRequest, Customer, CustomerListResponse,
    ErrorResponse, Location, LocationListResponse, PageMeta, Product, ProductListResponse,
    SpecialDrugCategory, SpecialDrugCategoryListResponse, Supplier, SupplierListResponse,
    UpdateCustomerRequest, UpdateLocationRequest, UpdateProductRequest,
    UpdateSpecialDrugCategoryRequest, UpdateSupplierRequest, UpdateWarehouseRequest,
    UpdateWarehouseZoneRequest, Warehouse, WarehouseListResponse, WarehouseZone,
    WarehouseZoneListResponse,
};

use crate::{
    auth::{AuthContext, AuthError},
    master_data::{MasterDataError, MasterDataStore},
    master_data_postgres::PgMasterDataReadRepository,
};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const MASTER_DATA_WRITE_PERMISSION: &str = "m1.master_data.write";

#[derive(Clone, Debug)]
pub struct MasterDataAppState {
    store: Arc<RwLock<MasterDataStore>>,
    read_repository: Option<PgMasterDataReadRepository>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MasterDataHandlerError {
    Auth(AuthError),
    MasterData(MasterDataError),
    MissingIdempotencyKey,
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

    async fn create_product(
        &self,
        ctx: &AuthContext,
        req: CreateProductRequest,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Product, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.create_product(ctx, req, now).await?);
        }
        Ok(self.write_store()?.create_product(ctx, req, now)?)
    }

    async fn get_product(
        &self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<Product, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.get_product(ctx, id).await?);
        }
        Ok(self.read_store()?.get_product(ctx, id)?)
    }

    async fn update_product(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateProductRequest,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Product, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.update_product(ctx, id, req, now).await?);
        }
        Ok(self.write_store()?.update_product(ctx, id, req, now)?)
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

    async fn create_supplier(
        &self,
        ctx: &AuthContext,
        req: CreateSupplierRequest,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Supplier, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.create_supplier(ctx, req, now).await?);
        }
        Ok(self.write_store()?.create_supplier(ctx, req, now)?)
    }

    async fn create_customer(
        &self,
        ctx: &AuthContext,
        req: CreateCustomerRequest,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Customer, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.create_customer(ctx, req, now).await?);
        }
        Ok(self.write_store()?.create_customer(ctx, req, now)?)
    }

    async fn update_supplier(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateSupplierRequest,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Supplier, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.update_supplier(ctx, id, req, now).await?);
        }
        Ok(self.write_store()?.update_supplier(ctx, id, req, now)?)
    }

    async fn update_customer(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateCustomerRequest,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Customer, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.update_customer(ctx, id, req, now).await?);
        }
        Ok(self.write_store()?.update_customer(ctx, id, req, now)?)
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

    async fn create_warehouse(
        &self,
        ctx: &AuthContext,
        req: CreateWarehouseRequest,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Warehouse, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.create_warehouse(ctx, req, now).await?);
        }
        Ok(self.write_store()?.create_warehouse(ctx, req, now)?)
    }

    async fn update_warehouse(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateWarehouseRequest,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Warehouse, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.update_warehouse(ctx, id, req, now).await?);
        }
        Ok(self.write_store()?.update_warehouse(ctx, id, req, now)?)
    }

    async fn create_location(
        &self,
        ctx: &AuthContext,
        req: CreateLocationRequest,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Location, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.create_location(ctx, req, now).await?);
        }
        Ok(self.write_store()?.create_location(ctx, req, now)?)
    }

    async fn list_warehouse_zones(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<WarehouseZone>, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_warehouse_zones(ctx).await?);
        }
        Err(MasterDataHandlerError::PostgresReadNotImplemented)
    }

    async fn create_warehouse_zone(
        &self,
        ctx: &AuthContext,
        req: CreateWarehouseZoneRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<WarehouseZone, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .create_warehouse_zone(ctx, req, now, idempotency_key)
                .await?);
        }
        Err(MasterDataHandlerError::PostgresWriteNotImplemented)
    }

    async fn update_warehouse_zone(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateWarehouseZoneRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<WarehouseZone, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .update_warehouse_zone(ctx, id, req, now, idempotency_key)
                .await?);
        }
        Err(MasterDataHandlerError::PostgresWriteNotImplemented)
    }

    async fn update_location(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateLocationRequest,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Location, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.update_location(ctx, id, req, now).await?);
        }
        Ok(self.write_store()?.update_location(ctx, id, req, now)?)
    }

    async fn batch_create_locations(
        &self,
        ctx: &AuthContext,
        req: BatchCreateLocationsRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<LocationListResponse, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .batch_create_locations(ctx, req, now, idempotency_key)
                .await?);
        }
        Err(MasterDataHandlerError::PostgresWriteNotImplemented)
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

impl From<AuthError> for MasterDataHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

include!("master_data_handlers_part2.rs");
