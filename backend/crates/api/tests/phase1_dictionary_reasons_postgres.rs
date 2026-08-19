use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request, StatusCode},
};
use chrono::{TimeZone, Utc};
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
    system_dictionary::{
        validate_container_quality_lock_reason_in_tx, PgSystemDictionaryRepository,
    },
    system_dictionary_handlers::{system_dictionary_router, SystemDictionaryAppState},
};
use wms_domain::{
    DisableSystemDictionaryItemRequest, SystemDictionaryItemListResponse,
    UpsertSystemDictionaryItemRequest, CONTAINER_QUARANTINE_REASON_DAMAGED_PENDING_INSPECT,
    CONTAINER_QUARANTINE_REASON_ROUTINE_SAMPLING, CONTAINER_QUARANTINE_REASON_SALES_RETURN_PENDING,
    CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY, CONTAINER_REJECTED_REASON_DAMAGED_LEAKAGE,
    CONTAINER_REJECTED_REASON_EXPIRED, CONTAINER_REJECTED_REASON_INSPECTION_FAILED,
    CONTAINER_REJECTED_REASON_REGULATORY_RECALL, SYSTEM_DICTIONARY_CONTAINER_QUARANTINE_REASON,
    SYSTEM_DICTIONARY_CONTAINER_REJECTED_REASON,
};

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
        _changed_at: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }
}

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "quality-lock-dictionary-test".to_string(),
        permissions: vec![
            "m1.system_dictionary.read".to_string(),
            "m1.system_dictionary.write".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn auth_token(user_id: Uuid, owner_id: Uuid, permissions: Vec<String>) -> String {
    let secret = "test-dictionary-reasons-secret";
    std::env::set_var(JWT_SECRET_ENV, secret);
    let claims = wms_api::auth::Claims {
        sub: user_id,
        owner_id,
        user_name: "test-dictionary-user".to_string(),
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

#[sqlx::test(migrations = "../../migrations")]
async fn container_quarantine_reason_presets_are_queryable(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool);
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 8, 18, 9, 0, 0)
        .single()
        .expect("valid time");

    let items = repo
        .list_effective_items(
            &ctx(owner_id),
            SYSTEM_DICTIONARY_CONTAINER_QUARANTINE_REASON,
            now,
        )
        .await
        .expect("container_quarantine_reason presets should be queryable");

    let codes: Vec<_> = items.iter().map(|item| item.item_code.as_str()).collect();
    assert_eq!(
        codes,
        vec![
            CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY,
            CONTAINER_QUARANTINE_REASON_DAMAGED_PENDING_INSPECT,
            CONTAINER_QUARANTINE_REASON_SALES_RETURN_PENDING,
            CONTAINER_QUARANTINE_REASON_ROUTINE_SAMPLING,
        ]
    );

    assert!(items.iter().all(|item| item.enabled));
    assert!(items.iter().all(|item| item.source == "global"));

    let temp_item = items
        .iter()
        .find(|i| i.item_code == CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY)
        .unwrap();
    assert_eq!(temp_item.item_name, "温控异常");

    let damaged_item = items
        .iter()
        .find(|i| i.item_code == CONTAINER_QUARANTINE_REASON_DAMAGED_PENDING_INSPECT)
        .unwrap();
    assert_eq!(damaged_item.item_name, "包装破损待检");

    let return_item = items
        .iter()
        .find(|i| i.item_code == CONTAINER_QUARANTINE_REASON_SALES_RETURN_PENDING)
        .unwrap();
    assert_eq!(return_item.item_name, "销退待验");

    let sampling_item = items
        .iter()
        .find(|i| i.item_code == CONTAINER_QUARANTINE_REASON_ROUTINE_SAMPLING)
        .unwrap();
    assert_eq!(sampling_item.item_name, "例行抽样");
}

#[sqlx::test(migrations = "../../migrations")]
async fn container_rejected_reason_presets_are_queryable(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool);
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 8, 18, 9, 0, 0)
        .single()
        .expect("valid time");

    let items = repo
        .list_effective_items(
            &ctx(owner_id),
            SYSTEM_DICTIONARY_CONTAINER_REJECTED_REASON,
            now,
        )
        .await
        .expect("container_rejected_reason presets should be queryable");

    let codes: Vec<_> = items.iter().map(|item| item.item_code.as_str()).collect();
    assert_eq!(
        codes,
        vec![
            CONTAINER_REJECTED_REASON_EXPIRED,
            CONTAINER_REJECTED_REASON_DAMAGED_LEAKAGE,
            CONTAINER_REJECTED_REASON_INSPECTION_FAILED,
            CONTAINER_REJECTED_REASON_REGULATORY_RECALL,
        ]
    );

    assert!(items.iter().all(|item| item.enabled));
    assert!(items.iter().all(|item| item.source == "global"));

    let expired_item = items
        .iter()
        .find(|i| i.item_code == CONTAINER_REJECTED_REASON_EXPIRED)
        .unwrap();
    assert_eq!(expired_item.item_name, "药品过期");

    let leakage_item = items
        .iter()
        .find(|i| i.item_code == CONTAINER_REJECTED_REASON_DAMAGED_LEAKAGE)
        .unwrap();
    assert_eq!(leakage_item.item_name, "破损泄漏");

    let inspect_item = items
        .iter()
        .find(|i| i.item_code == CONTAINER_REJECTED_REASON_INSPECTION_FAILED)
        .unwrap();
    assert_eq!(inspect_item.item_name, "检验不合格");

    let recall_item = items
        .iter()
        .find(|i| i.item_code == CONTAINER_REJECTED_REASON_REGULATORY_RECALL)
        .unwrap();
    assert_eq!(recall_item.item_name, "药监召回");
}

