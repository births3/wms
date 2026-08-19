use sqlx::PgPool;
use uuid::Uuid;

pub async fn audit_count(pool: &PgPool, owner_id: Uuid) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
          FROM audit_event
         WHERE owner_id = $1
           AND module = ANY($2)
        "#,
    )
    .bind(owner_id)
    .bind(vec!["M-PK", "M8", "M9", "M10"])
    .fetch_one(pool)
    .await
    .expect("count audit events")
}
