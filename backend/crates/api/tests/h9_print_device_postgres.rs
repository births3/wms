use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    print_device::{resolve_lease_release_mode, PrintDeviceError, PrintDeviceService},
};
use wms_domain::{
    CreatePrintSiteRequest, CreatePrinterRequest, CreatePrinterTrayRequest,
    CreateSiteOwnerMappingRequest, ReleaseDeviceLeaseRequest, TestPrintRequest,
    UpdatePrinterRequest, UpdatePrinterTrayRequest,
};

fn ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "h9-device-test".to_string(),
        permissions: permissions.iter().map(|p| p.to_string()).collect(),
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_owner(pool: &PgPool, owner_id: Uuid, code: &str) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H9 设备测试货主')",
    )
    .bind(owner_id)
    .bind(code)
    .execute(pool)
    .await
    .expect("owner should insert");
}

async fn grant_owner_permissions(
    pool: &PgPool,
    user_id: Uuid,
    owner_id: Uuid,
    permissions: &[&str],
) {
    sqlx::query(
        r#"
        INSERT INTO auth_users (
            id, username, display_name, password_hash, status
        )
        VALUES ($1, $2, 'H9 多货主测试用户', 'not-used-in-test', 'active')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(format!("h9-device-{}", &user_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("auth user should insert");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active) VALUES ($1, $2, TRUE)",
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("owner binding should insert");
    let role_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_roles (id, owner_id, role_code, role_name) VALUES ($1, $2, $3, 'H9 设备多货主管理')",
    )
    .bind(role_id)
    .bind(owner_id)
    .bind(format!("h9_device_{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("owner role should insert");
    sqlx::query("INSERT INTO auth_user_roles (user_id, owner_id, role_id) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(owner_id)
        .bind(role_id)
        .execute(pool)
        .await
        .expect("user role should insert");
    sqlx::query(
        r#"
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT $1, id
          FROM auth_permissions
         WHERE permission_code = ANY($2)
        "#,
    )
    .bind(role_id)
    .bind(permissions)
    .execute(pool)
    .await
    .expect("role permissions should insert");
}

fn site_request(code: &str, name: &str) -> CreatePrintSiteRequest {
    CreatePrintSiteRequest {
        site_code: code.to_string(),
        site_name: name.to_string(),
    }
}

fn printer_request(site_id: Uuid, name: &str, connection_type: &str) -> CreatePrinterRequest {
    CreatePrinterRequest {
        site_id,
        printer_name: name.to_string(),
        printer_model: Some("HP LaserJet 5200".to_string()),
        connection_type: connection_type.to_string(),
        release_mode_override: None,
    }
}

fn tray_request(code: &str) -> CreatePrinterTrayRequest {
    CreatePrinterTrayRequest {
        tray_code: code.to_string(),
        paper_size: "A4".to_string(),
        paper_type: "普通纸".to_string(),
    }
}

async fn insert_lease(
    pool: &PgPool,
    site_id: Uuid,
    printer_id: Uuid,
    release_mode: &str,
    busy_state: &str,
    status: &str,
) -> Result<Uuid, sqlx::Error> {
    let lease_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO h9_device_leases (
            id, site_id, printer_id, holder_agent_id, lease_token, release_mode,
            busy_state, status, assigned_at, acquired_at, released_at
        )
        VALUES (
            $1, $2, $3, NULL, $4, $5, $6, $7, '2026-07-27T08:00:00Z',
            '2026-07-27T08:00:01Z',
            CASE WHEN $7 = 'released' THEN '2026-07-27T08:30:00Z'::timestamptz ELSE NULL END
        )
        "#,
    )
    .bind(lease_id)
    .bind(site_id)
    .bind(printer_id)
    .bind(format!("lease-token-{lease_id}"))
    .bind(release_mode)
    .bind(busy_state)
    .bind(status)
    .execute(pool)
    .await
    .map(|_| lease_id)
}

#[sqlx::test(migrations = "../../migrations")]
async fn disable_site_owner_mapping_create_and_update_printer_tray_and_test_print_are_idempotent(
    pool: PgPool,
) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id, "H9D01").await;
    let auth = ctx(owner_id, &["h9.print_device.read", "h9.print_device.write"]);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 27, 9, 0, 0)
        .single()
        .expect("valid test time");
    let service = PrintDeviceService::with_postgres(pool.clone());

    // AC1：站点 + 显式货主仓映射
    let site = service
        .create_site(
            &auth,
            site_request("SITE-EAST", "东区打印站"),
            now,
            "h9d-site-1",
        )
        .await
        .expect("site should create")
        .value;
    let duplicate = service
        .create_site(
            &auth,
            site_request("SITE-EAST", "重复站点"),
            now,
            "h9d-site-dup",
        )
        .await;
    assert_eq!(duplicate, Err(PrintDeviceError::SiteCodeConflict));

    let warehouse_id = Uuid::new_v4();
    let mapping = service
        .create_site_owner_mapping(
            &auth,
            site.id,
            CreateSiteOwnerMappingRequest {
                owner_id,
                warehouse_id,
            },
            now,
            "h9d-map-1",
        )
        .await
        .expect("mapping should create")
        .value;
    assert_eq!(mapping.status, "active");
    let mapping_conflict = service
        .create_site_owner_mapping(
            &auth,
            site.id,
            CreateSiteOwnerMappingRequest {
                owner_id,
                warehouse_id,
            },
            now,
            "h9d-map-dup",
        )
        .await;
    assert_eq!(mapping_conflict, Err(PrintDeviceError::MappingConflict));

    // 停用软删后允许重新映射；重复停用被拒绝
    let disabled_mutation = service
        .disable_site_owner_mapping(&auth, site.id, mapping.id, now, "h9d-map-disable")
        .await
        .expect("mapping should disable");
    assert!(!disabled_mutation.replayed);
    let disabled = disabled_mutation.value;
    assert_eq!(disabled.status, "disabled");
    assert!(disabled.disabled_at.is_some());
    let disabled_replay = service
        .disable_site_owner_mapping(&auth, site.id, mapping.id, now, "h9d-map-disable")
        .await
        .expect("same mapping disable should replay");
    assert!(disabled_replay.replayed);
    assert_eq!(disabled_replay.value.id, disabled.id);
    let disable_again = service
        .disable_site_owner_mapping(&auth, site.id, mapping.id, now, "h9d-map-disable-2")
        .await;
    assert_eq!(disable_again, Err(PrintDeviceError::MappingAlreadyDisabled));
    service
        .create_site_owner_mapping(
            &auth,
            site.id,
            CreateSiteOwnerMappingRequest {
                owner_id,
                warehouse_id,
            },
            now,
            "h9d-map-again",
        )
        .await
        .expect("softly disabled mapping should allow a fresh active row");

    // 打印机归属唯一站点；不存在的站点拒绝
    let printer = service
        .create_printer(
            &auth,
            printer_request(site.id, "东区网络打印机", "network"),
            now,
            "h9d-printer-1",
        )
        .await
        .expect("printer should create")
        .value;
    assert_eq!(printer.site_id, site.id);
    assert_eq!(printer.effective_release_mode, "manual_only");
    let name_conflict = service
        .create_printer(
            &auth,
            printer_request(site.id, "东区网络打印机", "usb"),
            now,
            "h9d-printer-dup",
        )
        .await;
    assert_eq!(name_conflict, Err(PrintDeviceError::PrinterNameConflict));
    let orphan = service
        .create_printer(
            &auth,
            printer_request(Uuid::new_v4(), "无站点打印机", "network"),
            now,
            "h9d-printer-orphan",
        )
        .await;
    assert_eq!(orphan, Err(PrintDeviceError::SiteNotFound));

    // AC2：纸盒纸张能力、启用状态与设备标识
    let tray_mutation = service
        .create_printer_tray(&auth, printer.id, tray_request("TRAY-1"), now, "h9d-tray-1")
        .await
        .expect("tray should create");
    assert!(!tray_mutation.replayed);
    let tray = tray_mutation.value;
    let tray_replay = service
        .create_printer_tray(&auth, printer.id, tray_request("TRAY-1"), now, "h9d-tray-1")
        .await
        .expect("same tray create should replay");
    assert!(tray_replay.replayed);
    assert_eq!(tray_replay.value.id, tray.id);
    let tray_conflict = service
        .create_printer_tray(
            &auth,
            printer.id,
            tray_request("TRAY-1"),
            now,
            "h9d-tray-dup",
        )
        .await;
    assert_eq!(tray_conflict, Err(PrintDeviceError::TrayConflict));
    let tray_update_request = UpdatePrinterTrayRequest {
        paper_size: Some("A5".to_string()),
        paper_type: Some("不干胶标签纸".to_string()),
        enabled: Some(false),
    };
    let updated_tray_mutation = service
        .update_printer_tray(
            &auth,
            printer.id,
            tray.id,
            tray_update_request.clone(),
            now,
            "h9d-tray-update",
        )
        .await
        .expect("tray capability should update");
    assert!(!updated_tray_mutation.replayed);
    let updated_tray = updated_tray_mutation.value;
    assert_eq!(updated_tray.paper_size, "A5");
    assert_eq!(updated_tray.paper_type, "不干胶标签纸");
    assert!(!updated_tray.enabled);
    let updated_tray_replay = service
        .update_printer_tray(
            &auth,
            printer.id,
            tray.id,
            tray_update_request,
            now,
            "h9d-tray-update",
        )
        .await
        .expect("same tray update should replay");
    assert!(updated_tray_replay.replayed);
    assert_eq!(updated_tray_replay.value.id, updated_tray.id);

    // AC3：测试打印落表；停用纸盒拒绝
    let disabled_tray_test = service
        .test_print(
            &auth,
            printer.id,
            TestPrintRequest { tray_id: tray.id },
            now,
            "h9d-test-disabled",
        )
        .await;
    assert_eq!(disabled_tray_test, Err(PrintDeviceError::TrayDisabled));
    service
        .update_printer_tray(
            &auth,
            printer.id,
            tray.id,
            UpdatePrinterTrayRequest {
                paper_size: None,
                paper_type: None,
                enabled: Some(true),
            },
            now,
            "h9d-tray-enable",
        )
        .await
        .expect("tray should re-enable");
    let test_print_request = TestPrintRequest { tray_id: tray.id };
    let test_print_mutation = service
        .test_print(
            &auth,
            printer.id,
            test_print_request.clone(),
            now,
            "h9d-test-1",
        )
        .await
        .expect("test print should dispatch");
    assert!(!test_print_mutation.replayed);
    let test_print = test_print_mutation.value;
    assert_eq!(test_print.result, "dispatched");
    let test_print_replay = service
        .test_print(&auth, printer.id, test_print_request, now, "h9d-test-1")
        .await
        .expect("same test print should replay");
    assert!(test_print_replay.replayed);
    assert_eq!(test_print_replay.value.id, test_print.id);
    let stored_result: String =
        sqlx::query_scalar("SELECT result FROM h9_printer_test_prints WHERE id = $1")
            .bind(test_print.id)
            .fetch_one(&pool)
            .await
            .expect("test print row should persist");
    assert_eq!(stored_result, "dispatched");

    // AC1：跨站点引用在数据库层被复合外键拒绝
    let west = service
        .create_site(
            &auth,
            site_request("SITE-WEST", "西区打印站"),
            now,
            "h9d-site-2",
        )
        .await
        .expect("second site should create")
        .value;
    let cross_site_tray = sqlx::query(
        r#"
        INSERT INTO h9_printer_trays (
            id, site_id, printer_id, tray_code, paper_size, paper_type, enabled, created_by, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'TRAY-X', 'A4', '普通纸', TRUE, $4, now(), now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(west.id)
    .bind(printer.id)
    .bind(auth.user_id)
    .execute(&pool)
    .await;
    let cross_error = format!(
        "{:?}",
        cross_site_tray.expect_err("cross-site tray reference must fail")
    );
    assert!(
        cross_error.contains("foreign key"),
        "unexpected: {cross_error}"
    );
    let cross_site_lease =
        insert_lease(&pool, west.id, printer.id, "manual_only", "idle", "active").await;
    let cross_lease_error = format!(
        "{:?}",
        cross_site_lease.expect_err("cross-site lease reference must fail")
    );
    assert!(
        cross_lease_error.contains("foreign key"),
        "unexpected: {cross_lease_error}"
    );

    for (action, expected) in [
        ("disable_print_site_owner_mapping", 1i64),
        ("create_printer_tray", 1),
        ("update_printer_tray", 2),
        ("test_print_printer", 1),
    ] {
        let audit_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H9' AND action = $2",
        )
        .bind(owner_id)
        .bind(action)
        .fetch_one(&pool)
        .await
        .expect("print device audit count should load");
        assert_eq!(audit_count, expected, "audit missing for {action}");
    }
    let idempotency_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM idempotency_request
         WHERE owner_id = $1
           AND idempotency_key = ANY($2)
        "#,
    )
    .bind(owner_id)
    .bind([
        "h9d-map-disable",
        "h9d-tray-1",
        "h9d-tray-update",
        "h9d-test-1",
    ])
    .fetch_one(&pool)
    .await
    .expect("print device idempotency rows should load");
    assert_eq!(idempotency_count, 4);
}

