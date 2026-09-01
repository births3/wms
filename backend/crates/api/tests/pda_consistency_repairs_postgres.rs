use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    auth_repository::AuthRepository,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{Quantity, QuickSpotCountRequest, RelocateInventoryRequest};

fn ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "pda-consistency-test".to_string(),
        permissions: vec![
            "m3.write".to_string(),
            "m3.relocation.write".to_string(),
            "m3.inventory_count.write".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_owner_user(pool: &PgPool) -> (Uuid, Uuid) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '一致性测试货主')",
    )
    .bind(owner_id)
    .bind(format!("OWNER-{}", &owner_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed owner");
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '一致性测试员', 'unused', 'active')",
    )
    .bind(user_id)
    .bind(format!("user-{}", &user_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed user");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("bind user");
    (owner_id, user_id)
}

async fn seed_location(pool: &PgPool, owner_id: Uuid, code: &str) -> (Uuid, Uuid, Uuid) {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status)
        VALUES ($1, $2, $3, '一致性测试仓', 'physical', 'active')
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name,
            temperature_zone, quality_color, status
        ) VALUES (
            $1, $2, $3, $4, '一致性测试区',
            'normal_10_30', 'qualified_green', 'active'
        )
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(format!("Z-{}", &zone_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed zone");
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code,
            row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3,
            max_sku_count, location_type, status
        ) VALUES (
            $1, $2, $3, $4, $5,
            1, 1, 1, 1000000, 0, 10, 'storage', 'available'
        )
        "#,
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(code)
    .execute(pool)
    .await
    .expect("seed location");
    (warehouse_id, zone_id, location_id)
}

#[allow(clippy::too_many_arguments)]
async fn seed_batch(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    zone_id: Uuid,
    location_id: Uuid,
    location_code: &str,
    batch_id: Uuid,
    status: &str,
    qty: i64,
) {
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_frozen, status, location_id, location_code,
            warehouse_id, zone_id
        ) VALUES (
            $1, $2, 'MED-CONSISTENCY', 'BATCH-CONSISTENCY', $3, $4,
            $5, 0, $6, $7, $8, $9, $10
        )
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid production date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid expiry date"))
    .bind(qty)
    .bind(status)
    .bind(location_id)
    .bind(location_code)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(pool)
    .await
    .expect("seed inventory batch");
}

