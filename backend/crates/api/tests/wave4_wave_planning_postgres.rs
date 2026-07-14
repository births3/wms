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

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "wave-planning-postgres-test".to_string(),
        permissions: vec!["m4.write".to_string()],
        jti: Uuid::new_v4().to_string(),
    }
}

async fn seed_inventory(pool: &PgPool, owner_id: Uuid, now: chrono::DateTime<Utc>) {
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
    .bind(Uuid::new_v4())
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
    let order = repo
        .create_outbound_order(
            &ctx,
            CreateOutboundOrderRequest {
                document_type: "sales_outbound".to_string(),
                wms_order_no: "WMS-WAVE-001".to_string(),
                erp_order_no: None,
                customer_id: Uuid::new_v4(),
                warehouse_id: Uuid::new_v4(),
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
