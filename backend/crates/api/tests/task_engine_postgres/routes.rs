use super::*;

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
                member_qualifications: vec![],
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
            "mte.priority_rule.write".to_string(),
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

    let priority_rule = app
        .clone()
        .oneshot(
            Request::get("/api/v1/task-engine/priority-rule")
                .header(AUTHORIZATION, format!("Bearer {manager_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("priority rule should respond");
    assert_eq!(priority_rule.status(), StatusCode::OK);

    let forbidden_priority_write = app
        .clone()
        .oneshot(
            Request::put("/api/v1/task-engine/priority-rule")
                .header(AUTHORIZATION, format!("Bearer {worker_token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "mte-api-priority-worker")
                .body(Body::from(
                    serde_json::json!({
                        "urgent_order_bonus": 30,
                        "waiting_minutes_per_point": 5,
                        "cold_chain_bonus": 20,
                        "manual_expedite_bonus": 40
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("worker priority rule write should respond");
    assert_eq!(forbidden_priority_write.status(), StatusCode::FORBIDDEN);

    let forbidden_group_write = app
        .clone()
        .oneshot(
            Request::put("/api/v1/task-engine/task-groups/pick-forbidden")
                .header(AUTHORIZATION, format!("Bearer {worker_token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "mte-api-group-worker")
                .body(Body::from(
                    serde_json::json!({
                        "task_group_name": "无权限组",
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
        .expect("worker task group write should respond");
    assert_eq!(forbidden_group_write.status(), StatusCode::FORBIDDEN);

    let invalid_priority = app
        .clone()
        .oneshot(
            Request::put("/api/v1/task-engine/priority-rule")
                .header(AUTHORIZATION, format!("Bearer {manager_token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "mte-api-priority-invalid")
                .body(Body::from(
                    serde_json::json!({
                        "urgent_order_bonus": 30,
                        "waiting_minutes_per_point": 0,
                        "cold_chain_bonus": 20,
                        "manual_expedite_bonus": 40
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("invalid priority rule should respond");
    assert_eq!(invalid_priority.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let invalid_body: serde_json::Value = serde_json::from_slice(
        &to_bytes(invalid_priority.into_body(), usize::MAX)
            .await
            .expect("invalid priority response should read"),
    )
    .expect("invalid priority response should be json");
    assert_eq!(invalid_body["code"], "M_TE_RULE_INVALID");

    let saved_priority = app
        .clone()
        .oneshot(
            Request::put("/api/v1/task-engine/priority-rule")
                .header(AUTHORIZATION, format!("Bearer {manager_token}"))
                .header("content-type", "application/json")
                .header("Idempotency-Key", "mte-api-priority-save")
                .body(Body::from(
                    serde_json::json!({
                        "urgent_order_bonus": 30,
                        "waiting_minutes_per_point": 5,
                        "cold_chain_bonus": 20,
                        "manual_expedite_bonus": 40
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("priority rule save should respond");
    assert_eq!(saved_priority.status(), StatusCode::OK);

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
