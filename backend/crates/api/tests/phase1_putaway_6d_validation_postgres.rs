use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header::AUTHORIZATION, Request, StatusCode},
    Router,
};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthRevocationStore,
        AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
    },
    wave3_handlers::{wave3_router, Wave3AppState},
};
use wms_domain::{PutawayLocationValidationRequest, Quantity};

struct AllowAllRevocationStore;

#[axum::async_trait]
impl AuthRevocationStore for AllowAllRevocationStore {
    async fn jti_is_blacklisted(&self, _jti: &str) -> Result<bool, AuthRevocationStoreError> {
        Ok(false)
    }
    async fn permissions_changed_at(
        &self,
        _user_id: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError> {
        Ok(None)
    }
    async fn blacklist_jti(
        &self,
        _jti: &str,
        _ttl_seconds: u64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }
    async fn set_permissions_changed_at(
        &self,
        _user_id: Uuid,
        _changed_at_unix: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }
}

fn test_app(pool: PgPool) -> Router {
    wave3_router(Wave3AppState::with_postgres(pool)).layer(auth_runtime_layer(
        AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore)),
    ))
}

fn auth_token(user_id: Uuid, owner_id: Uuid) -> String {
    std::env::set_var(JWT_SECRET_ENV, "test-putaway-6d-secret");
    let claims = build_access_claims(
        user_id,
        owner_id,
        "putaway-tester",
        vec!["m2.putaway.write".to_string(), "m3.write".to_string()],
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-putaway-6d-secret").expect("token should encode")
}

async fn seed_base(pool: &PgPool, owner_id: Uuid, user_id: Uuid) -> (Uuid, Uuid, Uuid) {
    let warehouse_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '测试货主') ON CONFLICT (id) DO NOTHING")
        .bind(owner_id).bind(format!("OWNER-{}", &owner_id.to_string()[..8]))
        .execute(pool).await.unwrap();

    sqlx::query("INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '操作人', 'hash', 'active') ON CONFLICT (id) DO NOTHING")
        .bind(user_id).bind(format!("USER-{}", &user_id.to_string()[..8]))
        .execute(pool).await.unwrap();

    sqlx::query("INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, '仓库', 'normal', 'active')")
        .bind(warehouse_id).bind(owner_id).bind(format!("WH-{}", &warehouse_id.to_string()[..8]))
        .execute(pool).await.unwrap();

    let zone_id = Uuid::new_v4();
    sqlx::query(r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, allowed_categories, is_external_use_zone, is_fragrant_zone,
            is_special_drug_zone, status
        ) VALUES ($1, $2, $3, 'ZONE-BASE', '基础区', 'normal_10_30', 'qualified_green', '["drug"]'::jsonb, false, false, false, 'active')
    "#).bind(zone_id).bind(owner_id).bind(warehouse_id).execute(pool).await.unwrap();

    let location_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            location_type, allows_container, status, max_volume_cm3, used_volume_cm3
        ) VALUES ($1, $2, $3, $4, 'LOC-BASE-01', 1, 1, 1, 'storage', true, 'available', 100000, 0)
    "#,
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(pool)
    .await
    .unwrap();

    (warehouse_id, zone_id, location_id)
}

async fn seed_product(
    pool: &PgPool,
    owner_id: Uuid,
    product_id: Uuid,
    product_code: &str,
    category: &str,
    storage_condition: &str,
    is_external_use: bool,
    is_fragrant: bool,
    special_drug_category: &str,
) {
    let attrs = json!({ "category": category });
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, dosage_form,
            storage_condition, special_drug_category, status, attrs, is_external_use, is_fragrant
        ) VALUES ($1, $2, $3, '测试商品', '瓶', '口服液', $4, $5, 'active', $6, $7, $8)
    "#,
    )
    .bind(product_id)
    .bind(owner_id)
    .bind(product_code)
    .bind(storage_condition)
    .bind(special_drug_category)
    .bind(attrs)
    .bind(is_external_use)
    .bind(is_fragrant)
    .execute(pool)
    .await
    .unwrap();
}

