#[sqlx::test(migrations = "../../migrations")]
async fn site_resources_reject_cross_owner_reads_and_mutations(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id, "H9D04").await;
    seed_owner(&pool, other_owner_id, "H9D05").await;
    let auth = ctx(owner_id, &["h9.print_device.read", "h9.print_device.write"]);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 27, 12, 0, 0)
        .single()
        .expect("valid test time");
    let service = PrintDeviceService::with_postgres(pool.clone());
    let foreign_site_id = Uuid::new_v4();
    let foreign_mapping_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO h9_print_sites (id, site_code, site_name, created_by) VALUES ($1, 'SITE-FOREIGN', '其他货主打印站', $2)",
    )
    .bind(foreign_site_id)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("foreign site should insert");
    sqlx::query(
        r#"
        INSERT INTO h9_print_site_owner_mappings (
            id, site_id, owner_id, warehouse_id, status, created_by
        )
        VALUES ($1, $2, $3, $4, 'active', $5)
        "#,
    )
    .bind(foreign_mapping_id)
    .bind(foreign_site_id)
    .bind(other_owner_id)
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("foreign owner mapping should insert");

    let sites = service
        .list_sites(&auth)
        .await
        .expect("authorized site list should load");
    assert!(
        sites.data.iter().all(|site| site.id != foreign_site_id),
        "another owner's mapped site must not be disclosed"
    );
    assert!(
        service
            .list_site_owner_mappings(&auth, foreign_site_id)
            .await
            .is_err(),
        "another owner's mapping details must not be disclosed"
    );
    assert!(
        service
            .create_printer(
                &auth,
                printer_request(foreign_site_id, "越权打印机", "network"),
                now,
                "h9d-cross-owner-printer",
            )
            .await
            .is_err(),
        "another owner's site must reject device mutation"
    );

    let own_site = service
        .create_site(
            &auth,
            site_request("SITE-OWN", "当前货主待映射站"),
            now,
            "h9d-own-site",
        )
        .await
        .expect("unmapped site should create")
        .value;
    assert!(
        service
            .create_site_owner_mapping(
                &auth,
                own_site.id,
                CreateSiteOwnerMappingRequest {
                    owner_id: other_owner_id,
                    warehouse_id: Uuid::new_v4(),
                },
                now,
                "h9d-cross-owner-map",
            )
            .await
            .is_err(),
        "request.owner_id must not grant cross-owner mapping authority"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn shared_site_requires_owner_union_and_audits_each_owner(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id, "H9D06").await;
    seed_owner(&pool, other_owner_id, "H9D07").await;
    let auth = ctx(owner_id, &["h9.print_device.read", "h9.print_device.write"]);
    grant_owner_permissions(
        &pool,
        auth.user_id,
        other_owner_id,
        &["h9.print_device.read", "h9.print_device.write"],
    )
    .await;
    let now = Utc
        .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
        .single()
        .expect("valid test time");
    let service = PrintDeviceService::with_postgres(pool.clone());
    let site = service
        .create_site(
            &auth,
            site_request("SITE-SHARED", "多货主共享打印站"),
            now,
            "h9d-shared-site",
        )
        .await
        .expect("shared site should create")
        .value;
    for (mapping_owner_id, key) in [
        (owner_id, "h9d-shared-map-current"),
        (other_owner_id, "h9d-shared-map-other"),
    ] {
        service
            .create_site_owner_mapping(
                &auth,
                site.id,
                CreateSiteOwnerMappingRequest {
                    owner_id: mapping_owner_id,
                    warehouse_id: Uuid::new_v4(),
                },
                now,
                key,
            )
            .await
            .expect("actor authorized across the full owner union");
    }
    let printer = service
        .create_printer(
            &auth,
            printer_request(site.id, "共享站点打印机", "network"),
            now,
            "h9d-shared-printer",
        )
        .await
        .expect("owner-union authorized mutation should succeed")
        .value;
    let tray = service
        .create_printer_tray(
            &auth,
            printer.id,
            tray_request("TRAY-SHARED"),
            now,
            "h9d-shared-tray",
        )
        .await
        .expect("shared-site tray should create")
        .value;
    let test_print = service
        .test_print(
            &auth,
            printer.id,
            TestPrintRequest { tray_id: tray.id },
            now,
            "h9d-shared-test-print",
        )
        .await
        .expect("shared-site test print should dispatch")
        .value;
    assert!(service
        .list_sites(&auth)
        .await
        .expect("owner-union authorized read should succeed")
        .data
        .iter()
        .any(|item| item.id == site.id));

    for (action, resource_id) in [
        ("create_printer", printer.id),
        ("create_printer_tray", tray.id),
        ("test_print_printer", test_print.id),
    ] {
        for audit_owner_id in [owner_id, other_owner_id] {
            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*)
                  FROM audit_event
                 WHERE owner_id = $1
                   AND module = 'H9'
                   AND action = $2
                   AND resource_id = $3
                "#,
            )
            .bind(audit_owner_id)
            .bind(action)
            .bind(resource_id.to_string())
            .fetch_one(&pool)
            .await
            .expect("per-owner audit count should load");
            assert_eq!(count, 1, "shared-site action must audit every mapped owner");
        }
    }

    let owner_column_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = 'h9_printer_test_prints'
           AND column_name = 'owner_id'
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("test-print schema should be inspectable");
    assert_eq!(
        owner_column_count, 0,
        "physical-site test-print facts must not be assigned to an arbitrary owner"
    );
}