#[sqlx::test(migrations = "../../migrations")]
async fn lease_uniqueness_and_release_mode_snapshot(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id, "H9D02").await;
    let auth = ctx(owner_id, &["h9.print_device.write"]);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 27, 10, 0, 0)
        .single()
        .expect("valid test time");
    let service = PrintDeviceService::with_postgres(pool.clone());
    let site = service
        .create_site(
            &auth,
            site_request("SITE-LEASE", "租约测试站"),
            now,
            "h9d-lease-site",
        )
        .await
        .expect("site should create")
        .value;
    let printer = service
        .create_printer(
            &auth,
            printer_request(site.id, "租约网络打印机", "network"),
            now,
            "h9d-lease-printer",
        )
        .await
        .expect("printer should create")
        .value;

    // AC6：全局默认仅人工释放；打印机可单机覆盖
    assert_eq!(
        resolve_lease_release_mode(&pool, printer.id)
            .await
            .expect("release mode should resolve"),
        "manual_only"
    );
    let updated = service
        .update_printer(
            &auth,
            printer.id,
            UpdatePrinterRequest {
                status: None,
                release_mode_override: Some("safe_auto".to_string()),
            },
            now,
            "h9d-lease-override",
        )
        .await
        .expect("override should update")
        .value;
    assert_eq!(updated.release_mode_override.as_deref(), Some("safe_auto"));
    assert_eq!(updated.effective_release_mode, "safe_auto");
    assert_eq!(
        resolve_lease_release_mode(&pool, printer.id)
            .await
            .expect("release mode should resolve"),
        "safe_auto"
    );

    // AC5：同一打印机同一时点仅一个活动租约（部分唯一索引）
    let lease_id = insert_lease(&pool, site.id, printer.id, "manual_only", "idle", "active")
        .await
        .expect("first active lease should insert");
    let second_active =
        insert_lease(&pool, site.id, printer.id, "manual_only", "idle", "active").await;
    let unique_error = format!(
        "{:?}",
        second_active.expect_err("second active lease must be rejected")
    );
    assert!(
        unique_error.contains("h9_device_leases_one_active_uidx"),
        "unexpected: {unique_error}"
    );
    insert_lease(&pool, site.id, printer.id, "safe_auto", "idle", "released")
        .await
        .expect("released lease rows are not limited");

    // AC6：运行中的租约使用配置快照，后续覆盖变更不回写
    let clear = service
        .update_printer(
            &auth,
            printer.id,
            UpdatePrinterRequest {
                status: None,
                release_mode_override: Some("inherit".to_string()),
            },
            now,
            "h9d-lease-clear",
        )
        .await
        .expect("override should clear")
        .value;
    assert_eq!(clear.release_mode_override, None);
    assert_eq!(clear.effective_release_mode, "manual_only");
    let frozen_mode: String =
        sqlx::query_scalar("SELECT release_mode FROM h9_device_leases WHERE id = $1")
            .bind(lease_id)
            .fetch_one(&pool)
            .await
            .expect("lease snapshot should load");
    assert_eq!(frozen_mode, "manual_only");
}

