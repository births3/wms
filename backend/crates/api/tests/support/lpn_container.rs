use chrono::{DateTime, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext, inventory::STATUS_QUALIFIED,
    lpn_container_repository::PgLpnContainerRepository,
};
use wms_domain::{
    CreateLpnContainerRequest, LpnContainer, PutawayRequest, UpdateLpnContainerRequest,
    LPN_CONTAINER_STATUS_IN_USE, LPN_CONTAINER_TYPE_PALLET,
};

pub async fn insert_owner(pool: &PgPool, owner_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'LPN test owner') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("LPN{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("auth owner");
}

pub fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "lpn-test".to_string(),
        permissions: vec![
            "m1.master_data.write".to_string(),
            "m2.putaway.write".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

pub fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 15, hour, 0, 0)
        .single()
        .expect("test timestamp should be valid")
}

pub async fn seed_lpn_numbering(pool: &PgPool, now: DateTime<Utc>, owner_id: Uuid) {
    insert_owner(pool, owner_id).await;
    for (item_code, item_name, rule_code, template) in [
        (
            "lpn_pallet",
            "容器LPN-托盘",
            "TEST-LPN-PALLET",
            "LPN-PL-{OWNER}-{YYYY}{MM}{DD}-{SEQ}",
        ),
        (
            "lpn_tote",
            "容器LPN-周转箱",
            "TEST-LPN-TOTE",
            "LPN-TT-{OWNER}-{YYYY}{MM}{DD}-{SEQ}",
        ),
        (
            "lpn_outbound_box",
            "容器LPN-出库箱",
            "TEST-LPN-OUT",
            "LPN-OB-{OWNER}-{YYYY}{MM}{DD}-{SEQ}",
        ),
        (
            "lpn_insulated_box",
            "容器LPN-保温箱",
            "TEST-LPN-INS",
            "LPN-IB-{OWNER}-{YYYY}{MM}{DD}-{SEQ}",
        ),
        (
            "lpn_blind_label",
            "容器LPN-盲标签",
            "TEST-LPN-BLD",
            "LPN-BL-{OWNER}-{YYYY}{MM}{DD}-{SEQ}",
        ),
    ] {
        sqlx::query(
            "INSERT INTO system_dictionary_items (id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at) VALUES ($1, 'document_type', $2, $3, TRUE, NULL, '{\"direction\":\"internal\",\"workflow_template\":\"lpn_container\",\"batch_policy\":\"none\"}'::jsonb, 'global', $4, $4) ON CONFLICT DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(item_code)
        .bind(item_name)
        .bind(now)
        .execute(pool)
        .await
        .expect("lpn document type");
        sqlx::query(
            "INSERT INTO document_number_rules (id, owner_id, document_type, rule_code, rule_name, template, reset_policy, sequence_width, sequence_mode, enabled, created_at, updated_at) VALUES ($1, NULL, $2, $3, $4, $5, 'daily', 4, 'no_gap', TRUE, $6, $6) ON CONFLICT DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(item_code)
        .bind(rule_code)
        .bind(item_name)
        .bind(template)
        .bind(now)
        .execute(pool)
        .await
        .expect("lpn numbering rule");
    }
    seed_quality_lock_move_targets(pool, owner_id).await;
}

/// 为加锁隔离移库准备隔离区/不合格区存储位。
pub async fn seed_quality_lock_move_targets(pool: &PgPool, owner_id: Uuid) {
    let warehouse_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, '质量锁测试仓', 'normal', 'active') ON CONFLICT DO NOTHING",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("QLWH{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("quality lock warehouse");
    for (suffix, color) in [
        ("Q", "quarantine_yellow"),
        ("R", "unqualified_red"),
        ("G", "qualified_green"),
    ] {
        let zone_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO warehouse_zones (
                id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
                quality_color, allowed_categories, is_external_use_zone, is_fragrant_zone,
                is_special_drug_zone, status
            ) VALUES ($1, $2, $3, $4, $5, 'normal_10_30', $6, '[]'::jsonb, false, false, false, 'active')
            "#,
        )
        .bind(zone_id)
        .bind(owner_id)
        .bind(warehouse_id)
        .bind(format!("QLZ{suffix}"))
        .bind(format!("质量锁{suffix}区"))
        .bind(color)
        .execute(pool)
        .await
        .expect("quality lock zone");
        sqlx::query(
            r#"
            INSERT INTO warehouse_locations (
                id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
                location_type, allows_container, status, max_volume_cm3, used_volume_cm3
            ) VALUES (gen_random_uuid(), $1, $2, $3, $4, 1, 1, 1, 'storage', true, 'available', 100000, 0)
            "#,
        )
        .bind(owner_id)
        .bind(warehouse_id)
        .bind(zone_id)
        .bind(format!("QLLOC-{suffix}-01"))
        .execute(pool)
        .await
        .expect("quality lock location");
    }
}

pub fn create_req() -> CreateLpnContainerRequest {
    CreateLpnContainerRequest {
        container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
        capacity_cm3: Some(8000),
    }
}

/// 创建容器并置为 in_use（质量锁相关测试共用夹具）。
#[allow(dead_code)]
pub async fn setup_container_in_use(
    repo: &PgLpnContainerRepository,
    actor: &AuthContext,
    key: &str,
) -> LpnContainer {
    let created = repo
        .create(actor, create_req(), at(1), &format!("{key}-c"))
        .await
        .expect("create container");
    repo.update(
        actor,
        created.id,
        UpdateLpnContainerRequest {
            status: Some(LPN_CONTAINER_STATUS_IN_USE.to_string()),
            location_id: None,
            capacity_cm3: None,
        },
        at(2),
        &format!("{key}-u"),
    )
    .await
    .expect("set container in_use")
}

pub struct PutawayFixture {
    pub owner_id: Uuid,
    pub order_id: Uuid,
    pub location_id: Uuid,
    pub location_code: String,
}

pub async fn seed_putaway(pool: &PgPool) -> PutawayFixture {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'LPN WH', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("LPN-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("warehouse");
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, 'Z1', 'zone', 'normal_10_30', 'qualified_green', 'active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("zone");
    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status) VALUES ($1, $2, $3, $4, 'LOC-1', 1, 1, 1, 1000, 0, 3, 'storage', 'available')",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(pool)
    .await
    .expect("location");
    sqlx::query(
        "INSERT INTO products (id, owner_id, erp_goods_id, product_code, product_name, specification, storage_condition, volume_cm3, status) VALUES ($1, $2, 2001, 'LPN-P-001', 'p', '1', 'normal_10_30', 10, 'active')",
    )
    .bind(product_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("product");
    sqlx::query(
        "INSERT INTO receiving_orders (id, owner_id, receipt_no, document_type, warehouse_id, erp_bill_id, erp_bill_code, erp_revision, erp_line_no, erp_correlation_id, status, expected_arrival_at) VALUES ($1, $2, 'LPN-ASN-001', 'purchase_inbound', $3, 9002, 'ERP-LPN-001', 1, 1, 'corr-lpn-001', 'putaway', $4)",
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("order");
    sqlx::query(
        "INSERT INTO receiving_order_lines (id, receiving_order_id, owner_id, line_no, product_id, product_code, expected_qty, batch_no, production_date, expiry_date) VALUES ($1, $2, $3, 1, $4, 'LPN-P-001', 10, 'LPN-B-001', '2026-01-01', '2028-01-01')",
    )
    .bind(Uuid::new_v4())
    .bind(order_id)
    .bind(owner_id)
    .bind(product_id)
    .execute(pool)
    .await
    .expect("line");
    sqlx::query(
        "INSERT INTO receiving_inspections (id, receiving_order_id, owner_id, batch_no, accepted_qty, rejected_qty, production_date, expiry_date, quality_status, occurred_at) VALUES ($1, $2, $3, 'LPN-B-001', 10, 0, '2026-01-01', '2028-01-01', 'qualified', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(order_id)
    .bind(owner_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("inspection");

    PutawayFixture {
        owner_id,
        order_id,
        location_id,
        location_code: "LOC-1".to_string(),
    }
}

pub fn putaway_req(fixture: &PutawayFixture, lpn: &str) -> PutawayRequest {
    PutawayRequest {
        batch_no: "LPN-B-001".to_string(),
        product_code: "LPN-P-001".to_string(),
        qty: 2.into(),
        location_id: fixture.location_id,
        location_code: fixture.location_code.clone(),
        quality_status: STATUS_QUALIFIED.to_string(),
        lpn_code: Some(lpn.to_string()),
    }
}

#[allow(dead_code)]
pub fn loose_putaway_req(fixture: &PutawayFixture) -> PutawayRequest {
    PutawayRequest {
        batch_no: "LPN-B-001".to_string(),
        product_code: "LPN-P-001".to_string(),
        qty: 2.into(),
        location_id: fixture.location_id,
        location_code: fixture.location_code.clone(),
        quality_status: STATUS_QUALIFIED.to_string(),
        lpn_code: None,
    }
}

#[allow(dead_code)]
pub async fn batch_container_lpn(pool: &PgPool, owner_id: Uuid) -> Option<String> {
    sqlx::query_scalar(
        "SELECT container_lpn FROM inventory_batches WHERE owner_id = $1 AND product_code = 'LPN-P-001' AND batch_no = 'LPN-B-001'",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .expect("batch container_lpn")
}

#[allow(dead_code)]
pub async fn batch_qty(pool: &PgPool, owner_id: Uuid) -> wms_domain::Quantity {
    sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE owner_id = $1 AND product_code = 'LPN-P-001' AND batch_no = 'LPN-B-001'",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .expect("batch qty")
}

#[allow(dead_code)]
pub async fn lpn_status(pool: &PgPool, owner_id: Uuid, lpn_code: &str) -> String {
    sqlx::query_scalar("SELECT status FROM lpn_containers WHERE owner_id = $1 AND lpn_code = $2")
        .bind(owner_id)
        .bind(lpn_code)
        .fetch_one(pool)
        .await
        .expect("lpn status")
}

#[allow(dead_code)]
pub async fn putaway_count(pool: &PgPool, owner_id: Uuid, order_id: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM receiving_putaways WHERE owner_id = $1 AND receiving_order_id = $2",
    )
    .bind(owner_id)
    .bind(order_id)
    .fetch_one(pool)
    .await
    .expect("putaway rows")
}

#[allow(dead_code)]
pub async fn lpn_product_codes(pool: &PgPool, owner_id: Uuid, lpn_code: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT product_code FROM inventory_batches WHERE owner_id = $1 AND container_lpn = $2 ORDER BY product_code",
    )
    .bind(owner_id)
    .bind(lpn_code)
    .fetch_all(pool)
    .await
    .expect("lpn product codes")
}
