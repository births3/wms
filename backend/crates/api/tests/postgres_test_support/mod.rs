use chrono::{DateTime, Utc};
use sqlx::PgPool;

pub async fn ensure_audit_partition(pool: &PgPool, occurred_at: DateTime<Utc>) {
    sqlx::query("SELECT create_audit_partition($1)")
        .bind(occurred_at.date_naive())
        .execute(pool)
        .await
        .expect("audit partition for fixed test time should exist");
}
