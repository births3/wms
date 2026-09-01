use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    wave3_repository::PgWave3Repository,
};
use wms_domain::{Quantity, QuickSpotCountRequest};

mod postgres_test_support;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "spot-count-t3".to_string(),
        permissions: vec!["m3.inventory_count.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn quick_spot_count_replays_idempotency_and_writes_audit(pool: PgPool) {
    // POST /api/v1/inventory/counts/quick-spot-count
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '快速抽盘测试货主')",
    )
    .bind(owner_id)
    .bind(format!("SP-{}", &owner_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed owner");
    postgres_test_support::ensure_audit_partition(&pool, now).await;

    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, '抽盘仓', 'physical', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, 'SP-Z', '抽盘区', 'normal_10_30', 'qualified_green', 'active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await
    .expect("seed zone");
    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status) VALUES ($1, $2, $3, $4, 'LOC-SPOT-T3', 1, 1, 1, 5000000, 0, 10, 'storage', 'available')",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(&pool)
    .await
    .expect("seed location");
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_frozen, status, location_id, location_code
        ) VALUES ($1, $2, 'MED-SPOT-T3', 'BAT-SPOT-T3', $3, $4, 15, 0, 'qualified', $5, 'LOC-SPOT-T3')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("date"))
    .bind(location_id)
    .execute(&pool)
    .await
    .expect("seed inventory");

    let actor = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let request = QuickSpotCountRequest {
        location_code: "LOC-SPOT-T3".to_string(),
        product_code: "MED-SPOT-T3".to_string(),
        batch_no: "BAT-SPOT-T3".to_string(),
        physical_qty: 15.into(),
        reason: Some("T3 快速抽盘".to_string()),
        operated_at: Some(now),
    };
    let audit = AuditWriteRequest::from_auth_context(
        &actor,
        "quick_spot_count",
        "M3",
        "inventory_count",
        "LOC-SPOT-T3:MED-SPOT-T3",
        None,
    );
    let first = repo
        .quick_spot_count(&actor, request.clone(), now, "t3-spot-1", Some(audit))
        .await
        .expect("first spot count");
    assert_eq!(first.value.variance_type, "MATCH");
    assert_eq!(first.value.variance_qty, Quantity::ZERO);

    let replay_audit = AuditWriteRequest::from_auth_context(
        &actor,
        "quick_spot_count",
        "M3",
        "inventory_count",
        "LOC-SPOT-T3:MED-SPOT-T3",
        None,
    );
    let replay = repo
        .quick_spot_count(&actor, request, now, "t3-spot-1", Some(replay_audit))
        .await
        .expect("spot count replay");
    assert_eq!(replay.value.variance_type, first.value.variance_type);
    assert_eq!(replay.value.book_qty, first.value.book_qty);

    postgres_test_support::audit_event(&pool, owner_id, 1).await;
    postgres_test_support::idempotency_request(&pool, owner_id, "t3-spot-1").await;
}
