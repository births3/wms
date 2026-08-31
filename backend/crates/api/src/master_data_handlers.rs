//! Runtime Axum handlers for M1 master data.

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    BatchCreateLocationsRequest, BatchGenerateLocationsRequest, BatchGenerateLocationsResponse,
    CreateCustomerAddressRequest, CreateCustomerRequest, CreateLocationRequest,
    CreateProductRequest, CreateSpecialDrugCategoryRequest, CreateSupplierRequest,
    CreateWarehouseRequest, CreateWarehouseZoneRequest, Customer, CustomerAddress,
    CustomerAddressListResponse, CustomerListResponse, CustomerProfile, ErrorResponse, Location,
    LocationListResponse, PageMeta, Product, ProductListResponse, SpecialDrugCategory,
    SpecialDrugCategoryListResponse, Supplier, SupplierListResponse, UpdateCustomerAddressRequest,
    UpdateCustomerRequest, UpdateLocationRequest, UpdateProductRequest,
    UpdateSpecialDrugCategoryRequest, UpdateSupplierRequest, UpdateWarehouseRequest,
    UpdateWarehouseZoneRequest, UpsertCustomerProfileRequest, Warehouse, WarehouseListResponse,
    WarehouseZone, WarehouseZoneListResponse,
};

use crate::{
    auth::{AuthContext, AuthError},
    master_data::{MasterDataError, MasterDataStore},
    master_data_postgres::PgMasterDataReadRepository,
};

mod batch_sync;

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const MASTER_DATA_READ_PERMISSION: &str = "m1.master_data.read";
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

