use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::inventory::STATUS_QUALIFIED;

pub async fn seed_inventory_batch(
    pool: &PgPool,
    owner_id: Uuid,
    now: chrono::DateTime<Utc>,
) -> Uuid {
    let batch_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, 'P-COLD-001', 'B-TEMP-001', $3, $4, 10, 0, $5, $6, 'COLD-A-01', FALSE, $7, $7)
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid date"))
    .bind(STATUS_QUALIFIED)
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(pool)
    .await
    .expect("seed inventory batch");
    batch_id
}