#[sqlx::test(migrations = "../../migrations")]
async fn container_quality_lock_reasons_api_query(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let app = system_dictionary_router(SystemDictionaryAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let token = auth_token(
        user_id,
        owner_id,
        vec!["m1.system_dictionary.read".to_string()],
    );

    // Query quarantine reasons via HTTP API
    let req = Request::builder()
        .uri(format!(
            "/api/v1/system-dictionaries/{}/items",
            SYSTEM_DICTIONARY_CONTAINER_QUARANTINE_REASON
        ))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: SystemDictionaryItemListResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed.data.len(), 4);
    assert!(parsed
        .data
        .iter()
        .any(|i| i.item_code == CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY));

    // Query rejected reasons via HTTP API
    let req = Request::builder()
        .uri(format!(
            "/api/v1/system-dictionaries/{}/items",
            SYSTEM_DICTIONARY_CONTAINER_REJECTED_REASON
        ))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: SystemDictionaryItemListResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed.data.len(), 4);
    assert!(parsed
        .data
        .iter()
        .any(|i| i.item_code == CONTAINER_REJECTED_REASON_EXPIRED));
}

#[sqlx::test(migrations = "../../migrations")]
async fn container_quality_lock_reason_validation_and_disable_blocking(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool.clone());
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let auth_a = ctx(owner_a);
    let now = Utc
        .with_ymd_and_hms(2026, 8, 18, 10, 0, 0)
        .single()
        .expect("valid time");
    ensure_audit_partition(&pool, now).await;

    // 1. Valid enabled reason for quarantine
    let valid_quarantine = repo
        .validate_container_quality_lock_reason(
            owner_a,
            "quarantine",
            CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY,
            now,
        )
        .await
        .expect("validate quarantine reason");
    assert!(valid_quarantine);

    // 2. Mismatched category: rejected reason used for quarantine -> false
    let mismatch_quarantine = repo
        .validate_container_quality_lock_reason(
            owner_a,
            "quarantine",
            CONTAINER_REJECTED_REASON_EXPIRED,
            now,
        )
        .await
        .expect("validate mismatched quarantine reason");
    assert!(!mismatch_quarantine);

    // 3. Valid enabled reason for rejected
    let valid_rejected = repo
        .validate_container_quality_lock_reason(
            owner_a,
            "rejected",
            CONTAINER_REJECTED_REASON_EXPIRED,
            now,
        )
        .await
        .expect("validate rejected reason");
    assert!(valid_rejected);

    // 4. Non-existent code -> false
    let invalid_code = repo
        .validate_container_quality_lock_reason(
            owner_a,
            "quarantine",
            "nonexistent_reason_code",
            now,
        )
        .await
        .expect("validate invalid reason code");
    assert!(!invalid_code);

    // 5. In-tx validation helper
    let mut tx = pool.begin().await.unwrap();
    let in_tx_valid = validate_container_quality_lock_reason_in_tx(
        &mut tx,
        owner_a,
        "quarantine",
        CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY,
        now,
    )
    .await
    .unwrap();
    assert!(in_tx_valid);
    tx.rollback().await.unwrap();

    // 6. Owner A overrides and disables temp_anomaly -> owner A validation fails (blocked), owner B still passes
    repo.upsert_item(
        &auth_a,
        SYSTEM_DICTIONARY_CONTAINER_QUARANTINE_REASON,
        CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY,
        UpsertSystemDictionaryItemRequest {
            owner_id: Some(owner_a),
            item_name: "温控异常".to_string(),
            enabled: true,
            sort_order: 10,
            params: json!({}),
            effective_from: None,
            effective_to: None,
        },
        now,
        "owner-a-override-temp-anomaly",
    )
    .await
    .expect("upsert owner A override");

    repo.disable_item(
        &auth_a,
        SYSTEM_DICTIONARY_CONTAINER_QUARANTINE_REASON,
        CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY,
        DisableSystemDictionaryItemRequest {
            owner_id: Some(owner_a),
            disabled_reason: Some("货主 A 暂不使用温控异常作为隔离原因".to_string()),
        },
        now,
        "disable-temp-anomaly-owner-a",
    )
    .await
    .expect("disable item for owner A");

    let disabled_for_a = repo
        .validate_container_quality_lock_reason(
            owner_a,
            "quarantine",
            CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY,
            now,
        )
        .await
        .expect("validate disabled reason for owner A");
    assert!(
        !disabled_for_a,
        "disabled reason must be blocked for owner A"
    );

    let enabled_for_b = repo
        .validate_container_quality_lock_reason(
            owner_b,
            "quarantine",
            CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY,
            now,
        )
        .await
        .expect("validate reason for owner B");
    assert!(
        enabled_for_b,
        "global reason must remain active for owner B"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn container_quality_lock_reason_owner_customization_and_audit(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool.clone());
    let owner_id = Uuid::new_v4();
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 8, 18, 11, 0, 0)
        .single()
        .expect("valid time");
    ensure_audit_partition(&pool, now).await;

    // Owner creates custom quarantine reason
    let custom_req = UpsertSystemDictionaryItemRequest {
        owner_id: Some(owner_id),
        item_name: "供应商外包装受潮待验".to_string(),
        enabled: true,
        sort_order: 50,
        params: json!({}),
        effective_from: None,
        effective_to: None,
    };

    let created = repo
        .upsert_item(
            &auth,
            SYSTEM_DICTIONARY_CONTAINER_QUARANTINE_REASON,
            "supplier_package_damp",
            custom_req,
            now,
            "custom-quarantine-reason-key",
        )
        .await
        .expect("create custom quarantine reason");

    assert_eq!(created.value.item_name, "供应商外包装受潮待验");
    assert_eq!(created.value.owner_id, Some(owner_id));

    // Validate custom reason is available for this owner
    let is_valid = repo
        .validate_container_quality_lock_reason(
            owner_id,
            "quarantine",
            "supplier_package_damp",
            now,
        )
        .await
        .expect("validate custom reason");
    assert!(is_valid);

    // Other owner cannot see or use this custom reason
    let other_owner = Uuid::new_v4();
    let other_valid = repo
        .validate_container_quality_lock_reason(
            other_owner,
            "quarantine",
            "supplier_package_damp",
            now,
        )
        .await
        .expect("validate other owner custom reason");
    assert!(!other_valid);

    // Verify audit event
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'upsert_system_dictionary_item' AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(created.value.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("count audit event");
    assert_eq!(audit_count, 1);
}
