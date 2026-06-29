//! PostgreSQL read repository for M1 master data.

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use wms_domain::{Customer, Location, Product, SpecialDrugCategory, Supplier, Warehouse};

use crate::{auth::AuthContext, master_data::MasterDataError};

const SPECIAL_DRUG_CATEGORY_DICT: &str = "special_drug_category";

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
                   status, created_at, updated_at
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

    pub async fn list_suppliers(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<Supplier>, MasterDataError> {
        let rows = sqlx::query_as::<_, SupplierRow>(
            r#"
            SELECT id, owner_id, supplier_code, supplier_name, uscc, contact_name,
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
            SELECT id, owner_id, customer_code, customer_name, status, created_at, updated_at
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

impl From<ProductRow> for Product {
    fn from(row: ProductRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            product_code: row.product_code,
            product_name: row.product_name,
            approval_no: row.approval_no,
            spec: Some(row.specification),
            dosage_form: row.dosage_form,
            manufacturer: row.manufacturer,
            special_drug_category_code: Some(row.special_drug_category),
            status: row.status,
            attrs: json!({ "storage_condition": row.storage_condition }),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<SupplierRow> for Supplier {
    fn from(row: SupplierRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            supplier_code: row.supplier_code,
            supplier_name: row.supplier_name,
            license_no: Some(row.uscc),
            contact_name: row.contact_name,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<CustomerRow> for Customer {
    fn from(row: CustomerRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            customer_code: row.customer_code,
            customer_name: row.customer_name,
            license_no: None,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<WarehouseRow> for Warehouse {
    fn from(row: WarehouseRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            warehouse_code: row.warehouse_code,
            warehouse_name: row.warehouse_name,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<LocationRow> for Location {
    fn from(row: LocationRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            warehouse_id: row.warehouse_id,
            zone_id: row.zone_id,
            location_code: row.location_code,
            row_no: row.row_no,
            column_no: row.column_no,
            layer_no: row.layer_no,
            max_volume_cm3: row.max_volume_cm3,
            used_volume_cm3: row.used_volume_cm3,
            max_sku_count: row.max_sku_count,
            location_type: row.location_type,
            bound_owner_id: row.bound_owner_id,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn map_db_error(error: sqlx::Error) -> MasterDataError {
    MasterDataError::Database(error.to_string())
}
