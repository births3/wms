use chrono::{NaiveDate, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    inventory::STATUS_QUALIFIED,
    wave4_repository::{PgWave4Repository, Wave4RepositoryError},
};
use wms_domain::{
    CreateOutboundOrderLineRequest, CreateOutboundOrderRequest, CreateOutboundWaveRequest,
};

#[path = "support/h9.rs"]
mod h9_support;
use h9_support::seed_outbound_route_binding;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "wave-planning-postgres-test".to_string(),
        permissions: vec!["m4.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_inventory(pool: &PgPool, owner_id: Uuid, now: chrono::DateTime<Utc>) {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '波次计划测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("WVP-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("wave owner should be seeded");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, '波次计划测试仓', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WVP-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("wave warehouse should be seeded");
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, $4, '波次计划测试区', 'normal', 'qualified_green', 'active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(format!("WVP-ZONE-{}", &zone_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("wave zone should be seeded");
    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status) VALUES ($1, $2, $3, $4, 'OUT-A-01', 1, 1, 1, 100000, 0, 100, 'storage', 'available')",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(pool)
    .await
    .expect("wave location should be seeded");
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, 'P-WAVE-001', 'B-WAVE-001', $3, $4, 10, 0, $5, $6, 'OUT-A-01', FALSE, $7, $7)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid date"))
    .bind(STATUS_QUALIFIED)
    .bind(location_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("wave inventory should be seeded");
}

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_wave_release_persists_tasks_locks_audit_and_replays(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 8, 15, 0)
        .single()
        .expect("valid time");
    seed_inventory(&pool, owner_id, now).await;
    let customer_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let delivery_address_id =
        seed_outbound_route_binding(&pool, owner_id, warehouse_id, customer_id, now).await;
    let order = repo
        .create_outbound_order(
            &ctx,
            CreateOutboundOrderRequest {
                document_type: "sales_outbound".to_string(),
                wms_order_no: "WMS-WAVE-001".to_string(),
                erp_order_no: None,
                customer_id,
                warehouse_id,
                delivery_address_id,
                required_ship_at: None,
                lines: vec![CreateOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: "P-WAVE-001".to_string(),
                    batch_no: "B-WAVE-001".to_string(),
                    planned_qty: 6,
                }],
            },
            now,
            "wave-order-1",
            None,
        )
        .await
        .expect("wave order should be created")
        .value;
    let request = CreateOutboundWaveRequest {
        wave_no: "WAVE-20260605-TEST-001".to_string(),
        order_ids: vec![order.id],
    };
    let wave = repo
        .create_outbound_wave(&ctx, request.clone(), now, "wave-create-1", None)
        .await
        .expect("wave should be created")
        .value;
    let evidence: (i64, i64, Option<String>, i64, i64) = sqlx::query_as(
        r#"SELECT
             (SELECT COUNT(*) FROM outbound_pick_tasks WHERE owner_id = $1 AND wave_id = $2),
             (SELECT COALESCE(SUM(planned_qty), 0)::BIGINT FROM outbound_pick_tasks WHERE owner_id = $1 AND wave_id = $2),
             (SELECT MIN(location_code) FROM outbound_pick_tasks WHERE owner_id = $1 AND wave_id = $2),
             (SELECT qty_locked FROM inventory_batches WHERE owner_id = $1 AND product_code = 'P-WAVE-001' AND batch_no = 'B-WAVE-001'),
             (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'create_outbound_wave' AND resource_type = 'outbound_order')"#,
    )
    .bind(owner_id)
    .bind(wave.id)
    .fetch_one(&pool)
    .await
    .expect("wave evidence should query");
    assert_eq!(evidence, (1, 6, Some("OUT-A-01".to_string()), 6, 1));

    let replay = repo
        .create_outbound_wave(&ctx, request.clone(), now, "wave-create-1", None)
        .await
        .expect("same wave key should replay");
    assert!(replay.replayed);
    assert_eq!(replay.value.id, wave.id);
    let duplicate = repo
        .create_outbound_wave(
            &ctx,
            CreateOutboundWaveRequest {
                wave_no: "WAVE-20260605-TEST-002".to_string(),
                order_ids: vec![order.id],
            },
            now,
            "wave-create-duplicate",
            None,
        )
        .await
        .expect_err("an order must not enter two waves");
    assert!(matches!(
        duplicate,
        Wave4RepositoryError::OrderAlreadyInWave
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn h8_outbound_order_create_replays_without_duplicate_lines(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 23, 9, 30, 0)
        .single()
        .expect("valid time");
    let customer_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let delivery_address_id =
        seed_outbound_route_binding(&pool, owner_id, warehouse_id, customer_id, now).await;
    let request = CreateOutboundOrderRequest {
        document_type: "sales_outbound".to_string(),
        wms_order_no: "WMS-H8-L11-001".to_string(),
        erp_order_no: Some("ERP-H8-L11-001".to_string()),
        customer_id,
        warehouse_id,
        delivery_address_id,
        required_ship_at: None,
        lines: vec![CreateOutboundOrderLineRequest {
            line_no: 1,
            product_code: "P-H8-L11-001".to_string(),
            batch_no: "B-H8-L11-001".to_string(),
            planned_qty: 3,
        }],
    };

    let first = repo
        .create_outbound_order(&ctx, request.clone(), now, "h8-outbound-order-1", None)
        .await
        .expect("outbound order should be created");
    let replayed = repo
        .create_outbound_order(&ctx, request, now, "h8-outbound-order-1", None)
        .await
        .expect("outbound order should replay");

    assert!(!first.replayed);
    assert!(replayed.replayed);
    assert_eq!(replayed.value.id, first.value.id);
    let evidence: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM outbound_orders WHERE owner_id = $1 AND id = $2), (SELECT COUNT(*) FROM outbound_order_lines WHERE owner_id = $1 AND outbound_order_id = $2), (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'create_outbound_order' AND resource_id = $2::text), (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'h8-outbound-order-1')",
    )
    .bind(owner_id)
    .bind(first.value.id)
    .fetch_one(&pool)
    .await
    .expect("outbound order evidence should query");
    assert_eq!(evidence, (1, 1, 1, 1));
}
