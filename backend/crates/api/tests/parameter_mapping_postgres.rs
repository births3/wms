use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header::AUTHORIZATION, Request, StatusCode},
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
    parameter_mapping::{parameter_mapping_router, ParameterMappingAppState},
};
use wms_domain::ErrorResponse;

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

fn token(owner_id: Uuid, permissions: &[&str]) -> String {
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "parameter-mapping-test",
        permissions.iter().map(|value| value.to_string()).collect(),
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-secret").expect("token should encode")
}

fn map_request(token: &str, source_value: &str, idempotency_key: &str) -> Request<Body> {
    map_dictionary_request(token, "storage_condition", source_value, idempotency_key)
}

fn map_dictionary_request(
    token: &str,
    dict_code: &str,
    source_value: &str,
    idempotency_key: &str,
) -> Request<Body> {
    map_dictionary_source_request(token, dict_code, source_value, "ERP", idempotency_key)
}

fn map_dictionary_source_request(
    token: &str,
    dict_code: &str,
    source_value: &str,
    source_system: &str,
    idempotency_key: &str,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/parameter-mapping/map")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(
            json!({
                "dict_code": dict_code,
                "source_value": source_value,
                "source_system": source_system,
                "source_record_id": "ERP-P-001"
            })
            .to_string(),
        ))
        .expect("request should build")
}

