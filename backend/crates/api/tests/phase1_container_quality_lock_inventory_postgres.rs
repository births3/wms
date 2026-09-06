use axum::{
    body::{to_bytes, Body},
    http::{header::AUTHORIZATION, Request, StatusCode},
};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, AuthContext, AuthRevocationStore, AuthRevocationStoreError,
        AuthRuntimePolicy, JWT_SECRET_ENV,
    },
    lpn_container_handlers::{lpn_container_router, LpnContainerAppState},
    lpn_container_repository::PgLpnContainerRepository,
};
use wms_domain::{
    ApplyContainerQualityLockRequest, ReleaseContainerQualityLockRequest,
    CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY, LPN_LOCK_CATEGORY_QUARANTINE,
};

#[path = "support/lpn_container.rs"]
mod lpn_support;
mod postgres_test_support;
use lpn_support::{seed_lpn_numbering, setup_container_in_use};
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
        _changed_at: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }
}

fn auth_token(user_id: Uuid, owner_id: Uuid, permissions: Vec<String>) -> String {
    let secret = "test-quality-lock-jwt-secret-key-12345";
    std::env::set_var(JWT_SECRET_ENV, secret);
    let claims = wms_api::auth::Claims {
        sub: user_id,
        owner_id,
        user_name: "quality-lock-operator".to_string(),
        permissions,
        jti: Uuid::new_v4().to_string(),
        iat: Utc::now().timestamp(),
        exp: (Utc::now() + chrono::Duration::hours(1)).timestamp(),
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode token")
}

fn test_ctx(user_id: Uuid, owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "test-operator".to_string(),
        permissions: vec![
            "m1.master_data.read".to_string(),
            "m1.master_data.write".to_string(),
            "m1.quality-lock.manage".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 18, hour, 0, 0)
        .single()
        .expect("valid timestamp")
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_inventory_batches_linkage_and_allocation_release(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "inv-link-test").await;

    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();

    // Insert warehouse, zone, location, product, batch
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type) VALUES ($1, $2, 'WH-QL-1', '质量锁测试仓', 'normal') ON CONFLICT DO NOTHING",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("warehouse");

    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color) VALUES ($1, $2, $3, 'ZONE-QL-1', '合格区', 'normal_10_30', 'green') ON CONFLICT DO NOTHING",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await
    .expect("zone");

    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, location_type, status) VALUES ($1, $2, $3, $4, 'LOC-QL-1', 1, 1, 1, 10000000, 'storage', 'occupied') ON CONFLICT DO NOTHING",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(&pool)
    .await
    .expect("location");

    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, special_drug_category) VALUES ($1, $2, 'PROD-QL-1', '质量锁测试药品', '10mg*100片/盒', 'normal_10_30', 'none') ON CONFLICT DO NOTHING",
    )
    .bind(product_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("product");

    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, warehouse_id, zone_id, location_id, product_id, product_code, batch_no, production_date, expiry_date,
            container_lpn, qty_on_hand, qty_allocated, qty_frozen, status
        ) VALUES (
            $1, $2, $3, $4, $5, $6, 'PROD-QL-1', 'BATCH-20260818-01', '2026-01-01', '2027-12-31',
            $7, 100, 25, 0, 'qualified'
        )
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(location_id)
    .bind(product_id)
    .bind(&container.lpn_code)
    .execute(&pool)
    .await
    .expect("batch");

    // Pre-check batch
    let (pre_status, pre_alloc): (String, i64) =
        sqlx::query_as("SELECT status, qty_allocated::bigint FROM inventory_batches WHERE id = $1")
            .bind(batch_id)
            .fetch_one(&pool)
            .await
            .expect("fetch pre batch");
    assert_eq!(pre_status, "qualified");
    assert_eq!(pre_alloc, 25);

    // Apply lock on container
    let req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: Some("温控异常待查".to_string()),
        evidence_urls: vec![],
        witness_id,
        quality_liaison_id: None,
        note: None,
        create_liaison: false,
    };
    repo.quality_lock()
        .apply_quality_lock(&actor, container.id, req, at(3), "inv-lock-1")
        .await
        .expect("apply lock");

    // Verify batch status linked to quarantined and allocated released
    let (post_status, post_alloc): (String, i64) =
        sqlx::query_as("SELECT status, qty_allocated::bigint FROM inventory_batches WHERE id = $1")
            .bind(batch_id)
            .fetch_one(&pool)
            .await
            .expect("fetch post batch");
    assert_eq!(post_status, "quarantined");
    assert_eq!(post_alloc, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_precise_batch_write_back_on_release(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "prec-test").await;

    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let batch_1 = Uuid::new_v4();
    let batch_2 = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type) VALUES ($1, $2, 'WH-QL-2', '精准回写测试仓', 'normal') ON CONFLICT DO NOTHING",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("warehouse");

    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color) VALUES ($1, $2, $3, 'ZONE-QL-2', '合格区', 'normal_10_30', 'green') ON CONFLICT DO NOTHING",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await
    .expect("zone");

    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, location_type, status) VALUES ($1, $2, $3, $4, 'LOC-QL-2', 1, 1, 1, 10000000, 'storage', 'occupied') ON CONFLICT DO NOTHING",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(&pool)
    .await
    .expect("location");

    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, special_drug_category) VALUES ($1, $2, 'PROD-QL-2', '精准回写药品', '10mg*100片/盒', 'normal_10_30', 'none') ON CONFLICT DO NOTHING",
    )
    .bind(product_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("product");

    // Insert 2 batches
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, warehouse_id, zone_id, location_id, product_id, product_code, batch_no, production_date, expiry_date,
            container_lpn, qty_on_hand, qty_allocated, qty_frozen, status
        ) VALUES
        ($1, $2, $3, $4, $5, $6, 'PROD-QL-2', 'BATCH-A', '2026-01-01', '2027-12-31', $7, 50, 0, 0, 'qualified'),
        ($8, $2, $3, $4, $5, $6, 'PROD-QL-2', 'BATCH-B', '2026-01-01', '2027-12-31', $7, 50, 0, 0, 'qualified')
        "#,
    )
    .bind(batch_1)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(location_id)
    .bind(product_id)
    .bind(&container.lpn_code)
    .bind(batch_2)
    .execute(&pool)
    .await
    .expect("batches");

    // Apply quarantine lock
    let req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: Some("温控异常".to_string()),
        evidence_urls: vec![],
        witness_id,
        quality_liaison_id: None,
        note: None,
        create_liaison: false,
    };
    repo.quality_lock()
        .apply_quality_lock(&actor, container.id, req, at(3), "prec-lock")
        .await
        .expect("lock");

    // Simulate batch_2 being changed by another process (e.g. loss deduction or damage)
    sqlx::query("UPDATE inventory_batches SET status = 'loss_deducted' WHERE id = $1")
        .bind(batch_2)
        .execute(&pool)
        .await
        .expect("update B2 status");

    // Unlock container
    let rel_req = ReleaseContainerQualityLockRequest {
        witness_id,
        reason_desc: Some("核验合格解除".to_string()),
        quality_liaison_id: None,
        note: None,
    };
    repo.quality_lock()
        .release_quality_lock(&actor, container.id, rel_req, at(4), "prec-rel")
        .await
        .expect("release");

    // B1 was quarantined by this lock and untouched -> reverts to qualified
    let s1: String = sqlx::query_scalar("SELECT status FROM inventory_batches WHERE id = $1")
        .bind(batch_1)
        .fetch_one(&pool)
        .await
        .expect("fetch s1");
    assert_eq!(s1, "qualified");

    // B2 was changed to loss_deducted -> must NOT be overwritten to qualified!
    let s2: String = sqlx::query_scalar("SELECT status FROM inventory_batches WHERE id = $1")
        .bind(batch_2)
        .fetch_one(&pool)
        .await
        .expect("fetch s2");
    assert_eq!(s2, "loss_deducted");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_pure_insert_audit_table_and_http_endpoints(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "http-test").await;

    let token = auth_token(
        operator_id,
        owner_id,
        vec![
            "m1.master_data.read".to_string(),
            "m1.master_data.write".to_string(),
            "m1.quality-lock.manage".to_string(),
        ],
    );

    let app = lpn_container_router(LpnContainerAppState {
        repository: Arc::new(repo.clone()),
    })
    .layer(auth_runtime_layer(AuthRuntimePolicy::strict(Arc::new(
        AllowAllRevocationStore,
    ))));

    // 1. HTTP POST apply lock
    let lock_body = json!({
        "lock_category": "quarantine",
        "reason_dict_item_code": "temp_anomaly",
        "reason_desc": "HTTP 测试加锁",
        "evidence_urls": ["https://oss.example.com/test.jpg"],
        "witness_id": witness_id,
        "note": "现场封存"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/master-data/lpn-containers/{}/quality-lock",
            container.id
        ))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("Idempotency-Key", "http-lock-1")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&lock_body).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. HTTP Missing Idempotency-Key -> 400
    let req_no_idem = Request::builder()
        .method("PATCH")
        .uri(format!(
            "/api/v1/master-data/lpn-containers/{}/quality-lock",
            container.id
        ))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "reason_dict_item_code": "sales_return_pending",
                "witness_id": witness_id
            }))
            .unwrap(),
        ))
        .unwrap();
    let res_no_idem = app.clone().oneshot(req_no_idem).await.unwrap();
    assert_eq!(res_no_idem.status(), StatusCode::BAD_REQUEST);

    // 3. HTTP PATCH change reason
    let patch_body = json!({
        "reason_dict_item_code": "sales_return_pending",
        "reason_desc": "换为销退待验",
        "witness_id": witness_id
    });
    let patch_req = Request::builder()
        .method("PATCH")
        .uri(format!(
            "/api/v1/master-data/lpn-containers/{}/quality-lock",
            container.id
        ))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("Idempotency-Key", "http-patch-1")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&patch_body).unwrap()))
        .unwrap();
    let patch_res = app.clone().oneshot(patch_req).await.unwrap();
    assert_eq!(patch_res.status(), StatusCode::OK);

    // 4. HTTP POST release
    let rel_body = json!({
        "witness_id": witness_id,
        "reason_desc": "复检合格解除",
        "note": "经双人确认"
    });
    let rel_req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/master-data/lpn-containers/{}/quality-lock/release",
            container.id
        ))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("Idempotency-Key", "http-rel-1")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&rel_body).unwrap()))
        .unwrap();
    let rel_res = app.clone().oneshot(rel_req).await.unwrap();
    assert_eq!(rel_res.status(), StatusCode::OK);

    // 5. Verify container_quality_lock_events has 3 pure INSERT records
    let event_types: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM container_quality_lock_events WHERE owner_id = $1 AND container_id = $2 ORDER BY occurred_at ASC",
    )
    .bind(owner_id)
    .bind(container.id)
    .fetch_all(&pool)
    .await
    .expect("fetch lock events");

    assert_eq!(
        event_types,
        vec![
            "lock".to_string(),
            "change_reason".to_string(),
            "release".to_string()
        ]
    );
}

