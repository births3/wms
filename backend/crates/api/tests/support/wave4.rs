use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::inventory::STATUS_QUALIFIED;

pub async fn seed_customer_delivery_address(pool: &PgPool, owner_id: Uuid) -> (Uuid, Uuid) {
    let customer_id = Uuid::new_v4();
    let address_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name)
         VALUES ($1, $2, 'M4 配送地址测试货主')
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("M4-ADDR-{}", &owner_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed delivery owner");
    sqlx::query(
        "INSERT INTO customers (
            id, owner_id, customer_code, customer_name, customer_type, status
         )
         VALUES ($1, $2, $3, 'M4 配送客户', 'customer', 'active')",
    )
    .bind(customer_id)
    .bind(owner_id)
    .bind(format!("M4-C-{}", &customer_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed delivery customer");
    sqlx::query(
        "INSERT INTO customer_addresses (
            id, owner_id, customer_id, province, city, district,
            detail_address, contact_name, contact_phone, is_default
         )
         VALUES ($1, $2, $3, '上海市', '上海市', '浦东新区',
                 '测试路 1 号', '测试收货人', '13800000000', TRUE)",
    )
    .bind(address_id)
    .bind(owner_id)
    .bind(customer_id)
    .execute(pool)
    .await
    .expect("seed delivery address");
    (customer_id, address_id)
}

#[allow(dead_code)]
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