#[sqlx::test(migrations = "../../migrations")]
async fn unmatched_queue_deduplicates_same_dictionary_value_across_sources(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let auth = token(owner_id, &["mpm.execute"]);

    for (index, source_system) in ["ERP-A", "ERP-B"].into_iter().enumerate() {
        let response = app(pool.clone())
            .oneshot(map_dictionary_source_request(
                &auth,
                "storage_condition",
                "跨来源未知温区",
                source_system,
                &format!("map-cross-source-{index}"),
            ))
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let evidence: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(MAX(occurrence_count), 0) FROM parameter_mapping_queue WHERE owner_id = $1 AND normalized_source_value = '跨来源未知温区'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("queue evidence should query");
    assert_eq!(evidence, (1, 2));
}

#[sqlx::test(migrations = "../../migrations")]
async fn document_types_use_system_dictionary_codes_as_mapping_targets(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let auth = token(owner_id, &["mpm.execute"]);
    for (index, (source, target)) in [
        ("采购入库", "purchase_inbound"),
        ("销售退货入库", "sales_return"),
        ("销售出库", "sales_outbound"),
    ]
    .into_iter()
    .enumerate()
    {
        let response = app(pool.clone())
            .oneshot(map_dictionary_request(
                &auth,
                "document_type",
                source,
                &format!("map-document-type-{index}"),
            ))
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        let payload: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("response should read"),
        )
        .expect("response should be json");
        assert_eq!(payload["status"], "matched");
        assert_eq!(payload["target_value"], target);
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn dosage_forms_use_story_defined_standard_value(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let auth = token(owner_id, &["mpm.execute"]);
    for (index, source) in ["片", "片剂", "普通片", "薄膜衣片"].into_iter().enumerate() {
        let response = app(pool.clone())
            .oneshot(map_dictionary_request(
                &auth,
                "dosage_form",
                source,
                &format!("map-dosage-form-{index}"),
            ))
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        let payload: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("response should read"),
        )
        .expect("response should be json");
        assert_eq!(payload["status"], "matched");
        assert_eq!(payload["target_value"], "片剂");
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn special_drug_categories_use_system_dictionary_codes_as_mapping_targets(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let auth = token(owner_id, &["mpm.execute"]);
    for (index, (source, target)) in [
        ("普通药品", "none"),
        ("麻醉药品", "narcotic"),
        ("第一类精神药品", "psychotropic_1"),
    ]
    .into_iter()
    .enumerate()
    {
        let response = app(pool.clone())
            .oneshot(map_dictionary_request(
                &auth,
                "special_drug_category",
                source,
                &format!("map-special-drug-category-{index}"),
            ))
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        let payload: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("response should read"),
        )
        .expect("response should be json");
        assert_eq!(payload["status"], "matched");
        assert_eq!(payload["target_value"], target);
    }
}

fn app(pool: PgPool) -> axum::Router {
    parameter_mapping_router(ParameterMappingAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn map_route_requires_mpm_execute_permission(pool: PgPool) {
    let response = app(pool)
        .oneshot(map_request(
            &token(Uuid::new_v4(), &[]),
            "冷藏",
            "map-no-permission",
        ))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let error: ErrorResponse = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should be permission error");
    assert_eq!(error.code, "AUTH-005");
}

#[sqlx::test(migrations = "../../migrations")]
async fn persisted_rule_survives_router_restart_and_unmatched_value_is_queued(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let auth = token(owner_id, &["mpm.execute"]);
    for (key, expected_target) in [("map-1", "cold"), ("map-2", "cold")] {
        let response = app(pool.clone())
            .oneshot(map_request(&auth, " 2-8℃避光保存 ", key))
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
        let payload: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("response should read"),
        )
        .expect("response should be json");
        assert_eq!(payload["status"], "matched");
        assert_eq!(payload["target_value"], expected_target);
        assert_eq!(payload["queued"], false);
    }

    for _ in 0..2 {
        let response = app(pool.clone())
            .oneshot(map_request(&auth, "低温保存", "map-unmatched"))
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
        let payload: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("response should read"),
        )
        .expect("response should be json");
        assert_eq!(payload["status"], "unmatched");
        assert_eq!(payload["queued"], true);
    }

    let queued: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM parameter_mapping_queue WHERE owner_id=$1 AND source_value='低温保存'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("queue should query");
    assert_eq!(queued, 1);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id=$1 AND action='map_parameter'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit should query");
    assert_eq!(
        audit_count, 3,
        "idempotent replay must not append another audit"
    );

    sqlx::query(
        "UPDATE idempotency_request SET expires_at=now()-interval '1 second' WHERE owner_id=$1 AND idempotency_key='map-unmatched'",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("idempotency row should expire");
    let response = app(pool.clone())
        .oneshot(map_request(&auth, "低温保存", "map-unmatched"))
        .await
        .expect("expired request should run again");
    assert_eq!(response.status(), StatusCode::OK);
    let occurrence_count: i64 = sqlx::query_scalar(
        "SELECT occurrence_count FROM parameter_mapping_queue WHERE owner_id=$1 AND source_value='低温保存'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("queue count should query");
    assert_eq!(occurrence_count, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn owner_rule_overrides_global_without_leaking_to_another_owner(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    for (owner_id, code) in [(owner_a, "MPM-A"), (owner_b, "MPM-B")] {
        sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$2)")
            .bind(owner_id)
            .bind(code)
            .execute(&pool)
            .await
            .expect("owner should seed");
    }
    let dictionary_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO parameter_mapping_dictionaries (id,owner_id,dict_code,dict_name,target_values) VALUES ($1,$2,'storage_condition','货主储存条件','[\"frozen\",\"cold\"]')",
    )
    .bind(dictionary_id)
    .bind(owner_a)
    .execute(&pool)
    .await
    .expect("owner dictionary should seed");
    sqlx::query(
        "INSERT INTO parameter_mapping_rules (id,dictionary_id,owner_id,source_system,match_type,source_pattern,normalized_source_pattern,target_value,priority,confidence) VALUES ($1,$2,$3,'ERP','exact','冷藏','冷藏','frozen',1,100)",
    )
    .bind(Uuid::new_v4())
    .bind(dictionary_id)
    .bind(owner_a)
    .execute(&pool)
    .await
    .expect("owner rule should seed");

    for (owner_id, expected) in [(owner_a, "frozen"), (owner_b, "cold")] {
        let response = app(pool.clone())
            .oneshot(map_request(
                &token(owner_id, &["mpm.execute"]),
                "冷藏",
                &format!("map-{owner_id}"),
            ))
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
        let payload: serde_json::Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("response should read"),
        )
        .expect("response should be json");
        assert_eq!(payload["target_value"], expected);
    }

    let fallback = app(pool.clone())
        .oneshot(map_request(
            &token(owner_a, &["mpm.execute"]),
            "常温",
            "map-owner-global-fallback",
        ))
        .await
        .expect("router should respond");
    assert_eq!(fallback.status(), StatusCode::OK);
    let payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(fallback.into_body(), usize::MAX)
            .await
            .expect("response should read"),
    )
    .expect("response should be json");
    assert_eq!(payload["status"], "matched");
    assert_eq!(
        payload["target_value"], "normal",
        "an owner override must retain global fallback rules"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn newest_rule_wins_when_matching_rules_have_the_same_priority(pool: PgPool) {
    let dictionary_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM parameter_mapping_dictionaries WHERE owner_id IS NULL AND dict_code = 'storage_condition'",
    )
    .fetch_one(&pool)
    .await
    .expect("storage condition dictionary should exist");
    let older_rule_id = Uuid::new_v4();
    let newer_rule_id = Uuid::new_v4();

    for (rule_id, target_value, created_at) in [
        (older_rule_id, "normal", "2026-07-25 08:00:00+00"),
        (newer_rule_id, "cold", "2026-07-25 08:01:00+00"),
    ] {
        sqlx::query(
            r#"
            INSERT INTO parameter_mapping_rules (
                id, dictionary_id, source_system, match_type, source_pattern,
                normalized_source_pattern, target_value, priority, confidence, created_at, updated_at
            ) VALUES ($1,$2,'ERP','exact','同优先级测试','同优先级测试',$3,20,100,$4::timestamptz,$4::timestamptz)
            "#,
        )
        .bind(rule_id)
        .bind(dictionary_id)
        .bind(target_value)
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("same-priority rule should seed");
    }

    let response = app(pool)
        .oneshot(map_request(
            &token(Uuid::new_v4(), &["mpm.execute"]),
            "同优先级测试",
            "map-same-priority-newest",
        ))
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response should read"),
    )
    .expect("response should be json");

    assert_eq!(payload["status"], "matched");
    assert_eq!(payload["target_value"], "cold");
    assert_eq!(payload["rule_id"], newer_rule_id.to_string());
}
