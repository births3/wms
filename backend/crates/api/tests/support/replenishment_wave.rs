use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    replenishment_repository::PgReplenishmentRepository,
    replenishment_service::{CreateWaveGapTasksRequest, ReplenishmentService},
};
use wms_domain::Quantity;

pub fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::nil(),
        owner_id,
        actor_name: "wave-engine".into(),
        permissions: vec!["m3.replenishment.manage".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

pub struct World {
    pub owner_id: Uuid,
    pub product_id: Uuid,
    pub pick_id: Uuid,
    pub source_batch_id: Uuid,
}

pub async fn seed_world(pool: &PgPool, pick_on_hand: i64, source_on_hand: i64) -> World {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '缺口货主')")
        .bind(owner_id)
        .bind(format!("WG-{}", &owner_id.simple().to_string()[..8]))
        .execute(pool)
        .await
        .expect("owner");
    let product_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '缺口商品', '1', 'pending_mapping', now(), now())
        "#,
    )
    .bind(product_id)
    .bind(owner_id)
    .bind(format!("P-{}", &product_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("product");
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '缺口仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("wh");
    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, status, created_at, updated_at
        ) VALUES ($1, $2, $3, 'Z-WG', '合格区', 'normal_10_30', 'qualified_green', 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("zone");
    let storage_id = seed_loc(pool, owner_id, warehouse_id, zone_id, "ST-01", "storage").await;
    let pick_id = seed_loc(pool, owner_id, warehouse_id, zone_id, "PP-01", "piece_pick").await;
    if pick_on_hand > 0 {
        seed_batch(pool, owner_id, product_id, pick_id, "PP-01", pick_on_hand).await;
    }
    let source_batch_id = seed_batch(
        pool,
        owner_id,
        product_id,
        storage_id,
        "ST-01",
        source_on_hand,
    )
    .await;
    World {
        owner_id,
        product_id,
        pick_id,
        source_batch_id,
    }
}

pub async fn seed_loc(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    zone_id: Uuid,
    code: &str,
    location_type: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code,
            row_no, column_no, layer_no, max_volume_cm3, max_sku_count,
            location_type, current_owner_id, status, allows_container,
            mix_product_policy, mix_batch_policy, lock_status, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, 1, 1, 1, 100000, 10,
            $6, $2, 'available', FALSE, 'single_product_only', 'single_batch', 'normal', now(), now()
        )
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(code)
    .bind(location_type)
    .execute(pool)
    .await
    .expect("loc");
    id
}

async fn seed_batch(
    pool: &PgPool,
    owner_id: Uuid,
    product_id: Uuid,
    location_id: Uuid,
    location_code: &str,
    on_hand: i64,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_id, product_code, batch_no,
            production_date, expiry_date, qty_on_hand, qty_frozen, qty_allocated,
            qty_replenish_in_transit, qty_replenish_out_transit,
            status, location_id, location_code, recall_flag, created_at, updated_at, version
        ) VALUES (
            $1, $2, $3, 'P-WG', 'B1', $4, $5, $6, 0, 0, 0, 0,
            'qualified', $7, $8, FALSE, now(), now(), 1
        )
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(product_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("p"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("e"))
    .bind(Quantity::from(on_hand))
    .bind(location_id)
    .bind(location_code)
    .execute(pool)
    .await
    .expect("batch");
    id
}

pub async fn insert_wave_gap_strategy(
    pool: &PgPool,
    owner_id: Uuid,
    product_id: Uuid,
    pick_id: Uuid,
) -> Uuid {
    let strategy_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO replenishment_strategies (
            id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
            location_type, source_type, target_type,
            min_safety_threshold, max_replenish_target, trigger_modes, enabled
        ) VALUES (
            $1, $2, 'STR-WG', '缺口策略', 'product', $3,
            'piece_pick', 'storage', 'piece_pick',
            0, 100, ARRAY['wave_gap'], TRUE
        )
        "#,
    )
    .bind(strategy_id)
    .bind(owner_id)
    .bind(product_id)
    .execute(pool)
    .await
    .expect("strategy");
    sqlx::query(
        "UPDATE warehouse_locations SET replenish_strategy_id = $2 WHERE id = $1 AND owner_id = $3",
    )
    .bind(pick_id)
    .bind(strategy_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("bind");
    strategy_id
}

pub fn service(pool: PgPool) -> ReplenishmentService {
    ReplenishmentService::new(PgReplenishmentRepository::new(pool))
}

pub fn gap_req(world: &World, demand: i64) -> CreateWaveGapTasksRequest {
    CreateWaveGapTasksRequest {
        wave_id: Uuid::new_v4(),
        outbound_order_id: Uuid::new_v4(),
        outbound_line_no: 1,
        product_id: world.product_id,
        demand_qty: Quantity::from(demand),
        target_location_id: world.pick_id,
    }
}