/// L4：无 `m1.quality-lock.manage` 权限调用加锁/换原因/解锁接口均被拒绝（403 AUTH-005），
/// 且不产生任何审计事件；带权限的对照组可正常加锁，证明权限点是唯一拦截。
#[sqlx::test(migrations = "../../migrations")]
async fn test_quality_lock_endpoints_require_manage_permission(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "no-perm-test").await;

    let no_perm_token = auth_token(
        operator_id,
        owner_id,
        vec![
            "m1.master_data.read".to_string(),
            "m1.master_data.write".to_string(),
        ],
    );
    let app = lpn_container_router(LpnContainerAppState {
        repository: Arc::new(repo.clone()),
    })
    .layer(auth_runtime_layer(AuthRuntimePolicy::strict(Arc::new(
        AllowAllRevocationStore,
    ))));

    // 1. POST 加锁 → 403 AUTH-005
    let lock_body = json!({
        "lock_category": "quarantine",
        "reason_dict_item_code": "temp_anomaly",
        "witness_id": witness_id
    });
    let lock_req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/master-data/lpn-containers/{}/quality-lock",
            container.id
        ))
        .header(AUTHORIZATION, format!("Bearer {no_perm_token}"))
        .header("Idempotency-Key", "no-perm-lock")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&lock_body).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(lock_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let error_body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let error: serde_json::Value = serde_json::from_slice(&error_body).unwrap();
    assert_eq!(error["code"], "AUTH-005");

    // 2. PATCH 换原因 → 403
    let patch_body = json!({
        "reason_dict_item_code": "sales_return_pending",
        "witness_id": witness_id
    });
    let patch_req = Request::builder()
        .method("PATCH")
        .uri(format!(
            "/api/v1/master-data/lpn-containers/{}/quality-lock",
            container.id
        ))
        .header(AUTHORIZATION, format!("Bearer {no_perm_token}"))
        .header("Idempotency-Key", "no-perm-patch")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&patch_body).unwrap()))
        .unwrap();
    let patch_res = app.clone().oneshot(patch_req).await.unwrap();
    assert_eq!(patch_res.status(), StatusCode::FORBIDDEN);

    // 3. POST 解锁 → 403
    let rel_body = json!({
        "witness_id": witness_id,
        "reason_desc": "无权限尝试解锁"
    });
    let rel_req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/master-data/lpn-containers/{}/quality-lock/release",
            container.id
        ))
        .header(AUTHORIZATION, format!("Bearer {no_perm_token}"))
        .header("Idempotency-Key", "no-perm-rel")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&rel_body).unwrap()))
        .unwrap();
    let rel_res = app.clone().oneshot(rel_req).await.unwrap();
    assert_eq!(rel_res.status(), StatusCode::FORBIDDEN);

    // 被拒请求不得产生审计事件与容器状态变更
    let event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM container_quality_lock_events WHERE owner_id = $1 AND container_id = $2",
    )
    .bind(owner_id)
    .bind(container.id)
    .fetch_one(&pool)
    .await
    .expect("event count");
    assert_eq!(event_count, 0, "无权限请求不得写入审计事件");
    let category: Option<String> =
        sqlx::query_scalar("SELECT current_lock_category FROM lpn_containers WHERE id = $1")
            .bind(container.id)
            .fetch_one(&pool)
            .await
            .expect("lock category");
    assert_eq!(category.as_deref(), Some("qualified"));

    // 对照组：带 m1.quality-lock.manage 权限加锁成功
    let with_perm_token = auth_token(
        operator_id,
        owner_id,
        vec![
            "m1.master_data.read".to_string(),
            "m1.master_data.write".to_string(),
            "m1.quality-lock.manage".to_string(),
        ],
    );
    let ok_req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/master-data/lpn-containers/{}/quality-lock",
            container.id
        ))
        .header(AUTHORIZATION, format!("Bearer {with_perm_token}"))
        .header("Idempotency-Key", "perm-ok-lock")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&lock_body).unwrap()))
        .unwrap();
    let ok_res = app.clone().oneshot(ok_req).await.unwrap();
    assert_eq!(ok_res.status(), StatusCode::OK);
}

