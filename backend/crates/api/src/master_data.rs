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
        self.products.create(Product {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            product_code: req.product_code,
            product_name: req.product_name,
            approval_no: req.approval_no,
            spec: req.spec,
            dosage_form: req.dosage_form,
            manufacturer: req.manufacturer,
            special_drug_category_code: req.special_drug_category_code,
            status: "active".to_string(),
            attrs: req.attrs,
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
                product.attrs = value;
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
        self.suppliers.create(Supplier {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            supplier_code: req.supplier_code,
            supplier_name: req.supplier_name,
            license_no: req.license_no,
            contact_name: req.contact_name,
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
        self.suppliers.update(ctx.owner_id, id, now, |supplier| {
            if let Some(value) = req.supplier_name {
                supplier.supplier_name = value;
            }
            if let Some(value) = req.license_no {
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
        self.warehouses.create(Warehouse {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            warehouse_code: req.warehouse_code,
            warehouse_name: req.warehouse_name,
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
        self.warehouses.update(ctx.owner_id, id, now, |warehouse| {
            if let Some(value) = req.warehouse_name {
                warehouse.warehouse_name = value;
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
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use uuid::Uuid;
    use wms_domain::{CreateLocationRequest, CreateProductRequest, CreateSupplierRequest, UpdateProductRequest};

    use super::{MasterDataError, MasterDataStore};
    use crate::auth::AuthContext;

    fn ctx(owner_id: Uuid) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "tester".to_string(),
            permissions: vec!["m1.write".to_string()],
            jti: Uuid::new_v4().to_string(),
        }
    }

    #[test]
    fn product_crud_is_owner_scoped() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 9, 0, 0)
            .single()
            .expect("valid time");
        let owner_a = Uuid::new_v4();
        let owner_b = Uuid::new_v4();
        let ctx_a = ctx(owner_a);
        let ctx_b = ctx(owner_b);
        let mut store = MasterDataStore::default();

        let created = store
            .create_product(
                &ctx_a,
                CreateProductRequest {
                    product_code: "P-001".to_string(),
                    product_name: "感冒灵颗粒".to_string(),
                    approval_no: Some("国药准字Z0001".to_string()),
                    spec: Some("10g*9袋".to_string()),
                    dosage_form: Some("颗粒剂".to_string()),
                    manufacturer: Some("示例药业".to_string()),
                    special_drug_category_code: None,
                    attrs: json!({"storage": "常温"}),
                },
                now,
            )
            .expect("create product");

        assert_eq!(store.list_products(&ctx_a).len(), 1);
        assert!(matches!(
            store.get_product(&ctx_b, created.id),
            Err(MasterDataError::NotFound)
        ));

        let updated = store
            .update_product(
                &ctx_a,
                created.id,
                UpdateProductRequest {
                    product_name: Some("感冒灵颗粒新版".to_string()),
                    approval_no: None,
                    spec: None,
                    dosage_form: None,
                    manufacturer: None,
                    special_drug_category_code: None,
                    status: None,
                    attrs: None,
                },
                now,
            )
            .expect("update product");
        assert_eq!(updated.product_name, "感冒灵颗粒新版");

        let deleted = store
            .delete_product(&ctx_a, created.id)
            .expect("delete product");
        assert_eq!(deleted.product_code, "P-001");
        assert!(store.list_products(&ctx_a).is_empty());
    }

    #[test]
    fn supplier_codes_are_unique_per_owner() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 9, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = MasterDataStore::default();
        let req = CreateSupplierRequest {
            supplier_code: "S-001".to_string(),
            supplier_name: "国药控股".to_string(),
            license_no: Some("LIC-001".to_string()),
            contact_name: Some("张三".to_string()),
        };

        store
            .create_supplier(&ctx, req.clone(), now)
            .expect("first supplier");
        let duplicate = store.create_supplier(&ctx, req, now);

        assert!(matches!(duplicate, Err(MasterDataError::DuplicateCode(code)) if code == "S-001"));
    }

    #[test]
    fn location_contract_keeps_zone_grid_and_capacity_fields() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 9, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = MasterDataStore::default();
        let warehouse_id = Uuid::new_v4();
        let zone_id = Uuid::new_v4();

        let location = store
            .create_location(
                &ctx,
                CreateLocationRequest {
                    warehouse_id,
                    zone_id,
                    location_code: "A01-01-02-03".to_string(),
                    row_no: 1,
                    column_no: 2,
                    layer_no: 3,
                    max_volume_cm3: 5_000_000,
                    max_sku_count: 1,
                    location_type: "storage".to_string(),
                    bound_owner_id: Some(ctx.owner_id),
                },
                now,
            )
            .expect("create location");

        assert_eq!(location.warehouse_id, warehouse_id);
        assert_eq!(location.zone_id, zone_id);
        assert_eq!(location.location_code, "A01-01-02-03");
        assert_eq!(location.row_no, 1);
        assert_eq!(location.column_no, 2);
        assert_eq!(location.layer_no, 3);
        assert_eq!(location.max_volume_cm3, 5_000_000);
        assert_eq!(location.used_volume_cm3, 0);
        assert_eq!(location.max_sku_count, 1);
        assert_eq!(location.location_type, "storage");
        assert_eq!(location.bound_owner_id, Some(ctx.owner_id));
        assert_eq!(location.status, "available");
    }
}
