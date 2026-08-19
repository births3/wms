use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn warehouse_zones_temperature_zone_5_zones_and_new_attributes(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("OWNER-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed owner");

    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, '测试仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed warehouse");

    // Test all 5 valid temperature zones
    let valid_zones = [
        ("ZONE-NORM", "normal_10_30"),
        ("ZONE-COOL", "cool_le_20"),
        ("ZONE-COLD", "cold_2_8"),
        ("ZONE-FRZ", "freeze_le_minus_20"),
        ("ZONE-ULTRA", "ultra_cold_minus_80"),
    ];

    for (code, temp_zone) in valid_zones {
        let zone_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO warehouse_zones (
                id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
                quality_color, allowed_categories, is_external_use_zone, is_fragrant_zone,
                is_special_drug_zone, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, '测试区', $5, 'qualified_green',
                    '["drug", "medical_device"]'::jsonb, TRUE, FALSE, TRUE, 'active', now(), now())
            "#,
        )
        .bind(zone_id)
        .bind(owner_id)
        .bind(warehouse_id)
        .bind(code)
        .bind(temp_zone)
        .execute(&pool)
        .await
        .unwrap_or_else(|e| panic!("insert valid temperature_zone {temp_zone} failed: {e}"));

        // Verify inserted fields
        let (tz, is_ext, is_frag, is_spec, allowed): (String, bool, bool, bool, serde_json::Value) =
            sqlx::query_as(
                r#"
                SELECT temperature_zone, is_external_use_zone, is_fragrant_zone, is_special_drug_zone, allowed_categories
                FROM warehouse_zones WHERE id = $1
                "#,
            )
            .bind(zone_id)
            .fetch_one(&pool)
            .await
            .expect("fetch zone");

        assert_eq!(tz, temp_zone);
        assert!(is_ext);
        assert!(!is_frag);
        assert!(is_spec);
        assert_eq!(allowed, serde_json::json!(["drug", "medical_device"]));
    }

    // Old or invalid temperature_zone should be rejected by CHECK constraint
    let invalid_res = sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'ZONE-OLD-FROZEN', '旧冷冻区', 'frozen', 'qualified_green', 'active', now(), now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await;

    assert!(
        invalid_res.is_err(),
        "old temperature_zone 'frozen' should violate CHECK constraint"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn warehouse_locations_status_3_values_lock_status_and_attributes(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("OWNER-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed owner");

    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, '测试仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed warehouse");

    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'ZONE-A', '合格区', 'normal_10_30', 'qualified_green', 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await
    .expect("seed warehouse zone");

    // Test location_type='staging' and all new location attributes
    let loc_staging_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code,
            row_no, column_no, layer_no, max_volume_cm3, max_sku_count,
            location_type, current_owner_id, status, allows_container,
            mix_product_policy, mix_batch_policy, lock_status,
            pick_zone_level, pick_sequence_no, putaway_sequence_no,
            is_agv_managed, agv_pod_code, created_at, updated_at
        )
        VALUES (
            $1, $2, $3, $4, 'STAGING-01',
            1, 1, 1, 10000000, 10,
            'staging', $2, 'available', TRUE,
            'restricted_mix', 'multi_batch', 'normal',
            'gold', 100, 50,
            TRUE, 'POD-01', now(), now()
        )
        "#,
    )
    .bind(loc_staging_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(&pool)
    .await
    .expect("insert staging location");

    // Verify all fields
    let (
        loc_type,
        current_owner,
        status,
        allows_cnt,
        mix_prod,
        mix_btch,
        lock_st,
        pick_lvl,
        pick_seq,
        put_seq,
        agv_mng,
        pod_code,
    ): (
        String,
        Option<Uuid>,
        String,
        bool,
        String,
        String,
        String,
        Option<String>,
        Option<i32>,
        Option<i32>,
        bool,
        Option<String>,
    ) = sqlx::query_as(
        r#"
        SELECT location_type, current_owner_id, status, allows_container,
               mix_product_policy, mix_batch_policy, lock_status,
               pick_zone_level, pick_sequence_no, putaway_sequence_no,
               is_agv_managed, agv_pod_code
        FROM warehouse_locations WHERE id = $1
        "#,
    )
    .bind(loc_staging_id)
    .fetch_one(&pool)
    .await
    .expect("fetch location");

    assert_eq!(loc_type, "staging");
    assert_eq!(current_owner, Some(owner_id));
    assert_eq!(status, "available");
    assert!(allows_cnt);
    assert_eq!(mix_prod, "restricted_mix");
    assert_eq!(mix_btch, "multi_batch");
    assert_eq!(lock_st, "normal");
    assert_eq!(pick_lvl.as_deref(), Some("gold"));
    assert_eq!(pick_seq, Some(100));
    assert_eq!(put_seq, Some(50));
    assert!(agv_mng);
    assert_eq!(pod_code.as_deref(), Some("POD-01"));

    // Verify lock_status values: lock_in, lock_out, lock_all (none 不在模型中)
    for lk in ["lock_in", "lock_out", "lock_all"] {
        sqlx::query("UPDATE warehouse_locations SET lock_status = $2 WHERE id = $1")
            .bind(loc_staging_id)
            .bind(lk)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("set lock_status to {lk} failed: {e}"));
    }

    // Verify status 3 values: available, occupied, disabled
    for st in ["occupied", "disabled", "available"] {
        sqlx::query("UPDATE warehouse_locations SET status = $2 WHERE id = $1")
            .bind(loc_staging_id)
            .bind(st)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("set status to {st} failed: {e}"));
    }

    // Verify old status 'locked' is rejected by CHECK constraint
    let invalid_st_res =
        sqlx::query("UPDATE warehouse_locations SET status = 'locked' WHERE id = $1")
            .bind(loc_staging_id)
            .execute(&pool)
            .await;
    assert!(
        invalid_st_res.is_err(),
        "status='locked' should be rejected by 3-value CHECK constraint"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_batches_in_transit_and_available_qty_formulas(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_pick_id = Uuid::new_v4();
    let location_store_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("OWNER-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed owner");

    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, '测试仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed warehouse");

    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'ZONE-INV', '合格区', 'normal_10_30', 'qualified_green', 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await
    .expect("seed warehouse zone");

    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code,
            row_no, column_no, layer_no, max_volume_cm3, max_sku_count,
            location_type, status, created_at, updated_at
        )
        VALUES
            ($1, $3, $4, $5, 'PICK-01', 1, 1, 1, 5000000, 1, 'piece_pick', 'available', now(), now()),
            ($2, $3, $4, $5, 'STORE-01', 1, 1, 2, 10000000, 1, 'storage', 'available', now(), now())
        "#,
    )
    .bind(location_pick_id)
    .bind(location_store_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(&pool)
    .await
    .expect("seed locations");

    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification,
            storage_condition, special_drug_category, is_external_use, is_fragrant,
            status, created_at, updated_at
        )
        VALUES (
            $1, $2, 'PROD-001', '阿莫西林胶囊', '0.25g*24粒',
            'normal_10_30', 'general', FALSE, FALSE,
            'active', now(), now()
        )
        "#,
    )
    .bind(product_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("seed product");

    let batch_pick_id = Uuid::new_v4();
    let batch_store_id = Uuid::new_v4();

    // 1. Picking location batch: on_hand=100, allocated=10, frozen=5, replenish_in_transit=20, replenish_out_transit=0
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, warehouse_id, zone_id, location_id, product_id,
            batch_no, production_date, expiry_date,
            qty_on_hand, qty_allocated, qty_frozen,
            qty_replenish_in_transit, qty_replenish_out_transit,
            status, recall_flag, created_at, updated_at, version
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            'BATCH-P01', '2026-01-01', '2028-01-01',
            100.0000, 10.0000, 5.0000,
            20.0000, 0.0000,
            'qualified', FALSE, now(), now(), 1
        )
        "#,
    )
    .bind(batch_pick_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(location_pick_id)
    .bind(product_id)
    .execute(&pool)
    .await
    .expect("insert pick inventory batch");

    // 2. Storage location batch: on_hand=200, allocated=20, frozen=10, replenish_in_transit=0, replenish_out_transit=30, container_lpn='PALLET-001'
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, warehouse_id, zone_id, location_id, product_id,
            batch_no, production_date, expiry_date, container_lpn,
            qty_on_hand, qty_allocated, qty_frozen,
            qty_replenish_in_transit, qty_replenish_out_transit,
            status, recall_flag, created_at, updated_at, version
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            'BATCH-S01', '2026-01-01', '2028-01-01', 'PALLET-001',
            200.0000, 20.0000, 10.0000,
            0.0000, 30.0000,
            'qualified', FALSE, now(), now(), 1
        )
        "#,
    )
    .bind(batch_store_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(location_store_id)
    .bind(product_id)
    .execute(&pool)
    .await
    .expect("insert storage inventory batch");

    // Contract formula 1: Picking location available qty = on_hand - allocated - frozen + replenish_in_transit
    let pick_available: rust_decimal::Decimal = sqlx::query_scalar(
        r#"
        SELECT (qty_on_hand - qty_allocated - qty_frozen + qty_replenish_in_transit)
        FROM inventory_batches WHERE id = $1
        "#,
    )
    .bind(batch_pick_id)
    .fetch_one(&pool)
    .await
    .expect("calc pick available");

    // 100 - 10 - 5 + 20 = 105
    assert_eq!(pick_available.to_string(), "105.0000");

    // Contract formula 2: Storage location offshelf available qty = on_hand - allocated - frozen - replenish_out_transit
    let store_available: rust_decimal::Decimal = sqlx::query_scalar(
        r#"
        SELECT (qty_on_hand - qty_allocated - qty_frozen - qty_replenish_out_transit)
        FROM inventory_batches WHERE id = $1
        "#,
    )
    .bind(batch_store_id)
    .fetch_one(&pool)
    .await
    .expect("calc store available");

    // 200 - 20 - 10 - 30 = 140
    assert_eq!(store_available.to_string(), "140.0000");

    // Test unique constraint: (owner_id, product_id, batch_no, location_id, status)
    let dup_res = sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, warehouse_id, zone_id, location_id, product_id,
            batch_no, production_date, expiry_date,
            qty_on_hand, qty_allocated, qty_frozen,
            qty_replenish_in_transit, qty_replenish_out_transit,
            status, recall_flag, created_at, updated_at, version
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            'BATCH-P01', '2026-01-01', '2028-01-01',
            50.0000, 0.0000, 0.0000,
            0.0000, 0.0000,
            'qualified', FALSE, now(), now(), 1
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(location_pick_id)
    .bind(product_id)
    .execute(&pool)
    .await;

    assert!(
        dup_res.is_err(),
        "duplicate (owner_id, product_id, batch_no, location_id, status) should be rejected"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn location_device_bindings_and_iot_devices_schema(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();
    let binding_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("OWNER-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed owner");

    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, '测试仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed warehouse");

    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'ZONE-DEV', '合格区', 'normal_10_30', 'qualified_green', 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await
    .expect("seed warehouse zone");

    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code,
            row_no, column_no, layer_no, max_volume_cm3, max_sku_count,
            location_type, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, 'LOC-PTL-01', 1, 1, 1, 5000000, 1, 'piece_pick', 'available', now(), now())
        "#,
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(&pool)
    .await
    .expect("seed location");

    // 1. Insert location_device_binding (warehouse_id, location_id, device_id, binding_role, point_address)
    //    Note: iot_devices 表属 Phase 2（设备中台），Phase 1 仅建绑定表，device_id 暂为无外键 UUID。
    sqlx::query(
        r#"
        INSERT INTO location_device_bindings (
            id, warehouse_id, location_id, device_id, binding_role, point_address,
            valid_from, valid_to, created_at, updated_at
        )
        VALUES (
            $1, $2, $3, $4, 'ptl_light', 'TAG-01-01',
            now(), NULL, now(), now()
        )
        "#,
    )
    .bind(binding_id)
    .bind(warehouse_id)
    .bind(location_id)
    .bind(device_id)
    .execute(&pool)
    .await
    .expect("insert location device binding");

    // 3. Unique active index: inserting another active binding for same (location_id, binding_role) should fail
    let duplicate_active_res = sqlx::query(
        r#"
        INSERT INTO location_device_bindings (
            id, warehouse_id, location_id, device_id, binding_role, point_address,
            valid_from, valid_to, created_at, updated_at
        )
        VALUES (
            $1, $2, $3, $4, 'ptl_light', 'TAG-01-02',
            now(), NULL, now(), now()
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(warehouse_id)
    .bind(location_id)
    .bind(device_id)
    .execute(&pool)
    .await;

    assert!(
        duplicate_active_res.is_err(),
        "duplicate active binding for same (location_id, binding_role) must be rejected"
    );

    // 4. Soft unbind by setting valid_to, then new active binding succeeds
    sqlx::query("UPDATE location_device_bindings SET valid_to = now() WHERE id = $1")
        .bind(binding_id)
        .execute(&pool)
        .await
        .expect("soft unbind");

    sqlx::query(
        r#"
        INSERT INTO location_device_bindings (
            id, warehouse_id, location_id, device_id, binding_role, point_address,
            valid_from, valid_to, created_at, updated_at
        )
        VALUES (
            $1, $2, $3, $4, 'ptl_light', 'TAG-01-02',
            now(), NULL, now(), now()
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(warehouse_id)
    .bind(location_id)
    .bind(device_id)
    .execute(&pool)
    .await
    .expect("insert replacement active binding after previous was expired");
}

#[sqlx::test(migrations = "../../migrations")]
async fn products_storage_condition_5_zones_and_flags(pool: PgPool) {
    let owner_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("OWNER-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed owner");

    // 1. Verify all 5 valid storage conditions are accepted
    let valid_conditions = [
        ("PROD-NORM", "normal_10_30"),
        ("PROD-COOL", "cool_le_20"),
        ("PROD-COLD", "cold_2_8"),
        ("PROD-FRZ", "freeze_le_minus_20"),
        ("PROD-ULTRA", "ultra_cold_minus_80"),
    ];

    for (pcode, cond) in valid_conditions {
        let pid = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO products (
                id, owner_id, product_code, product_name, specification,
                storage_condition, special_drug_category, is_external_use, is_fragrant,
                status, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, '测试药品', '100mg*10片',
                $4, 'general', TRUE, FALSE,
                'active', now(), now()
            )
            "#,
        )
        .bind(pid)
        .bind(owner_id)
        .bind(pcode)
        .bind(cond)
        .execute(&pool)
        .await
        .unwrap_or_else(|e| panic!("insert product with storage_condition {cond} failed: {e}"));

        let (stored_cond, ext, frag): (Option<String>, bool, bool) = sqlx::query_as(
            "SELECT storage_condition, is_external_use, is_fragrant FROM products WHERE id = $1",
        )
        .bind(pid)
        .fetch_one(&pool)
        .await
        .expect("fetch product");

        assert_eq!(stored_cond.as_deref(), Some(cond));
        assert!(ext);
        assert!(!frag);
    }

    // 2. Old temperature condition 'frozen' should violate CHECK constraint
    let invalid_res = sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification,
            storage_condition, special_drug_category, status, created_at, updated_at
        )
        VALUES (
            $1, $2, 'PROD-INVALID', '非法温区药品', '100mg',
            'frozen', 'general', 'active', now(), now()
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(&pool)
    .await;

    assert!(
        invalid_res.is_err(),
        "old storage_condition 'frozen' must violate CHECK constraint"
    );
}
