#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn ensure_audit_partition(pool: &PgPool, occurred_at: DateTime<Utc>) {
    sqlx::query("SELECT create_audit_partition($1)")
        .bind(occurred_at.date_naive())
        .execute(pool)
        .await
        .expect("audit partition for fixed test time should exist");
}

pub async fn audit_event(pool: &PgPool, owner_id: Uuid, minimum_count: i64) {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_event WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(pool)
        .await
        .expect("audit evidence should query");
    assert!(
        count >= minimum_count,
        "expected at least {minimum_count} audit event(s), got {count}"
    );
}

pub async fn idempotency_request(pool: &PgPool, owner_id: Uuid, idempotency_key: &str) {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_one(pool)
    .await
    .expect("idempotency evidence should query");
    assert_eq!(count, 1, "idempotency key should persist exactly once");
}

pub async fn seed_idle_lpn(pool: &PgPool, owner_id: Uuid, lpn_code: &str) {
    sqlx::query(
        r#"
        INSERT INTO lpn_containers (
            id, owner_id, lpn_code, container_type, status, current_lock_category, created_at, updated_at
        ) VALUES ($1, $2, $3, 'pallet', 'idle', 'qualified', now(), now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(lpn_code)
    .execute(pool)
    .await
    .expect("idle LPN should seed");
}
