    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use uuid::Uuid;
    use wms_domain::{
        CreateLocationRequest, CreateProductRequest, CreateSupplierRequest, UpdateProductRequest,
        UpdateSupplierRequest,
    };

    use super::{MasterDataError, MasterDataStore};
    use crate::auth::AuthContext;

    mod m1_product_validation {
        include!("master_data_m1_tests.rs");
    }

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
                    attrs: json!({"storage_condition": "normal"}),
                },
                now,
            )
            .expect("create product");

        assert_eq!(store.list_products(&ctx_a).len(), 1);
        assert_eq!(created.attrs["source"], "api_import");
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
            license_no: Some("91350100M000100Y43".to_string()),
            contact_name: Some("张三".to_string()),
            source: Some("manual".to_string()),
        };

        let created = store
            .create_supplier(&ctx, req.clone(), now)
            .expect("first supplier");
        assert_eq!(created.source, "manual");
        let duplicate = store.create_supplier(&ctx, req, now);

        assert!(matches!(duplicate, Err(MasterDataError::DuplicateCode(code)) if code == "S-001"));
    }

    #[test]
    fn supplier_uscc_is_normalized_and_rejected_on_create_and_update() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 9, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = MasterDataStore::default();
        let invalid = store.create_supplier(
            &ctx,
            CreateSupplierRequest {
                supplier_code: "S-BAD".to_string(),
                supplier_name: "非法供应商".to_string(),
                license_no: Some("91350100M000100Y49".to_string()),
                contact_name: None,
                source: None,
            },
            now,
        );
        assert!(matches!(invalid, Err(MasterDataError::InvalidSupplierUscc)));

        let created = store
            .create_supplier(
                &ctx,
                CreateSupplierRequest {
                    supplier_code: "S-USCC".to_string(),
                    supplier_name: "合法供应商".to_string(),
                    license_no: Some(" 91350100m000100y43 ".to_string()),
                    contact_name: None,
                    source: None,
                },
                now,
            )
            .expect("valid USCC should create supplier");
        assert_eq!(created.license_no.as_deref(), Some("91350100M000100Y43"));

        let invalid_update = store.update_supplier(
            &ctx,
            created.id,
            UpdateSupplierRequest {
                supplier_name: None,
                license_no: Some("INVALID-USCC".to_string()),
                contact_name: None,
                status: None,
            },
            now,
        );
        assert!(matches!(
            invalid_update,
            Err(MasterDataError::InvalidSupplierUscc)
        ));
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
