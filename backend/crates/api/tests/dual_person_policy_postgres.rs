use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header::AUTHORIZATION, Request, StatusCode},
};
use chrono::Utc;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthContext,
        AuthRevocationStore, AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
    },
    dual_person_policy::{DualPersonPolicyError, PgDualPersonPolicyRepository},
    dual_person_policy_handlers::{dual_person_policy_router, DualPersonPolicyAppState},
};
use wms_domain::{
    DualPersonPolicy, DualPersonPolicyScope, ResolveDualPersonPolicyQuery,
    UpsertDualPersonPolicyRuleRequest,
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
        _changed_at_unix: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }
}

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "dual-person-policy-test".to_string(),
        permissions: vec!["mvr.dual_person.read".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn write_ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "dual-person-policy-writer".to_string(),
        permissions: vec![
            "mvr.dual_person.read".to_string(),
            "mvr.dual_person.write".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_product(pool: &PgPool, category: &str) -> (Uuid, Uuid, Uuid) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '双人策略测试货主')",
    )
    .bind(owner_id)
    .bind(format!("MVR-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("owner should seed");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, '双人策略测试仓', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("MVR-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("warehouse should seed");
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, special_drug_category, status) VALUES ($1, $2, $3, '双人策略测试商品', '1 unit', 'normal', $4, 'active')",
    )
    .bind(product_id)
    .bind(owner_id)
    .bind(format!("MVR-P-{}", &product_id.to_string()[..8]))
    .bind(category)
    .execute(pool)
    .await
    .expect("product should seed");
    (owner_id, warehouse_id, product_id)
}