/// 内存存储兜底路径的等价分页：按 page/page_size 切片并返回总数。
fn memory_page<T>(all: Vec<T>, page: u32, page_size: u32) -> (Vec<T>, i64) {
    let total = all.len() as i64;
    let page = page.max(1);
    let page_size = page_size.clamp(1, 200);
    let offset = ((page - 1) as usize) * (page_size as usize);
    (
        all.into_iter()
            .skip(offset)
            .take(page_size as usize)
            .collect(),
        total,
    )
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
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<Product>, i64), MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_products(ctx, page, page_size).await?);
        }
        let all = self.read_store()?.list_products(ctx);
        Ok(memory_page(all, page, page_size))
    }

    async fn create_product(
        &self,
        ctx: &AuthContext,
        req: CreateProductRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<Product, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .create_product(ctx, req, now, idempotency_key)
                .await?);
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
        idempotency_key: &str,
    ) -> Result<Product, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .update_product(ctx, id, req, now, idempotency_key)
                .await?);
        }
        Ok(self.write_store()?.update_product(ctx, id, req, now)?)
    }

    async fn list_suppliers(
        &self,
        ctx: &AuthContext,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<Supplier>, i64), MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_suppliers(ctx, page, page_size).await?);
        }
        let all = self.read_store()?.list_suppliers(ctx);
        Ok(memory_page(all, page, page_size))
    }

    async fn list_customers(
        &self,
        ctx: &AuthContext,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<Customer>, i64), MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_customers(ctx, page, page_size).await?);
        }
        let all = self.read_store()?.list_customers(ctx);
        Ok(memory_page(all, page, page_size))
    }

    async fn create_supplier(
        &self,
        ctx: &AuthContext,
        req: CreateSupplierRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<Supplier, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .create_supplier(ctx, req, now, idempotency_key)
                .await?);
        }
        Ok(self.write_store()?.create_supplier(ctx, req, now)?)
    }

    async fn create_customer(
        &self,
        ctx: &AuthContext,
        req: CreateCustomerRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<Customer, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .create_customer(ctx, req, now, idempotency_key)
                .await?);
        }
        Ok(self.write_store()?.create_customer(ctx, req, now)?)
    }

    async fn update_supplier(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateSupplierRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<Supplier, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .update_supplier(ctx, id, req, now, idempotency_key)
                .await?);
        }
        Ok(self.write_store()?.update_supplier(ctx, id, req, now)?)
    }

    async fn update_customer(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateCustomerRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<Customer, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .update_customer(ctx, id, req, now, idempotency_key)
                .await?);
        }
        Ok(self.write_store()?.update_customer(ctx, id, req, now)?)
    }

    async fn list_warehouses(
        &self,
        ctx: &AuthContext,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<Warehouse>, i64), MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_warehouses(ctx, page, page_size).await?);
        }
        let all = self.read_store()?.list_warehouses(ctx);
        Ok(memory_page(all, page, page_size))
    }

    async fn list_locations(
        &self,
        ctx: &AuthContext,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<Location>, i64), MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository.list_locations(ctx, page, page_size).await?);
        }
        let all = self.read_store()?.list_locations(ctx);
        Ok(memory_page(all, page, page_size))
    }

    async fn get_pda_location_by_code(
        &self,
        ctx: &AuthContext,
        location_code: &str,
    ) -> Result<wms_domain::PdaLocationInfo, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .get_pda_location_by_code(ctx, location_code)
                .await?);
        }
        let loc = self
            .read_store()?
            .list_locations(ctx)
            .into_iter()
            .find(|l| l.location_code.eq_ignore_ascii_case(location_code))
            .ok_or(MasterDataHandlerError::MasterData(
                MasterDataError::NotFound,
            ))?;
        let remaining_volume_cm3 = (loc.max_volume_cm3 - loc.used_volume_cm3).max(0);
        Ok(wms_domain::PdaLocationInfo {
            location_id: loc.id,
            location_code: loc.location_code,
            zone_code: "Z01".to_string(),
            temperature_zone: "normal".to_string(),
            status: loc.status,
            mix_product_policy: loc.mix_product_policy,
            mix_batch_policy: loc.mix_batch_policy,
            max_volume_cm3: loc.max_volume_cm3,
            used_volume_cm3: loc.used_volume_cm3,
            remaining_volume_cm3,
        })
    }

    async fn create_warehouse(
        &self,
        ctx: &AuthContext,
        req: CreateWarehouseRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<Warehouse, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .create_warehouse(ctx, req, now, idempotency_key)
                .await?);
        }
        Ok(self.write_store()?.create_warehouse(ctx, req, now)?)
    }

    async fn update_warehouse(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateWarehouseRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<Warehouse, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .update_warehouse(ctx, id, req, now, idempotency_key)
                .await?);
        }
        Ok(self.write_store()?.update_warehouse(ctx, id, req, now)?)
    }

    async fn create_location(
        &self,
        ctx: &AuthContext,
        req: CreateLocationRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<Location, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .create_location(ctx, req, now, idempotency_key)
                .await?);
        }
        Ok(self.write_store()?.create_location(ctx, req, now)?)
    }

    async fn list_warehouse_zones(
        &self,
        ctx: &AuthContext,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<WarehouseZone>, i64), MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .list_warehouse_zones(ctx, page, page_size)
                .await?);
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
        idempotency_key: &str,
    ) -> Result<Location, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .update_location(ctx, id, req, now, idempotency_key)
                .await?);
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

    async fn batch_generate_locations(
        &self,
        ctx: &AuthContext,
        req: BatchGenerateLocationsRequest,
        now: chrono::DateTime<chrono::Utc>,
        idempotency_key: &str,
    ) -> Result<BatchGenerateLocationsResponse, MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .batch_generate_locations(ctx, req, now, idempotency_key)
                .await?);
        }
        Err(MasterDataHandlerError::PostgresWriteNotImplemented)
    }

    async fn list_special_drug_categories(
        &self,
        ctx: &AuthContext,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<SpecialDrugCategory>, i64), MasterDataHandlerError> {
        if let Some(repository) = &self.read_repository {
            return Ok(repository
                .list_special_drug_categories(ctx, page, page_size)
                .await?);
        }
        let all = self.read_store()?.list_special_drug_categories(ctx);
        Ok(memory_page(all, page, page_size))
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

pub(super) fn require_internal_product_write(
    ctx: &AuthContext,
) -> Result<(), MasterDataHandlerError> {
    ctx.require_permission(MASTER_DATA_WRITE_PERMISSION)?;
    Err(AuthError::PermissionDenied(
        "商品主数据只能由 ERP 通过 H8 商品主数据防腐层同步".to_string(),
    )
    .into())
}

include!("master_data_handlers/customer_addresses.rs");
include!("master_data_handlers/customer_profile.rs");
include!("master_data_handlers_part2.rs");
