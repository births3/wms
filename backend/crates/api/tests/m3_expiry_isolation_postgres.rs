use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{NaiveDate, TimeZone, Utc};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    inventory::{STATUS_QUALIFIED, STATUS_UNQUALIFIED},
    wave3_handlers::{wave3_router, Wave3AppState},
    wave3_repository::PgWave3Repository,
};
use wms_domain::InventoryBatchQuery;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m3-expiry-test".to_string(),
        permissions: vec!["m3.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_batch(
    pool: &PgPool,
    owner_id: Uuid,
    batch_no: &str,
    expiry_date: NaiveDate,
    status: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_frozen, status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, 'P-EXPIRY-001', $3, $4, $5, 10, 0, $6, $7, $8, FALSE, $9, $9)
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(batch_no)
    .bind(NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid production date"))
    .bind(expiry_date)
    .bind(status)
    .bind(Uuid::new_v4())
    .bind(format!("EXP-{}", &id.to_string()[..8]))
    .bind(now)
    .execute(pool)
    .await
    .expect("seed inventory batch");
    id
}

async fn seed_location(
    pool: &PgPool,
    owner_id: Uuid,
    zone_code: &str,
    location_type: &str,
) -> (Uuid, String) {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let location_code = format!("LOC-{}", &location_id.to_string()[..8]);
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type) VALUES ($1, $2, $3, 'query test warehouse', 'normal')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed query warehouse");
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color) VALUES ($1, $2, $3, $4, 'query test zone', 'normal_10_30', 'qualified_green')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_code)
    .execute(pool)
    .await
    .expect("seed query zone");
    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, max_sku_count, location_type) VALUES ($1, $2, $3, $4, $5, 1, 1, 1, 1000, 3, $6)",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(&location_code)
    .bind(location_type)
    .execute(pool)
    .await
    .expect("seed query location");
    (location_id, location_code)
}

async fn seed_product(pool: &PgPool, owner_id: Uuid, product_code: &str, product_name: &str) {
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition) VALUES ($1, $2, $3, $4, '10mg', 'normal_10_30')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(product_code)
    .bind(product_name)
    .execute(pool)
    .await
    .expect("seed query product");
}

async fn seed_query_batch(
    pool: &PgPool,
    owner_id: Uuid,
    batch_no: &str,
    location_id: Uuid,
    location_code: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO inventory_batches (id, owner_id, product_code, batch_no, production_date, expiry_date, qty_on_hand, status, location_id, location_code, created_at, updated_at) VALUES ($1, $2, 'P-QUERY-LOCATION', $3, '2026-01-01', '2027-01-01', 10, $4, $5, $6, $7, $7)",
    )
    .bind(id)
    .bind(owner_id)
    .bind(batch_no)
    .bind(STATUS_QUALIFIED)
    .bind(location_id)
    .bind(location_code)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed location query batch");
    id
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_query_returns_location_snapshot_and_supports_product_and_temperature_search(
    pool: PgPool,
) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let repo = PgWave3Repository::new(pool.clone());
    let (location_id, location_code) = seed_location(&pool, owner_id, "ZONE-A", "storage").await;
    let (other_location_id, other_location_code) =
        seed_location(&pool, other_owner_id, "ZONE-B", "storage").await;
    seed_product(&pool, owner_id, "P-QUERY-LOCATION", "阿莫西林胶囊").await;
    seed_product(&pool, owner_id, "P-QUERY-SECOND", "维生素片").await;
    seed_product(
        &pool,
        other_owner_id,
        "P-QUERY-LOCATION",
        "其他货主阿莫西林",
    )
    .await;
    seed_query_batch(&pool, owner_id, "B-LOCATION-1", location_id, &location_code).await;
    sqlx::query(
        "INSERT INTO inventory_batches (id, owner_id, product_code, batch_no, production_date, expiry_date, qty_on_hand, status, location_id, location_code) VALUES ($1, $2, 'P-QUERY-SECOND', 'B-LOCATION-2', '2026-01-01', '2027-01-01', 5, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(STATUS_QUALIFIED)
    .bind(location_id)
    .bind(&location_code)
    .execute(&pool)
    .await
    .expect("seed second sku");
    seed_query_batch(
        &pool,
        other_owner_id,
        "B-OTHER-OWNER",
        other_location_id,
        &other_location_code,
    )
    .await;

    let (rows, _total) = repo
        .list_inventory_batches_with_query(
            &ctx(owner_id),
            InventoryBatchQuery {
                q: Some("阿莫西林".to_string()),
                temperature_zone: Some("normal_10_30".to_string()),
                ..InventoryBatchQuery::default()
            },
            1,
            200,
        )
        .await
        .expect("query enriched inventory snapshot");

    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.product_name.as_deref(), Some("阿莫西林胶囊"));
    assert_eq!(row.zone_code.as_deref(), Some("ZONE-A"));
    assert_eq!(row.temperature_zone.as_deref(), Some("normal_10_30"));
    assert_eq!(row.quality_color.as_deref(), Some("qualified_green"));
    assert_eq!(row.row_no, Some(1));
    assert_eq!(row.column_no, Some(1));
    assert_eq!(row.layer_no, Some(1));
    assert_eq!(row.max_volume_cm3, Some(1000));
    assert_eq!(row.used_volume_cm3, Some(0));
    assert_eq!(row.remaining_volume_cm3, Some(1000));
    assert_eq!(row.max_sku_count, Some(3));
    assert_eq!(row.current_sku_count, Some(2));
}

