use super::*;

#[sqlx::test(migrations = "../../migrations")]
async fn release_route_enforces_condition_permission_and_idempotency(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let warehouse_id = seed_warehouse(&pool, owner_id).await;
    let manager_id = Uuid::new_v4();
    let worker_id = Uuid::new_v4();
    seed_user(&pool, owner_id, manager_id, "释放 API 主管").await;
    seed_user(&pool, owner_id, worker_id, "释放 API 人员").await;
    let manager = ctx(owner_id, manager_id);
    let now = Utc::now();
    let tasks = PgTaskEngineRepository::new(pool.clone());
    tasks
        .upsert_task_group(
            &manager,
            "pick-a",
            UpsertTaskGroupRequest {
                task_group_name: "释放 API 组".to_string(),
                warehouse_id,
                zone_ids: vec![],
                task_type_codes: vec!["pick".to_string()],
                member_user_ids: vec![worker_id],
                member_qualifications: vec![],
                enabled: true,
            },
            now,
            "mte-release-api-group-1",
        )
        .await
        .expect("release API group should persist");
    PgTaskTypeRepository::new(pool.clone())
        .upsert(
            &manager,
            "pick",
            UpsertTaskTypeRequest {
                task_type_name: "拣选".to_string(),
                default_priority: 100,
                estimated_minutes: 15,
                mergeable: true,
                insertable: true,
                enabled: true,
                release_strategy: TaskReleaseStrategy::Scheduled,
                release_interval_minutes: Some(10),
                release_batch_size: Some(10),
            },
            now,
            "mte-release-api-rule-1",
        )
        .await
        .expect("release API rule should persist");
    let task = tasks
        .create_task(
            &manager,
            create_request(warehouse_id),
            now,
            "mte-release-api-create-1",
        )
        .await
        .expect("release API task should create")
        .value;

    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let manager_token = encode_access_token(
        &build_access_claims(
            manager_id,
            owner_id,
            "mte-release-api-manager",
            vec!["mte.task.assign".to_string()],
            Uuid::new_v4().to_string(),
            now,
        ),
        "test-secret",
    )
    .expect("manager token should encode");
    let worker_token = encode_access_token(
        &build_access_claims(
            worker_id,
            owner_id,
            "mte-release-api-worker",
            vec!["mte.task.read".to_string()],
            Uuid::new_v4().to_string(),
            now,
        ),
        "test-secret",
    )
    .expect("worker token should encode");
    let app = task_engine_router(TaskEngineAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );
    let request = |token: &str, key: &str| {
        Request::post(format!("/api/v1/task-engine/tasks/{}/transitions", task.id))
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header("content-type", "application/json")
            .header("Idempotency-Key", key)
            .body(Body::from(r#"{"action":"release"}"#))
            .expect("release request should build")
    };

    let forbidden = app
        .clone()
        .oneshot(request(&worker_token, "mte-release-api-worker-1"))
        .await
        .expect("worker release should respond");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    let early = app
        .clone()
        .oneshot(request(&manager_token, "mte-release-api-early-1"))
        .await
        .expect("early release should respond");
    assert_eq!(early.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let early_body: serde_json::Value = serde_json::from_slice(
        &to_bytes(early.into_body(), usize::MAX)
            .await
            .expect("early release body should read"),
    )
    .expect("early release body should be json");
    assert_eq!(early_body["code"], "M_TE_RELEASE_CONDITION_NOT_MET");

    sqlx::query("UPDATE warehouse_tasks SET release_due_at = $1 WHERE owner_id = $2 AND id = $3")
        .bind(now - Duration::minutes(1))
        .bind(owner_id)
        .bind(task.id)
        .execute(&pool)
        .await
        .expect("release due time should move into the past");
    let released = app
        .clone()
        .oneshot(request(&manager_token, "mte-release-api-success-1"))
        .await
        .expect("due release should respond");
    assert_eq!(released.status(), StatusCode::OK);
    let replayed = app
        .clone()
        .oneshot(request(&manager_token, "mte-release-api-success-1"))
        .await
        .expect("release replay should respond");
    assert_eq!(replayed.status(), StatusCode::OK);
    let repeated = app
        .oneshot(request(&manager_token, "mte-release-api-success-2"))
        .await
        .expect("released task should return its current state");
    assert_eq!(repeated.status(), StatusCode::OK);
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event WHERE owner_id = $1 AND module = 'M-TE' AND action = 'release_task'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("release audits should count");
    assert_eq!(audits, 1);
}
