use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    BatchCreateLocationsRequest, CreateCustomerRequest, CreateProductRequest,
    CreateLocationRequest, CreateSupplierRequest, CreateWarehouseRequest,
    CreateWarehouseZoneRequest, Customer, Location,
    LocationListResponse, PageMeta, Product, SpecialDrugCategory, Supplier,
    UpdateCustomerRequest, UpdateLocationRequest, UpdateProductRequest, UpdateSupplierRequest,
    UpdateWarehouseRequest, UpdateWarehouseZoneRequest, Warehouse, WarehouseZone,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    master_data::{product_attrs_with_default_source, MasterDataError},
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
    storage_condition: String,
    special_drug_category: String,
    approval_no: Option<String>,
    manufacturer: Option<String>,
    source: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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
                   source, status, created_at, updated_at
              FROM products
             WHERE owner_id = $1
             ORDER BY updated_at DESC, product_code
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(Product::from).collect())
    }

    pub async fn create_product(
        &self,
        ctx: &AuthContext,
        req: CreateProductRequest,
        now: DateTime<Utc>,
    ) -> Result<Product, MasterDataError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let id = Uuid::new_v4();
        let attrs = product_attrs_with_default_source(req.attrs, "api_import");
        let specification = non_empty_or(req.spec, "-");
        let storage_condition =
            string_attr(&attrs, "storage_condition").unwrap_or_else(|| "normal".to_string());
        let source = string_attr(&attrs, "source").unwrap_or_else(|| "api_import".to_string());
        let special_drug_category = non_empty_or(req.special_drug_category_code, "normal");
        let row = sqlx::query_as::<_, ProductRow>(
            r#"
            INSERT INTO products (
                id, owner_id, product_code, product_name, specification, dosage_form,
                storage_condition, special_drug_category, approval_no, manufacturer,
                source, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'active', $12, $12)
            RETURNING id, owner_id, product_code, product_name, specification, dosage_form,
                      storage_condition, special_drug_category, approval_no, manufacturer,
                      source, status, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.product_code)
        .bind(&req.product_name)
        .bind(&specification)
        .bind(&req.dosage_form)
        .bind(&storage_condition)
        .bind(&special_drug_category)
        .bind(&req.approval_no)
        .bind(&req.manufacturer)
        .bind(&source)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| map_catalog_write_error(error, &req.product_code))?;
        let product = Product::from(row);
        append_master_data_audit(
            &mut tx,
            ctx,
            "create_product",
            "product",
            product.id,
            &product,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(product)
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
    ) -> Result<Supplier, MasterDataError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
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
        tx.commit().await.map_err(map_db_error)?;
        Ok(supplier)
    }

    pub async fn create_customer(
        &self,
        ctx: &AuthContext,
        req: CreateCustomerRequest,
        now: DateTime<Utc>,
    ) -> Result<Customer, MasterDataError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
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
        tx.commit().await.map_err(map_db_error)?;
        Ok(customer)
    }

    pub async fn list_warehouses(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Warehouse>, MasterDataError> {
        let rows = sqlx::query_as::<_, WarehouseRow>(
            r#"
            SELECT id, owner_id, warehouse_code, warehouse_name, status, created_at, updated_at
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
