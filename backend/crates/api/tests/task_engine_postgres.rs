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
    task_engine::{PgTaskEngineRepository, TaskEngineError},
    task_engine_handlers::{task_engine_router, TaskEngineAppState},
};
use wms_domain::{
    CreateWarehouseTaskRequest, TaskListQuery, TaskTransitionAction,
    TransitionWarehouseTaskRequest, UpsertTaskGroupRequest,
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

fn ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: format!("mte-{user_id}"),
        permissions: vec![
            "mte.task.read".to_string(),
            "mte.task.write".to_string(),
            "mte.task.assign".to_string(),
            "mte.task_group.write".to_string(),
            "mte.task.execute".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
    }
}

fn worker_ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: format!("mte-worker-{user_id}"),
        permissions: vec!["mte.task.read".to_string(), "mte.task.execute".to_string()],
        jti: Uuid::new_v4().to_string(),
    }
}

async fn seed_owner(pool: &PgPool, owner_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'M-TE 测试货主')",
    )
    .bind(owner_id)
    .bind(format!("MTE-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("owner should insert");
}

async fn seed_user(pool: &PgPool, owner_id: Uuid, user_id: Uuid, name: &str) {
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, $3, 'test-hash', 'active')",
    )
    .bind(user_id)
    .bind(format!("{name}-{}", &user_id.to_string()[..8]))
    .bind(name)
    .execute(pool)
    .await
    .expect("user should insert");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("owner binding should insert");
}

async fn seed_warehouse(pool: &PgPool, owner_id: Uuid) -> Uuid {
    let warehouse_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status
        ) VALUES ($1, $2, $3, 'M-TE 测试仓', 'distribution_center', 'active')
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("warehouse should insert");
    warehouse_id
}

fn create_request(warehouse_id: Uuid) -> CreateWarehouseTaskRequest {
    CreateWarehouseTaskRequest {
        task_type_code: "pick".to_string(),
        source_module: "M4".to_string(),
        source_doc_type: "outbound_order".to_string(),
        source_doc_id: Some(Uuid::new_v4()),
        source_doc_no: "SO-MTE-001".to_string(),
        source_line_no: Some(1),
        source_task_key: "M4:SO-MTE-001:1:pick".to_string(),
        warehouse_id,
        task_group_code: "pick-a".to_string(),
        product_id: None,
        product_code: "P-001".to_string(),
        batch_id: None,
        batch_no: Some("LOT-001".to_string()),
        planned_qty: 10,
        source_location_id: None,
        source_location_code: Some("A-01-01".to_string()),
        target_location_id: None,
        target_location_code: Some("PACK-01".to_string()),
        priority: None,
    }
}

