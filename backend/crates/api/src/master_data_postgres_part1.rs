// @governance: skip-page-size - 拆分文件仍包含同一事务族，当前优先保持批量写、审计和幂等的原子边界。
use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    BatchCreateLocationsRequest, CreateCustomerAddressRequest, CreateCustomerRequest,
    CreateLocationRequest, CreateProductRequest, CreateSupplierRequest, CreateWarehouseRequest,
    CreateWarehouseZoneRequest, Customer, CustomerAddress, CustomerProfile, Location,
    LocationListResponse, PageMeta, Product, ProductMappingTrace, ProductMappingTraceInput,
    ProductPackagingLevel, ProductPackagingLevelInput, SpecialDrugCategory, Supplier,
    UpdateCustomerAddressRequest, UpdateCustomerRequest, UpdateLocationRequest,
    UpdateProductRequest, UpdateSupplierRequest, UpdateWarehouseRequest,
    UpdateWarehouseZoneRequest, UpsertCustomerProfileRequest, Warehouse, WarehouseZone,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    master_data::{
        normalize_product_physical_patch, normalize_product_volume, normalize_supplier_uscc,
        product_attrs_with_default_source, validate_create_product_fields,
        validate_location_capacity, validate_location_code, validate_product_mapping_traces,
        validate_product_packaging_levels, validate_product_storage_condition,
        validate_product_update_transition,
        validate_update_product_fields, validate_warehouse_type, MasterDataError,
    },
};

const SPECIAL_DRUG_CATEGORY_DICT: &str = "special_drug_category";
const LOCATION_TYPE_DICT: &str = "location_type";
const QUALITY_COLOR_DICT: &str = "quality_color";
const TEMPERATURE_ZONE_DICT: &str = "temperature_zone";
const LOCATION_BATCH_MAX_COUNT: i32 = 500;

#[derive(Clone, Debug)]
pub struct PgMasterDataReadRepository {
    pool: PgPool,
}

