//! PostgreSQL repository for M1 master data.

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    BatchCreateLocationsRequest, Customer, Location, LocationListResponse, PageMeta, Product,
    SpecialDrugCategory, Supplier, Warehouse,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    master_data::MasterDataError,
};

const SPECIAL_DRUG_CATEGORY_DICT: &str = "special_drug_category";
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

    pub async fn batch_create_locations(
        &self,
        ctx: &AuthContext,
        req: BatchCreateLocationsRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<LocationListResponse, MasterDataError> {
        let drafts = location_drafts(ctx.owner_id, &req, now)?;
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

fn location_drafts(
    owner_id: Uuid,
    req: &BatchCreateLocationsRequest,
    now: DateTime<Utc>,
) -> Result<Vec<Location>, MasterDataError> {
    let area_code = req.area_code.trim().to_uppercase();
    let valid_range = req.row_start >= 1
        && req.row_start <= req.row_end
        && req.row_end <= 99
        && req.column_start >= 1
        && req.column_start <= req.column_end
        && req.column_end <= 99
        && req.layer_start >= 1
        && req.layer_start <= req.layer_end
        && req.layer_end <= 99
        && req.max_volume_cm3 >= 0
        && req.max_sku_count > 0
        && matches!(
            req.location_type.as_str(),
            "storage" | "case_pick" | "piece_pick"
        )
        && area_code.len() == 3
        && area_code.chars().all(|item| item.is_ascii_alphanumeric());
    if !valid_range {
        return Err(MasterDataError::InvalidLocationBatchRange);
    }
    let total_count = (req.row_end - req.row_start + 1)
        * (req.column_end - req.column_start + 1)
        * (req.layer_end - req.layer_start + 1);
    if total_count > LOCATION_BATCH_MAX_COUNT {
        return Err(MasterDataError::InvalidLocationBatchRange);
    }

    let mut locations = Vec::with_capacity(total_count as usize);
    for row_no in req.row_start..=req.row_end {
        for column_no in req.column_start..=req.column_end {
            for layer_no in req.layer_start..=req.layer_end {
                locations.push(Location {
                    id: Uuid::new_v4(),
                    owner_id,
                    warehouse_id: req.warehouse_id,
                    zone_id: req.zone_id,
                    location_code: format!("{area_code}-{row_no:02}-{column_no:02}-{layer_no:02}"),
                    row_no,
                    column_no,
                    layer_no,
                    max_volume_cm3: req.max_volume_cm3,
                    used_volume_cm3: 0,
                    max_sku_count: req.max_sku_count,
                    location_type: req.location_type.clone(),
                    bound_owner_id: req.bound_owner_id,
                    status: "available".to_string(),
                    created_at: now,
                    updated_at: now,
                });
            }
        }
    }
    Ok(locations)
}

async fn ensure_warehouse_zone_in_owner(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    warehouse_id: Uuid,
    zone_id: Uuid,
) -> Result<(), MasterDataError> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM warehouses warehouse
              JOIN warehouse_zones zone
                ON zone.owner_id = warehouse.owner_id
               AND zone.warehouse_id = warehouse.id
             WHERE warehouse.owner_id = $1
               AND warehouse.id = $2
               AND zone.id = $3
        )
        "#,
    )
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(MasterDataError::NotFound)
    }
}

async fn existing_location_code(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    location_codes: &[String],
) -> Result<Option<String>, MasterDataError> {
    sqlx::query_scalar(
        r#"
        SELECT location_code
          FROM warehouse_locations
         WHERE owner_id = $1
           AND location_code = ANY($2)
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .bind(location_codes)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, MasterDataError> {
    let row: Option<(String, Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT request_hash, response_body, expires_at
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((stored_hash, response_body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        return Ok(None);
    }
    if stored_hash != request_hash {
        return Err(MasterDataError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| MasterDataError::Serialize(error.to_string()))
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), MasterDataError> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(idempotency_lock_id(owner_id, idempotency_key))
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), MasterDataError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| MasterDataError::Serialize(error.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES (
            $1, $2, $3, $4, 'POST', '/api/v1/master-data/locations/batch-create',
            200, $5, 'warehouse_location', 'batch-create', $6, $7
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(response_body)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

fn request_hash(value: &Value) -> Result<String, MasterDataError> {
    let text = serde_json::to_string(value)
        .map_err(|error| MasterDataError::Serialize(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn idempotency_lock_id(owner_id: Uuid, idempotency_key: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(owner_id.as_bytes());
    hasher.update([0]);
    hasher.update(idempotency_key.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
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

fn map_location_write_error(error: sqlx::Error) -> MasterDataError {
    if let sqlx::Error::Database(db_error) = &error {
        if db_error.code().as_deref() == Some("23505") {
            return MasterDataError::DuplicateLocationCode("warehouse_location".to_string());
        }
    }
    map_db_error(error)
}