async fn seed_matrix_approvers(pool: &PgPool, owner_id: Uuid) -> (Uuid, Uuid) {
    let actor_id = Uuid::new_v4();
    let confirmer_id = Uuid::new_v4();
    let role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND lower(role_code) = 'warehouse_manager'",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .expect("owner seed should include warehouse manager role");
    for (index, user_id) in [actor_id, confirmer_id].into_iter().enumerate() {
        sqlx::query(
            "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, $3, 'test-hash', 'active')",
        )
        .bind(user_id)
        .bind(format!("mvr-approver-{index}-{}", &user_id.to_string()[..8]))
        .bind(format!("双人策略确认人 {index}"))
        .execute(pool)
        .await
        .expect("matrix approver should seed");
        sqlx::query(
            "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, $3)",
        )
        .bind(user_id)
        .bind(owner_id)
        .bind(index == 0)
        .execute(pool)
        .await
        .expect("matrix approver owner binding should seed");
        sqlx::query("INSERT INTO auth_user_roles (user_id, owner_id, role_id) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(owner_id)
            .bind(role_id)
            .execute(pool)
            .await
            .expect("matrix approver role should seed");
    }
    (actor_id, confirmer_id)
}

fn query(
    owner_id: Uuid,
    warehouse_id: Uuid,
    product_id: Uuid,
    process: &str,
    node: &str,
) -> ResolveDualPersonPolicyQuery {
    ResolveDualPersonPolicyQuery {
        product_id,
        process: process.to_string(),
        node: node.to_string(),
        owner_id,
        warehouse_id: Some(warehouse_id),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn seeded_matrix_resolves_by_category_process_and_node(pool: PgPool) {
    let (owner_id, warehouse_id, product_id) = seed_product(&pool, "narcotic").await;
    let repository = PgDualPersonPolicyRepository::new(pool);

    let result = repository
        .resolve(
            &ctx(owner_id),
            &query(owner_id, warehouse_id, product_id, "入库", "收货"),
        )
        .await
        .expect("seeded matrix should resolve");

    assert_eq!(result.policy, DualPersonPolicy::DualScanWithApproval);
    assert!(result.source_rule_id.is_some());
    assert_eq!(result.process, "入库");
    assert_eq!(result.node, "收货");
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_process_node_pair_is_rejected(pool: PgPool) {
    let (owner_id, warehouse_id, product_id) = seed_product(&pool, "none").await;
    let repository = PgDualPersonPolicyRepository::new(pool);

    let error = repository
        .resolve(
            &ctx(owner_id),
            &query(owner_id, warehouse_id, product_id, "入库", "报损执行"),
        )
        .await
        .expect_err("cross-process node must be rejected");

    assert!(matches!(error, DualPersonPolicyError::InvalidProcessNode));
}

#[sqlx::test(migrations = "../../migrations")]
async fn owner_rule_is_dual_confirmed_idempotent_audited_and_dictionary_synced(pool: PgPool) {
    let (owner_id, warehouse_id, product_id) = seed_product(&pool, "none").await;
    let (actor_id, confirmer_id) = seed_matrix_approvers(&pool, owner_id).await;
    let repository = PgDualPersonPolicyRepository::new(pool.clone());
    let request = UpsertDualPersonPolicyRuleRequest {
        special_drug_category: "none".to_string(),
        process: "入库".to_string(),
        node: "收货".to_string(),
        policy: DualPersonPolicy::DualScan,
        scope: DualPersonPolicyScope::Owner,
        warehouse_id: None,
        priority: 300,
        enabled: true,
        confirmed_by_user_id: confirmer_id,
    };
    let first_resolution = repository
        .resolve(
            &write_ctx(owner_id, actor_id),
            &query(owner_id, warehouse_id, product_id, "入库", "收货"),
        )
        .await
        .expect("default policy should resolve before owner override");
    assert_eq!(first_resolution.policy, DualPersonPolicy::Single);
    let first = repository
        .upsert(
            &write_ctx(owner_id, actor_id),
            request.clone(),
            Utc::now(),
            "mvr-matrix-owner-1",
        )
        .await
        .expect("dual-confirmed owner rule should persist");
    let replay = repository
        .upsert(
            &write_ctx(owner_id, actor_id),
            request,
            Utc::now(),
            "mvr-matrix-owner-1",
        )
        .await
        .expect("same request should replay");
    assert_eq!(first.value.id, replay.value.id);
    assert!(replay.replayed);

    let resolved = repository
        .resolve(
            &write_ctx(owner_id, actor_id),
            &query(owner_id, warehouse_id, product_id, "入库", "收货"),
        )
        .await
        .expect("owner override should resolve");
    assert_eq!(resolved.policy, DualPersonPolicy::DualScan);
    assert_eq!(resolved.source_rule_id, Some(first.value.id));

    let rules = repository
        .list(&write_ctx(owner_id, actor_id), None)
        .await
        .expect("matrix rules should list");
    assert!(rules.iter().any(|rule| rule.id == first.value.id));
    assert!(rules.iter().any(|rule| rule.owner_id.is_none()));

    let matrix: serde_json::Value = sqlx::query_scalar(
        "SELECT params -> 'requires_dual_person_matrix' FROM system_dictionary_items WHERE dict_code = 'special_drug_category' AND item_code = 'none' AND owner_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("owner matrix should be synchronized to M1 dictionary");
    assert!(matrix
        .as_array()
        .is_some_and(|cells| cells.iter().any(|cell| {
            cell["process"] == "入库" && cell["node"] == "收货" && cell["policy"] == "dual_scan"
        })));

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'M-VR' AND resource_type = 'dual_person_policy_rule' AND action = 'upsert_dual_person_policy_rule'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("matrix audit should query");
    assert_eq!(audit_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn matrix_write_rejects_same_person_confirmation(pool: PgPool) {
    let (owner_id, _, _) = seed_product(&pool, "none").await;
    let (actor_id, _) = seed_matrix_approvers(&pool, owner_id).await;
    let repository = PgDualPersonPolicyRepository::new(pool);
    let error = repository
        .upsert(
            &write_ctx(owner_id, actor_id),
            UpsertDualPersonPolicyRuleRequest {
                special_drug_category: "none".to_string(),
                process: "入库".to_string(),
                node: "收货".to_string(),
                policy: DualPersonPolicy::DualScan,
                scope: DualPersonPolicyScope::Owner,
                warehouse_id: None,
                priority: 100,
                enabled: true,
                confirmed_by_user_id: actor_id,
            },
            Utc::now(),
            "mvr-matrix-same-person",
        )
        .await
        .expect_err("same person cannot activate matrix change");
    assert_eq!(error, DualPersonPolicyError::SamePerson);
}

#[sqlx::test(migrations = "../../migrations")]
async fn policy_route_resolves_and_rejects_invalid_process_node(pool: PgPool) {
    let (owner_id, warehouse_id, product_id) = seed_product(&pool, "narcotic").await;
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "dual-person-route-test",
        vec!["mvr.dual_person.read".to_string()],
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    let token = encode_access_token(&claims, "test-secret").expect("token should encode");
    let app = dual_person_policy_router(DualPersonPolicyAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let base = format!(
        "/api/v1/m-vr/dual-person-policy?product_id={product_id}&process=%E5%85%A5%E5%BA%93&node=%E6%94%B6%E8%B4%A7&owner_id={owner_id}&warehouse_id={warehouse_id}"
    );

    let response = app
        .clone()
        .oneshot(
            Request::get(&base)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("policy request should build"),
        )
        .await
        .expect("policy route should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("policy response body should read"),
    )
    .expect("policy response should be json");
    assert_eq!(body["policy"], "dual_scan_with_approval");
    assert_eq!(body["process"], "入库");
    assert_eq!(body["node"], "收货");

    let rules = app
        .clone()
        .oneshot(
            Request::get("/api/v1/m-vr/dual-person-policy/rules")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("policy rule request should build"),
        )
        .await
        .expect("policy rule route should respond");
    assert_eq!(rules.status(), StatusCode::OK);

    let invalid = app
        .oneshot(
            Request::get(
                base.replace("%E6%94%B6%E8%B4%A7", "%E6%8A%A5%E6%8D%9F%E6%89%A7%E8%A1%8C"),
            )
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .expect("invalid policy request should build"),
        )
        .await
        .expect("invalid policy route should respond");
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
