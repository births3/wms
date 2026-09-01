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
    master_data_handlers::{master_data_router, MasterDataAppState},
};
use wms_domain::BatchGenerateLocationsResponse;

mod postgres_test_support;

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

fn app(pool: PgPool) -> Router {
    master_data_router(MasterDataAppState::with_postgres(pool)).layer(auth_runtime_layer(
        AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore)),
    ))
}

fn token(owner_id: Uuid) -> String {
    let secret = "test-batch-t3-secret";
    std::env::set_var(JWT_SECRET_ENV, secret);
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "batch-t3",
        vec![
            "m1.master_data.read".to_string(),
            "m1.location.batch-generate".to_string(),
        ],
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, secret).expect("token")
}

async fn seed_warehouse_and_zone(pool: &PgPool, owner_id: Uuid) -> (Uuid, Uuid) {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '批量库位 T3 货主')",
    )
    .bind(owner_id)
    .bind(format!("BG-{}", &owner_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed owner");
    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status, created_at, updated_at
        ) VALUES ($1, $2, $3, '批量库位 T3 仓', 'physical', 'active', now(), now())
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone,
            quality_color, allowed_categories, is_external_use_zone, is_fragrant_zone,
            is_special_drug_zone, status, created_at, updated_at
        ) VALUES ($1, $2, $3, 'BG-ZONE', '批量库位 T3 区', 'normal_10_30', 'qualified_green',
                  '["drug"]'::jsonb, FALSE, FALSE, FALSE, 'active', now(), now())
        "#,
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("seed zone");
    (warehouse_id, zone_id)
}

#[sqlx::test(migrations = "../../migrations")]
async fn batch_generate_locations_replays_with_audit_and_idempotency(pool: PgPool) {
    // POST /api/v1/master-data/locations/batch-generate
    let owner_id = Uuid::new_v4();
    let (warehouse_id, zone_id) = seed_warehouse_and_zone(&pool, owner_id).await;
    postgres_test_support::ensure_audit_partition(&pool, Utc::now()).await;

    let router = app(pool.clone());
    let bearer = token(owner_id);
    let idempotency_key = "batch-generate-t3-1";
    let request_body = json!({
        "warehouse_id": warehouse_id,
        "zone_id": zone_id,
        "rule_type": "high_rack",
        "prefix": "T3",
        "row_start": 1,
        "row_end": 1,
        "column_start": 1,
        "column_end": 2,
        "layer_start": 1,
        "layer_end": 1,
        "max_volume_cm3": 120000,
        "max_sku_count": 5,
        "location_type": "storage",
        "allows_container": true,
        "mix_product_policy": "single_product_only",
        "mix_batch_policy": "single_batch"
    });

    let send = |router: Router| {
        let bearer = bearer.clone();
        let request_body = request_body.clone();
        async move {
            router
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/master-data/locations/batch-generate")
                        .header(AUTHORIZATION, format!("Bearer {bearer}"))
                        .header("content-type", "application/json")
                        .header("Idempotency-Key", idempotency_key)
                        .body(Body::from(request_body.to_string()))
                        .expect("request"),
                )
                .await
                .expect("oneshot")
        }
    };

    let first_response = send(router.clone()).await;
    assert_eq!(first_response.status(), StatusCode::OK);
    let first_body = to_bytes(first_response.into_body(), usize::MAX)
        .await
        .expect("first body");
    let first: BatchGenerateLocationsResponse =
        serde_json::from_slice(&first_body).expect("first response");
    assert_eq!(first.total_generated, 2);

    let replay_response = send(router).await;
    assert_eq!(replay_response.status(), StatusCode::OK);
    let replay_body = to_bytes(replay_response.into_body(), usize::MAX)
        .await
        .expect("replay body");
    let replay: BatchGenerateLocationsResponse =
        serde_json::from_slice(&replay_body).expect("replay response");
    assert_eq!(replay.total_generated, first.total_generated);
    assert_eq!(
        replay
            .locations
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        first.locations.iter().map(|row| row.id).collect::<Vec<_>>()
    );

    let location_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM warehouse_locations WHERE owner_id = $1 AND warehouse_id = $2",
    )
    .bind(owner_id)
    .bind(warehouse_id)
    .fetch_one(&pool)
    .await
    .expect("location count");
    assert_eq!(
        location_count, 2,
        "idempotent replay must not duplicate locations"
    );

    postgres_test_support::audit_event(&pool, owner_id, 1).await;
    postgres_test_support::idempotency_request(&pool, owner_id, idempotency_key).await;
}
