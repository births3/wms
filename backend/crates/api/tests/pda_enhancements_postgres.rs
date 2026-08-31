use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext, auth_repository::AuthRepository, wave3_repository::PgWave3Repository,
    wave4_repository::PgWave4Repository, wave5_repository::PgWave5Repository,
};
use wms_domain::{
    BatchCompletePickItem, BatchCompletePickTaskRequest, Quantity, QuickSpotCountRequest,
    RelocateInventoryRequest,
};

fn test_ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "pda_operator".to_string(),
        permissions: vec![
            "m1.read".to_string(),
            "m1.write".to_string(),
            "m2.read".to_string(),
            "m2.write".to_string(),
            "m3.read".to_string(),
            "m3.write".to_string(),
            "m3.relocation.write".to_string(),
            "m3.inventory_count.write".to_string(),
            "m4.read".to_string(),
            "m4.write".to_string(),
            "m-pk.write".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_test_location(
    pool: &PgPool,
    owner_id: Uuid,
    loc_id: Uuid,
    code: &str,
    zone_code: &str,
    temp_zone: &str,
) {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status)
        VALUES ($1, $2, $3, 'PDA测试仓', 'physical', 'active')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &loc_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse failed");

    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status)
        VALUES ($1, $2, $3, $4, 'PDA测试库区', $5, 'qualified_green', 'active')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_code)
    .bind(temp_zone)
    .execute(pool)
    .await
    .expect("seed zone failed");

    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status
        ) VALUES ($1, $2, $3, $4, $5, 1, 1, 1, 5000000, 1000000, 10, 'storage', 'available')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(loc_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(code)
    .execute(pool)
    .await
    .expect("seed location failed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_p0_1_login_owner_code_auto_derivation(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let username = format!("pda_user_{}", &user_id.to_string()[..8]);
    let password = "Password123!";
    let password_hash = bcrypt::hash(password, 4).expect("hash password");

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'OWNER_PDA_01', 'PDA货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("seed owner");

    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, 'PDA测试员', $3, 'active')",
    )
    .bind(user_id)
    .bind(&username)
    .bind(&password_hash)
    .execute(&pool)
    .await
    .expect("seed user");

    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("bind primary owner");

    let auth_repo = AuthRepository::new(pool.clone());

    // 1. Query without owner_code (None) -> auto derives OWNER_PDA_01
    let login_user = auth_repo
        .find_login_user(None, &username)
        .await
        .expect("find login user without owner code")
        .expect("user should exist");
    assert_eq!(login_user.user_id, user_id);
    assert_eq!(login_user.owner_code, "OWNER_PDA_01");

    // 2. Query with explicit owner_code -> finds successfully
    let login_user_explicit = auth_repo
        .find_login_user(Some("OWNER_PDA_01"), &username)
        .await
        .expect("find login user with explicit owner code")
        .expect("user should exist");
    assert_eq!(login_user_explicit.owner_id, owner_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_p0_2_relocation_by_location_code_and_timestamp(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let from_id = Uuid::new_v4();
    let to_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let ctx = test_ctx(owner_id, user_id);

    seed_test_location(&pool, owner_id, from_id, "LOC-SRC-01", "Z-01", "normal").await;
    seed_test_location(&pool, owner_id, to_id, "LOC-DST-02", "Z-01", "normal").await;

    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_frozen, status, location_id, location_code
        ) VALUES ($1, $2, 'MED-REL-01', 'BAT-REL-01', $3, $4, 50, 0, 'qualified', $5, 'LOC-SRC-01')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).unwrap())
    .bind(from_id)
    .execute(&pool)
    .await
    .expect("seed batch");

    let repo = PgWave3Repository::new(pool.clone());
    let req = RelocateInventoryRequest {
        batch_id,
        qty: 20.into(),
        to_location_id: None, // Omitted, resolve via to_location_code
        to_location_code: "LOC-DST-02".to_string(),
        from_location_code: Some("LOC-SRC-01".to_string()),
        relocation_mode: Some("direct".to_string()),
        lpn_code: None,
        reason: Some("PDA现场快速移库".to_string()),
        operated_at: Some(Utc::now()),
    };

    let result = repo
        .relocate_inventory_with_audit(&ctx, req, Utc::now(), "idem-pda-reloc-01", None)
        .await
        .expect("relocation by location_code should succeed");

    assert_eq!(result.value.qty, Quantity::from(20));
    assert_eq!(result.value.to_location_code, "LOC-DST-02");

    let (src_qty, dst_qty): (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT qty_on_hand::BIGINT FROM inventory_batches WHERE id = $1),
            (SELECT COALESCE(SUM(qty_on_hand), 0)::BIGINT FROM inventory_batches WHERE owner_id = $2 AND location_code = 'LOC-DST-02')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("query balances");

    assert_eq!(src_qty, 30);
    assert_eq!(dst_qty, 20);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_p2_8_quick_spot_count_variance_calculation(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let loc_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let ctx = test_ctx(owner_id, user_id);

    seed_test_location(&pool, owner_id, loc_id, "LOC-SPOT-01", "Z-01", "normal").await;

    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_frozen, status, location_id, location_code
        ) VALUES ($1, $2, 'MED-SPOT-01', 'BAT-SPOT-01', $3, $4, 15, 0, 'qualified', $5, 'LOC-SPOT-01')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).unwrap())
    .bind(loc_id)
    .execute(&pool)
    .await
    .expect("seed spot batch");

    let repo = PgWave3Repository::new(pool.clone());

    // 1. MATCH scenario: physical 15 == book 15
    let req_match = QuickSpotCountRequest {
        location_code: "LOC-SPOT-01".to_string(),
        product_code: "MED-SPOT-01".to_string(),
        batch_no: "BAT-SPOT-01".to_string(),
        physical_qty: 15.into(),
        reason: Some("抽盘核对".to_string()),
        operated_at: Some(Utc::now()),
    };
    let res_match = repo
        .quick_spot_count(&ctx, req_match, Utc::now(), "idem-spot-01", None)
        .await
        .expect("spot count match");
    assert_eq!(res_match.value.variance_type, "MATCH");
    assert_eq!(res_match.value.variance_qty, Quantity::ZERO);

    // 2. SURPLUS scenario: physical 18 > book 15
    let req_surplus = QuickSpotCountRequest {
        location_code: "LOC-SPOT-01".to_string(),
        product_code: "MED-SPOT-01".to_string(),
        batch_no: "BAT-SPOT-01".to_string(),
        physical_qty: 18.into(),
        reason: Some("盘盈核对".to_string()),
        operated_at: Some(Utc::now()),
    };
    let res_surplus = repo
        .quick_spot_count(&ctx, req_surplus, Utc::now(), "idem-spot-02", None)
        .await
        .expect("spot count surplus");
    assert_eq!(res_surplus.value.variance_type, "SURPLUS");
    assert_eq!(res_surplus.value.variance_qty, Quantity::from(3));

    // 3. SHORTAGE scenario: physical 12 < book 15
    let req_shortage = QuickSpotCountRequest {
        location_code: "LOC-SPOT-01".to_string(),
        product_code: "MED-SPOT-01".to_string(),
        batch_no: "BAT-SPOT-01".to_string(),
        physical_qty: 12.into(),
        reason: Some("盘亏核对".to_string()),
        operated_at: Some(Utc::now()),
    };
    let res_shortage = repo
        .quick_spot_count(&ctx, req_shortage, Utc::now(), "idem-spot-03", None)
        .await
        .expect("spot count shortage");
    assert_eq!(res_shortage.value.variance_type, "SHORTAGE");
    assert_eq!(res_shortage.value.variance_qty, Quantity::from(-3));

    // 4. SURPLUS scenario on unrecorded batch (book qty is 0): physical 5 > book 0
    let req_unrecorded = QuickSpotCountRequest {
        location_code: "LOC-SPOT-01".to_string(),
        product_code: "MED-SPOT-01".to_string(),
        batch_no: "BAT-SPOT-UNRECORDED".to_string(),
        physical_qty: 5.into(),
        reason: Some("未建档批号盘盈".to_string()),
        operated_at: Some(Utc::now()),
    };
    let res_unrecorded = repo
        .quick_spot_count(&ctx, req_unrecorded, Utc::now(), "idem-spot-04", None)
        .await
        .expect("spot count unrecorded batch surplus");
    assert_eq!(res_unrecorded.value.variance_type, "SURPLUS");
    assert_eq!(res_unrecorded.value.variance_qty, Quantity::from(5));
    assert_eq!(res_unrecorded.value.book_qty, Quantity::ZERO);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_p1_6_tote_status_preflight(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let ctx = test_ctx(owner_id, user_id);

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'OWNER_TOTE_01', '周转箱测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("seed owner");

    sqlx::query(
        "INSERT INTO lpn_containers (id, owner_id, lpn_code, container_type, status, created_at, updated_at) VALUES ($1, $2, 'TOTE-TEST-01', 'tote', 'idle', now(), now())",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("seed lpn");

    let repo = PgWave5Repository::new(pool.clone());
    let tote_status = repo
        .get_tote_status(&ctx, "TOTE-TEST-01")
        .await
        .expect("get tote status");

    assert_eq!(tote_status.tote_code, "TOTE-TEST-01");
    assert_eq!(tote_status.status, "AVAILABLE");
    assert_eq!(tote_status.loaded_sku_count, 0);

    // Verify non-existent tote returns NotFound
    let not_found_result = repo.get_tote_status(&ctx, "TOTE-NONEXISTENT").await;
    assert!(matches!(
        not_found_result,
        Err(wms_api::wave5_repository::Wave5RepositoryError::NotFound)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_p1_7_batch_complete_pick_tasks_with_tote_and_trace_codes(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let ctx = test_ctx(owner_id, user_id);
    let op_time = Utc::now();

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'OWNER_PICK_01', '拣货测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("seed owner");

    sqlx::query(
        r#"
        INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status)
        VALUES ($1, $2, 'WH-PICK-01', '拣货测试仓', 'physical', 'active')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("seed warehouse");

    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, wms_order_no, erp_order_no, customer_id,
            delivery_address_id, delivery_address_snapshot, warehouse_id, status
        ) VALUES ($1, $2, 'OUT-BATCH-01', 'ERP-001', $3, gen_random_uuid(), '{}'::jsonb, $4, 'in_wave')
        "#,
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(customer_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await
    .expect("seed outbound order");

    sqlx::query(
        r#"
        INSERT INTO outbound_order_lines (
            id, outbound_order_id, owner_id, line_no, product_code, batch_no, planned_qty, picked_qty
        ) VALUES
            ($1, $2, $3, 1, 'MED-01', 'BAT-01', 10, 0),
            ($4, $2, $3, 2, 'MED-02', 'BAT-02', 15, 0)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(order_id)
    .bind(owner_id)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("seed outbound order lines");

    sqlx::query(
        "INSERT INTO lpn_containers (id, owner_id, lpn_code, container_type, status, created_at, updated_at) VALUES ($1, $2, 'TOT-PICK-99', 'tote', 'idle', now(), now())",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("seed lpn");

    let repo = PgWave4Repository::new(pool.clone());
    let req = BatchCompletePickTaskRequest {
        order_id,
        outbound_lpn: Some("TOT-PICK-99".to_string()),
        items: vec![
            BatchCompletePickItem {
                line_no: 1,
                picked_qty: 10.into(),
                trace_codes: vec!["TR-001".to_string(), "TR-002".to_string()],
            },
            BatchCompletePickItem {
                line_no: 2,
                picked_qty: 15.into(),
                trace_codes: vec!["TR-003".to_string()],
            },
        ],
        operated_at: Some(op_time),
    };

    let audit_diff = req.operated_at.map(|t| {
        wms_api::audit::AuditDiff::compute(
            serde_json::json!({}),
            serde_json::json!({ "operated_at": t }),
        )
    });
    let audit = wms_api::audit::AuditWriteRequest::from_auth_context(
        &ctx,
        "batch_complete_pick_task",
        "M4",
        "outbound_order",
        order_id.to_string(),
        audit_diff,
    );

    let res = repo
        .batch_complete_pick_tasks(&ctx, req, op_time, "idem-pick-test-01", Some(audit))
        .await
        .expect("batch complete pick tasks");

    assert_eq!(res.value.order_id, order_id);
    assert_eq!(res.value.completed_lines, 2);
    assert_eq!(res.value.outbound_lpn.as_deref(), Some("TOT-PICK-99"));
    assert_eq!(res.value.status, "picked");

    // Verify tote status changed from idle to in_use
    let tote_status: String = sqlx::query_scalar(
        "SELECT status FROM lpn_containers WHERE owner_id = $1 AND lpn_code = 'TOT-PICK-99'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("query tote status");
    assert_eq!(tote_status, "in_use");

    // Verify audit event diff recorded operated_at
    let audit_diff_val: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT diff FROM audit_event WHERE owner_id = $1 AND action = 'batch_complete_pick_task' ORDER BY id DESC LIMIT 1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("query audit event");

    let diff_json = audit_diff_val.expect("audit event should have diff");
    assert!(diff_json["after"]["operated_at"].is_string());
}