#[sqlx::test(migrations = "../../migrations")]
async fn expired_batches_are_isolated_idempotently_and_owner_scoped(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let owner_ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let today = NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid today");
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
        .single()
        .expect("valid timestamp");

    let expired_id = seed_batch(&pool, owner_id, "B-EXPIRED", today, STATUS_QUALIFIED).await;
    let future_id = seed_batch(
        &pool,
        owner_id,
        "B-FUTURE",
        today.succ_opt().expect("next day"),
        STATUS_QUALIFIED,
    )
    .await;
    let other_expired_id = seed_batch(
        &pool,
        other_owner_id,
        "B-OTHER-EXPIRED",
        today,
        STATUS_QUALIFIED,
    )
    .await;
    seed_batch(
        &pool,
        owner_id,
        "B-ALREADY-UNQUALIFIED",
        today,
        STATUS_UNQUALIFIED,
    )
    .await;

    let first = repo
        .isolate_expired_inventory_batches(&owner_ctx, today, now, "m3-expiry-001", None)
        .await
        .expect("expired batches should be isolated");
    assert!(!first.replayed);
    assert_eq!(first.value.len(), 1);
    assert_eq!(first.value[0].id, expired_id);
    assert_eq!(first.value[0].status, STATUS_UNQUALIFIED);

    let states: (String, String, String, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT status FROM inventory_batches WHERE id = $1),
            (SELECT status FROM inventory_batches WHERE id = $2),
            (SELECT status FROM inventory_batches WHERE id = $3),
            (SELECT COUNT(*) FROM inventory_status_changes
              WHERE owner_id = $4 AND batch_id = $1 AND approval_source = 'M3-002-EXPIRY'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $4 AND action = 'isolate_expired_inventory_batch' AND resource_id = $1::TEXT)
        "#,
    )
    .bind(expired_id)
    .bind(future_id)
    .bind(other_expired_id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("query expiry state evidence");
    assert_eq!(
        states,
        (
            STATUS_UNQUALIFIED.to_string(),
            STATUS_QUALIFIED.to_string(),
            STATUS_QUALIFIED.to_string(),
            1,
            1,
        )
    );

    let replay = repo
        .isolate_expired_inventory_batches(&owner_ctx, today, now, "m3-expiry-001", None)
        .await
        .expect("same expiry job should replay");
    assert!(replay.replayed);
    assert_eq!(replay.value.len(), first.value.len());
    assert_eq!(replay.value[0].id, first.value[0].id);
    assert_eq!(replay.value[0].status, first.value[0].status);

    let duplicate_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventory_status_changes WHERE owner_id = $1 AND batch_id = $2 AND approval_source = 'M3-002-EXPIRY'",
    )
    .bind(owner_id)
    .bind(expired_id)
    .fetch_one(&pool)
    .await
    .expect("count status changes");
    assert_eq!(duplicate_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn near_expiry_query_filters_owner_and_orders_by_expiry(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let repo = PgWave3Repository::new(pool.clone());
    let owner_ctx = ctx(owner_id);

    seed_batch(
        &pool,
        owner_id,
        "B-NEAR-AUG",
        NaiveDate::from_ymd_opt(2026, 8, 1).expect("valid expiry date"),
        STATUS_QUALIFIED,
    )
    .await;
    seed_batch(
        &pool,
        owner_id,
        "B-NEAR-JUL",
        NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid expiry date"),
        STATUS_QUALIFIED,
    )
    .await;
    seed_batch(
        &pool,
        other_owner_id,
        "B-OTHER-AUG",
        NaiveDate::from_ymd_opt(2026, 8, 1).expect("valid expiry date"),
        STATUS_QUALIFIED,
    )
    .await;

    let (rows, _total) = repo
        .list_inventory_batches_with_query(
            &owner_ctx,
            wms_domain::InventoryBatchQuery {
                expiry_from: Some("2026-06-04".to_string()),
                expiry_to: Some("2026-08-31".to_string()),
                ..Default::default()
            },
            1,
            200,
        )
        .await
        .expect("near-expiry query should succeed");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].batch_no, "B-NEAR-JUL");
    assert_eq!(rows[1].batch_no, "B-NEAR-AUG");
    assert!(rows.iter().all(|row| row.owner_id == owner_id));
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_query_combines_filters_without_cross_owner_rows(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let repo = PgWave3Repository::new(pool.clone());
    let owner_ctx = ctx(owner_id);
    let target_id = Uuid::new_v4();
    let now = Utc::now();
    let target_location = "A-01-01";

    for (id, owner, product, batch, location, status) in [
        (
            target_id,
            owner_id,
            "P-QUERY-001",
            "B-QUERY-TARGET",
            target_location,
            STATUS_QUALIFIED,
        ),
        (
            Uuid::new_v4(),
            owner_id,
            "P-QUERY-001",
            "B-QUERY-OTHER",
            target_location,
            STATUS_UNQUALIFIED,
        ),
        (
            Uuid::new_v4(),
            other_owner_id,
            "P-QUERY-001",
            "B-QUERY-TARGET",
            target_location,
            STATUS_QUALIFIED,
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO inventory_batches (
                id, owner_id, product_code, batch_no, production_date, expiry_date,
                qty_on_hand, qty_frozen, status, location_id, location_code,
                recall_flag, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 10, 0, $7, $8, $9, FALSE, $10, $10)
            "#,
        )
        .bind(id)
        .bind(owner)
        .bind(product)
        .bind(batch)
        .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid production date"))
        .bind(NaiveDate::from_ymd_opt(2027, 1, 1).expect("valid expiry date"))
        .bind(status)
        .bind(Uuid::new_v4())
        .bind(location)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed query batch");
    }

    let (rows, _total) = repo
        .list_inventory_batches_with_query(
            &owner_ctx,
            wms_domain::InventoryBatchQuery {
                product_code: Some("QUERY-001".to_string()),
                batch_no: Some("TARGET".to_string()),
                location_code: Some(target_location.to_string()),
                status: Some(STATUS_QUALIFIED.to_string()),
                expiry_from: Some("2026-12-01".to_string()),
                expiry_to: Some("2027-12-31".to_string()),
                ..Default::default()
            },
            1,
            200,
        )
        .await
        .expect("combined inventory query should succeed");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, target_id);
    assert_eq!(rows[0].owner_id, owner_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_query_filters_location_type_and_zone_without_cross_owner_rows(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let repo = PgWave3Repository::new(pool.clone());
    let owner_ctx = ctx(owner_id);
    let (target_location_id, target_location_code) =
        seed_location(&pool, owner_id, "ZONE-TARGET", "storage").await;
    let (other_location_id, other_location_code) =
        seed_location(&pool, owner_id, "ZONE-OTHER", "case_pick").await;
    let (cross_owner_location_id, cross_owner_location_code) =
        seed_location(&pool, other_owner_id, "ZONE-TARGET", "storage").await;
    let target_id = seed_query_batch(
        &pool,
        owner_id,
        "B-LOCATION-TARGET",
        target_location_id,
        &target_location_code,
    )
    .await;
    seed_query_batch(
        &pool,
        owner_id,
        "B-LOCATION-MISS",
        other_location_id,
        &other_location_code,
    )
    .await;
    seed_query_batch(
        &pool,
        other_owner_id,
        "B-LOCATION-OTHER-OWNER",
        cross_owner_location_id,
        &cross_owner_location_code,
    )
    .await;

    let (rows, _total) = repo
        .list_inventory_batches_with_query(
            &owner_ctx,
            wms_domain::InventoryBatchQuery {
                location_type: Some("storage".to_string()),
                zone_code: Some("ZONE-TARGET".to_string()),
                ..Default::default()
            },
            1,
            200,
        )
        .await
        .expect("location metadata query should succeed");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, target_id);
    assert_eq!(rows[0].owner_id, owner_id);

    let (rows_with_blank_filters, _t1) = repo
        .list_inventory_batches_with_query(
            &owner_ctx,
            wms_domain::InventoryBatchQuery {
                location_type: Some("  ".to_string()),
                zone_code: Some(String::new()),
                ..Default::default()
            },
            1,
            200,
        )
        .await
        .expect("blank location metadata filters should be ignored");
    assert_eq!(rows_with_blank_filters.len(), 2);

    let (rows_with_unknown_filter, _t2) = repo
        .list_inventory_batches_with_query(
            &owner_ctx,
            wms_domain::InventoryBatchQuery {
                location_type: Some("unknown".to_string()),
                ..Default::default()
            },
            1,
            200,
        )
        .await
        .expect("unknown location metadata filter should return empty");
    assert!(rows_with_unknown_filter.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_query_filters_production_and_created_date_ranges(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let repo = PgWave3Repository::new(pool.clone());
    let owner_ctx = ctx(owner_id);
    let target_id = Uuid::new_v4();
    let production_in_range = NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid production date");
    let production_out_of_range =
        NaiveDate::from_ymd_opt(2026, 2, 1).expect("valid production date");
    let created_in_range = Utc
        .with_ymd_and_hms(2026, 6, 15, 10, 0, 0)
        .single()
        .expect("valid created timestamp");
    let created_out_of_range = Utc
        .with_ymd_and_hms(2026, 7, 1, 10, 0, 0)
        .single()
        .expect("valid created timestamp");

    for (id, owner, batch_no, production_date, created_at) in [
        (
            target_id,
            owner_id,
            "B-DATE-TARGET",
            production_in_range,
            created_in_range,
        ),
        (
            Uuid::new_v4(),
            owner_id,
            "B-DATE-PRODUCTION-MISS",
            production_out_of_range,
            created_in_range,
        ),
        (
            Uuid::new_v4(),
            owner_id,
            "B-DATE-CREATED-MISS",
            production_in_range,
            created_out_of_range,
        ),
        (
            Uuid::new_v4(),
            other_owner_id,
            "B-DATE-OTHER-OWNER",
            production_in_range,
            created_in_range,
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO inventory_batches (
                id, owner_id, product_code, batch_no, production_date, expiry_date,
                qty_on_hand, qty_frozen, status, location_id, location_code,
                recall_flag, created_at, updated_at
            )
            VALUES ($1, $2, 'P-DATE-001', $3, $4, '2027-01-01', 10, 0, $5, $6, $7, FALSE, $8, $8)
            "#,
        )
        .bind(id)
        .bind(owner)
        .bind(batch_no)
        .bind(production_date)
        .bind(STATUS_QUALIFIED)
        .bind(Uuid::new_v4())
        .bind(format!("DATE-{}", &id.to_string()[..8]))
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("seed date filter batch");
    }

    let (rows, _total) = repo
        .list_inventory_batches_with_query(
            &owner_ctx,
            wms_domain::InventoryBatchQuery {
                production_from: Some("2026-01-01".to_string()),
                production_to: Some("2026-01-31".to_string()),
                created_from: Some("2026-06-01T00:00:00Z".to_string()),
                created_to: Some("2026-06-30T23:59:59Z".to_string()),
                ..Default::default()
            },
            1,
            200,
        )
        .await
        .expect("date range inventory query should succeed");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, target_id);
    assert_eq!(rows[0].owner_id, owner_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_query_http_requires_m3_read_or_write(pool: PgPool) {
    let app = wave3_router(Wave3AppState::with_postgres(pool));
    let mut request = Request::builder()
        .method("GET")
        .uri("/api/v1/inventory/batches")
        .body(Body::empty())
        .expect("inventory query request should build");
    request
        .extensions_mut()
        .insert(AuthContext {
            permissions: vec!["m2.write".to_string()],
            ..ctx(Uuid::new_v4())
        });
    let response = app
        .oneshot(request)
        .await
        .expect("inventory query should respond");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
