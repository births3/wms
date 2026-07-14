use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    inventory::{STATUS_QUALIFIED, STATUS_UNQUALIFIED},
    inventory_expiry_job,
};

async fn seed_batch(
    pool: &PgPool,
    owner_id: Uuid,
    batch_no: &str,
    expiry_date: NaiveDate,
    quality_status: &str,
    now: DateTime<Utc>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, 'P-EXPIRY-SCHEDULER', $3, '2025-01-01', $4,
                10, 0, $5, $6, $7, FALSE, $8, $8)
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(batch_no)
    .bind(expiry_date)
    .bind(quality_status)
    .bind(Uuid::new_v4())
    .bind(format!("SCHED-{}", &id.to_string()[..8]))
    .bind(now)
    .execute(pool)
    .await
    .expect("scheduler inventory batch should seed");
    id
}

async fn evidence(
    pool: &PgPool,
    owner_a: Uuid,
    expired_a: Uuid,
    future_a: Uuid,
    owner_b: Uuid,
    expired_b: Uuid,
    future_b: Uuid,
) -> (String, String, String, String, i64, i64, i64, i64, i64, i64) {
    sqlx::query_as(
        r#"
        SELECT
            (SELECT quality_status FROM inventory_batches WHERE id = $1),
            (SELECT quality_status FROM inventory_batches WHERE id = $2),
            (SELECT quality_status FROM inventory_batches WHERE id = $3),
            (SELECT quality_status FROM inventory_batches WHERE id = $4),
            (SELECT COUNT(*) FROM inventory_status_changes
              WHERE owner_id = $5 AND batch_id = $1 AND approval_source = 'M3-002-EXPIRY'),
            (SELECT COUNT(*) FROM inventory_status_changes
              WHERE owner_id = $6 AND batch_id = $3 AND approval_source = 'M3-002-EXPIRY'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $5 AND action = 'isolate_expired_inventory_batch'
                AND resource_id = $1::TEXT),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $6 AND action = 'isolate_expired_inventory_batch'
                AND resource_id = $3::TEXT),
            (SELECT COUNT(*) FROM idempotency_request
              WHERE owner_id = $5 AND idempotency_key LIKE 'm3-expiry-scheduler:%'),
            (SELECT COUNT(*) FROM idempotency_request
              WHERE owner_id = $6 AND idempotency_key LIKE 'm3-expiry-scheduler:%')
        "#,
    )
    .bind(expired_a)
    .bind(future_a)
    .bind(expired_b)
    .bind(future_b)
    .bind(owner_a)
    .bind(owner_b)
    .fetch_one(pool)
    .await
    .expect("scheduler evidence should query")
}

#[sqlx::test(migrations = "../../migrations")]
async fn expiry_scheduler_is_owner_scoped_and_idempotent(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
        .single()
        .expect("scheduler timestamp should be valid");
    let as_of = now.date_naive();
    let expired_a = seed_batch(
        &pool,
        owner_a,
        "B-SCHED-A-EXPIRED",
        as_of,
        STATUS_QUALIFIED,
        now,
    )
    .await;
    let future_a = seed_batch(
        &pool,
        owner_a,
        "B-SCHED-A-FUTURE",
        as_of.succ_opt().expect("next date should exist"),
        STATUS_QUALIFIED,
        now,
    )
    .await;
    let expired_b = seed_batch(
        &pool,
        owner_b,
        "B-SCHED-B-EXPIRED",
        as_of,
        STATUS_QUALIFIED,
        now,
    )
    .await;
    let future_b = seed_batch(
        &pool,
        owner_b,
        "B-SCHED-B-FUTURE",
        as_of.succ_opt().expect("next date should exist"),
        STATUS_QUALIFIED,
        now,
    )
    .await;

    let first = inventory_expiry_job::run_once(&pool, now)
        .await
        .expect("first expiry scheduler run should succeed");
    assert_eq!(first, 2);
    let after_first = evidence(
        &pool, owner_a, expired_a, future_a, owner_b, expired_b, future_b,
    )
    .await;
    assert_eq!(
        after_first,
        (
            STATUS_UNQUALIFIED.to_string(),
            STATUS_QUALIFIED.to_string(),
            STATUS_UNQUALIFIED.to_string(),
            STATUS_QUALIFIED.to_string(),
            1,
            1,
            1,
            1,
            1,
            1,
        )
    );

    let second = inventory_expiry_job::run_once(&pool, now)
        .await
        .expect("repeated expiry scheduler run should succeed");
    assert_eq!(second, 0);
    assert_eq!(
        evidence(&pool, owner_a, expired_a, future_a, owner_b, expired_b, future_b).await,
        after_first
    );
}
