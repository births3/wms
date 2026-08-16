use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    lpn_container_repository::{LpnContainerRepositoryError, PgLpnContainerRepository},
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{CreateLpnContainerRequest, UpdateLpnContainerRequest, LPN_CONTAINER_TYPE_PALLET};

#[path = "support/lpn_container.rs"]
mod lpn_support;
mod postgres_test_support;
use lpn_support::{
    at, batch_container_lpn, batch_qty, create_req, ctx, insert_owner, lpn_product_codes,
    lpn_status, putaway_count, putaway_req, seed_lpn_numbering, seed_putaway,
};
use postgres_test_support::ensure_audit_partition;

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
    let own = repo
        .list(&actor, None, None, None)
        .await
        .expect("owner scoped list");
    assert_eq!(own.len(), 2);
    assert!(own.iter().all(|row| row.owner_id == owner_id));
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

    let foreign = lpn_repo
        .create(&other, create_req(), at(8), "lpn-other-owner")
        .await
        .expect("other owner lpn");
    let isolated = wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            putaway_req(&fixture, &foreign.lpn_code),
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
        ("LPN-REC-01", "recycling", "lpn-putaway-recycling"),
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
async fn putaway_mix_sku_follows_type_policy(pool: PgPool) {
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
    let wave3 = PgWave3Repository::new(pool.clone());
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
            mixed.clone(),
            Utc::now(),
            "lpn-mix-second",
            None,
        )
        .await
        .expect_err("default policy denies mix sku");
    assert_eq!(denied, Wave3RepositoryError::LpnMixDenied);
    assert_eq!(
        lpn_product_codes(&pool, fixture.owner_id, &created.lpn_code).await,
        vec!["LPN-P-001".to_string()]
    );
    assert_eq!(
        putaway_count(&pool, fixture.owner_id, fixture.order_id).await,
        1
    );

    lpn_repo
        .upsert_type_policy(
            &actor,
            wms_domain::UpsertLpnContainerTypePolicyRequest {
                container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
                allow_mix_batch: false,
                allow_mix_sku: true,
            },
        )
        .await
        .expect("open mix sku only");
    let still_denied = wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            mixed.clone(),
            Utc::now(),
            "lpn-mix-sku-only",
            None,
        )
        .await
        .expect_err("mix sku without mix batch still denies other batch");
    assert_eq!(still_denied, Wave3RepositoryError::LpnMixDenied);
    assert_eq!(
        lpn_product_codes(&pool, fixture.owner_id, &created.lpn_code).await,
        vec!["LPN-P-001".to_string()]
    );
    assert_eq!(
        putaway_count(&pool, fixture.owner_id, fixture.order_id).await,
        1
    );

    lpn_repo
        .upsert_type_policy(
            &actor,
            wms_domain::UpsertLpnContainerTypePolicyRequest {
                container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
                allow_mix_batch: true,
                allow_mix_sku: true,
            },
        )
        .await
        .expect("open mix sku and batch");
    wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            mixed,
            Utc::now(),
            "lpn-mix-allowed",
            None,
        )
        .await
        .expect("policy on allows mix sku");
    assert_eq!(
        lpn_product_codes(&pool, fixture.owner_id, &created.lpn_code).await,
        vec!["LPN-P-001".to_string(), "LPN-P-002".to_string()]
    );
    assert_eq!(
        putaway_count(&pool, fixture.owner_id, fixture.order_id).await,
        2
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_replays_same_idempotency_key_without_second_row(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = ctx(owner_id);

    let first = repo
        .create(&actor, create_req(), at(9), "lpn-create-replay")
        .await
        .expect("first create should persist");
    let replay = repo
        .create(&actor, create_req(), at(10), "lpn-create-replay")
        .await
        .expect("same idempotency key should replay");
    assert_eq!(replay.id, first.id);
    assert_eq!(replay.lpn_code, first.lpn_code);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lpn_containers WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("lpn row count");
    assert_eq!(count, 1);

    let idempotency_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'lpn-create-replay'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("idempotency row count");
    assert_eq!(idempotency_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_same_sku_batch_putaway_adds_qty(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let fixture = seed_putaway(&pool).await;
    seed_lpn_numbering(&pool, at(0), fixture.owner_id).await;
    let actor = ctx(fixture.owner_id);
    let lpn_repo = PgLpnContainerRepository::new(pool.clone());
    let wave3 = PgWave3Repository::new(pool.clone());
    let created = lpn_repo
        .create(&actor, create_req(), at(9), "lpn-concurrent-create")
        .await
        .expect("lpn master");

    let left_wave3 = wave3.clone();
    let right_wave3 = wave3;
    let left_actor = actor.clone();
    let right_actor = actor;
    let left_req = putaway_req(&fixture, &created.lpn_code);
    let right_req = putaway_req(&fixture, &created.lpn_code);
    let order_id = fixture.order_id;
    let (left, right) = tokio::join!(
        left_wave3.putaway_receiving_order_and_inventory_with_audit(
            &left_actor,
            order_id,
            left_req,
            Utc::now(),
            "lpn-concurrent-putaway-1",
            None,
        ),
        right_wave3.putaway_receiving_order_and_inventory_with_audit(
            &right_actor,
            order_id,
            right_req,
            Utc::now(),
            "lpn-concurrent-putaway-2",
            None,
        ),
    );
    let left = left.expect("first concurrent same-sku putaway should succeed");
    let right = right.expect("second concurrent same-sku putaway should succeed");
    assert!(!left.replayed, "distinct keys must not replay");
    assert!(!right.replayed, "distinct keys must not replay");

    let (qty_on_hand, bound, batch_count): (wms_domain::Quantity, Option<String>, i64) =
        sqlx::query_as(
            "SELECT qty_on_hand, container_lpn, COUNT(*) OVER () FROM inventory_batches WHERE owner_id = $1 AND product_code = 'LPN-P-001' AND batch_no = 'LPN-B-001'",
        )
        .bind(fixture.owner_id)
        .fetch_one(&pool)
        .await
        .expect("batch after concurrent putaway");
    assert_eq!(qty_on_hand, 4.into());
    assert_eq!(bound.as_deref(), Some(created.lpn_code.as_str()));
    assert_eq!(batch_count, 1);
    let (status, location_id, putaway_count): (String, Option<Uuid>, i64) = sqlx::query_as(
        "SELECT lpn.status, lpn.location_id, (SELECT COUNT(*) FROM receiving_putaways WHERE owner_id = $1 AND receiving_order_id = $3) FROM lpn_containers lpn WHERE lpn.owner_id = $1 AND lpn.lpn_code = $2",
    )
    .bind(fixture.owner_id)
    .bind(&created.lpn_code)
    .bind(fixture.order_id)
    .fetch_one(&pool)
    .await
    .expect("lpn bind state");
    assert_eq!(status, "in_use");
    assert_eq!(location_id, Some(fixture.location_id));
    assert_eq!(putaway_count, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_fails_when_numbering_rule_missing(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let actor = ctx(Uuid::new_v4());
    insert_owner(&pool, actor.owner_id).await;
    sqlx::query(
        "UPDATE document_number_rules SET enabled = FALSE WHERE document_type LIKE 'lpn_%'",
    )
    .execute(&pool)
    .await
    .expect("disable lpn rules");
    let repo = PgLpnContainerRepository::new(pool.clone());
    assert!(matches!(
        repo.create(&actor, create_req(), at(9), "lpn-no-rule")
            .await,
        Err(LpnContainerRepositoryError::NumberingUnavailable)
    ));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lpn_containers WHERE owner_id = $1")
        .bind(actor.owner_id)
        .fetch_one(&pool)
        .await
        .expect("no lpn row");
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_rejects_second_lpn_on_same_sku_batch_location(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let fixture = seed_putaway(&pool).await;
    seed_lpn_numbering(&pool, at(0), fixture.owner_id).await;
    let actor = ctx(fixture.owner_id);
    let lpn_repo = PgLpnContainerRepository::new(pool.clone());
    let wave3 = PgWave3Repository::new(pool.clone());
    let first = lpn_repo
        .create(&actor, create_req(), at(9), "lpn-identity-a")
        .await
        .expect("first lpn");
    let second = lpn_repo
        .create(&actor, create_req(), at(10), "lpn-identity-b")
        .await
        .expect("second lpn");
    wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            putaway_req(&fixture, &first.lpn_code),
            Utc::now(),
            "lpn-identity-first",
            None,
        )
        .await
        .expect("first lpn putaway");
    let denied = wave3
        .putaway_receiving_order_and_inventory_with_audit(
            &actor,
            fixture.order_id,
            putaway_req(&fixture, &second.lpn_code),
            Utc::now(),
            "lpn-identity-second",
            None,
        )
        .await
        .expect_err("second lpn must not overwrite first");
    assert_eq!(denied, Wave3RepositoryError::LpnNotUsable);
    assert_eq!(
        batch_container_lpn(&pool, fixture.owner_id)
            .await
            .as_deref(),
        Some(first.lpn_code.as_str())
    );
    assert_eq!(batch_qty(&pool, fixture.owner_id).await, 2.into());
    assert_eq!(
        lpn_status(&pool, fixture.owner_id, &first.lpn_code).await,
        "in_use"
    );
    assert_eq!(
        lpn_status(&pool, fixture.owner_id, &second.lpn_code).await,
        "idle"
    );
    assert_eq!(
        putaway_count(&pool, fixture.owner_id, fixture.order_id).await,
        1
    );
}