/// L11（HTTP 层）：同一 Idempotency-Key 顺序重放加锁/解锁不重复生成事件；
/// 同键不同请求体返回 409 M1_LPN_IDEMPOTENCY_CONFLICT。
#[sqlx::test(migrations = "../../migrations")]
async fn test_http_idempotency_key_replay_no_duplicate_events(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "http-idem-test").await;

    let token = auth_token(
        operator_id,
        owner_id,
        vec![
            "m1.master_data.read".to_string(),
            "m1.master_data.write".to_string(),
            "m1.quality-lock.manage".to_string(),
        ],
    );
    let app = lpn_container_router(LpnContainerAppState {
        repository: Arc::new(repo.clone()),
    })
    .layer(auth_runtime_layer(AuthRuntimePolicy::strict(Arc::new(
        AllowAllRevocationStore,
    ))));

    let lock_body = json!({
        "lock_category": "quarantine",
        "reason_dict_item_code": "temp_anomaly",
        "reason_desc": "HTTP 幂等重放加锁",
        "witness_id": witness_id
    });
    let post_lock = || {
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/v1/master-data/lpn-containers/{}/quality-lock",
                container.id
            ))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("Idempotency-Key", "http-idem-lock-1")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&lock_body).unwrap()))
            .unwrap()
    };
    let first = app.clone().oneshot(post_lock()).await.unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let replay = app.clone().oneshot(post_lock()).await.unwrap();
    assert_eq!(replay.status(), StatusCode::OK, "同键同体重放应返回成功");

    let event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM container_quality_lock_events WHERE owner_id = $1 AND container_id = $2",
    )
    .bind(owner_id)
    .bind(container.id)
    .fetch_one(&pool)
    .await
    .expect("event count");
    assert_eq!(event_count, 1, "HTTP 重放不得重复生成事件");

    // 同键不同请求体 → 409 M1_LPN_IDEMPOTENCY_CONFLICT
    let diff_body = json!({
        "lock_category": "quarantine",
        "reason_dict_item_code": "sales_return_pending",
        "witness_id": witness_id
    });
    let diff_req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/master-data/lpn-containers/{}/quality-lock",
            container.id
        ))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("Idempotency-Key", "http-idem-lock-1")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&diff_body).unwrap()))
        .unwrap();
    let conflict = app.clone().oneshot(diff_req).await.unwrap();
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let conflict_body = to_bytes(conflict.into_body(), usize::MAX).await.unwrap();
    let conflict_json: serde_json::Value = serde_json::from_slice(&conflict_body).unwrap();
    assert_eq!(conflict_json["code"], "M1_LPN_IDEMPOTENCY_CONFLICT");

    // 解锁 + 同键重放：仅一条 release 事件
    let rel_body = json!({
        "witness_id": witness_id,
        "reason_desc": "HTTP 幂等解锁"
    });
    let post_release = || {
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/v1/master-data/lpn-containers/{}/quality-lock/release",
                container.id
            ))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("Idempotency-Key", "http-idem-rel-1")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&rel_body).unwrap()))
            .unwrap()
    };
    let rel = app.clone().oneshot(post_release()).await.unwrap();
    assert_eq!(rel.status(), StatusCode::OK);
    let rel_replay = app.clone().oneshot(post_release()).await.unwrap();
    assert_eq!(rel_replay.status(), StatusCode::OK);

    let event_types: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM container_quality_lock_events WHERE owner_id = $1 AND container_id = $2 ORDER BY occurred_at",
    )
    .bind(owner_id)
    .bind(container.id)
    .fetch_all(&pool)
    .await
    .expect("event types");
    assert_eq!(
        event_types,
        vec!["lock".to_string(), "release".to_string()],
        "解锁 HTTP 重放不得重复生成 release 事件"
    );
}
