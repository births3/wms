use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    inventory::STATUS_QUALIFIED,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{PutawayRecommendationQuery, PutawayRequest};

struct Fixture {
    owner_id: Uuid,
    order_id: Uuid,
    same_product_location_id: Uuid,
    same_product_location_code: String,
}

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m2-smart-putaway-test".to_string(),
        permissions: vec!["m2.putaway.write".to_string()],
        jti: Uuid::new_v4().to_string(),
    }
}

async fn seed_fixture(pool: &PgPool) -> Fixture {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let cold_zone_id = Uuid::new_v4();
    let same_product_location_id = Uuid::new_v4();
    let other_location_id = Uuid::new_v4();
    let full_location_id = Uuid::new_v4();
    let cold_location_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let line_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'M2 test warehouse', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("M2-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("warehouse should seed");
    for (id, code, temperature_zone) in [
        (zone_id, "M2-ZONE-NORMAL", "normal"),
        (cold_zone_id, "M2-ZONE-COLD", "cold"),
    ] {
        sqlx::query(
            "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, $4, 'M2 test zone', $5, 'qualified_green', 'active')",
        )
        .bind(id)
        .bind(owner_id)
        .bind(warehouse_id)
        .bind(code)
        .bind(temperature_zone)
        .execute(pool)
        .await
        .expect("zone should seed");
    }

    let locations = [
        (
            same_product_location_id,
            zone_id,
            "M2-LOC-SAME",
            1_i32,
            10_i64,
            100_i64,
        ),
        (other_location_id, zone_id, "M2-LOC-OTHER", 2, 0, 100),
        (full_location_id, zone_id, "M2-LOC-FULL", 3, 100, 100),
        (cold_location_id, cold_zone_id, "M2-LOC-COLD", 4, 0, 100),
    ];
    for (id, location_zone_id, code, row_no, used_volume_cm3, max_volume_cm3) in locations {
        sqlx::query(
            "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status) VALUES ($1, $2, $3, $4, $5, $6, 1, 1, $7, $8, 3, 'storage', 'available')",
        )
        .bind(id)
        .bind(owner_id)
        .bind(warehouse_id)
        .bind(location_zone_id)
        .bind(code)
        .bind(row_no)
        .bind(max_volume_cm3)
        .bind(used_volume_cm3)
        .execute(pool)
        .await
        .expect("location should seed");
    }

    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, attrs, status) VALUES ($1, $2, 'M2-P-001', 'M2 test product', '1 unit', 'normal', '{\"unit_volume_cm3\": 10}', 'active')",
    )
    .bind(product_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("product should seed");
    sqlx::query(
        "INSERT INTO inventory_batches (id, owner_id, product_code, batch_no, production_date, expiry_date, qty_on_hand, qty_locked, quality_status, location_id, location_code) VALUES ($1, $2, 'M2-P-001', 'OLD-BATCH', '2026-01-01', '2028-01-01', 1, 0, 'qualified', $3, 'M2-LOC-SAME')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(same_product_location_id)
    .execute(pool)
    .await
    .expect("existing same-product inventory should seed");

    sqlx::query(
        "INSERT INTO receiving_orders (id, owner_id, receipt_no, document_type, warehouse_id, status, expected_arrival_at) VALUES ($1, $2, 'M2-ASN-001', 'purchase_inbound', $3, 'putaway', $4)",
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("receiving order should seed");
    sqlx::query(
        "INSERT INTO receiving_order_lines (id, receiving_order_id, owner_id, line_no, product_id, product_code, expected_qty, batch_no, production_date, expiry_date) VALUES ($1, $2, $3, 1, $4, 'M2-P-001', 10, 'M2-BATCH-001', '2026-01-01', '2028-01-01')",
    )
    .bind(line_id)
    .bind(order_id)
    .bind(owner_id)
    .bind(product_id)
    .execute(pool)
    .await
    .expect("receiving line should seed");
    sqlx::query(
        "INSERT INTO receiving_inspections (id, receiving_order_id, owner_id, batch_no, accepted_qty, rejected_qty, production_date, expiry_date, quality_status, occurred_at) VALUES ($1, $2, $3, 'M2-BATCH-001', 10, 0, '2026-01-01', '2028-01-01', 'qualified', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(order_id)
    .bind(owner_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("inspection should seed");

    Fixture {
        owner_id,
        order_id,
        same_product_location_id,
        same_product_location_code: "M2-LOC-SAME".to_string(),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn smart_putaway_recommends_and_commits_owner_scoped_inventory_atomically(pool: PgPool) {
    let fixture = seed_fixture(&pool).await;
    let ctx = ctx(fixture.owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let query = PutawayRecommendationQuery {
        product_code: "M2-P-001".to_string(),
        batch_no: "M2-BATCH-001".to_string(),
        qty: 4,
        quality_status: STATUS_QUALIFIED.to_string(),
        limit: Some(5),
    };

    let recommendations = repository
        .recommend_putaway_locations(&ctx, fixture.order_id, query)
        .await
        .expect("recommendations should be available");
    assert_eq!(recommendations.data.len(), 2);
    assert_eq!(recommendations.data[0].location_code, "M2-LOC-SAME");
    assert!(recommendations.data[0].same_product);
    assert!(recommendations
        .data
        .iter()
        .all(|item| item.temperature_zone == "normal"));
    assert!(!recommendations
        .data
        .iter()
        .any(|item| item.location_code == "M2-LOC-FULL"));

    let request = PutawayRequest {
        batch_no: "M2-BATCH-001".to_string(),
        product_code: "M2-P-001".to_string(),
        qty: 4,
        location_id: fixture.same_product_location_id,
        location_code: fixture.same_product_location_code,
        quality_status: STATUS_QUALIFIED.to_string(),
    };
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "putaway",
        "M2",
        "receiving_order",
        fixture.order_id.to_string(),
        None,
    );
    let first = repository
        .putaway_receiving_order_and_inventory_with_audit(
            &ctx,
            fixture.order_id,
            request.clone(),
            Utc::now(),
            "m2-putaway-idem-1",
            Some(audit.clone()),
        )
        .await
        .expect("putaway should commit");
    let replay = repository
        .putaway_receiving_order_and_inventory_with_audit(
            &ctx,
            fixture.order_id,
            request,
            Utc::now(),
            "m2-putaway-idem-1",
            Some(audit),
        )
        .await
        .expect("same putaway should replay");

    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(first.value.putaway.id, replay.value.putaway.id);
    let state: (i64, i64, i64, String) = sqlx::query_as(
        "SELECT (SELECT used_volume_cm3 FROM warehouse_locations WHERE id = $1), (SELECT qty_on_hand FROM inventory_batches WHERE owner_id = $2 AND product_code = 'M2-P-001' AND batch_no = 'M2-BATCH-001'), (SELECT COUNT(*) FROM receiving_putaways WHERE receiving_order_id = $3), (SELECT status FROM receiving_orders WHERE id = $3)",
    )
    .bind(fixture.same_product_location_id)
    .bind(fixture.owner_id)
    .bind(fixture.order_id)
    .fetch_one(&pool)
    .await
    .expect("putaway state should be readable");
    assert_eq!(state, (50, 4, 1, "putaway".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn smart_putaway_rejects_cross_owner_invalid_state_and_invalid_quantity(pool: PgPool) {
    let fixture = seed_fixture(&pool).await;
    let repository = PgWave3Repository::new(pool.clone());
    let foreign_ctx = ctx(Uuid::new_v4());
    let base_query = || PutawayRecommendationQuery {
        product_code: "M2-P-001".to_string(),
        batch_no: "M2-BATCH-001".to_string(),
        qty: 1,
        quality_status: STATUS_QUALIFIED.to_string(),
        limit: Some(5),
    };
    assert!(matches!(
        repository
            .recommend_putaway_locations(&foreign_ctx, fixture.order_id, base_query())
            .await,
        Err(Wave3RepositoryError::NotFound)
    ));

    let ctx = ctx(fixture.owner_id);
    assert!(matches!(
        repository
            .recommend_putaway_locations(
                &ctx,
                fixture.order_id,
                PutawayRecommendationQuery {
                    qty: 0,
                    ..base_query()
                },
            )
            .await,
        Err(Wave3RepositoryError::InvalidQuantity)
    ));
    assert!(matches!(
        repository
            .recommend_putaway_locations(
                &ctx,
                fixture.order_id,
                PutawayRecommendationQuery {
                    qty: 11,
                    ..base_query()
                },
            )
            .await,
        Err(Wave3RepositoryError::QuantityClosureMismatch)
    ));

    sqlx::query("UPDATE receiving_orders SET status = 'inspecting' WHERE id = $1")
        .bind(fixture.order_id)
        .execute(&pool)
        .await
        .expect("order status should update");
    assert!(matches!(
        repository
            .recommend_putaway_locations(&ctx, fixture.order_id, base_query())
            .await,
        Err(Wave3RepositoryError::InvalidStatus { expected, actual })
            if expected == "putaway" && actual == "inspecting"
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_permission_is_granted_to_custodian_but_not_receiving_clerk(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'M2 permission owner')",
    )
    .bind(owner_id)
    .bind(format!("M2-PERM-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("owner should seed default roles");
    let grants: Vec<(String, i64)> = sqlx::query_as(
        "SELECT role.role_code, COUNT(permission.permission_code)::BIGINT FROM auth_roles role LEFT JOIN auth_role_permissions rp ON rp.role_id = role.id LEFT JOIN auth_permissions permission ON permission.id = rp.permission_id AND permission.permission_code = 'm2.putaway.write' WHERE role.owner_id = $1 GROUP BY role.role_code",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("role grants should be readable");
    let grant_for = |role_code: &str| {
        grants
            .iter()
            .find(|(code, _)| code == role_code)
            .map(|(_, count)| *count)
            .expect("default role should exist")
    };
    assert_eq!(grant_for("custodian"), 1);
    assert_eq!(grant_for("receiving_clerk"), 0);
}
