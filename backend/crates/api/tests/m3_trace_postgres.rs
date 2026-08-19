use chrono::{NaiveDate, Utc};
use uuid::Uuid;
use wms_api::auth::AuthContext;
use wms_api::wave3_repository::PgWave3Repository;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "trace-test".to_string(),
        permissions: vec!["m3.read".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_batch_trace_is_owner_scoped_and_contains_movements_and_status_changes(
    pool: sqlx::PgPool,
) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_frozen, status, location_id, location_code
        ) VALUES ($1, $2, 'P-TRACE-001', 'B-TRACE-001', $3, $4, 10, 0, 'quarantined', $5, 'A01-01-01-01')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("production date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("expiry date"))
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("batch");
    sqlx::query(
        r#"
        INSERT INTO inventory_movements (
            id, owner_id, batch_id, movement_type, qty_delta,
            source_document_type, source_document_id, occurred_at
        ) VALUES ($1, $2, $3, 'inbound_putaway', 10, 'receiving_order', $4, $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(batch_id)
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(&pool)
    .await
    .expect("movement");
    sqlx::query(
        r#"
        INSERT INTO inventory_status_changes (
            id, owner_id, batch_id, from_status, to_status,
            reason, approval_source, approval_id, occurred_at
        ) VALUES ($1, $2, $3, 'qualified', 'quarantined', '质量异常', 'M-QL', 'QL-TRACE-001', $4)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(batch_id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("status change");

    let repository = PgWave3Repository::new(pool);
    let trace = repository
        .get_inventory_batch_trace(&ctx(owner_id), batch_id)
        .await
        .expect("trace");
    assert_eq!(trace.batch.id, batch_id);
    assert_eq!(trace.movements.len(), 1);
    assert_eq!(trace.status_changes.len(), 1);
    assert_eq!(trace.status_changes[0].approval_id, "QL-TRACE-001");
    assert!(repository
        .get_inventory_batch_trace(&ctx(other_owner_id), batch_id)
        .await
        .is_err());
}
