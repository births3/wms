use std::sync::Arc;

use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request, StatusCode},
};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
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
    ApplyContainerQualityLockRequest, ChangeContainerQualityLockReasonRequest,
    ReleaseContainerQualityLockRequest, CONTAINER_QUARANTINE_REASON_SALES_RETURN_PENDING,
    CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY, CONTAINER_REJECTED_REASON_EXPIRED,
    LPN_CONTAINER_STATUS_IDLE, LPN_LOCK_CATEGORY_QUALIFIED, LPN_LOCK_CATEGORY_QUARANTINE,
    LPN_LOCK_CATEGORY_REJECTED,
};

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

#[path = "support/lpn_container.rs"]
mod lpn_support;
mod postgres_test_support;
use lpn_support::{create_req, seed_lpn_numbering, setup_container_in_use};
use postgres_test_support::ensure_audit_partition;

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

async fn insert_test_mql_order(
    pool: &PgPool,
    owner_id: Uuid,
    mql_id: Uuid,
    status: &str,
    created_by: Uuid,
) {
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '测试操作人', 'test-hash', 'active') ON CONFLICT (id) DO NOTHING"
    )
    .bind(created_by)
    .bind(format!("user_{}", &created_by.to_string()[..8]))
    .execute(pool)
    .await
    .expect("insert auth user for mql");

    sqlx::query(
        r#"
        INSERT INTO quality_liaison_types (id, owner_id, type_code, type_name, approval_template_id, approver_user_id, timeout_seconds, enabled, created_by, created_at, updated_at, version)
        VALUES (gen_random_uuid(), $1, 'container_quality_defect', '容器质量异常', 'tpl_quality_default', $2, 3600, true, $2, now(), now(), 1)
        ON CONFLICT (owner_id, type_code) DO NOTHING
        "#,
    )
    .bind(owner_id)
    .bind(created_by)
    .execute(pool)
    .await
    .expect("seed liaison type");

    sqlx::query(
        r#"
        INSERT INTO quality_liaison_orders (
            id, owner_id, liaison_no, type_code, related_document_type, related_document_no,
            problem_description, disposition_suggestion, trigger_source, status, created_by, created_at, updated_at
        ) VALUES (
            $1, $2, $3, 'container_quality_defect', 'container_quality_lock', 'LPN-TEST',
            '容器异常待核查', '隔离复核', 'manual', $4, $5, now(), now()
        )
        "#,
    )
    .bind(mql_id)
    .bind(owner_id)
    .bind(format!("MQL-{}", &mql_id.to_string()[..8]))
    .bind(status)
    .bind(created_by)
    .execute(pool)
    .await
    .expect("insert mql order");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_quality_lock_lifecycle_flow(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "flow-test").await;

    assert_eq!(
        container.current_lock_category.as_deref(),
        Some(LPN_LOCK_CATEGORY_QUALIFIED)
    );
    assert_eq!(container.current_lock_reason_item_code, None);

    // 1. 加隔离锁 (quarantine)
    let lock_req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: Some("冷藏周转箱温度超标至 12℃".to_string()),
        evidence_urls: vec!["https://oss.example.com/temp_12c.jpg".to_string()],
        quality_liaison_id: None,
        witness_id,
        note: Some("双人见证现场测温".to_string()),
        create_liaison: false,
    };
    let locked = repo
        .quality_lock()
        .apply_quality_lock(&actor, container.id, lock_req, at(3), "lock-flow-1")
        .await
        .expect("apply quarantine lock");
    assert_eq!(
        locked.current_lock_category.as_deref(),
        Some(LPN_LOCK_CATEGORY_QUARANTINE)
    );
    assert_eq!(
        locked.current_lock_reason_item_code.as_deref(),
        Some(CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY)
    );

    // 2. 换原因 (change_reason) -> 改为销退待验
    let change_req = ChangeContainerQualityLockReasonRequest {
        lock_category: None,
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_SALES_RETURN_PENDING.to_string(),
        reason_desc: Some("复核确认改按销退待验流程".to_string()),
        evidence_urls: vec![],
        quality_liaison_id: None,
        witness_id,
        note: None,
    };
    let changed = repo
        .quality_lock()
        .change_quality_lock_reason(&actor, container.id, change_req, at(4), "lock-flow-2")
        .await
        .expect("change lock reason");
    assert_eq!(
        changed.current_lock_category.as_deref(),
        Some(LPN_LOCK_CATEGORY_QUARANTINE)
    );
    assert_eq!(
        changed.current_lock_reason_item_code.as_deref(),
        Some(CONTAINER_QUARANTINE_REASON_SALES_RETURN_PENDING)
    );

    // 3. 解锁 (release)
    let release_req = ReleaseContainerQualityLockRequest {
        witness_id,
        reason_desc: Some("验收入库核验合格，解除隔离".to_string()),
        quality_liaison_id: None,
        note: Some("见证人核验实物封签正常".to_string()),
    };
    let released = repo
        .quality_lock()
        .release_quality_lock(&actor, container.id, release_req, at(5), "lock-flow-3")
        .await
        .expect("release lock");
    assert_eq!(
        released.container.current_lock_category.as_deref(),
        Some(LPN_LOCK_CATEGORY_QUALIFIED)
    );
    assert_eq!(released.container.current_lock_reason_item_code, None);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_quality_lock_state_precondition(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);

    // Idle container cannot be locked
    let idle_container = repo
        .create(&actor, create_req(), at(1), "idle-create")
        .await
        .expect("create idle container");
    assert_eq!(idle_container.status, LPN_CONTAINER_STATUS_IDLE);

    let lock_req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: None,
        evidence_urls: vec![],
        quality_liaison_id: None,
        witness_id,
        note: None,
        create_liaison: false,
    };
    let err = repo
        .quality_lock()
        .apply_quality_lock(&actor, idle_container.id, lock_req, at(2), "idle-lock")
        .await
        .expect_err("idle container must be blocked from locking");

    assert_eq!(
        err,
        wms_api::lpn_container_repository::LpnContainerRepositoryError::StateInvalid
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_witness_validation_and_defense(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "witness-test").await;

    // Same person witness defense for locking
    let same_witness_req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: None,
        evidence_urls: vec![],
        quality_liaison_id: None,
        witness_id: operator_id, // SAME AS OPERATOR
        note: None,
        create_liaison: false,
    };
    let err = repo
        .quality_lock()
        .apply_quality_lock(
            &actor,
            container.id,
            same_witness_req,
            at(3),
            "witness-lock-err",
        )
        .await
        .expect_err("same person witness must be rejected");
    assert_eq!(
        err,
        wms_api::lpn_container_repository::LpnContainerRepositoryError::WitnessInvalid
    );

    // Apply valid lock first
    let witness_id = Uuid::new_v4();
    let valid_lock_req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: None,
        evidence_urls: vec![],
        quality_liaison_id: None,
        witness_id,
        note: None,
        create_liaison: false,
    };
    repo.quality_lock()
        .apply_quality_lock(
            &actor,
            container.id,
            valid_lock_req,
            at(4),
            "witness-lock-ok",
        )
        .await
        .expect("valid lock");

    // Same person witness defense for release
    let same_witness_release = ReleaseContainerQualityLockRequest {
        witness_id: operator_id, // SAME AS OPERATOR
        reason_desc: None,
        quality_liaison_id: None,
        note: None,
    };
    let release_err = repo
        .quality_lock()
        .release_quality_lock(
            &actor,
            container.id,
            same_witness_release,
            at(5),
            "witness-rel-err",
        )
        .await
        .expect_err("same person witness must be rejected on release");
    assert_eq!(
        release_err,
        wms_api::lpn_container_repository::LpnContainerRepositoryError::WitnessInvalid
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_mql_quality_liaison_association_and_release_gate(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "mql-test").await;

    // 1. Rejected lock requires quality_liaison_id
    let reject_no_mql = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_REJECTED.to_string(),
        reason_dict_item_code: CONTAINER_REJECTED_REASON_EXPIRED.to_string(),
        reason_desc: Some("近效期且已过期".to_string()),
        evidence_urls: vec![],
        quality_liaison_id: None, // Missing
        witness_id,
        note: None,
        create_liaison: false,
    };
    let mql_req_err = repo
        .quality_lock()
        .apply_quality_lock(&actor, container.id, reject_no_mql, at(3), "mql-req-err")
        .await
        .expect_err("rejected lock without mql must fail");
    assert_eq!(
        mql_req_err,
        wms_api::lpn_container_repository::LpnContainerRepositoryError::MqlRequired
    );

    // 2. Create M-QL in pending_approval state
    let mql_id = Uuid::new_v4();
    insert_test_mql_order(&pool, owner_id, mql_id, "pending_approval", operator_id).await;

    let reject_with_mql = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_REJECTED.to_string(),
        reason_dict_item_code: CONTAINER_REJECTED_REASON_EXPIRED.to_string(),
        reason_desc: Some("不合格品加锁".to_string()),
        evidence_urls: vec![],
        quality_liaison_id: Some(mql_id),
        witness_id,
        note: None,
        create_liaison: false,
    };
    repo.quality_lock()
        .apply_quality_lock(
            &actor,
            container.id,
            reject_with_mql,
            at(4),
            "mql-lock-pending",
        )
        .await
        .expect("rejected lock with mql should succeed");

    // 3. Attempt release while M-QL is pending_approval -> fails with MqlNotFinal
    let rel_req = ReleaseContainerQualityLockRequest {
        witness_id,
        reason_desc: Some("尝试提前解锁".to_string()),
        quality_liaison_id: Some(mql_id),
        note: None,
    };
    let not_final_err = repo
        .quality_lock()
        .release_quality_lock(&actor, container.id, rel_req, at(5), "mql-rel-pending")
        .await
        .expect_err("release must be blocked when M-QL is pending_approval");
    assert_eq!(
        not_final_err,
        wms_api::lpn_container_repository::LpnContainerRepositoryError::MqlNotFinal
    );

    // 4. Update M-QL to closed -> release succeeds!
    sqlx::query("UPDATE quality_liaison_orders SET status = 'closed' WHERE id = $1")
        .bind(mql_id)
        .execute(&pool)
        .await
        .expect("update mql closed");

    let rel_closed_req = ReleaseContainerQualityLockRequest {
        witness_id,
        reason_desc: Some("处置办结正常解锁".to_string()),
        quality_liaison_id: Some(mql_id),
        note: None,
    };
    let released = repo
        .quality_lock()
        .release_quality_lock(
            &actor,
            container.id,
            rel_closed_req,
            at(6),
            "mql-rel-closed",
        )
        .await
        .expect("release should succeed when M-QL is closed");
    assert_eq!(
        released.container.current_lock_category.as_deref(),
        Some(LPN_LOCK_CATEGORY_QUALIFIED)
    );

    // 5. Test rejected M-QL (驳回解锁，消灭审批死锁)
    let container2 = setup_container_in_use(&repo, &actor, "mql-reject-test").await;
    let mql_id_2 = Uuid::new_v4();
    insert_test_mql_order(&pool, owner_id, mql_id_2, "rejected", operator_id).await;

    let reject_lock_2 = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_REJECTED.to_string(),
        reason_dict_item_code: CONTAINER_REJECTED_REASON_EXPIRED.to_string(),
        reason_desc: Some("疑似过期加锁".to_string()),
        evidence_urls: vec![],
        quality_liaison_id: Some(mql_id_2),
        witness_id,
        note: None,
        create_liaison: false,
    };
    repo.quality_lock()
        .apply_quality_lock(&actor, container2.id, reject_lock_2, at(7), "mql-lock-rej")
        .await
        .expect("lock 2");

    let rel_rejected_mql = ReleaseContainerQualityLockRequest {
        witness_id,
        reason_desc: Some("审批驳回回退解锁".to_string()),
        quality_liaison_id: Some(mql_id_2),
        note: None,
    };
    let released_2 = repo
        .quality_lock()
        .release_quality_lock(
            &actor,
            container2.id,
            rel_rejected_mql,
            at(8),
            "mql-rel-rej",
        )
        .await
        .expect("release should succeed when M-QL is rejected (fallback release)");
    assert_eq!(
        released_2.container.current_lock_category.as_deref(),
        Some(LPN_LOCK_CATEGORY_QUALIFIED)
    );
}

