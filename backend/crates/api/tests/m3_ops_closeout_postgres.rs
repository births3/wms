use chrono::{NaiveDate, Utc};
use uuid::Uuid;
use wms_api::auth::AuthContext;
use wms_api::wave3_repository::PgWave3Repository;
use wms_domain::{
    ChangeInventoryStatusRequest, HandleInventoryAlertRequest, InventoryAbcQuery,
    InventoryAlertQuery, OverrideInventoryAbcRequest, RecomputeInventoryAbcRequest,
    RelocateInventoryRequest,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m3-ops-test".to_string(),
        permissions: vec![
            "m3.read".to_string(),
            "m3.write".to_string(),
            "m3.relocation.write".to_string(),
            "m3.alert.read".to_string(),
            "m3.alert.write".to_string(),
            "m3.abc.read".to_string(),
            "m3.abc.write".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_location(pool: &sqlx::PgPool, owner_id: Uuid, location_id: Uuid, code: &str) {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let suffix = code.replace('-', "");
    sqlx::query(
        r#"
        INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status)
        VALUES ($1,$2,$3,'M3 移库仓','normal','active')
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{suffix}"))
    .execute(pool)
    .await
    .expect("warehouse");
    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status
        ) VALUES ($1,$2,$3,$4,'冷藏区','cold','qualified_green','active')
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(format!("Z-{suffix}"))
    .execute(pool)
    .await
    .expect("zone");
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status
        ) VALUES ($1,$2,$3,$4,$5,1,1,1,100000,0,10,'storage','available')
        "#,
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(code)
    .execute(pool)
    .await
    .expect("location");
}

#[sqlx::test(migrations = "../../migrations")]
async fn relocate_inventory_moves_qty_atomically_and_rejects_quarantined(pool: sqlx::PgPool) {
    let owner_id = Uuid::new_v4();
    let from_id = Uuid::new_v4();
    let to_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    seed_location(&pool, owner_id, from_id, "A01-01-01-01").await;
    seed_location(&pool, owner_id, to_id, "A01-01-01-02").await;
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code
        ) VALUES ($1,$2,'P-REL-001','B-REL-001',$3,$4,20,0,'qualified',$5,'A01-01-01-01')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).unwrap())
    .bind(from_id)
    .execute(&pool)
    .await
    .expect("batch");

    let repository = PgWave3Repository::new(pool.clone());
    let result = repository
        .relocate_inventory_with_audit(
            &ctx(owner_id),
            RelocateInventoryRequest {
                batch_id,
                qty: 5,
                to_location_id: to_id,
                to_location_code: "A01-01-01-02".to_string(),
                relocation_mode: Some("direct".to_string()),
                lpn_code: None,
                reason: Some("优化库位".to_string()),
            },
            Utc::now(),
            "idem-relocate-1",
            None,
        )
        .await
        .expect("relocate");
    assert_eq!(result.value.qty, 5);
    assert_eq!(result.value.to_location_code, "A01-01-01-02");

    let (from_qty, to_qty): (i64, i64) = sqlx::query_as(
        r#"
        SELECT
          (SELECT qty_on_hand::BIGINT FROM inventory_batches WHERE id = $1),
          (SELECT COALESCE(SUM(qty_on_hand),0)::BIGINT FROM inventory_batches
            WHERE owner_id = $2 AND location_code = 'A01-01-01-02' AND product_code = 'P-REL-001')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("qty");
    assert_eq!(from_qty, 15);
    assert_eq!(to_qty, 5);

    sqlx::query("UPDATE inventory_batches SET quality_status = 'quarantined' WHERE id = $1")
        .bind(batch_id)
        .execute(&pool)
        .await
        .expect("quarantine");
    let blocked = repository
        .relocate_inventory_with_audit(
            &ctx(owner_id),
            RelocateInventoryRequest {
                batch_id,
                qty: 1,
                to_location_id: to_id,
                to_location_code: "A01-01-01-02".to_string(),
                relocation_mode: None,
                lpn_code: None,
                reason: None,
            },
            Utc::now(),
            "idem-relocate-2",
            None,
        )
        .await;
    assert!(blocked.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
async fn status_change_enqueues_erp_outbox_and_process_succeeds(pool: sqlx::PgPool) {
    let owner_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code
        ) VALUES ($1,$2,'P-ERP-001','B-ERP-001',$3,$4,10,0,'qualified',$5,'L-ERP')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).unwrap())
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("batch");

    let repository = PgWave3Repository::new(pool.clone());
    repository
        .change_inventory_status_with_audit(
            &ctx(owner_id),
            ChangeInventoryStatusRequest {
                batch_id,
                target_status: "quarantined".to_string(),
                reason: "质量隔离".to_string(),
                approval_source: "质量联系单".to_string(),
                approval_id: "QL-001".to_string(),
            },
            Utc::now(),
            "idem-status-erp",
            None,
        )
        .await
        .expect("status");

    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventory_status_erp_feedback_outbox WHERE owner_id = $1 AND status = 'pending'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("pending");
    assert!(pending >= 1);

    let processed = repository
        .process_status_erp_feedback_outbox(Utc::now(), 20)
        .await
        .expect("process");
    assert!(processed >= 1);
    let succeeded: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventory_status_erp_feedback_outbox WHERE owner_id = $1 AND status = 'succeeded'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("succeeded");
    assert!(succeeded >= 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn near_expiry_alerts_and_lifecycle_are_owner_scoped(pool: sqlx::PgPool) {
    let owner_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let soon = Utc::now().date_naive() + chrono::Duration::days(10);
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code
        ) VALUES ($1,$2,'P-AL-001','B-AL-001',$3,$4,8,0,'qualified',$5,'L-AL')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    .bind(soon)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("batch");

    let repository = PgWave3Repository::new(pool);
    let created = repository
        .generate_near_expiry_alerts(&ctx(owner_id), Utc::now(), 180)
        .await
        .expect("generate");
    assert!(created >= 1);
    let list = repository
        .list_inventory_alerts(
            &ctx(owner_id),
            &InventoryAlertQuery {
                alert_type: Some("near_expiry".to_string()),
                lifecycle_status: Some("open".to_string()),
                ..InventoryAlertQuery::default()
            },
        )
        .await
        .expect("list");
    assert!(!list.data.is_empty());
    let handled = repository
        .handle_inventory_alert(
            &ctx(owner_id),
            list.data[0].id,
            HandleInventoryAlertRequest {
                lifecycle_status: "handled".to_string(),
            },
            Utc::now(),
        )
        .await
        .expect("handle");
    assert_eq!(handled.lifecycle_status, "handled");
}

