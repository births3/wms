//! Wave 2 M1.a master-data CRUD service.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use wms_domain::{
    CreateCustomerRequest, CreateLocationRequest, CreateProductRequest,
    CreateSpecialDrugCategoryRequest, CreateSupplierRequest, CreateWarehouseRequest, Customer,
    Location, Product, SpecialDrugCategory, Supplier, UpdateCustomerRequest, UpdateLocationRequest,
    UpdateProductRequest, UpdateSpecialDrugCategoryRequest, UpdateSupplierRequest,
    UpdateWarehouseRequest, Warehouse,
};

use crate::auth::AuthContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MasterDataError {
    NotFound,
    DuplicateCode(String),
    DuplicateLocationCode(String),
    LocationHasStock,
    InvalidLocationCapacity,
    InvalidLocationOwner,
    InvalidWarehouseType,
    InvalidStorageCondition,
    InvalidSpecialDrugCategory,
    InvalidSupplierUscc,
    InvalidCustomerAddress,
    InvalidCustomerProfile,
    InvalidLocationBatchRange,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

trait CatalogEntity: Clone {
    fn id(&self) -> Uuid;
    fn owner_id(&self) -> Uuid;
    fn code(&self) -> &str;
    fn touch(&mut self, at: DateTime<Utc>);
}

#[derive(Clone, Debug)]
struct CatalogStore<T: CatalogEntity> {
    records: BTreeMap<Uuid, T>,
}

impl<T: CatalogEntity> Default for CatalogStore<T> {
    fn default() -> Self {
        Self {
            records: BTreeMap::new(),
        }
    }
}

impl<T: CatalogEntity> CatalogStore<T> {
    fn create(&mut self, record: T) -> Result<T, MasterDataError> {
        if self.records.values().any(|existing| {
            existing.owner_id() == record.owner_id() && existing.code() == record.code()
        }) {
            return Err(MasterDataError::DuplicateCode(record.code().to_string()));
        }
        self.records.insert(record.id(), record.clone());
        Ok(record)
    }

    fn list(&self, owner_id: Uuid) -> Vec<T> {
        self.records
            .values()
            .filter(|record| record.owner_id() == owner_id)
            .cloned()
            .collect()
    }

    fn get(&self, owner_id: Uuid, id: Uuid) -> Result<T, MasterDataError> {
        self.records
            .get(&id)
            .filter(|record| record.owner_id() == owner_id)
            .cloned()
            .ok_or(MasterDataError::NotFound)
    }

    fn update(
        &mut self,
        owner_id: Uuid,
        id: Uuid,
        now: DateTime<Utc>,
        apply: impl FnOnce(&mut T),
    ) -> Result<T, MasterDataError> {
        let record = self.records.get_mut(&id).ok_or(MasterDataError::NotFound)?;
        if record.owner_id() != owner_id {
            return Err(MasterDataError::NotFound);
        }
        apply(record);
        record.touch(now);
        Ok(record.clone())
    }

    fn delete(&mut self, owner_id: Uuid, id: Uuid) -> Result<T, MasterDataError> {
        let record = self.get(owner_id, id)?;
        self.records.remove(&id);
        Ok(record)
    }
}

#[derive(Clone, Debug, Default)]
pub struct MasterDataStore {
    products: CatalogStore<Product>,
    suppliers: CatalogStore<Supplier>,
    customers: CatalogStore<Customer>,
    warehouses: CatalogStore<Warehouse>,
    locations: CatalogStore<Location>,
    special_drug_categories: CatalogStore<SpecialDrugCategory>,
}

impl MasterDataStore {
    pub fn create_product(
        &mut self,
        ctx: &AuthContext,
        req: CreateProductRequest,
        now: DateTime<Utc>,
    ) -> Result<Product, MasterDataError> {
        let attrs = product_attrs_with_default_source(req.attrs, "api_import");
        validate_product_storage_condition(&attrs)?;
        let special_drug_category_code = req
            .special_drug_category_code
            .unwrap_or_else(|| "none".to_string());
        validate_special_drug_category(&special_drug_category_code)?;
        self.products.create(Product {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            product_code: req.product_code,
            product_name: req.product_name,
            approval_no: req.approval_no,
            spec: req.spec,
            dosage_form: req.dosage_form,
            manufacturer: req.manufacturer,
            special_drug_category_code: Some(special_drug_category_code),
            status: "active".to_string(),
            attrs,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_products(&self, ctx: &AuthContext) -> Vec<Product> {
        self.products.list(ctx.owner_id)
    }

    pub fn get_product(&self, ctx: &AuthContext, id: Uuid) -> Result<Product, MasterDataError> {
        self.products.get(ctx.owner_id, id)
    }

    pub fn update_product(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateProductRequest,
        now: DateTime<Utc>,
    ) -> Result<Product, MasterDataError> {
        if let Some(attrs) = req.attrs.as_ref() {
            if attrs.get("storage_condition").is_some() {
                validate_product_storage_condition(attrs)?;
            }
        }
        if let Some(category) = req.special_drug_category_code.as_deref() {
            validate_special_drug_category(category)?;
        }
        self.products.update(ctx.owner_id, id, now, |product| {
            if let Some(value) = req.product_name {
                product.product_name = value;
            }
            if let Some(value) = req.approval_no {
                product.approval_no = Some(value);
            }
            if let Some(value) = req.spec {
                product.spec = Some(value);
            }
            if let Some(value) = req.dosage_form {
                product.dosage_form = Some(value);
            }
            if let Some(value) = req.manufacturer {
                product.manufacturer = Some(value);
            }
            if let Some(value) = req.special_drug_category_code {
                product.special_drug_category_code = Some(value);
            }
            if let Some(value) = req.status {
                product.status = value;
            }
            if let Some(value) = req.attrs {
                if let (Some(existing), Some(next)) =
                    (product.attrs.as_object_mut(), value.as_object())
                {
                    for (key, item) in next {
                        existing.insert(key.clone(), item.clone());
                    }
                } else {
                    product.attrs = value;
                }
            }
        })
    }

    pub fn delete_product(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<Product, MasterDataError> {
        self.products.delete(ctx.owner_id, id)
    }

    pub fn create_supplier(
        &mut self,
        ctx: &AuthContext,
        req: CreateSupplierRequest,
        now: DateTime<Utc>,
    ) -> Result<Supplier, MasterDataError> {
        let license_no = normalize_supplier_uscc(req.license_no)?;
        self.suppliers.create(Supplier {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            supplier_code: req.supplier_code,
            supplier_name: req.supplier_name,
            license_no,
            contact_name: req.contact_name,
            source: req.source.unwrap_or_else(|| "api_import".to_string()),
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_suppliers(&self, ctx: &AuthContext) -> Vec<Supplier> {
        self.suppliers.list(ctx.owner_id)
    }

    pub fn get_supplier(&self, ctx: &AuthContext, id: Uuid) -> Result<Supplier, MasterDataError> {
        self.suppliers.get(ctx.owner_id, id)
    }

    pub fn update_supplier(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateSupplierRequest,
        now: DateTime<Utc>,
    ) -> Result<Supplier, MasterDataError> {
        let license_no = normalize_supplier_uscc(req.license_no)?;
        self.suppliers.update(ctx.owner_id, id, now, |supplier| {
            if let Some(value) = req.supplier_name {
                supplier.supplier_name = value;
            }
            if let Some(value) = license_no {
                supplier.license_no = Some(value);
            }
            if let Some(value) = req.contact_name {
                supplier.contact_name = Some(value);
            }
            if let Some(value) = req.status {
                supplier.status = value;
            }
        })
    }

    pub fn delete_supplier(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<Supplier, MasterDataError> {
        self.suppliers.delete(ctx.owner_id, id)
    }

    pub fn create_customer(
        &mut self,
        ctx: &AuthContext,
        req: CreateCustomerRequest,
        now: DateTime<Utc>,
    ) -> Result<Customer, MasterDataError> {
        self.customers.create(Customer {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            customer_code: req.customer_code,
            customer_name: req.customer_name,
            license_no: req.license_no,
            source: req.source.unwrap_or_else(|| "api_import".to_string()),
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_customers(&self, ctx: &AuthContext) -> Vec<Customer> {
        self.customers.list(ctx.owner_id)
    }

    pub fn update_customer(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateCustomerRequest,
        now: DateTime<Utc>,
    ) -> Result<Customer, MasterDataError> {
        self.customers.update(ctx.owner_id, id, now, |customer| {
            if let Some(value) = req.customer_name {
                customer.customer_name = value;
            }
            if let Some(value) = req.license_no {
                customer.license_no = Some(value);
            }
            if let Some(value) = req.status {
                customer.status = value;
            }
        })
    }

    pub fn delete_customer(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<Customer, MasterDataError> {
        self.customers.delete(ctx.owner_id, id)
    }

    pub fn create_warehouse(
        &mut self,
        ctx: &AuthContext,
        req: CreateWarehouseRequest,
        now: DateTime<Utc>,
    ) -> Result<Warehouse, MasterDataError> {
        validate_warehouse_type(&req.warehouse_type)?;
        self.warehouses.create(Warehouse {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            warehouse_code: req.warehouse_code,
            warehouse_name: req.warehouse_name,
            warehouse_type: req.warehouse_type,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_warehouses(&self, ctx: &AuthContext) -> Vec<Warehouse> {
        self.warehouses.list(ctx.owner_id)
    }

    pub fn update_warehouse(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateWarehouseRequest,
        now: DateTime<Utc>,
    ) -> Result<Warehouse, MasterDataError> {
        let current = self.warehouses.get(ctx.owner_id, id)?;
        let warehouse_type = req
            .warehouse_type
            .as_deref()
            .unwrap_or(&current.warehouse_type);
        validate_warehouse_type(warehouse_type)?;
        self.warehouses.update(ctx.owner_id, id, now, |warehouse| {
            if let Some(value) = req.warehouse_name {
                warehouse.warehouse_name = value;
            }
            if let Some(value) = req.warehouse_type {
                warehouse.warehouse_type = value;
            }
            if let Some(value) = req.status {
                warehouse.status = value;
            }
        })
    }

    pub fn delete_warehouse(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<Warehouse, MasterDataError> {
        self.warehouses.delete(ctx.owner_id, id)
    }

    pub fn create_location(
        &mut self,
        ctx: &AuthContext,
        req: CreateLocationRequest,
        now: DateTime<Utc>,
    ) -> Result<Location, MasterDataError> {
        validate_location_code(&req.location_code, req.row_no, req.column_no, req.layer_no)?;
        validate_location_capacity(req.max_volume_cm3, 0)?;
        self.locations.create(Location {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            warehouse_id: req.warehouse_id,
            zone_id: req.zone_id,
            location_code: req.location_code,
            row_no: req.row_no,
            column_no: req.column_no,
            layer_no: req.layer_no,
            max_volume_cm3: req.max_volume_cm3,
            used_volume_cm3: 0,
            max_sku_count: req.max_sku_count,
            location_type: req.location_type,
            bound_owner_id: req.bound_owner_id,
            status: "available".to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_locations(&self, ctx: &AuthContext) -> Vec<Location> {
        self.locations.list(ctx.owner_id)
    }

    pub fn update_location(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateLocationRequest,
        now: DateTime<Utc>,
    ) -> Result<Location, MasterDataError> {
        let current = self.locations.get(ctx.owner_id, id)?;
        validate_location_code(
            req.location_code
                .as_deref()
                .unwrap_or(&current.location_code),
            req.row_no.unwrap_or(current.row_no),
            req.column_no.unwrap_or(current.column_no),
            req.layer_no.unwrap_or(current.layer_no),
        )?;
        let max_volume_cm3 = req.max_volume_cm3.unwrap_or(current.max_volume_cm3);
        let used_volume_cm3 = req.used_volume_cm3.unwrap_or(current.used_volume_cm3);
        validate_location_capacity(max_volume_cm3, used_volume_cm3)?;
        self.locations.update(ctx.owner_id, id, now, |location| {
            if let Some(value) = req.zone_id {
                location.zone_id = value;
            }
            if let Some(value) = req.location_code {
                location.location_code = value;
            }
            if let Some(value) = req.row_no {
                location.row_no = value;
            }
            if let Some(value) = req.column_no {
                location.column_no = value;
            }
            if let Some(value) = req.layer_no {
                location.layer_no = value;
            }
            if let Some(value) = req.max_volume_cm3 {
                location.max_volume_cm3 = value;
            }
            if let Some(value) = req.used_volume_cm3 {
                location.used_volume_cm3 = value;
            }
            if let Some(value) = req.max_sku_count {
                location.max_sku_count = value;
            }
            if let Some(value) = req.location_type {
                location.location_type = value;
            }
            if let Some(value) = req.bound_owner_id {
                location.bound_owner_id = Some(value);
            }
            if let Some(value) = req.status {
                location.status = value;
            }
        })
    }

    pub fn delete_location(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<Location, MasterDataError> {
        self.locations.delete(ctx.owner_id, id)
    }

    pub fn create_special_drug_category(
        &mut self,
        ctx: &AuthContext,
        req: CreateSpecialDrugCategoryRequest,
        now: DateTime<Utc>,
    ) -> Result<SpecialDrugCategory, MasterDataError> {
        self.special_drug_categories.create(SpecialDrugCategory {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            category_code: req.category_code,
            category_name: req.category_name,
            requires_dual_sign: req.requires_dual_sign,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_special_drug_categories(&self, ctx: &AuthContext) -> Vec<SpecialDrugCategory> {
        self.special_drug_categories.list(ctx.owner_id)
    }

    pub fn update_special_drug_category(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateSpecialDrugCategoryRequest,
        now: DateTime<Utc>,
    ) -> Result<SpecialDrugCategory, MasterDataError> {
        self.special_drug_categories
            .update(ctx.owner_id, id, now, |category| {
                if let Some(value) = req.category_name {
                    category.category_name = value;
                }
                if let Some(value) = req.requires_dual_sign {
                    category.requires_dual_sign = value;
                }
                if let Some(value) = req.status {
                    category.status = value;
                }
            })
    }

    pub fn delete_special_drug_category(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<SpecialDrugCategory, MasterDataError> {
        self.special_drug_categories.delete(ctx.owner_id, id)
    }
}

pub(crate) fn product_attrs_with_default_source(
    attrs: serde_json::Value,
    default_source: &str,
) -> serde_json::Value {
    let mut attrs = attrs;
    if let Some(object) = attrs.as_object_mut() {
        object
            .entry("source")
            .or_insert_with(|| serde_json::Value::String(default_source.to_string()));
        return attrs;
    }
    serde_json::json!({ "source": default_source })
}

pub(crate) fn validate_product_storage_condition(
    attrs: &serde_json::Value,
) -> Result<(), MasterDataError> {
    let Some(value) = attrs
        .get("storage_condition")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(MasterDataError::InvalidStorageCondition);
    };
    if matches!(value, "frozen" | "cold" | "cool" | "normal") {
        Ok(())
    } else {
        Err(MasterDataError::InvalidStorageCondition)
    }
}

pub(crate) fn validate_special_drug_category(category: &str) -> Result<(), MasterDataError> {
    if matches!(
        category,
        "none"
            | "narcotic"
            | "psychotropic_1"
            | "psychotropic_2"
            | "toxic_medical"
            | "radioactive"
            | "vaccine"
            | "blood_product"
    ) {
        Ok(())
    } else {
        Err(MasterDataError::InvalidSpecialDrugCategory)
    }
}

pub(crate) fn normalize_supplier_uscc(
    value: Option<String>,
) -> Result<Option<String>, MasterDataError> {
    const CHARACTERS: &[u8] = b"0123456789ABCDEFGHJKLMNPQRTUWXY";
    const WEIGHTS: [usize; 17] = [
        1, 3, 9, 27, 19, 26, 16, 17, 20, 29, 25, 13, 8, 24, 10, 30, 28,
    ];
    let Some(value) = value
        .map(|item| item.trim().to_ascii_uppercase())
        .filter(|item| !item.is_empty())
    else {
        return Ok(None);
    };
    let bytes = value.as_bytes();
    if bytes.len() != 18 || bytes.iter().any(|item| !CHARACTERS.contains(item)) {
        return Err(MasterDataError::InvalidSupplierUscc);
    }
    let checksum = WEIGHTS
        .iter()
        .zip(bytes)
        .map(|(weight, item)| {
            weight
                * CHARACTERS
                    .iter()
                    .position(|candidate| candidate == item)
                    .unwrap_or_default()
        })
        .sum::<usize>();
    if CHARACTERS[(31 - checksum % 31) % 31] != bytes[17] {
        return Err(MasterDataError::InvalidSupplierUscc);
    }
    Ok(Some(value))
}

pub(crate) fn validate_location_code(
    code: &str,
    row_no: i32,
    column_no: i32,
    layer_no: i32,
) -> Result<(), MasterDataError> {
    let parts = code.trim().split('-').collect::<Vec<_>>();
    let valid = parts.len() == 4
        && parts[0].len() == 3
        && parts[0].chars().all(|item| item.is_ascii_alphanumeric())
        && [row_no, column_no, layer_no]
            .into_iter()
            .zip(&parts[1..])
            .all(|(expected, actual)| {
                actual.len() == 2
                    && actual.chars().all(|item| item.is_ascii_digit())
                    && actual.parse::<i32>().ok() == Some(expected)
                    && (1..=99).contains(&expected)
            });
    if valid {
        Ok(())
    } else {
        Err(MasterDataError::InvalidLocationBatchRange)
    }
}

pub(crate) fn validate_location_capacity(
    max_volume_cm3: i64,
    used_volume_cm3: i64,
) -> Result<(), MasterDataError> {
    if max_volume_cm3 >= 0 && used_volume_cm3 >= 0 && used_volume_cm3 <= max_volume_cm3 {
        Ok(())
    } else {
        Err(MasterDataError::InvalidLocationCapacity)
    }
}

pub(crate) fn validate_warehouse_type(value: &str) -> Result<(), MasterDataError> {
    if matches!(value, "physical" | "logical" | "virtual") {
        Ok(())
    } else {
        Err(MasterDataError::InvalidWarehouseType)
    }
}

impl CatalogEntity for Product {
    fn id(&self) -> Uuid {
        self.id
    }
    fn owner_id(&self) -> Uuid {
        self.owner_id
    }
    fn code(&self) -> &str {
        &self.product_code
    }
    fn touch(&mut self, at: DateTime<Utc>) {
        self.updated_at = at;
    }
}

impl CatalogEntity for Supplier {
    fn id(&self) -> Uuid {
        self.id
    }
    fn owner_id(&self) -> Uuid {
        self.owner_id
    }
    fn code(&self) -> &str {
        &self.supplier_code
    }
    fn touch(&mut self, at: DateTime<Utc>) {
        self.updated_at = at;
    }
}

impl CatalogEntity for Customer {
    fn id(&self) -> Uuid {
        self.id
    }
    fn owner_id(&self) -> Uuid {
        self.owner_id
    }
    fn code(&self) -> &str {
        &self.customer_code
    }
    fn touch(&mut self, at: DateTime<Utc>) {
        self.updated_at = at;
    }
}

impl CatalogEntity for Warehouse {
    fn id(&self) -> Uuid {
        self.id
    }
    fn owner_id(&self) -> Uuid {
        self.owner_id
    }
    fn code(&self) -> &str {
        &self.warehouse_code
    }
    fn touch(&mut self, at: DateTime<Utc>) {
        self.updated_at = at;
    }
}

impl CatalogEntity for Location {
    fn id(&self) -> Uuid {
        self.id
    }
    fn owner_id(&self) -> Uuid {
        self.owner_id
    }
    fn code(&self) -> &str {
        &self.location_code
    }
    fn touch(&mut self, at: DateTime<Utc>) {
        self.updated_at = at;
    }
}

impl CatalogEntity for SpecialDrugCategory {
    fn id(&self) -> Uuid {
        self.id
    }
    fn owner_id(&self) -> Uuid {
        self.owner_id
    }
    fn code(&self) -> &str {
        &self.category_code
    }
    fn touch(&mut self, at: DateTime<Utc>) {
        self.updated_at = at;
    }
}

#[cfg(test)]
mod tests {
    include!("master_data_tests.rs");
}