#[sqlx::test(migrations = "../../migrations")]
async fn manual_release_enforces_permission_reason_confirm_and_hard_safety(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id, "H9D03").await;
    let writer = ctx(owner_id, &["h9.print_device.write"]);
    let releaser = ctx(
        owner_id,
        &["h9.print_device.write", "h9.device_lease.release"],
    );
    let now = Utc
        .with_ymd_and_hms(2026, 7, 27, 11, 0, 0)
        .single()
        .expect("valid test time");
    let service = PrintDeviceService::with_postgres(pool.clone());
    let site = service
        .create_site(
            &writer,
            site_request("SITE-REL", "释放测试站"),
            now,
            "h9d-rel-site",
        )
        .await
        .expect("site should create")
        .value;
    service
        .create_site_owner_mapping(
            &writer,
            site.id,
            CreateSiteOwnerMappingRequest {
                owner_id,
                warehouse_id: Uuid::new_v4(),
            },
            now,
            "h9d-rel-mapping",
        )
        .await
        .expect("release test site should map to its owner");
    let printer = service
        .create_printer(
            &writer,
            printer_request(site.id, "释放测试打印机", "network"),
            now,
            "h9d-rel-printer",
        )
        .await
        .expect("printer should create")
        .value;
    let lease_id = insert_lease(&pool, site.id, printer.id, "manual_only", "idle", "active")
        .await
        .expect("lease should insert");
    let release_request = ReleaseDeviceLeaseRequest {
        reason: "打印机迁移到新站点，人工回收租约".to_string(),
        confirm: true,
    };

    // AC7：专用权限
    let no_permission = service
        .release_lease(
            &writer,
            lease_id,
            release_request.clone(),
            now,
            "h9d-rel-noperm",
        )
        .await;
    assert_eq!(
        no_permission,
        Err(PrintDeviceError::ReleasePermissionRequired)
    );

    // AC7：二次确认与原因必填
    let unconfirmed = service
        .release_lease(
            &releaser,
            lease_id,
            ReleaseDeviceLeaseRequest {
                reason: "未确认".to_string(),
                confirm: false,
            },
            now,
            "h9d-rel-unconfirmed",
        )
        .await;
    assert_eq!(unconfirmed, Err(PrintDeviceError::ConfirmationRequired));
    let no_reason = service
        .release_lease(
            &releaser,
            lease_id,
            ReleaseDeviceLeaseRequest {
                reason: "   ".to_string(),
                confirm: true,
            },
            now,
            "h9d-rel-noreason",
        )
        .await;
    assert_eq!(no_reason, Err(PrintDeviceError::InvalidRequest));

    // AC7：printing / result_unknown / reconciling 是任何人不可覆盖的硬安全条件
    for busy_state in ["printing", "result_unknown", "reconciling"] {
        sqlx::query("UPDATE h9_device_leases SET busy_state = $2 WHERE id = $1")
            .bind(lease_id)
            .bind(busy_state)
            .execute(&pool)
            .await
            .expect("busy state should seed");
        let blocked = service
            .release_lease(
                &releaser,
                lease_id,
                release_request.clone(),
                now,
                &format!("h9d-rel-busy-{busy_state}"),
            )
            .await;
        assert_eq!(
            blocked,
            Err(PrintDeviceError::LeaseBusy(busy_state.to_string())),
            "busy state {busy_state} must block release"
        );
    }

    // 空闲后授权人工释放成功，重复请求幂等重放
    sqlx::query("UPDATE h9_device_leases SET busy_state = 'idle' WHERE id = $1")
        .bind(lease_id)
        .execute(&pool)
        .await
        .expect("busy state should reset");
    let released = service
        .release_lease(
            &releaser,
            lease_id,
            release_request.clone(),
            now,
            "h9d-rel-ok",
        )
        .await
        .expect("idle manual_only lease should release with dedicated permission");
    assert!(!released.replayed);
    assert_eq!(released.value.status, "released");
    assert_eq!(
        released.value.release_reason.as_deref(),
        Some("打印机迁移到新站点，人工回收租约")
    );
    let replayed = service
        .release_lease(
            &releaser,
            lease_id,
            release_request.clone(),
            now,
            "h9d-rel-ok",
        )
        .await
        .expect("same idempotency key should replay");
    assert!(replayed.replayed);
    assert_eq!(replayed.value.id, released.value.id);
    let already = service
        .release_lease(&releaser, lease_id, release_request, now, "h9d-rel-again")
        .await;
    assert_eq!(already, Err(PrintDeviceError::LeaseAlreadyReleased));

    // 全动作 H2 审计
    for (action, expected) in [
        ("create_print_site", 1i64),
        ("create_printer", 1),
        ("release_device_lease", 1),
    ] {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H9' AND action = $2",
        )
        .bind(owner_id)
        .bind(action)
        .fetch_one(&pool)
        .await
        .expect("audit count should load");
        assert_eq!(count, expected, "audit missing for {action}");
    }
}

include!("h9_print_device_postgres/part2.rs");
