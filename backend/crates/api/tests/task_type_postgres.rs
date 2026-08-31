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
    task_type::{PgTaskTypeRepository, TaskTypeError},
    task_type_handlers::{task_type_router, TaskTypeAppState},
};
use wms_domain::{SetTaskTypeEnabledRequest, TaskReleaseStrategy, UpsertTaskTypeRequest};

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
        actor_name: "task-type-test".to_string(),
        permissions: vec![
            "mte.task_type.read".to_string(),
            "mte.task_type.write".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_owner(pool: &PgPool, owner_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '任务类型测试货主')",
    )
    .bind(owner_id)
    .bind(format!("MTE-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("task type test owner should insert");
}

fn request() -> UpsertTaskTypeRequest {
    UpsertTaskTypeRequest {
        task_type_name: " 波次补拣 ".to_string(),
        default_priority: 321,
        estimated_minutes: 45,
        mergeable: false,
        insertable: true,
        release_strategy: TaskReleaseStrategy::Immediate,
        release_interval_minutes: None,
        release_batch_size: None,
        enabled: true,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn task_type_defaults_are_extensible_and_owner_scoped(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    seed_owner(&pool, owner_a).await;
    seed_owner(&pool, owner_b).await;

    let repository = PgTaskTypeRepository::new(pool.clone());
    let defaults = repository
        .list(&ctx(owner_a))
        .await
        .expect("default task types should be queryable");
    let codes: Vec<_> = defaults
        .iter()
        .map(|item| item.task_type_code.as_str())
        .collect();
    assert_eq!(
        codes,
        vec![
            "inventory_count",
            "loading",
            "pick",
            "putaway",
            "relocation",
            "replenish",
            "return_putaway"
        ]
    );
    assert!(defaults.iter().all(|item| item.owner_id == owner_a));
    assert!(defaults.iter().all(|item| item.default_priority >= 0));

    let custom = repository
        .upsert(
            &ctx(owner_a),
            " CrossDock.V2 ",
            request(),
            Utc::now(),
            "task-type-custom-1",
        )
        .await
        .expect("custom task type should persist");
    assert_eq!(custom.value.task_type_code, "crossdock.v2");
    assert_eq!(custom.value.task_type_name, "波次补拣");
    assert_eq!(custom.value.default_priority, 321);
    assert_eq!(custom.value.estimated_minutes, 45);
    assert!(!custom.value.mergeable);
    assert!(custom.value.insertable);

    let other_owner = repository
        .list(&ctx(owner_b))
        .await
        .expect("other owner task types should be queryable");
    assert_eq!(other_owner.len(), 7);
    assert!(!other_owner
        .iter()
        .any(|item| item.task_type_code == "crossdock.v2"));
    assert!(other_owner.iter().all(|item| item.owner_id == owner_b));
}

#[sqlx::test(migrations = "../../migrations")]
async fn task_type_mutations_are_idempotent_and_audited(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let repository = PgTaskTypeRepository::new(pool.clone());
    let now = Utc::now();

    let mut invalid_request = request();
    invalid_request.estimated_minutes = 0;
    let invalid = repository
        .upsert(
            &ctx(owner_id),
            "invalid_task_type",
            invalid_request,
            now,
            "task-type-invalid-1",
        )
        .await
        .expect_err("invalid task type configuration must be rejected");
    assert!(matches!(invalid, TaskTypeError::Validation(_)));

    let mut invalid_release = request();
    invalid_release.release_strategy = TaskReleaseStrategy::Scheduled;
    let invalid = repository
        .upsert(
            &ctx(owner_id),
            "invalid_release_rule",
            invalid_release,
            now,
            "task-type-invalid-release-1",
        )
        .await
        .expect_err("scheduled release must configure interval and batch size");
    assert!(matches!(invalid, TaskTypeError::Validation(_)));

    let first = repository
        .upsert(&ctx(owner_id), "pick", request(), now, "task-type-idem-1")
        .await
        .expect("task type update should persist");
    let replay = repository
        .upsert(&ctx(owner_id), "PICK", request(), now, "task-type-idem-1")
        .await
        .expect("same task type request should replay");
    assert_eq!(first.value.id, replay.value.id);
    assert!(replay.replayed);

    let mut conflicting_request = request();
    conflicting_request.default_priority = 322;
    let conflict = repository
        .upsert(
            &ctx(owner_id),
            "pick",
            conflicting_request,
            now,
            "task-type-idem-1",
        )
        .await
        .expect_err("same key with different request must conflict");
    assert_eq!(conflict, TaskTypeError::IdempotencyConflict);

    let disabled = repository
        .set_enabled(
            &ctx(owner_id),
            "crossdock.v2",
            SetTaskTypeEnabledRequest { enabled: false },
            now,
            "task-type-disabled-1",
        )
        .await
        .expect_err("unknown task type should be rejected");
    assert_eq!(disabled, TaskTypeError::NotFound);

    let disabled = repository
        .set_enabled(
            &ctx(owner_id),
            "pick",
            SetTaskTypeEnabledRequest { enabled: false },
            now,
            "task-type-disabled-1",
        )
        .await
        .expect("task type should be disabled");
    assert!(!disabled.value.enabled);
    let disabled_replay = repository
        .set_enabled(
            &ctx(owner_id),
            "pick",
            SetTaskTypeEnabledRequest { enabled: false },
            now,
            "task-type-disabled-1",
        )
        .await
        .expect("disable replay should succeed");
    assert!(disabled_replay.replayed);

    let audit_rows: Vec<(String, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT action, diff FROM audit_event WHERE owner_id = $1 AND module = 'M-TE' AND resource_type = 'task_type' ORDER BY id",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("task type audit rows should query");
    assert_eq!(audit_rows.len(), 2);
    assert_eq!(
        audit_rows
            .iter()
            .map(|(action, _)| action.as_str())
            .collect::<Vec<_>>(),
        vec!["upsert_task_type", "set_task_type_enabled"]
    );
    assert!(audit_rows.iter().all(|(_, diff)| diff.is_some()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn task_type_route_persists_configuration_and_rejects_missing_idempotency(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "task-type-api-test",
        vec![
            "mte.task_type.read".to_string(),
            "mte.task_type.write".to_string(),
        ],
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    let token = encode_access_token(&claims, "test-secret").expect("test token should encode");
    let app = task_type_router(TaskTypeAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let response = app
        .clone()
        .oneshot(
            Request::put("/api/v1/task-engine/task-types/urgent_pick")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "task-type-api-1")
                .body(Body::from(
                    serde_json::json!({
                        "task_type_name": "紧急拣选",
                        "default_priority": 999,
                        "estimated_minutes": 5,
                        "mergeable": false,
                        "insertable": true,
                        "enabled": true
                    })
                    .to_string(),
                ))
                .expect("task type put request should build"),
        )
        .await
        .expect("task type put route should respond");
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::get("/api/v1/task-engine/task-types")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("task type list request should build"),
        )
        .await
        .expect("task type list route should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("task type list body should read"),
    )
    .expect("task type list should be json");
    assert_eq!(body["page"]["count"], 8);
    assert_eq!(body["data"][7]["task_type_code"], "urgent_pick");

    let response = app
        .oneshot(
            Request::put("/api/v1/task-engine/task-types/another")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&request()).expect("request should serialize"),
                ))
                .expect("missing key request should build"),
        )
        .await
        .expect("missing key route should respond");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn task_type_route_rejects_missing_write_permission(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "task-type-forbidden",
        vec!["mte.task_type.read".to_string()],
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    let token = encode_access_token(&claims, "test-secret").expect("test token should encode");
    let app = task_type_router(TaskTypeAppState::with_postgres(pool)).layer(auth_runtime_layer(
        AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore)),
    ));

    let response = app
        .oneshot(
            Request::put("/api/v1/task-engine/task-types/urgent_pick")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "task-type-forbidden")
                .body(Body::from(
                    serde_json::to_string(&request()).expect("request should serialize"),
                ))
                .expect("forbidden task type request should build"),
        )
        .await
        .expect("forbidden task type route should respond");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