#[sqlx::test(migrations = "../../migrations")]
async fn abc_recompute_and_manual_override(pool: sqlx::PgPool) {
    let owner_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code
        ) VALUES ($1,$2,'P-ABC-001','B-ABC-001',$3,$4,100,0,'qualified',$5,'L-ABC')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).unwrap())
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("batch");
    sqlx::query(
        r#"
        INSERT INTO inventory_movements (
            id, owner_id, batch_id, movement_type, qty_delta,
            source_document_type, source_document_id, occurred_at
        ) VALUES ($1,$2,$3,'outbound_ship',-50,'outbound_order',$4,now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(batch_id)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("movement");

    let repository = PgWave3Repository::new(pool);
    let recomputed = repository
        .recompute_abc_classifications(
            &ctx(owner_id),
            RecomputeInventoryAbcRequest {
                period_days: Some(30),
            },
            Utc::now(),
        )
        .await
        .expect("recompute");
    assert!(!recomputed.data.is_empty());
    let overridden = repository
        .override_abc_classification(
            &ctx(owner_id),
            OverrideInventoryAbcRequest {
                product_code: "P-ABC-001".to_string(),
                abc_class: "A".to_string(),
                reason: "人工覆盖".to_string(),
            },
            Utc::now(),
        )
        .await
        .expect("override");
    assert_eq!(overridden.source, "manual");
    assert_eq!(overridden.abc_class, "A");
    let listed = repository
        .list_abc_classifications(
            &ctx(owner_id),
            &InventoryAbcQuery {
                product_code: Some("P-ABC".to_string()),
                ..InventoryAbcQuery::default()
            },
        )
        .await
        .expect("list");
    assert!(!listed.data.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn generate_maintenance_tasks_for_near_expiry_batches(pool: sqlx::PgPool) {
    let owner_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let soon = Utc::now().date_naive() + chrono::Duration::days(20);
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code
        ) VALUES ($1,$2,'P-MAIN-001','B-MAIN-001',$3,$4,5,0,'qualified',$5,'L-MAIN')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    .bind(soon)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("batch");
    let repository = PgWave3Repository::new(pool);
    let created = repository
        .generate_maintenance_tasks(&ctx(owner_id), Utc::now(), 180)
        .await
        .expect("generate");
    assert!(created >= 1);
    let tasks = repository
        .list_maintenance_tasks(
            &ctx(owner_id),
            wms_domain::MaintenanceTaskQuery {
                status: Some("pending".to_string()),
                ..wms_domain::MaintenanceTaskQuery::default()
            },
        )
        .await
        .expect("list");
    assert!(!tasks.is_empty());
}