#[sqlx::test(migrations = "../../migrations")]
async fn omitted_owner_code_requires_exactly_one_active_binding(pool: PgPool) {
    let owner_1 = Uuid::new_v4();
    let owner_2 = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let username = format!("multi-{}", &user_id.simple().to_string()[..8]);
    for (owner_id, code) in [(owner_1, "MULTI-A"), (owner_2, "MULTI-B")] {
        sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, $2)")
            .bind(owner_id)
            .bind(code)
            .execute(&pool)
            .await
            .expect("seed owner");
    }
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '多货主用户', 'unused', 'active')",
    )
    .bind(user_id)
    .bind(&username)
    .execute(&pool)
    .await
    .expect("seed user");
    for owner_id in [owner_1, owner_2] {
        sqlx::query(
            "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, FALSE)",
        )
        .bind(user_id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("bind owner");
    }

    let repository = AuthRepository::new(pool);
    assert!(repository
        .find_login_user(None, &username)
        .await
        .expect("query multi-owner login")
        .is_none());
    assert_eq!(
        repository
            .find_login_user(Some("MULTI-A"), &username)
            .await
            .expect("query explicit owner")
            .expect("explicit owner should resolve")
            .owner_id,
        owner_1
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn relocation_rejects_mismatched_target_id_and_code(pool: PgPool) {
    let (owner_id, user_id) = seed_owner_user(&pool).await;
    let (warehouse_id, zone_id, source_id) = seed_location(&pool, owner_id, "LOC-SOURCE").await;
    let (_, _, target_a) = seed_location(&pool, owner_id, "LOC-TARGET-A").await;
    let (_, _, _target_b) = seed_location(&pool, owner_id, "LOC-TARGET-B").await;
    let batch_id = Uuid::new_v4();
    seed_batch(
        &pool,
        owner_id,
        warehouse_id,
        zone_id,
        source_id,
        "LOC-SOURCE",
        batch_id,
        "qualified",
        20,
    )
    .await;

    let result = PgWave3Repository::new(pool.clone())
        .relocate_inventory_with_audit(
            &ctx(owner_id, user_id),
            RelocateInventoryRequest {
                batch_id,
                qty: 5.into(),
                to_location_id: Some(target_a),
                to_location_code: "LOC-TARGET-B".to_string(),
                from_location_code: Some("LOC-SOURCE".to_string()),
                relocation_mode: Some("direct".to_string()),
                lpn_code: None,
                reason: Some("mismatch regression".to_string()),
                operated_at: Some(Utc::now()),
            },
            Utc::now(),
            "idem-relocation-mismatch",
            None,
        )
        .await;
    assert!(matches!(result, Err(Wave3RepositoryError::InvalidLocation)));

    let (qty, location_code): (i64, String) = sqlx::query_as(
        "SELECT qty_on_hand::BIGINT, location_code FROM inventory_batches WHERE id = $1",
    )
    .bind(batch_id)
    .fetch_one(&pool)
    .await
    .expect("query unchanged batch");
    assert_eq!(qty, 20);
    assert_eq!(location_code, "LOC-SOURCE");
}

#[sqlx::test(migrations = "../../migrations")]
async fn quick_spot_count_rejects_ambiguous_states_and_records_auto_approval(pool: PgPool) {
    let (owner_id, user_id) = seed_owner_user(&pool).await;
    let (warehouse_id, zone_id, location_id) =
        seed_location(&pool, owner_id, "LOC-SPOT-CONSISTENCY").await;
    let batch_id = Uuid::new_v4();
    seed_batch(
        &pool,
        owner_id,
        warehouse_id,
        zone_id,
        location_id,
        "LOC-SPOT-CONSISTENCY",
        batch_id,
        "qualified",
        12,
    )
    .await;
    let repository = PgWave3Repository::new(pool.clone());
    let server_now = Utc::now();
    let response = repository
        .quick_spot_count(
            &ctx(owner_id, user_id),
            QuickSpotCountRequest {
                location_code: "LOC-SPOT-CONSISTENCY".to_string(),
                product_code: "MED-CONSISTENCY".to_string(),
                batch_no: "BATCH-CONSISTENCY".to_string(),
                physical_qty: 12.into(),
                reason: Some("  日常抽盘  ".to_string()),
                operated_at: Some(server_now - Duration::minutes(3)),
            },
            server_now,
            "idem-spot-auto-approval",
            None,
        )
        .await
        .expect("quick spot count");
    assert_eq!(response.value.variance_type, "MATCH");

    type QuickSpotApprovalRow = (
        String,
        Option<Uuid>,
        Option<chrono::DateTime<Utc>>,
        Option<String>,
        Option<String>,
        Option<String>,
        chrono::DateTime<Utc>,
        chrono::DateTime<Utc>,
    );
    let approval: QuickSpotApprovalRow = sqlx::query_as(
        r#"
        SELECT status, approved_by, approved_at, approval_source, approval_id, reason,
               started_at, created_at
          FROM inventory_counts
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(response.value.count_id)
    .fetch_one(&pool)
    .await
    .expect("query count approval");
    assert_eq!(approval.0, "approved");
    assert_eq!(approval.1, Some(user_id));
    assert!(approval.2.is_some());
    assert_eq!(approval.3.as_deref(), Some("system_auto_match"));
    assert!(approval
        .4
        .as_deref()
        .is_some_and(|value| value.starts_with("quick-spot:")));
    assert_eq!(approval.5.as_deref(), Some("日常抽盘"));
    assert_eq!(
        approval.6.timestamp_micros(),
        (server_now - Duration::minutes(3)).timestamp_micros()
    );
    assert_eq!(approval.7.timestamp_micros(), server_now.timestamp_micros());

    seed_batch(
        &pool,
        owner_id,
        warehouse_id,
        zone_id,
        location_id,
        "LOC-SPOT-CONSISTENCY",
        Uuid::new_v4(),
        "quarantined",
        2,
    )
    .await;
    let ambiguous = repository
        .quick_spot_count(
            &ctx(owner_id, user_id),
            QuickSpotCountRequest {
                location_code: "LOC-SPOT-CONSISTENCY".to_string(),
                product_code: "MED-CONSISTENCY".to_string(),
                batch_no: "BATCH-CONSISTENCY".to_string(),
                physical_qty: 14.into(),
                reason: None,
                operated_at: Some(Utc::now()),
            },
            Utc::now(),
            "idem-spot-ambiguous",
            None,
        )
        .await;
    assert!(matches!(
        ambiguous,
        Err(Wave3RepositoryError::InvalidInventoryState)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn pda_operation_time_is_bounded_without_rewriting_server_time(pool: PgPool) {
    let (owner_id, user_id) = seed_owner_user(&pool).await;
    let (warehouse_id, zone_id, location_id) =
        seed_location(&pool, owner_id, "LOC-SPOT-TIME").await;
    seed_batch(
        &pool,
        owner_id,
        warehouse_id,
        zone_id,
        location_id,
        "LOC-SPOT-TIME",
        Uuid::new_v4(),
        "qualified",
        1,
    )
    .await;
    let repository = PgWave3Repository::new(pool);
    let now = Utc::now();

    let future = repository
        .quick_spot_count(
            &ctx(owner_id, user_id),
            QuickSpotCountRequest {
                location_code: "LOC-SPOT-TIME".to_string(),
                product_code: "MED-CONSISTENCY".to_string(),
                batch_no: "BATCH-CONSISTENCY".to_string(),
                physical_qty: Quantity::from(1),
                reason: None,
                operated_at: Some(now + Duration::minutes(6)),
            },
            now,
            "idem-spot-future",
            None,
        )
        .await;
    assert!(matches!(future, Err(Wave3RepositoryError::FutureTimestamp)));

    let expired = repository
        .quick_spot_count(
            &ctx(owner_id, user_id),
            QuickSpotCountRequest {
                location_code: "LOC-SPOT-TIME".to_string(),
                product_code: "MED-CONSISTENCY".to_string(),
                batch_no: "BATCH-CONSISTENCY".to_string(),
                physical_qty: Quantity::from(1),
                reason: None,
                operated_at: Some(now - Duration::hours(25)),
            },
            now,
            "idem-spot-expired",
            None,
        )
        .await;
    assert!(matches!(expired, Err(Wave3RepositoryError::InvalidDate(_))));
}