async fn post_validate(
    app: &Router,
    token: &str,
    req: &PutawayLocationValidationRequest,
) -> (StatusCode, serde_json::Value) {
    let body = serde_json::to_vec(req).unwrap();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/inbound/putaway/validate-location")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

fn make_req(
    loc_id: Uuid,
    prod_id: Option<Uuid>,
    container: Option<&str>,
) -> PutawayLocationValidationRequest {
    PutawayLocationValidationRequest {
        target_location_id: loc_id,
        target_location_code: None,
        product_id: prod_id,
        product_code: None,
        container_code: container.map(|c| c.to_string()),
        is_container: Some(container.is_some()),
        batch_status: None,
        witness_id: None,
        qty: Some(Quantity::from(10)),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_dim1_category_isolation(pool: PgPool) {
    let (owner_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
    let (_wh_id, _zone_id, location_id) = seed_base(&pool, owner_id, user_id).await;
    let app = test_app(pool.clone());
    let token = auth_token(user_id, owner_id);

    let product_id = Uuid::new_v4();
    seed_product(
        &pool,
        owner_id,
        product_id,
        "MED-DEV-01",
        "medical_device",
        "normal_10_30",
        false,
        false,
        "none",
    )
    .await;

    let req = make_req(location_id, Some(product_id), Some("CT-001"));
    let (status, err) = post_validate(&app, &token, &req).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_ZONE_CATEGORY_DENIED");

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM putaway_validation_rejection_logs WHERE owner_id = $1 AND error_code = 'M2_PUTAWAY_ZONE_CATEGORY_DENIED'",
    ).bind(owner_id).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_dim2_temperature_mismatch(pool: PgPool) {
    let (owner_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
    let (_wh_id, _zone_id, location_id) = seed_base(&pool, owner_id, user_id).await;
    let app = test_app(pool.clone());
    let token = auth_token(user_id, owner_id);

    let product_id = Uuid::new_v4();
    seed_product(
        &pool,
        owner_id,
        product_id,
        "COLD-DRUG-01",
        "drug",
        "cold_2_8",
        false,
        false,
        "none",
    )
    .await;

    let req = make_req(location_id, Some(product_id), Some("CT-002"));
    let (status, err) = post_validate(&app, &token, &req).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_TEMPERATURE_MISMATCH");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_dim3_container_and_batch_quality_lock(pool: PgPool) {
    let (owner_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
    let (wh_id, zone_id, location_id) = seed_base(&pool, owner_id, user_id).await;
    let app = test_app(pool.clone());
    let token = auth_token(user_id, owner_id);

    let product_id = Uuid::new_v4();
    seed_product(
        &pool,
        owner_id,
        product_id,
        "NORM-DRUG-01",
        "drug",
        "normal_10_30",
        false,
        false,
        "none",
    )
    .await;

    sqlx::query(r#"
        INSERT INTO lpn_containers (id, owner_id, lpn_code, container_type, status, current_lock_category, created_at, updated_at)
        VALUES ($1, $2, 'LPN-QUAR-01', 'pallet', 'in_use', 'quarantine', now(), now())
    "#).bind(Uuid::new_v4()).bind(owner_id).execute(&pool).await.unwrap();

    let req = make_req(location_id, Some(product_id), Some("LPN-QUAR-01"));
    let (status, err) = post_validate(&app, &token, &req).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_QUALITY_LOCKED");

    let pick_loc_id = Uuid::new_v4();
    sqlx::query(r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            location_type, allows_container, status, max_volume_cm3, used_volume_cm3
        ) VALUES ($1, $2, $3, $4, 'LOC-PICK-01', 1, 1, 2, 'piece_pick', false, 'available', 100000, 0)
    "#).bind(pick_loc_id).bind(owner_id).bind(wh_id).bind(zone_id).execute(&pool).await.unwrap();

    let mut loose_req = make_req(pick_loc_id, Some(product_id), None);
    loose_req.is_container = Some(false);
    loose_req.batch_status = Some("quarantined".to_string());
    let (status, err) = post_validate(&app, &token, &loose_req).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_QUALITY_LOCKED");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_dim4_special_drug_dual_verification(pool: PgPool) {
    let (owner_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
    let (_wh_id, _zone_id, location_id) = seed_base(&pool, owner_id, user_id).await;
    let app = test_app(pool.clone());
    let token = auth_token(user_id, owner_id);

    let product_id = Uuid::new_v4();
    seed_product(
        &pool,
        owner_id,
        product_id,
        "SPEC-DRUG-01",
        "drug",
        "normal_10_30",
        false,
        false,
        "narcotic",
    )
    .await;

    // Without witness -> rejected
    let req_no_witness = make_req(location_id, Some(product_id), Some("CT-SPEC-01"));
    let (status, err) = post_validate(&app, &token, &req_no_witness).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_SPECIAL_DUAL_REQUIRED");

    // Same witness -> rejected
    let mut req_same_witness = make_req(location_id, Some(product_id), Some("CT-SPEC-01"));
    req_same_witness.witness_id = Some(user_id);
    let (status, err) = post_validate(&app, &token, &req_same_witness).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_SPECIAL_DUAL_REQUIRED");

    // Different witness -> passes
    let mut req_valid_witness = make_req(location_id, Some(product_id), Some("CT-SPEC-01"));
    req_valid_witness.witness_id = Some(Uuid::new_v4());
    let (status, resp) = post_validate(&app, &token, &req_valid_witness).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["valid"], true);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_dim5_pack_granularity_and_location_type(pool: PgPool) {
    let (owner_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
    let (wh_id, zone_id, storage_loc_id) = seed_base(&pool, owner_id, user_id).await;
    let app = test_app(pool.clone());
    let token = auth_token(user_id, owner_id);

    let product_id = Uuid::new_v4();
    seed_product(
        &pool,
        owner_id,
        product_id,
        "NORM-DRUG-02",
        "drug",
        "normal_10_30",
        false,
        false,
        "none",
    )
    .await;

    // Loose goods without container on storage -> rejected
    let mut loose_on_storage = make_req(storage_loc_id, Some(product_id), None);
    loose_on_storage.is_container = Some(false);
    let (status, err) = post_validate(&app, &token, &loose_on_storage).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_PACK_GRANULARITY_INVALID");

    let pick_loc_id = Uuid::new_v4();
    sqlx::query(r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            location_type, allows_container, status, max_volume_cm3, used_volume_cm3
        ) VALUES ($1, $2, $3, $4, 'LOC-CASE-OK', 1, 3, 1, 'case_pick', false, 'available', 100000, 0)
    "#).bind(pick_loc_id).bind(owner_id).bind(wh_id).bind(zone_id).execute(&pool).await.unwrap();
    let locked_pick_loc_id = Uuid::new_v4();
    let reject_zone_id = Uuid::new_v4();
    sqlx::query(r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, allowed_categories, is_external_use_zone, is_fragrant_zone,
            is_special_drug_zone, status
        ) VALUES ($1, $2, $3, 'ZONE-REJ', '不合格区', 'normal_10_30', 'unqualified_red', '["drug"]'::jsonb, false, false, false, 'active')
    "#).bind(reject_zone_id).bind(owner_id).bind(wh_id).execute(&pool).await.unwrap();
    sqlx::query(r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            location_type, allows_container, status, max_volume_cm3, used_volume_cm3
        ) VALUES ($1, $2, $3, $4, 'LOC-CASE-01', 1, 2, 1, 'case_pick', false, 'available', 100000, 0)
    "#).bind(locked_pick_loc_id).bind(owner_id).bind(wh_id).bind(reject_zone_id).execute(&pool).await.unwrap();

    sqlx::query(r#"
        INSERT INTO lpn_containers (id, owner_id, lpn_code, container_type, status, current_lock_category, created_at, updated_at)
        VALUES (gen_random_uuid(), $1, 'LPN-CONTAINER-PICK', 'pallet', 'in_use', 'qualified', now(), now())
    "#).bind(owner_id).execute(&pool).await.unwrap();

    let unlocked_to_pick = make_req(pick_loc_id, Some(product_id), Some("LPN-CONTAINER-PICK"));
    let (status, resp) = post_validate(&app, &token, &unlocked_to_pick).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["valid"], true);

    sqlx::query(r#"
        INSERT INTO lpn_containers (id, owner_id, lpn_code, container_type, status, current_lock_category, created_at, updated_at)
        VALUES (gen_random_uuid(), $1, 'LPN-LOCKED-PICK', 'pallet', 'in_use', 'rejected', now(), now())
    "#).bind(owner_id).execute(&pool).await.unwrap();
    let locked_to_pick = make_req(
        locked_pick_loc_id,
        Some(product_id),
        Some("LPN-LOCKED-PICK"),
    );
    let (status, err) = post_validate(&app, &token, &locked_to_pick).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_PACK_GRANULARITY_INVALID");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_dim6_external_fragrant_exclusivity(pool: PgPool) {
    let (owner_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
    let (wh_id, _zone_id, location_id) = seed_base(&pool, owner_id, user_id).await;
    let app = test_app(pool.clone());
    let token = auth_token(user_id, owner_id);

    // External use drug on normal zone -> rejected
    let ext_prod_id = Uuid::new_v4();
    seed_product(
        &pool,
        owner_id,
        ext_prod_id,
        "EXT-DRUG-01",
        "drug",
        "normal_10_30",
        true,
        false,
        "none",
    )
    .await;
    let req_ext = make_req(location_id, Some(ext_prod_id), Some("CT-EXT-01"));
    let (status, err) = post_validate(&app, &token, &req_ext).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_EXTERNAL_FRAGRANT_CONFLICT");

    // Fragrant drug on normal zone -> rejected
    let frag_prod_id = Uuid::new_v4();
    seed_product(
        &pool,
        owner_id,
        frag_prod_id,
        "FRAG-DRUG-01",
        "drug",
        "normal_10_30",
        false,
        true,
        "none",
    )
    .await;
    let req_frag = make_req(location_id, Some(frag_prod_id), Some("CT-FRAG-01"));
    let (status, err) = post_validate(&app, &token, &req_frag).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_EXTERNAL_FRAGRANT_CONFLICT");

    // Normal drug on fragrant zone -> rejected
    let frag_zone_id = Uuid::new_v4();
    sqlx::query(r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, allowed_categories, is_external_use_zone, is_fragrant_zone,
            is_special_drug_zone, status
        ) VALUES ($1, $2, $3, 'ZONE-FRAG', '串味区', 'normal_10_30', 'qualified_green', '["drug"]'::jsonb, false, true, false, 'active')
    "#).bind(frag_zone_id).bind(owner_id).bind(wh_id).execute(&pool).await.unwrap();

    let frag_loc_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            location_type, allows_container, status, max_volume_cm3, used_volume_cm3
        ) VALUES ($1, $2, $3, $4, 'LOC-FRAG-01', 1, 1, 1, 'storage', true, 'available', 100000, 0)
    "#,
    )
    .bind(frag_loc_id)
    .bind(owner_id)
    .bind(wh_id)
    .bind(frag_zone_id)
    .execute(&pool)
    .await
    .unwrap();

    let normal_prod_id = Uuid::new_v4();
    seed_product(
        &pool,
        owner_id,
        normal_prod_id,
        "NORM-DRUG-03",
        "drug",
        "normal_10_30",
        false,
        false,
        "none",
    )
    .await;
    let req_norm_on_frag = make_req(frag_loc_id, Some(normal_prod_id), Some("CT-NORM-01"));
    let (status, err) = post_validate(&app, &token, &req_norm_on_frag).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_EXTERNAL_FRAGRANT_CONFLICT");
}

/// 6 维⑥ 容量维度：目标位剩余容量不足或混品上限已满时返回 M2_PUTAWAY_CAPACITY_EXCEEDED。
#[sqlx::test(migrations = "../../migrations")]
async fn test_capacity_dimension_exceeded(pool: PgPool) {
    let (owner_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
    let (wh_id, zone_id, location_id) = seed_base(&pool, owner_id, user_id).await;
    let app = test_app(pool.clone());
    let token = auth_token(user_id, owner_id);

    // 小容量目标位：max_volume_cm3=100
    let small_loc_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            location_type, allows_container, status, max_volume_cm3, used_volume_cm3, max_sku_count
        ) VALUES ($1, $2, $3, $4, 'LOC-SMALL-01', 1, 1, 2, 'storage', true, 'available', 100, 0, 3)
    "#,
    )
    .bind(small_loc_id)
    .bind(owner_id)
    .bind(wh_id)
    .bind(zone_id)
    .execute(&pool)
    .await
    .unwrap();

    // 单件体积 50cm3 的普通药品（其余 5 维均放行，仅容量维度拦截）
    let product_id = Uuid::new_v4();
    seed_product(
        &pool,
        owner_id,
        product_id,
        "BULK-DRUG-01",
        "drug",
        "normal_10_30",
        false,
        false,
        "none",
    )
    .await;
    sqlx::query("UPDATE products SET volume_cm3 = 50 WHERE id = $1")
        .bind(product_id)
        .execute(&pool)
        .await
        .unwrap();

    // qty=10 → 需求 500cm3 > 剩余 100cm3 → 容量不足
    let req = make_req(small_loc_id, Some(product_id), Some("CT-CAP-01"));
    let (status, err) = post_validate(&app, &token, &req).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["code"], "M2_PUTAWAY_CAPACITY_EXCEEDED");
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM putaway_validation_rejection_logs WHERE owner_id = $1 AND error_code = 'M2_PUTAWAY_CAPACITY_EXCEEDED'",
    ).bind(owner_id).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);

    // 混品上限：max_sku_count=1 的库位已有另一品库存 → 第二品同样被容量维度拦截
    let sku_loc_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            location_type, allows_container, status, max_volume_cm3, used_volume_cm3, max_sku_count
        ) VALUES ($1, $2, $3, $4, 'LOC-SKU-01', 2, 1, 1, 'storage', true, 'available', 100000, 0, 1)
    "#,
    )
    .bind(sku_loc_id)
    .bind(owner_id)
    .bind(wh_id)
    .bind(zone_id)
    .execute(&pool)
    .await
    .unwrap();

    let existing_product = Uuid::new_v4();
    seed_product(
        &pool,
        owner_id,
        existing_product,
        "EXIST-DRUG-01",
        "drug",
        "normal_10_30",
        false,
        false,
        "none",
    )
    .await;
    sqlx::query("UPDATE products SET volume_cm3 = 10 WHERE id = $1")
        .bind(existing_product)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            location_id, location_code, qty_on_hand, status
        ) VALUES (gen_random_uuid(), $1, 'EXIST-DRUG-01', 'BATCH-EXIST-01', '2026-01-01', '2027-12-31',
                  $2, 'LOC-SKU-01', 50, 'qualified')
    "#).bind(owner_id).bind(sku_loc_id).execute(&pool).await.unwrap();

    let req2 = make_req(sku_loc_id, Some(product_id), Some("CT-CAP-02"));
    let (status2, err2) = post_validate(&app, &token, &req2).await;
    assert_eq!(status2, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err2["code"], "M2_PUTAWAY_CAPACITY_EXCEEDED");
    let count2: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM putaway_validation_rejection_logs WHERE owner_id = $1 AND error_code = 'M2_PUTAWAY_CAPACITY_EXCEEDED'",
    ).bind(owner_id).fetch_one(&pool).await.unwrap();
    assert_eq!(count2, 2);

    // 对照组：容量充足且未超混品上限的正常目标位放行
    let (status_ok, resp) = post_validate(
        &app,
        &token,
        &make_req(location_id, Some(product_id), Some("CT-CAP-03")),
    )
    .await;
    assert_eq!(status_ok, StatusCode::OK);
    assert_eq!(resp["valid"], true);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_all_dimensions_pass_and_non_modification(pool: PgPool) {
    let (owner_id, user_id) = (Uuid::new_v4(), Uuid::new_v4());
    let (_wh_id, _zone_id, location_id) = seed_base(&pool, owner_id, user_id).await;
    let app = test_app(pool.clone());
    let token = auth_token(user_id, owner_id);

    let product_id = Uuid::new_v4();
    seed_product(
        &pool,
        owner_id,
        product_id,
        "PASS-DRUG-01",
        "drug",
        "normal_10_30",
        false,
        false,
        "none",
    )
    .await;

    let before_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM inventory_batches WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    let req = make_req(location_id, Some(product_id), Some("CT-PASS-01"));
    let (status, resp) = post_validate(&app, &token, &req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["valid"], true);

    let after_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM inventory_batches WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before_count, after_count);
}