#[derive(FromRow)]
struct ProductRow {
    id: Uuid,
    owner_id: Uuid,
    product_code: String,
    product_name: String,
    specification: String,
    dosage_form: Option<String>,
    storage_condition: Option<String>,
    special_drug_category: Option<String>,
    approval_no: Option<String>,
    manufacturer: Option<String>,
    udi_code: Option<String>,
    electronic_regulatory_code: Option<String>,
    length_mm: Option<f64>,
    width_mm: Option<f64>,
    height_mm: Option<f64>,
    volume_cm3: Option<f64>,
    weight_g: Option<f64>,
    source: String,
    attrs: Value,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct ProductPackagingLevelRow {
    id: Uuid,
    product_id: Uuid,
    unit_code: String,
    unit_name: String,
    ratio_to_base: i64,
    is_base: bool,
    is_default: bool,
    sort_order: i32,
}

#[derive(FromRow)]
struct ProductMappingTraceRow {
    id: Uuid,
    product_id: Uuid,
    field_name: String,
    rule_id: Option<Uuid>,
    source_system: String,
    source_value: String,
    target_value: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct SupplierRow {
    id: Uuid,
    owner_id: Uuid,
    supplier_code: String,
    supplier_name: String,
    uscc: String,
    contact_name: Option<String>,
    source: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct CustomerRow {
    id: Uuid,
    owner_id: Uuid,
    customer_code: String,
    customer_name: String,
    license_no: Option<String>,
    source: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct WarehouseRow {
    id: Uuid,
    owner_id: Uuid,
    warehouse_code: String,
    warehouse_name: String,
    warehouse_type: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct WarehouseZoneRow {
    id: Uuid,
    owner_id: Uuid,
    warehouse_id: Uuid,
    zone_code: String,
    zone_name: String,
    temperature_zone: String,
    quality_color: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct LocationRow {
    id: Uuid,
    owner_id: Uuid,
    warehouse_id: Uuid,
    zone_id: Uuid,
    location_code: String,
    row_no: i32,
    column_no: i32,
    layer_no: i32,
    max_volume_cm3: i64,
    used_volume_cm3: i64,
    max_sku_count: i32,
    location_type: String,
    bound_owner_id: Option<Uuid>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct SpecialDrugCategoryRow {
    id: Uuid,
    item_code: String,
    item_name: String,
    enabled: bool,
    owner_id: Option<Uuid>,
    params: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PgMasterDataReadRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_products(&self, ctx: &AuthContext) -> Result<Vec<Product>, MasterDataError> {
        let rows = sqlx::query_as::<_, ProductRow>(
            r#"
            SELECT id, owner_id, product_code, product_name, specification, dosage_form,
                   storage_condition, special_drug_category, approval_no, manufacturer,
                   udi_code, electronic_regulatory_code, length_mm, width_mm, height_mm,
                   volume_cm3, weight_g, source, attrs, status, created_at, updated_at
              FROM products
             WHERE owner_id = $1
             ORDER BY updated_at DESC, product_code
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let mut levels = load_product_packaging_levels_by_owner(&self.pool, ctx.owner_id).await?;
        let mut traces = load_product_mapping_traces_by_owner(&self.pool, ctx.owner_id).await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let mut product = Product::from(row);
                product.packaging_levels = levels.remove(&product.id).unwrap_or_default();
                product.mapping_traces = traces.remove(&product.id).unwrap_or_default();
                product
            })
            .collect())
    }

    pub async fn get_product_by_code(
        &self,
        ctx: &AuthContext,
        product_code: &str,
    ) -> Result<Product, MasterDataError> {
        let row = sqlx::query_as::<_, ProductRow>(
            r#"
            SELECT id, owner_id, product_code, product_name, specification, dosage_form,
                   storage_condition, special_drug_category, approval_no, manufacturer,
                   udi_code, electronic_regulatory_code, length_mm, width_mm, height_mm,
                   volume_cm3, weight_g, source, attrs, status, created_at, updated_at
              FROM products
             WHERE owner_id = $1 AND product_code = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(product_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(MasterDataError::NotFound)?;
        let mut product = Product::from(row);
        product.packaging_levels =
            load_product_packaging_levels(&self.pool, ctx.owner_id, product.id).await?;
        product.mapping_traces =
            load_product_mapping_traces(&self.pool, ctx.owner_id, product.id).await?;
        Ok(product)
    }

    pub async fn create_product(
        &self,
        ctx: &AuthContext,
        req: CreateProductRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Product, MasterDataError> {
        self.create_product_with_mapping_traces(ctx, req, Vec::new(), now, idempotency_key)
            .await
    }

    pub async fn create_product_with_mapping_traces(
        &self,
        ctx: &AuthContext,
        req: CreateProductRequest,
        mapping_traces: Vec<ProductMappingTraceInput>,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Product, MasterDataError> {
        self.create_product_with_mapping_traces_status(
            ctx,
            req,
            mapping_traces,
            "active",
            now,
            idempotency_key,
        )
        .await
    }

    pub async fn create_product_with_mapping_traces_status(
        &self,
        ctx: &AuthContext,
        req: CreateProductRequest,
        mapping_traces: Vec<ProductMappingTraceInput>,
        status: &str,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Product, MasterDataError> {
        validate_create_product_fields(&req)?;
        if !matches!(status, "active" | "pending_mapping") {
            return Err(MasterDataError::InvalidProductFields);
        }
        validate_product_mapping_traces(&mapping_traces)?;
        if status == "active" {
            validate_product_packaging_levels(&req.packaging_levels)?;
        } else if !req.packaging_levels.is_empty() {
            return Err(MasterDataError::InvalidProductPackaging);
        }
        let volume_cm3 = normalize_product_volume(
            req.length_mm,
            req.width_mm,
            req.height_mm,
            req.volume_cm3,
            req.weight_g,
        )?;
        let attrs = product_attrs_with_default_source(req.attrs.clone(), "api_import");
        let storage_condition = string_attr(&attrs, "storage_condition");
        if status == "active" {
            validate_product_storage_condition(&attrs)?;
        } else if storage_condition.is_some() {
            validate_product_storage_condition(&attrs)?;
        }
        let request_hash = request_hash(&json!({
            "path": "/api/v1/master-data/products",
            "request": &req,
            "mapping_traces": &mapping_traces,
            "status": status,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<Product>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let id = Uuid::new_v4();
        let specification = req.spec.trim().to_string();
        let product_code = req.product_code.trim().to_string();
        let product_name = req.product_name.trim().to_string();
        let udi_code = req.udi_code.as_deref().map(str::trim);
        let source = string_attr(&attrs, "source").unwrap_or_else(|| "api_import".to_string());
        let special_drug_category = req
            .special_drug_category_code
            .filter(|value| !value.trim().is_empty());
        if status == "active" && special_drug_category.is_none() {
            return Err(MasterDataError::InvalidSpecialDrugCategory);
        }
        if let Some(category) = special_drug_category.as_deref() {
            ensure_enabled_dictionary_item(
                &mut tx,
                ctx.owner_id,
                SPECIAL_DRUG_CATEGORY_DICT,
                category,
                now,
            )
            .await
            .map_err(|_| MasterDataError::InvalidSpecialDrugCategory)?;
        }
        let row = sqlx::query_as::<_, ProductRow>(
            r#"
            INSERT INTO products (
                id, owner_id, product_code, product_name, specification, dosage_form,
                storage_condition, special_drug_category, approval_no, manufacturer,
                udi_code, electronic_regulatory_code, length_mm, width_mm, height_mm,
                volume_cm3, weight_g, source, attrs, status, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19, $20, $21, $21
            )
            ON CONFLICT (owner_id, product_code) DO UPDATE
            SET product_name = EXCLUDED.product_name,
                specification = EXCLUDED.specification,
                dosage_form = EXCLUDED.dosage_form,
                storage_condition = EXCLUDED.storage_condition,
                special_drug_category = EXCLUDED.special_drug_category,
                approval_no = EXCLUDED.approval_no,
                manufacturer = EXCLUDED.manufacturer,
                udi_code = EXCLUDED.udi_code,
                electronic_regulatory_code = EXCLUDED.electronic_regulatory_code,
                length_mm = EXCLUDED.length_mm,
                width_mm = EXCLUDED.width_mm,
                height_mm = EXCLUDED.height_mm,
                volume_cm3 = EXCLUDED.volume_cm3,
                weight_g = EXCLUDED.weight_g,
                source = EXCLUDED.source,
                attrs = EXCLUDED.attrs,
                status = 'active',
                updated_at = EXCLUDED.updated_at,
                version = products.version + 1
            WHERE products.status = 'pending_mapping'
              AND EXCLUDED.status = 'active'
            RETURNING id, owner_id, product_code, product_name, specification, dosage_form,
                      storage_condition, special_drug_category, approval_no, manufacturer,
                      udi_code, electronic_regulatory_code, length_mm, width_mm, height_mm,
                      volume_cm3, weight_g, source, attrs, status, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&product_code)
        .bind(&product_name)
        .bind(&specification)
        .bind(&req.dosage_form)
        .bind(storage_condition.as_deref())
        .bind(special_drug_category.as_deref())
        .bind(&req.approval_no)
        .bind(&req.manufacturer)
        .bind(udi_code)
        .bind(&req.electronic_regulatory_code)
        .bind(req.length_mm)
        .bind(req.width_mm)
        .bind(req.height_mm)
        .bind(volume_cm3)
        .bind(req.weight_g)
        .bind(&source)
        .bind(&attrs)
        .bind(status)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| map_catalog_write_error(error, &product_code))?
        .ok_or_else(|| MasterDataError::DuplicateCode(product_code.clone()))?;
        let activated = row.id != id;
        let mut product = Product::from(row);
        if activated {
            sqlx::query(
                "DELETE FROM product_packaging_levels WHERE owner_id = $1 AND product_id = $2",
            )
            .bind(ctx.owner_id)
            .bind(product.id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        product.packaging_levels = insert_product_packaging_levels(
            &mut tx,
            ctx.owner_id,
            product.id,
            &req.packaging_levels,
            now,
        )
        .await?;
        product.mapping_traces =
            insert_product_mapping_traces(&mut tx, ctx.owner_id, product.id, &mapping_traces, now)
                .await?;
        append_master_data_audit(
            &mut tx,
            ctx,
            if activated {
                "activate_pending_mapping_product"
            } else {
                "create_product"
            },
            "product",
            product.id,
            &product,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &product,
            now,
            "POST",
            "/api/v1/master-data/products",
            "product",
            &product.id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(product)
    }

    pub async fn batch_create_products(
        &self,
        ctx: &AuthContext,
        requests: Vec<CreateProductRequest>,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Vec<Product>, MasterDataError> {
        let request_hash = request_hash(&json!({
            "path": "/api/v1/master-data/products/batch-sync",
            "request": &requests,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<Vec<Product>>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }

        let mut products = Vec::with_capacity(requests.len());
        for req in requests {
            validate_create_product_fields(&req)?;
            validate_product_packaging_levels(&req.packaging_levels)?;
            let volume_cm3 = normalize_product_volume(
                req.length_mm,
                req.width_mm,
                req.height_mm,
                req.volume_cm3,
                req.weight_g,
            )?;
            let attrs = product_attrs_with_default_source(req.attrs.clone(), "api_import");
            validate_product_storage_condition(&attrs)?;
            let storage_condition = string_attr(&attrs, "storage_condition")
                .ok_or(MasterDataError::InvalidStorageCondition)?;
            let id = Uuid::new_v4();
            let specification = req.spec.trim().to_string();
            let product_code = req.product_code.trim().to_string();
            let product_name = req.product_name.trim().to_string();
            let udi_code = req.udi_code.as_deref().map(str::trim);
            let source = string_attr(&attrs, "source").unwrap_or_else(|| "api_import".to_string());
            let special_drug_category = non_empty_or(req.special_drug_category_code, "none");
            ensure_enabled_dictionary_item(
                &mut tx,
                ctx.owner_id,
                SPECIAL_DRUG_CATEGORY_DICT,
                &special_drug_category,
                now,
            )
            .await
            .map_err(|_| MasterDataError::InvalidSpecialDrugCategory)?;
            let row = sqlx::query_as::<_, ProductRow>(
                r#"
                INSERT INTO products (
                    id, owner_id, product_code, product_name, specification, dosage_form,
                    storage_condition, special_drug_category, approval_no, manufacturer,
                    udi_code, electronic_regulatory_code, length_mm, width_mm, height_mm,
                    volume_cm3, weight_g, source, attrs, status, created_at, updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                    $14, $15, $16, $17, $18, $19, 'active', $20, $20
                )
                RETURNING id, owner_id, product_code, product_name, specification, dosage_form,
                          storage_condition, special_drug_category, approval_no, manufacturer,
                          udi_code, electronic_regulatory_code, length_mm, width_mm, height_mm,
                          volume_cm3, weight_g, source, attrs, status, created_at, updated_at
                "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(&product_code)
            .bind(&product_name)
            .bind(&specification)
            .bind(&req.dosage_form)
            .bind(&storage_condition)
            .bind(&special_drug_category)
            .bind(&req.approval_no)
            .bind(&req.manufacturer)
            .bind(udi_code)
            .bind(&req.electronic_regulatory_code)
            .bind(req.length_mm)
            .bind(req.width_mm)
            .bind(req.height_mm)
            .bind(volume_cm3)
            .bind(req.weight_g)
            .bind(&source)
            .bind(&attrs)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| map_catalog_write_error(error, &product_code))?;
            let mut product = Product::from(row);
            product.packaging_levels = insert_product_packaging_levels(
                &mut tx,
                ctx.owner_id,
                product.id,
                &req.packaging_levels,
                now,
            )
            .await?;
            append_master_data_audit(
                &mut tx,
                ctx,
                "batch_create_product",
                "product",
                product.id,
                &product,
                now,
            )
            .await?;
            products.push(product);
        }
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &products,
            now,
            "POST",
            "/api/v1/master-data/products/batch-sync",
            "product_batch",
            idempotency_key,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(products)
    }

    pub async fn list_suppliers(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Supplier>, MasterDataError> {
        let rows = sqlx::query_as::<_, SupplierRow>(
            r#"
            SELECT id, owner_id, supplier_code, supplier_name, uscc, contact_name, source,
                   status, created_at, updated_at
              FROM suppliers
             WHERE owner_id = $1
             ORDER BY updated_at DESC, supplier_code
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(Supplier::from).collect())
    }

    pub async fn list_customers(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Customer>, MasterDataError> {
        let rows = sqlx::query_as::<_, CustomerRow>(
            r#"
            SELECT id, owner_id, customer_code, customer_name, license_no, source, status, created_at, updated_at
              FROM customers
             WHERE owner_id = $1
             ORDER BY updated_at DESC, customer_code
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(Customer::from).collect())
    }

    pub async fn create_supplier(
        &self,
        ctx: &AuthContext,
        req: CreateSupplierRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Supplier, MasterDataError> {
        let mut req = req;
        req.license_no = normalize_supplier_uscc(req.license_no)?;
        let request_hash = request_hash(&json!({
            "path": "/api/v1/master-data/suppliers",
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<Supplier>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let id = Uuid::new_v4();
        let source = req.source.unwrap_or_else(|| "api_import".to_string());
        let uscc = req
            .license_no
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| req.supplier_code.clone());
        let row = sqlx::query_as::<_, SupplierRow>(
            r#"
            INSERT INTO suppliers (
                id, owner_id, supplier_code, supplier_name, uscc, contact_name,
                source, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'active', $8, $8)
            RETURNING id, owner_id, supplier_code, supplier_name, uscc, contact_name,
                      source, status, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.supplier_code)
        .bind(&req.supplier_name)
        .bind(&uscc)
        .bind(&req.contact_name)
        .bind(&source)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| map_catalog_write_error(error, &req.supplier_code))?;
        let supplier = Supplier::from(row);
        append_master_data_audit(
            &mut tx,
            ctx,
            "create_supplier",
            "supplier",
            supplier.id,
            &supplier,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &supplier,
            now,
            "POST",
            "/api/v1/master-data/suppliers",
            "supplier",
            &supplier.id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(supplier)
    }

    pub async fn create_customer(
        &self,
        ctx: &AuthContext,
        req: CreateCustomerRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<Customer, MasterDataError> {
        let request_hash = request_hash(&json!({
            "path": "/api/v1/master-data/customers",
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<Customer>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let id = Uuid::new_v4();
        let source = req.source.unwrap_or_else(|| "api_import".to_string());
        let row = sqlx::query_as::<_, CustomerRow>(
            r#"
            INSERT INTO customers (
                id, owner_id, customer_code, customer_name, customer_type,
                license_no, source, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, 'customer', $5, $6, 'active', $7, $7)
            RETURNING id, owner_id, customer_code, customer_name, license_no, source, status, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.customer_code)
        .bind(&req.customer_name)
        .bind(&req.license_no)
        .bind(&source)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| map_catalog_write_error(error, &req.customer_code))?;
        let customer = Customer::from(row);
        append_master_data_audit(
            &mut tx,
            ctx,
            "create_customer",
            "customer",
            customer.id,
            &customer,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &customer,
            now,
            "POST",
            "/api/v1/master-data/customers",
            "customer",
            &customer.id.to_string(),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(customer)
    }

    pub async fn list_warehouses(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Warehouse>, MasterDataError> {
        let rows = sqlx::query_as::<_, WarehouseRow>(
            r#"
            SELECT id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
              FROM warehouses
             WHERE owner_id = $1
             ORDER BY updated_at DESC, warehouse_code
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(Warehouse::from).collect())
    }

    pub async fn list_locations(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Location>, MasterDataError> {
        let rows = sqlx::query_as::<_, LocationRow>(
            r#"
            SELECT id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no,
                   layer_no, max_volume_cm3, used_volume_cm3, max_sku_count,
                   location_type, bound_owner_id, status, created_at, updated_at
              FROM warehouse_locations
             WHERE owner_id = $1
             ORDER BY updated_at DESC, location_code
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(Location::from).collect())
    }

    pub async fn batch_create_locations(
        &self,
        ctx: &AuthContext,
        req: BatchCreateLocationsRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<LocationListResponse, MasterDataError> {
        let request_hash = request_hash(&json!({
            "path": "/api/v1/master-data/locations/batch-create",
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(value);
        }

        ensure_warehouse_zone_in_owner(&mut tx, ctx.owner_id, req.warehouse_id, req.zone_id)
            .await?;
        ensure_bound_owner_exists(&mut tx, req.bound_owner_id).await?;
        ensure_enabled_dictionary_item(
            &mut tx,
            ctx.owner_id,
            LOCATION_TYPE_DICT,
            &req.location_type,
            now,
        )
        .await?;
        let drafts = location_drafts(ctx.owner_id, &req, now)?;
        let location_codes = drafts
            .iter()
            .map(|location| location.location_code.clone())
            .collect::<Vec<_>>();
        if let Some(code) = existing_location_code(&mut tx, ctx.owner_id, &location_codes).await? {
            return Err(MasterDataError::DuplicateLocationCode(code));
        }

        let mut locations = Vec::with_capacity(drafts.len());
        for draft in drafts {
            let row = sqlx::query_as::<_, LocationRow>(
                r#"
                INSERT INTO warehouse_locations (
                    id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no,
                    layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type,
                    bound_owner_id, status, created_at, updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, 0, $10, $11, $12,
                    'available', $13, $13
                )
                RETURNING id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no,
                          layer_no, max_volume_cm3, used_volume_cm3, max_sku_count,
                          location_type, bound_owner_id, status, created_at, updated_at
                "#,
            )
            .bind(draft.id)
            .bind(draft.owner_id)
            .bind(draft.warehouse_id)
            .bind(draft.zone_id)
            .bind(&draft.location_code)
            .bind(draft.row_no)
            .bind(draft.column_no)
            .bind(draft.layer_no)
            .bind(draft.max_volume_cm3)
            .bind(draft.max_sku_count)
            .bind(&draft.location_type)
            .bind(draft.bound_owner_id)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_location_write_error)?;
            locations.push(Location::from(row));
        }

        let response = LocationListResponse {
            page: PageMeta {
                next_cursor: None,
                count: locations.len() as u32,
            },
            data: locations,
        };
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &response,
            now,
            "POST",
            "/api/v1/master-data/locations/batch-create",
            "warehouse_location",
            "batch-create",
        )
        .await?;
        let response_body = serde_json::to_value(&response)
            .map_err(|error| MasterDataError::Serialize(error.to_string()))?;
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            "batch_create_locations",
            "M1",
            "warehouse_location",
            format!("{}:{}", req.warehouse_id, req.zone_id),
            Some(AuditDiff::compute(json!({}), response_body)),
        );
        audit.occurred_at = now;
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| MasterDataError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(response)
    }

    pub async fn list_special_drug_categories(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<SpecialDrugCategory>, MasterDataError> {
        let rows = sqlx::query_as::<_, SpecialDrugCategoryRow>(
            r#"
            WITH scoped_items AS (
                SELECT
                    item.id,
                    item.item_code,
                    item.item_name,
                    item.enabled,
                    item.owner_id,
                    item.params,
                    item.created_at,
                    item.updated_at,
                    ROW_NUMBER() OVER (
                        PARTITION BY item.item_code
                        ORDER BY
                            CASE WHEN item.owner_id = $2 THEN 1 ELSE 0 END DESC,
                            item.updated_at DESC
                    ) AS scope_rank
                  FROM system_dictionary_items item
                  JOIN system_dictionary_categories category
                    ON category.dict_code = item.dict_code
                   AND category.enabled = TRUE
                 WHERE item.dict_code = $1
                   AND (item.owner_id IS NULL OR item.owner_id = $2)
                   AND (item.effective_from IS NULL OR item.effective_from <= $3)
                   AND (item.effective_to IS NULL OR item.effective_to > $3)
            )
            SELECT id, item_code, item_name, enabled, owner_id, params, created_at, updated_at
              FROM scoped_items
             WHERE scope_rank = 1
             ORDER BY item_code
            "#,
        )
        .bind(SPECIAL_DRUG_CATEGORY_DICT)
        .bind(ctx.owner_id)
        .bind(Utc::now())
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows
            .into_iter()
            .map(|row| SpecialDrugCategory {
                id: row.id,
                owner_id: row.owner_id.unwrap_or(ctx.owner_id),
                category_code: row.item_code,
                category_name: row.item_name,
                requires_dual_sign: row
                    .params
                    .get("requires_dual_sign")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                status: if row.enabled { "active" } else { "disabled" }.to_string(),
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }
}
