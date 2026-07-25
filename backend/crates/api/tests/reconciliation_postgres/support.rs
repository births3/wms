use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn seed_batch(
    pool: &PgPool,
    owner_id: Uuid,
    product_code: &str,
    batch_no: &str,
    qty: i64,
) -> (Uuid, Uuid) {
    let suffix = Uuid::new_v4().simple().to_string();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO products
         (id, owner_id, product_code, product_name, specification, storage_condition,
          special_drug_category, status)
         VALUES ($1, $2, $3, $3, '测试规格', 'normal', 'normal', 'active')
         ON CONFLICT (owner_id, product_code) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(product_code)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status)
         VALUES ($1, $2, $3, 'RC 仓库', 'main', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("RC-WH-{}", &suffix[..8]))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO warehouse_zones
         (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status)
         VALUES ($1, $2, $3, $4, 'RC 库区', 'normal', 'qualified_green', 'active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(format!("RC-Z-{}", &suffix[..8]))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO warehouse_locations
         (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
          max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status)
         VALUES ($1, $2, $3, $4, $5, 1, 1, 1, 100000, 0, 10, 'storage', 'available')",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(format!("RC-L-{}", &suffix[..8]))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO inventory_batches
         (id, owner_id, product_code, batch_no, production_date, expiry_date,
          qty_on_hand, qty_locked, quality_status, location_id, location_code)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 0, 'qualified', $8, $9)",
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(product_code)
    .bind(batch_no)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).unwrap())
    .bind(qty)
    .bind(location_id)
    .bind(format!("RC-L-{}", &suffix[..8]))
    .execute(pool)
    .await
    .unwrap();
    (batch_id, warehouse_id)
}

pub(super) async fn seed_active_claim(
    pool: &PgPool,
    owner_id: Uuid,
    window_key: &str,
) -> (Uuid, Uuid) {
    let claim_id = Uuid::new_v4();
    let claim_token = Uuid::new_v4();
    let attempt_no: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(attempt_no), 0) + 1
           FROM reconciliation_schedule_claims
          WHERE owner_id=$1 AND window_key=$2",
    )
    .bind(owner_id)
    .bind(window_key)
    .fetch_one(pool)
    .await
    .expect("load next reconciliation test claim attempt");
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO reconciliation_schedule_claims
         (id, owner_id, window_key, claim_token, worker_id, attempt_no, status,
          lease_expires_at, claimed_at, updated_at)
         VALUES ($1,$2,$3,$4,'rc-repository-test',$5,'active',$6,$7,$7)",
    )
    .bind(claim_id)
    .bind(owner_id)
    .bind(window_key)
    .bind(claim_token)
    .bind(attempt_no)
    .bind(now + Duration::minutes(5))
    .bind(now)
    .execute(pool)
    .await
    .expect("seed active reconciliation test claim");
    (claim_id, claim_token)
}
