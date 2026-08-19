use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn seed_outbound_route_binding(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    customer_id: Uuid,
    now: DateTime<Utc>,
) -> Uuid {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '出库线路测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("H9-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("route fixture owner should insert");
    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type
        )
        VALUES ($1, $2, $3, '出库线路测试仓', 'distribution')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("H9-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("route fixture warehouse should insert");
    sqlx::query(
        r#"
        INSERT INTO customers (
            id, owner_id, customer_code, customer_name, customer_type
        )
        VALUES ($1, $2, $3, '出库线路测试客户', 'customer')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(customer_id)
    .bind(owner_id)
    .bind(format!("H9-CUS-{}", &customer_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("route fixture customer should insert");
    let address_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO customer_addresses (
            id, owner_id, customer_id, province, city, district,
            detail_address, contact_name, contact_phone
        )
        VALUES (
            $1, $2, $3, '浙江省', '杭州市', '拱墅区',
            '出库线路测试地址', '测试收货人', '13800000009'
        )
        "#,
    )
    .bind(address_id)
    .bind(owner_id)
    .bind(customer_id)
    .execute(pool)
    .await
    .expect("route fixture address should insert");
    sqlx::query(
        r#"
        INSERT INTO h9_route_bindings (
            id, owner_id, warehouse_id, customer_id, delivery_address_id,
            route_code, effective_from, created_by, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(address_id)
    .bind(format!("LINE-{}", &address_id.to_string()[..8]))
    .bind(now - Duration::days(1))
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(pool)
    .await
    .expect("route fixture binding should insert");
    address_id
}
