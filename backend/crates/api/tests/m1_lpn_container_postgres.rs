use chrono::{DateTime, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    inventory::STATUS_QUALIFIED,
    lpn_container_repository::{LpnContainerRepositoryError, PgLpnContainerRepository},
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{
    CreateLpnContainerRequest, PutawayRequest, UpdateLpnContainerRequest, LPN_CONTAINER_TYPE_PALLET,
};

mod postgres_test_support;
use postgres_test_support::ensure_audit_partition;

async fn insert_owner(pool: &PgPool, owner_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'LPN test owner') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("LPN{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("auth owner");
}

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "lpn-test".to_string(),
        permissions: vec![
            "m1.master_data.write".to_string(),
            "m2.putaway.write".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 15, hour, 0, 0)
        .single()
        .expect("test timestamp should be valid")
}

async fn seed_lpn_numbering(pool: &PgPool, now: DateTime<Utc>, owner_id: Uuid) {
    insert_owner(pool, owner_id).await;
    for (item_code, item_name, rule_code, template) in [
        (
            "lpn_pallet",
            "容器LPN-托盘",
            "TEST-LPN-PALLET",
            "LPN-PL-{OWNER}-{YYYY}{MM}{DD}-{SEQ}",
        ),
        (
            "lpn_tote",
            "容器LPN-周转箱",
            "TEST-LPN-TOTE",
            "LPN-TT-{OWNER}-{YYYY}{MM}{DD}-{SEQ}",
        ),
        (
            "lpn_outbound_box",
            "容器LPN-出库箱",
            "TEST-LPN-OUT",
            "LPN-OB-{OWNER}-{YYYY}{MM}{DD}-{SEQ}",
        ),
        (
            "lpn_insulated_box",
            "容器LPN-保温箱",
            "TEST-LPN-INS",
            "LPN-IB-{OWNER}-{YYYY}{MM}{DD}-{SEQ}",
        ),
        (
            "lpn_blind_label",
            "容器LPN-盲标签",
            "TEST-LPN-BLD",
            "LPN-BL-{OWNER}-{YYYY}{MM}{DD}-{SEQ}",
        ),
    ] {
        sqlx::query(
            "INSERT INTO system_dictionary_items (id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at) VALUES ($1, 'document_type', $2, $3, TRUE, NULL, '{\"direction\":\"internal\",\"workflow_template\":\"lpn_container\",\"batch_policy\":\"none\"}'::jsonb, 'global', $4, $4) ON CONFLICT DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(item_code)
        .bind(item_name)
        .bind(now)
        .execute(pool)
        .await
        .expect("lpn document type");
        sqlx::query(
            "INSERT INTO document_number_rules (id, owner_id, document_type, rule_code, rule_name, template, reset_policy, sequence_width, sequence_mode, enabled, created_at, updated_at) VALUES ($1, NULL, $2, $3, $4, $5, 'daily', 4, 'no_gap', TRUE, $6, $6) ON CONFLICT DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(item_code)
        .bind(rule_code)
        .bind(item_name)
        .bind(template)
        .bind(now)
        .execute(pool)
        .await
        .expect("lpn numbering rule");
    }
}

fn create_req() -> CreateLpnContainerRequest {
    CreateLpnContainerRequest {
        container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
        capacity_cm3: Some(8000),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn lpn_container_create_list_and_duplicate(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let other_owner = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    insert_owner(&pool, other_owner).await;
    let repo = PgLpnContainerRepository::new(pool);
    let actor = ctx(owner_id);

    let created = repo
        .create(&actor, create_req(), at(9), "lpn-create-1")
        .await
        .expect("lpn should create");
    assert!(created.lpn_code.starts_with("LPN-PL-"));
    assert_eq!(created.status, "idle");
    assert_eq!(created.container_type, LPN_CONTAINER_TYPE_PALLET);

    let listed = repo
        .list(
            &actor,
            Some(&created.lpn_code),
            Some("pallet"),
            Some("idle"),
        )
        .await
        .expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);

    let second = repo
        .create(&actor, create_req(), at(10), "lpn-create-2")
        .await
        .expect("second generated lpn");
    assert_ne!(second.lpn_code, created.lpn_code);

    repo.create(
        &ctx(other_owner),
        create_req(),
        at(11),
        "lpn-create-other-owner",
    )
    .await
    .expect("other owner can also allocate");
}

struct PutawayFixture {
    owner_id: Uuid,
    order_id: Uuid,
    location_id: Uuid,
    location_code: String,
}

async fn seed_putaway(pool: &PgPool) -> PutawayFixture {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'LPN WH', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("LPN-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("warehouse");
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, 'Z1', 'zone', 'normal', 'qualified_green', 'active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("zone");
    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status) VALUES ($1, $2, $3, $4, 'LOC-1', 1, 1, 1, 1000, 0, 3, 'storage', 'available')",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(pool)
    .await
    .expect("location");
    sqlx::query(
        "INSERT INTO products (id, owner_id, erp_goods_id, product_code, product_name, specification, storage_condition, volume_cm3, status) VALUES ($1, $2, 2001, 'LPN-P-001', 'p', '1', 'normal', 10, 'active')",
    )
    .bind(product_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("product");
    sqlx::query(
        "INSERT INTO receiving_orders (id, owner_id, receipt_no, document_type, warehouse_id, erp_bill_id, erp_bill_code, erp_revision, erp_line_no, erp_correlation_id, status, expected_arrival_at) VALUES ($1, $2, 'LPN-ASN-001', 'purchase_inbound', $3, 9002, 'ERP-LPN-001', 1, 1, 'corr-lpn-001', 'putaway', $4)",
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("order");
    sqlx::query(
        "INSERT INTO receiving_order_lines (id, receiving_order_id, owner_id, line_no, product_id, product_code, expected_qty, batch_no, production_date, expiry_date) VALUES ($1, $2, $3, 1, $4, 'LPN-P-001', 10, 'LPN-B-001', '2026-01-01', '2028-01-01')",
    )
    .bind(Uuid::new_v4())
    .bind(order_id)
    .bind(owner_id)
    .bind(product_id)
    .execute(pool)
    .await
    .expect("line");
    sqlx::query(
        "INSERT INTO receiving_inspections (id, receiving_order_id, owner_id, batch_no, accepted_qty, rejected_qty, production_date, expiry_date, quality_status, occurred_at) VALUES ($1, $2, $3, 'LPN-B-001', 10, 0, '2026-01-01', '2028-01-01', 'qualified', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(order_id)
    .bind(owner_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("inspection");

    PutawayFixture {
        owner_id,
        order_id,
        location_id,
        location_code: "LOC-1".to_string(),
    }
}

fn putaway_req(fixture: &PutawayFixture, lpn: &str) -> PutawayRequest {
    PutawayRequest {
        batch_no: "LPN-B-001".to_string(),
        product_code: "LPN-P-001".to_string(),
        qty: 2.into(),
        location_id: fixture.location_id,
        location_code: fixture.location_code.clone(),
        quality_status: STATUS_QUALIFIED.to_string(),
        lpn_code: Some(lpn.to_string()),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_unknown_lpn_fails_and_existing_lpn_succeeds(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let fixture = seed_putaway(&pool).await;
    seed_lpn_numbering(&pool, at(0), fixture.owner_id).await;
    let actor = ctx(fixture.owner_id);
    let lpn_repo = PgLpnContainerRepository::new(pool.clone());
    let wave3 = PgWave3Repository::new(pool.clone());

    let missing = wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            putaway_req(&fixture, "LPN-MISSING"),
            Utc::now(),
            "lpn-putaway-missing",
            None,
        )
        .await
        .expect_err("unknown lpn must fail");
    assert_eq!(missing, Wave3RepositoryError::LpnNotFound);

    let created = lpn_repo
        .create(&actor, create_req(), at(9), "lpn-ok-create")
        .await
        .expect("lpn master");

    let audit = AuditWriteRequest::from_auth_context(
        &actor,
        "putaway",
        "M2",
        "receiving_order",
        fixture.order_id.to_string(),
        None,
    );
    wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            putaway_req(&fixture, &created.lpn_code),
            Utc::now(),
            "lpn-putaway-ok",
            Some(audit),
        )
        .await
        .expect("existing lpn should putaway");

    let bound: Option<String> = sqlx::query_scalar(
        "SELECT container_lpn FROM inventory_batches WHERE owner_id = $1 AND product_code = 'LPN-P-001'",
    )
    .bind(fixture.owner_id)
    .fetch_one(&pool)
    .await
    .expect("batch lpn");
    assert_eq!(bound.as_deref(), Some(created.lpn_code.as_str()));

    let status: String = sqlx::query_scalar(
        "SELECT status FROM lpn_containers WHERE owner_id = $1 AND lpn_code = $2",
    )
    .bind(fixture.owner_id)
    .bind(&created.lpn_code)
    .fetch_one(&pool)
    .await
    .expect("lpn status");
    assert_eq!(status, "in_use");
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_rejects_invalid_type(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let actor = ctx(Uuid::new_v4());
    seed_lpn_numbering(&pool, at(0), actor.owner_id).await;
    let repo = PgLpnContainerRepository::new(pool);
    assert!(matches!(
        repo.create(
            &actor,
            CreateLpnContainerRequest {
                container_type: "nest".to_string(),
                capacity_cm3: None,
            },
            at(9),
            "lpn-bad-type",
        )
        .await,
        Err(LpnContainerRepositoryError::TypeInvalid)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_rejects_invalid_status(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let actor = ctx(Uuid::new_v4());
    seed_lpn_numbering(&pool, at(0), actor.owner_id).await;
    let repo = PgLpnContainerRepository::new(pool);
    let created = repo
        .create(&actor, create_req(), at(9), "lpn-status-create")
        .await
        .expect("create");
    assert!(matches!(
        repo.update(
            &actor,
            created.id,
            UpdateLpnContainerRequest {
                status: Some("broken".to_string()),
                location_id: None,
                capacity_cm3: None,
            },
            at(10),
            "lpn-status-bad",
        )
        .await,
        Err(LpnContainerRepositoryError::StatusInvalid)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_rejects_unusable_status_cross_location_and_other_owner(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let fixture = seed_putaway(&pool).await;
    seed_lpn_numbering(&pool, at(0), fixture.owner_id).await;
    let actor = ctx(fixture.owner_id);
    let other = ctx(Uuid::new_v4());
    insert_owner(&pool, other.owner_id).await;
    let lpn_repo = PgLpnContainerRepository::new(pool.clone());
    let wave3 = PgWave3Repository::new(pool.clone());

    lpn_repo
        .create(&other, create_req(), at(8), "lpn-other-owner")
        .await
        .expect("other owner lpn");
    let isolated = wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            putaway_req(&fixture, "LPN-OWN-01"),
            Utc::now(),
            "lpn-putaway-owner",
            None,
        )
        .await
        .expect_err("other owner lpn must not bind");
    assert_eq!(isolated, Wave3RepositoryError::LpnNotFound);

    for (_code, status, key) in [
        ("LPN-SHIP-01", "shipped", "lpn-putaway-shipped"),
        ("LPN-TRN-01", "in_transit", "lpn-putaway-transit"),
    ] {
        let created = lpn_repo
            .create(&actor, create_req(), at(9), &format!("{key}-c"))
            .await
            .expect("lpn");
        lpn_repo
            .update(
                &actor,
                created.id,
                UpdateLpnContainerRequest {
                    status: Some(status.to_string()),
                    location_id: None,
                    capacity_cm3: None,
                },
                at(10),
                &format!("{key}-u"),
            )
            .await
            .expect("set status");
        let err = wave3
            .putaway_receiving_order_and_inventory_with_audit(
                &actor,
                fixture.order_id,
                putaway_req(&fixture, &created.lpn_code),
                Utc::now(),
                key,
                None,
            )
            .await
            .expect_err("unusable lpn");
        assert_eq!(err, Wave3RepositoryError::LpnNotUsable);
    }

    let loc2 = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status)
         SELECT $1, owner_id, warehouse_id, zone_id, 'LOC-2', 1, 2, 1, 1000, 0, 3, 'storage', 'available'
           FROM warehouse_locations WHERE id = $2",
    )
    .bind(loc2)
    .bind(fixture.location_id)
    .execute(&pool)
    .await
    .expect("loc2");

    let xloc = lpn_repo
        .create(&actor, create_req(), at(11), "lpn-xloc-c")
        .await
        .expect("lpn");
    wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            putaway_req(&fixture, &xloc.lpn_code),
            Utc::now(),
            "lpn-xloc-first",
            None,
        )
        .await
        .expect("first putaway");

    let mut second = putaway_req(&fixture, &xloc.lpn_code);
    second.location_id = loc2;
    second.location_code = "LOC-2".to_string();
    let cross = wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            second,
            Utc::now(),
            "lpn-xloc-second",
            None,
        )
        .await
        .expect_err("cross location");
    assert_eq!(cross, Wave3RepositoryError::LpnNotUsable);

    let bound_loc: Option<Uuid> = sqlx::query_scalar(
        "SELECT location_id FROM lpn_containers WHERE owner_id = $1 AND lpn_code = $2",
    )
    .bind(fixture.owner_id)
    .bind(&xloc.lpn_code)
    .fetch_one(&pool)
    .await
    .expect("bound loc");
    assert_eq!(bound_loc, Some(fixture.location_id));
}

#[sqlx::test(migrations = "../../migrations")]
async fn upsert_type_policy_writes_audit(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let actor = ctx(Uuid::new_v4());
    seed_lpn_numbering(&pool, at(0), actor.owner_id).await;
    let repo = PgLpnContainerRepository::new(pool.clone());
    let saved = repo
        .upsert_type_policy(
            &actor,
            wms_domain::UpsertLpnContainerTypePolicyRequest {
                container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
                allow_mix_batch: true,
                allow_mix_sku: false,
            },
        )
        .await
        .expect("upsert policy");
    assert!(saved.allow_mix_batch);
    let action: String = sqlx::query_scalar(
        "SELECT action FROM audit_event WHERE owner_id = $1 AND action = 'upsert_lpn_type_policy' LIMIT 1",
    )
    .bind(actor.owner_id)
    .fetch_one(&pool)
    .await
    .expect("policy audit");
    assert_eq!(action, "upsert_lpn_type_policy");
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_denies_mixed_sku_when_policy_off(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let fixture = seed_putaway(&pool).await;
    seed_lpn_numbering(&pool, at(0), fixture.owner_id).await;
    let actor = ctx(fixture.owner_id);
    let product_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO products (id, owner_id, erp_goods_id, product_code, product_name, specification, storage_condition, volume_cm3, status) VALUES ($1, $2, 2002, 'LPN-P-002', 'p2', '1', 'normal', 10, 'active')",
    )
    .bind(product_id)
    .bind(fixture.owner_id)
    .execute(&pool)
    .await
    .expect("second product");
    sqlx::query(
        "INSERT INTO receiving_order_lines (id, receiving_order_id, owner_id, line_no, product_id, product_code, expected_qty, batch_no, production_date, expiry_date) VALUES ($1, $2, $3, 2, $4, 'LPN-P-002', 10, 'LPN-B-002', '2026-01-01', '2028-01-01')",
    )
    .bind(Uuid::new_v4())
    .bind(fixture.order_id)
    .bind(fixture.owner_id)
    .bind(product_id)
    .execute(&pool)
    .await
    .expect("second line");
    sqlx::query(
        "INSERT INTO receiving_inspections (id, receiving_order_id, owner_id, batch_no, accepted_qty, rejected_qty, production_date, expiry_date, quality_status, occurred_at) VALUES ($1, $2, $3, 'LPN-B-002', 10, 0, '2026-01-01', '2028-01-01', 'qualified', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(fixture.order_id)
    .bind(fixture.owner_id)
    .bind(at(0))
    .execute(&pool)
    .await
    .expect("second inspection");

    let lpn_repo = PgLpnContainerRepository::new(pool.clone());
    let wave3 = PgWave3Repository::new(pool);
    let created = lpn_repo
        .create(&actor, create_req(), at(9), "lpn-mix-create")
        .await
        .expect("lpn");
    wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            putaway_req(&fixture, &created.lpn_code),
            Utc::now(),
            "lpn-mix-first",
            None,
        )
        .await
        .expect("first sku");
    let mut mixed = putaway_req(&fixture, &created.lpn_code);
    mixed.product_code = "LPN-P-002".to_string();
    mixed.batch_no = "LPN-B-002".to_string();
    let denied = wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            mixed,
            Utc::now(),
            "lpn-mix-second",
            None,
        )
        .await
        .expect_err("default policy denies mix sku");
    assert_eq!(denied, Wave3RepositoryError::LpnMixDenied);
}