/// 换原因强制双人见证：HTTP PATCH 不传 witness_id 被拒（422），且不产生 change_reason 事件。
#[sqlx::test(migrations = "../../migrations")]
async fn test_change_reason_without_witness_rejected(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "witness-missing").await;

    // 先正常加锁，使容器处于可换原因状态
    let lock_req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: None,
        evidence_urls: vec![],
        quality_liaison_id: None,
        witness_id,
        note: None,
        create_liaison: false,
    };
    repo.quality_lock()
        .apply_quality_lock(&actor, container.id, lock_req, at(3), "wm-lock-1")
        .await
        .expect("lock before change reason");

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

    // PATCH 换原因不传 witness_id
    let patch_body = json!({
        "reason_dict_item_code": "sales_return_pending",
        "reason_desc": "换为销退待验"
    });
    let req = Request::builder()
        .method("PATCH")
        .uri(format!(
            "/api/v1/master-data/lpn-containers/{}/quality-lock",
            container.id
        ))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("Idempotency-Key", "wm-patch-no-witness")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&patch_body).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "换原因不传 witness_id 必须被拒"
    );

    let change_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM container_quality_lock_events WHERE owner_id = $1 AND container_id = $2 AND event_type = 'change_reason'",
    )
    .bind(owner_id)
    .bind(container.id)
    .fetch_one(&pool)
    .await
    .expect("change_reason event count");
    assert_eq!(change_count, 0, "被拒请求不得写入审计事件");
}