fn transition(action: TaskTransitionAction) -> TransitionWarehouseTaskRequest {
    TransitionWarehouseTaskRequest {
        action,
        assignee_user_id: None,
        actual_qty: None,
        exception_code: None,
        exception_note: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn task_main_chain_enforces_qualification_state_machine_and_idempotency(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let warehouse_id = seed_warehouse(&pool, owner_id).await;
    let manager_id = Uuid::new_v4();
    let worker_id = Uuid::new_v4();
    let outsider_id = Uuid::new_v4();
    seed_user(&pool, owner_id, manager_id, "主管").await;
    seed_user(&pool, owner_id, worker_id, "合格拣选员").await;
    seed_user(&pool, owner_id, outsider_id, "无资格人员").await;

    let repository = PgTaskEngineRepository::new(pool.clone());
    let manager = ctx(owner_id, manager_id);
    let worker = worker_ctx(owner_id, worker_id);
    let outsider = worker_ctx(owner_id, outsider_id);
    let now = Utc::now();

    let worker_candidates = repository
        .list_worker_candidates(&manager)
        .await
        .expect("task group worker candidates should query");
    assert_eq!(worker_candidates.len(), 3);
    assert!(worker_candidates
        .iter()
        .any(|worker| worker.user_id == worker_id));

    let group = repository
        .upsert_task_group(
            &manager,
            "pick-a",
            UpsertTaskGroupRequest {
                task_group_name: "A 区拣选组".to_string(),
                warehouse_id,
                zone_ids: vec![],
                task_type_codes: vec!["pick".to_string()],
                member_user_ids: vec![worker_id],
                enabled: true,
            },
            now,
            "mte-group-1",
        )
        .await
        .expect("task group should persist");
    assert_eq!(group.value.member_user_ids, vec![worker_id]);
    assert!(!group.replayed);
    assert_eq!(
        repository
            .list_task_groups(&worker)
            .await
            .expect("worker should read own task groups")
            .len(),
        1
    );
    assert!(repository
        .list_task_groups(&outsider)
        .await
        .expect("worker outside groups should receive an empty list")
        .is_empty());

    let request = create_request(warehouse_id);
    let first = repository
        .create_task(&manager, request.clone(), now, "mte-create-1")
        .await
        .expect("task should be created");
    assert_eq!(first.value.status, "pending_assignment");
    assert_eq!(first.value.priority, 100);
    assert_eq!(first.value.estimated_minutes, 15);
    let replay = repository
        .create_task(&manager, request.clone(), now, "mte-create-1")
        .await
        .expect("same request should replay");
    assert_eq!(replay.value.id, first.value.id);
    assert!(replay.replayed);
    let mut aliased_source = request;
    aliased_source.source_task_key = "caller-supplied-alias".to_string();
    let source_replay = repository
        .create_task(&manager, aliased_source, now, "mte-create-source-alias-1")
        .await
        .expect("same business source and task type must not create a duplicate");
    assert_eq!(source_replay.value.id, first.value.id);

    let not_qualified = repository
        .transition_task(
            &manager,
            first.value.id,
            TransitionWarehouseTaskRequest {
                assignee_user_id: Some(outsider_id),
                ..transition(TaskTransitionAction::Assign)
            },
            now,
            "mte-assign-outsider-1",
        )
        .await
        .expect_err("worker outside task group must be rejected");
    assert_eq!(not_qualified, TaskEngineError::WorkerNotQualified);

    let assigned = repository
        .transition_task(
            &manager,
            first.value.id,
            TransitionWarehouseTaskRequest {
                assignee_user_id: Some(worker_id),
                ..transition(TaskTransitionAction::Assign)
            },
            now,
            "mte-assign-1",
        )
        .await
        .expect("qualified worker should be assigned");
    assert_eq!(assigned.value.status, "assigned");
    assert_eq!(assigned.value.assignee_user_id, Some(worker_id));

    let assigned_to_worker = repository
        .list_tasks(
            &worker,
            TaskListQuery {
                mine_only: true,
                status: None,
                task_type_code: None,
                warehouse_id: None,
                limit: Some(50),
            },
        )
        .await
        .expect("PDA task list should query");
    assert_eq!(assigned_to_worker.len(), 1);
    assert_eq!(assigned_to_worker[0].id, first.value.id);

    let dispatched = repository
        .transition_task(
            &manager,
            first.value.id,
            transition(TaskTransitionAction::Dispatch),
            now,
            "mte-dispatch-1",
        )
        .await
        .expect("assigned task should dispatch");
    assert_eq!(dispatched.value.status, "dispatched");

    let forbidden = repository
        .transition_task(
            &outsider,
            first.value.id,
            transition(TaskTransitionAction::Start),
            now,
            "mte-start-outsider-1",
        )
        .await
        .expect_err("only assignee may start");
    assert_eq!(forbidden, TaskEngineError::NotAssignee);

    let started = repository
        .transition_task(
            &worker,
            first.value.id,
            transition(TaskTransitionAction::Start),
            now,
            "mte-start-1",
        )
        .await
        .expect("assignee should start task");
    assert_eq!(started.value.status, "in_progress");

    let mismatch = repository
        .transition_task(
            &worker,
            first.value.id,
            TransitionWarehouseTaskRequest {
                actual_qty: Some(9),
                ..transition(TaskTransitionAction::Complete)
            },
            now,
            "mte-complete-mismatch-1",
        )
        .await
        .expect_err("quantity mismatch without exception must be rejected");
    assert_eq!(
        mismatch,
        TaskEngineError::QuantityDifferenceRequiresException
    );

    let completed = repository
        .transition_task(
            &worker,
            first.value.id,
            TransitionWarehouseTaskRequest {
                actual_qty: Some(10),
                ..transition(TaskTransitionAction::Complete)
            },
            now,
            "mte-complete-1",
        )
        .await
        .expect("matching actual quantity should complete");
    assert_eq!(completed.value.status, "completed");
    assert_eq!(completed.value.actual_qty, Some(10));
    let completed_replay = repository
        .transition_task(
            &worker,
            first.value.id,
            TransitionWarehouseTaskRequest {
                actual_qty: Some(10),
                ..transition(TaskTransitionAction::Complete)
            },
            now,
            "mte-complete-1",
        )
        .await
        .expect("completion replay should return current result");
    assert!(completed_replay.replayed);

    let task_events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM task_execution_events WHERE owner_id = $1 AND task_id = $2",
    )
    .bind(owner_id)
    .bind(first.value.id)
    .fetch_one(&pool)
    .await
    .expect("events should query");
    assert_eq!(task_events, 5, "create/assign/dispatch/start/complete");

    let audit_events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event WHERE owner_id = $1 AND module = 'M-TE'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit should query");
    assert_eq!(
        audit_events, 6,
        "group change plus five task lifecycle events"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn automatic_assignment_uses_qualified_least_loaded_worker(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let warehouse_id = seed_warehouse(&pool, owner_id).await;
    let manager_id = Uuid::new_v4();
    let busy_worker_id = Uuid::new_v4();
    let idle_worker_id = Uuid::new_v4();
    seed_user(&pool, owner_id, manager_id, "自动分派主管").await;
    seed_user(&pool, owner_id, busy_worker_id, "已有任务人员").await;
    seed_user(&pool, owner_id, idle_worker_id, "空闲人员").await;
    let repository = PgTaskEngineRepository::new(pool);
    let manager = ctx(owner_id, manager_id);
    repository
        .upsert_task_group(
            &manager,
            "pick-a",
            UpsertTaskGroupRequest {
                task_group_name: "自动分派组".to_string(),
                warehouse_id,
                zone_ids: vec![],
                task_type_codes: vec!["pick".to_string()],
                member_user_ids: vec![busy_worker_id, idle_worker_id],
                enabled: true,
            },
            Utc::now(),
            "mte-auto-group-1",
        )
        .await
        .expect("automatic assignment group should persist");

    let first = repository
        .create_task(
            &manager,
            create_request(warehouse_id),
            Utc::now(),
            "mte-auto-create-1",
        )
        .await
        .expect("first task should create")
        .value;
    repository
        .transition_task(
            &manager,
            first.id,
            TransitionWarehouseTaskRequest {
                assignee_user_id: Some(busy_worker_id),
                ..transition(TaskTransitionAction::Assign)
            },
            Utc::now(),
            "mte-auto-busy-assign-1",
        )
        .await
        .expect("first task should occupy worker");

    let mut second_request = create_request(warehouse_id);
    second_request.source_doc_no = "SO-MTE-002".to_string();
    second_request.source_task_key = "M4:SO-MTE-002:1:pick".to_string();
    let second = repository
        .create_task(&manager, second_request, Utc::now(), "mte-auto-create-2")
        .await
        .expect("second task should create")
        .value;
    let automatically_assigned = repository
        .transition_task(
            &manager,
            second.id,
            transition(TaskTransitionAction::Assign),
            Utc::now(),
            "mte-auto-assign-2",
        )
        .await
        .expect("automatic assignment should select available worker");
    assert_eq!(
        automatically_assigned.value.assignee_user_id,
        Some(idle_worker_id)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn task_routes_require_idempotency_and_expose_worker_queue(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let warehouse_id = seed_warehouse(&pool, owner_id).await;
    let manager_id = Uuid::new_v4();
    let worker_id = Uuid::new_v4();
    seed_user(&pool, owner_id, manager_id, "API 主管").await;
    seed_user(&pool, owner_id, worker_id, "API 拣选员").await;
    let repository = PgTaskEngineRepository::new(pool.clone());
    repository
        .upsert_task_group(
            &ctx(owner_id, manager_id),
            "pick-a",
            UpsertTaskGroupRequest {
                task_group_name: "API A 区拣选组".to_string(),
                warehouse_id,
                zone_ids: vec![],
                task_type_codes: vec!["pick".to_string()],
                member_user_ids: vec![worker_id],
                enabled: true,
            },
            Utc::now(),
            "mte-api-group-1",
        )
        .await
        .expect("group should seed");

    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let manager_claims = build_access_claims(
        manager_id,
        owner_id,
        "mte-api-manager",
        vec![
            "mte.task.read".to_string(),
            "mte.task.write".to_string(),
            "mte.task.assign".to_string(),
            "mte.task_group.write".to_string(),
        ],
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    let worker_claims = build_access_claims(
        worker_id,
        owner_id,
        "mte-api-worker",
        vec!["mte.task.read".to_string(), "mte.task.execute".to_string()],
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    let manager_token =
        encode_access_token(&manager_claims, "test-secret").expect("manager token should encode");
    let worker_token =
        encode_access_token(&worker_claims, "test-secret").expect("worker token should encode");
    let app = task_engine_router(TaskEngineAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let groups = app
        .clone()
        .oneshot(
            Request::get("/api/v1/task-engine/task-groups")
                .header(AUTHORIZATION, format!("Bearer {manager_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("task group list should respond");
    assert_eq!(groups.status(), StatusCode::OK);

    let workers = app
        .clone()
        .oneshot(
            Request::get("/api/v1/task-engine/workers")
                .header(AUTHORIZATION, format!("Bearer {manager_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("worker candidate list should respond");
    assert_eq!(workers.status(), StatusCode::OK);

    let upserted_group = app
        .clone()
        .oneshot(
            Request::put("/api/v1/task-engine/task-groups/pick-b")
                .header(AUTHORIZATION, format!("Bearer {manager_token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "mte-api-group-2")
                .body(Body::from(
                    serde_json::json!({
                        "task_group_name": "API B 区拣选组",
                        "warehouse_id": warehouse_id,
                        "zone_ids": [],
                        "task_type_codes": ["pick"],
                        "member_user_ids": [worker_id],
                        "enabled": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("task group upsert should respond");
    assert_eq!(upserted_group.status(), StatusCode::OK);

    let missing_key = app
        .clone()
        .oneshot(
            Request::post("/api/v1/task-engine/tasks")
                .header(AUTHORIZATION, format!("Bearer {manager_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&create_request(warehouse_id)).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .expect("missing key request should respond");
    assert_eq!(missing_key.status(), StatusCode::BAD_REQUEST);

    let created = app
        .clone()
        .oneshot(
            Request::post("/api/v1/task-engine/tasks")
                .header(AUTHORIZATION, format!("Bearer {manager_token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "mte-api-create-1")
                .body(Body::from(
                    serde_json::to_string(&create_request(warehouse_id)).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .expect("create route should respond");
    assert_eq!(created.status(), StatusCode::CREATED);
    let body: serde_json::Value = serde_json::from_slice(
        &to_bytes(created.into_body(), usize::MAX)
            .await
            .expect("created response body should read"),
    )
    .expect("created response should be json");
    let task_id = body["id"].as_str().expect("task id should exist");

    let assigned = app
        .clone()
        .oneshot(
            Request::post(format!("/api/v1/task-engine/tasks/{task_id}/transitions"))
                .header(AUTHORIZATION, format!("Bearer {manager_token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "mte-api-assign-1")
                .body(Body::from(
                    serde_json::json!({
                        "action": "assign",
                        "assignee_user_id": worker_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("assign route should respond");
    assert_eq!(assigned.status(), StatusCode::OK);

    let queue = app
        .clone()
        .oneshot(
            Request::get("/api/v1/task-engine/tasks?mine_only=true")
                .header(AUTHORIZATION, format!("Bearer {worker_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("worker queue should respond");
    assert_eq!(queue.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &to_bytes(queue.into_body(), usize::MAX)
            .await
            .expect("queue body should read"),
    )
    .expect("queue should be json");
    assert_eq!(body["page"]["count"], 1);
    assert_eq!(body["data"][0]["assignee_user_id"], worker_id.to_string());

    let forbidden_all = app
        .oneshot(
            Request::get("/api/v1/task-engine/tasks?mine_only=false")
                .header(AUTHORIZATION, format!("Bearer {worker_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("worker all-task query should respond");
    assert_eq!(forbidden_all.status(), StatusCode::FORBIDDEN);
}
