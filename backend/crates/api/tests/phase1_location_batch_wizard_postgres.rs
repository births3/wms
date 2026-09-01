use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header::AUTHORIZATION, Request, StatusCode},
    Router,
};
use chrono::{TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthRevocationStore,
        AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
    },
    master_data_handlers::{master_data_router, MasterDataAppState},
};
use wms_domain::BatchGenerateLocationsResponse;

mod postgres_test_support;
use postgres_test_support::ensure_audit_partition;

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

fn test_batch_app(pool: PgPool) -> Router {
    master_data_router(MasterDataAppState::with_postgres(pool)).layer(auth_runtime_layer(
        AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore)),
    ))
}

fn bearer_token_with_permissions(owner_id: Uuid, permissions: &[&str]) -> String {
    std::env::set_var(JWT_SECRET_ENV, "test-batch-wizard-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "batch-wizard-tester",
        permissions.iter().map(|p| p.to_string()).collect(),
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-batch-wizard-secret").expect("token should encode")
}

async fn seed_warehouse_and_zone(pool: &PgPool, owner_id: Uuid) -> (Uuid, Uuid) {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO auth_owners (id, owner_code, owner_name)
        VALUES ($1, $2, '测试货主')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(owner_id)
    .bind(format!("OWN-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed owner");

    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, '向导测试仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");

    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, allowed_categories, is_external_use_zone, is_fragrant_zone,
            is_special_drug_zone, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'ZONE-WIZARD', '向导测试区', 'normal_10_30', 'qualified_green',
                '["drug"]'::jsonb, FALSE, FALSE, FALSE, 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("seed warehouse zone");

    (warehouse_id, zone_id)
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_batch_generate_high_rack_locations_success(pool: PgPool) {
    let now = Utc.with_ymd_and_hms(2026, 8, 18, 9, 0, 0).single().unwrap();
    ensure_audit_partition(&pool, now).await;
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_and_zone(&pool, owner_id).await;

    let app = test_batch_app(pool.clone());
    let token = bearer_token_with_permissions(
        owner_id,
        &["m1.master_data.read", "m1.location.batch-generate"],
    );

    let req_body = json!({
        "warehouse_id": warehouse_id,
        "zone_id": zone_id,
        "rule_type": "high_rack",
        "prefix": "HR",
        "row_start": 1,
        "row_end": 2,
        "column_start": 1,
        "column_end": 3,
        "layer_start": 1,
        "layer_end": 2,
        "max_volume_cm3": 120000,
        "max_sku_count": 5,
        "location_type": "storage",
        "allows_container": true,
        "mix_product_policy": "single_product_only",
        "mix_batch_policy": "single_batch"
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/master-data/locations/batch-generate")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("Idempotency-Key", Uuid::new_v4().to_string())
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    if status != StatusCode::OK {
        eprintln!(
            "RESPONSE ERROR: status={}, body={}",
            status,
            String::from_utf8_lossy(&body)
        );
    }
    assert_eq!(status, StatusCode::OK);

    let res: BatchGenerateLocationsResponse =
        serde_json::from_slice(&body).expect("parse response");

    // 2 rows * 3 columns * 2 layers = 12 locations
    assert_eq!(res.total_generated, 12);
    assert_eq!(res.locations.len(), 12);

    // Verify first and last generated location codes and attributes
    let first = &res.locations[0];
    assert_eq!(first.location_code, "HR-01-01-01");
    assert_eq!(first.row_no, 1);
    assert_eq!(first.column_no, 1);
    assert_eq!(first.layer_no, 1);
    assert_eq!(first.location_type, "storage");
    assert!(first.allows_container);
    assert_eq!(first.mix_product_policy, "single_product_only");
    assert_eq!(first.mix_batch_policy, "single_batch");
    assert_eq!(first.lock_status, "normal");
    assert_eq!(first.status, "available");
    assert_eq!(first.max_volume_cm3, 120000);
    assert_eq!(first.max_sku_count, 5);
    assert!(!first.is_agv_managed);
    assert_eq!(first.agv_pod_code, None);
    assert_eq!(first.pick_sequence_no, Some(1));
    assert_eq!(first.putaway_sequence_no, Some(1));

    let last = &res.locations[11];
    assert_eq!(last.location_code, "HR-02-03-02");
    assert_eq!(last.row_no, 2);
    assert_eq!(last.column_no, 3);
    assert_eq!(last.layer_no, 2);
    assert_eq!(last.pick_sequence_no, Some(12));
    assert_eq!(last.putaway_sequence_no, Some(12));

    // Verify sequence is strictly increasing from 1 to 12
    for (i, loc) in res.locations.iter().enumerate() {
        assert_eq!(loc.pick_sequence_no, Some((i + 1) as i32));
        assert_eq!(loc.putaway_sequence_no, Some((i + 1) as i32));
    }

    // Verify persisted in PostgreSQL
    let count: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM warehouse_locations WHERE owner_id = $1 AND warehouse_id = $2",
    )
    .bind(owner_id)
    .bind(warehouse_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count.0, 12);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_batch_generate_agv_pod_locations_success(pool: PgPool) {
    let now = Utc.with_ymd_and_hms(2026, 8, 18, 9, 0, 0).single().unwrap();
    ensure_audit_partition(&pool, now).await;
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_and_zone(&pool, owner_id).await;

    let app = test_batch_app(pool.clone());
    let token = bearer_token_with_permissions(
        owner_id,
        &["m1.master_data.read", "m1.location.batch-generate"],
    );

    let req_body = json!({
        "warehouse_id": warehouse_id,
        "zone_id": zone_id,
        "rule_type": "agv",
        "pod_prefix": "POD",
        "pod_start": 1,
        "pod_end": 2,
        "layer_start": 1,
        "layer_end": 3,
        "grid_start": 1,
        "grid_end": 4,
        "location_type": "storage"
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/master-data/locations/batch-generate")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("Idempotency-Key", Uuid::new_v4().to_string())
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let res: BatchGenerateLocationsResponse =
        serde_json::from_slice(&body).expect("parse response");

    // 2 pods * 3 layers * 4 grids = 24 locations
    assert_eq!(res.total_generated, 24);
    assert_eq!(res.locations.len(), 24);

    // Verify AGV Pod code format: POD[货架号]-F[层]-[格位] ↔ row_no=货架号, layer_no=层, column_no=格位
    let loc_f2_g3 = res
        .locations
        .iter()
        .find(|l| l.location_code == "POD01-F2-03")
        .expect("found POD01-F2-03");
    assert!(loc_f2_g3.is_agv_managed);
    assert_eq!(loc_f2_g3.agv_pod_code, Some("POD01".to_string()));
    assert_eq!(loc_f2_g3.row_no, 1);
    assert_eq!(loc_f2_g3.layer_no, 2);
    assert_eq!(loc_f2_g3.column_no, 3);
    assert!(loc_f2_g3.allows_container);

    let loc_p2_f3_g4 = res
        .locations
        .iter()
        .find(|l| l.location_code == "POD02-F3-04")
        .expect("found POD02-F3-04");
    assert!(loc_p2_f3_g4.is_agv_managed);
    assert_eq!(loc_p2_f3_g4.agv_pod_code, Some("POD02".to_string()));
    assert_eq!(loc_p2_f3_g4.row_no, 2);
    assert_eq!(loc_p2_f3_g4.layer_no, 3);
    assert_eq!(loc_p2_f3_g4.column_no, 4);

    // 与向导预览一致：货架 → 格 → 层；首个码 POD01-F1-01，第二为同格下一层
    assert_eq!(res.locations[0].location_code, "POD01-F1-01");
    assert_eq!(res.locations[1].location_code, "POD01-F2-01");
    // Sequence numbers should be continuous 1..=24
    for (i, loc) in res.locations.iter().enumerate() {
        assert_eq!(loc.pick_sequence_no, Some((i + 1) as i32));
        assert_eq!(loc.putaway_sequence_no, Some((i + 1) as i32));
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_batch_generate_custom_defaults_and_sequence_steps(pool: PgPool) {
    let now = Utc.with_ymd_and_hms(2026, 8, 18, 9, 0, 0).single().unwrap();
    ensure_audit_partition(&pool, now).await;
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_and_zone(&pool, owner_id).await;

    let app = test_batch_app(pool.clone());
    let token = bearer_token_with_permissions(
        owner_id,
        &["m1.master_data.read", "m1.location.batch-generate"],
    );

    let req_body = json!({
        "warehouse_id": warehouse_id,
        "zone_id": zone_id,
        "rule_type": "high_rack",
        "prefix": "PK",
        "row_start": 1,
        "row_end": 1,
        "column_start": 1,
        "column_end": 3,
        "layer_start": 1,
        "layer_end": 1,
        "location_type": "piece_pick",
        "allows_container": false,
        "mix_product_policy": "restricted_mix",
        "mix_batch_policy": "multi_batch",
        "pick_zone_level": "gold",
        "initial_pick_sequence": 100,
        "pick_sequence_step": 10,
        "initial_putaway_sequence": 200,
        "putaway_sequence_step": 5
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/master-data/locations/batch-generate")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("Idempotency-Key", Uuid::new_v4().to_string())
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let res: BatchGenerateLocationsResponse =
        serde_json::from_slice(&body).expect("parse response");

    assert_eq!(res.total_generated, 3);
    assert_eq!(res.locations[0].location_type, "piece_pick");
    assert!(!res.locations[0].allows_container);
    assert_eq!(res.locations[0].mix_product_policy, "restricted_mix");
    assert_eq!(res.locations[0].mix_batch_policy, "multi_batch");
    assert_eq!(res.locations[0].pick_zone_level.as_deref(), Some("gold"));

    assert_eq!(res.locations[0].pick_sequence_no, Some(100));
    assert_eq!(res.locations[0].putaway_sequence_no, Some(200));

    assert_eq!(res.locations[1].pick_sequence_no, Some(110));
    assert_eq!(res.locations[1].putaway_sequence_no, Some(205));

    assert_eq!(res.locations[2].pick_sequence_no, Some(120));
    assert_eq!(res.locations[2].putaway_sequence_no, Some(210));
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_batch_generate_duplicate_conflict_and_atomic_rollback(pool: PgPool) {
    let now = Utc.with_ymd_and_hms(2026, 8, 18, 9, 0, 0).single().unwrap();
    ensure_audit_partition(&pool, now).await;
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_and_zone(&pool, owner_id).await;

    // Pre-insert conflicting location: "DUP-01-02-01"
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status, created_at, updated_at
        )
        VALUES (
            $1, $2, $3, $4, 'DUP-01-02-01', 1, 2, 1,
            100000, 0, 3, 'storage', 'available', now(), now()
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(&pool)
    .await
    .expect("insert conflicting location");

    let app = test_batch_app(pool.clone());
    let token = bearer_token_with_permissions(
        owner_id,
        &["m1.master_data.read", "m1.location.batch-generate"],
    );

    let req_body = json!({
        "warehouse_id": warehouse_id,
        "zone_id": zone_id,
        "rule_type": "high_rack",
        "prefix": "DUP",
        "row_start": 1,
        "row_end": 1,
        "column_start": 1,
        "column_end": 3,
        "layer_start": 1,
        "layer_end": 1
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/master-data/locations/batch-generate")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("Idempotency-Key", Uuid::new_v4().to_string())
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    // Verify atomic rollback: Only the pre-existing 1 location exists; DUP-01-01-01 and DUP-01-03-01 were rolled back
    let count: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM warehouse_locations WHERE owner_id = $1 AND warehouse_id = $2",
    )
    .bind(owner_id)
    .bind(warehouse_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count.0, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_batch_generate_permission_enforcement(pool: PgPool) {
    let now = Utc.with_ymd_and_hms(2026, 8, 18, 9, 0, 0).single().unwrap();
    ensure_audit_partition(&pool, now).await;
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_and_zone(&pool, owner_id).await;

    let app = test_batch_app(pool.clone());

    // Token without m1.location.batch-generate
    let unauthorized_token =
        bearer_token_with_permissions(owner_id, &["m1.master_data.read", "m1.master_data.write"]);

    let req_body = json!({
        "warehouse_id": warehouse_id,
        "zone_id": zone_id,
        "rule_type": "high_rack",
        "prefix": "PERM",
        "row_start": 1,
        "row_end": 1,
        "column_start": 1,
        "column_end": 1,
        "layer_start": 1,
        "layer_end": 1
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/master-data/locations/batch-generate")
        .header(AUTHORIZATION, format!("Bearer {unauthorized_token}"))
        .header("content-type", "application/json")
        .header("Idempotency-Key", Uuid::new_v4().to_string())
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
