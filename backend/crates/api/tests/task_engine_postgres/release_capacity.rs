use super::*;

#[sqlx::test(migrations = "../../migrations")]
async fn capacity_release_reuses_worker_active_task_limit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let warehouse_id = seed_warehouse(&pool, owner_id).await;
    let manager_id = Uuid::new_v4();
    let worker_id = Uuid::new_v4();
    seed_user(&pool, owner_id, manager_id, "容量释放主管").await;
    seed_user(&pool, owner_id, worker_id, "容量释放执行人").await;
    let manager = ctx(owner_id, manager_id);
    let worker = worker_ctx(owner_id, worker_id);
    let now = Utc::now();
    let tasks = PgTaskEngineRepository::new(pool.clone());

    tasks
        .upsert_task_group(
            &manager,
            "pick-a",
            UpsertTaskGroupRequest {
                task_group_name: "容量释放组".to_string(),
                warehouse_id,
                zone_ids: vec![],
                task_type_codes: vec!["pick".to_string(), "putaway".to_string()],
                member_user_ids: vec![worker_id],
                member_qualifications: vec![TaskGroupMemberQualification {
                    user_id: worker_id,
                    valid_until: None,
                    max_active_tasks: Some(1),
                }],
                enabled: true,
            },
            now,
            "mte-capacity-release-group-1",
        )
        .await
        .expect("capacity release group should persist");
    let capacity_rule = |name: &str| UpsertTaskTypeRequest {
        task_type_name: name.to_string(),
        default_priority: 100,
        estimated_minutes: 15,
        mergeable: true,
        insertable: true,
        enabled: true,
        release_strategy: TaskReleaseStrategy::Capacity,
        release_interval_minutes: None,
        release_batch_size: None,
    };
    let types = PgTaskTypeRepository::new(pool.clone());
    types
        .upsert(
            &manager,
            "pick",
            capacity_rule("拣选"),
            now,
            "mte-capacity-release-rule-1",
        )
        .await
        .expect("capacity release rule should persist");
    types
        .upsert(
            &manager,
            "putaway",
            capacity_rule("上架"),
            now,
            "mte-capacity-release-rule-2",
        )
        .await
        .expect("second capacity release rule should persist");

    let first = tasks
        .create_task(
            &manager,
            create_request(warehouse_id),
            now,
            "mte-capacity-release-create-1",
        )
        .await
        .expect("task should release while worker capacity is available")
        .value;
    assert_eq!(first.status, "pending_assignment");
    let mut second_request = create_request(warehouse_id);
    second_request.source_doc_id = Some(Uuid::new_v4());
    second_request.source_doc_no = "SO-MTE-CAPACITY-RELEASE-002".to_string();
    second_request.source_task_key = "M4:SO-MTE-CAPACITY-RELEASE-002:1:pick".to_string();
    let second = tasks
        .create_task(
            &manager,
            second_request,
            now,
            "mte-capacity-release-create-2",
        )
        .await
        .expect("task should create while capacity is full")
        .value;
    assert_eq!(second.status, "pending_release");
    let mut third_request = create_request(warehouse_id);
    third_request.task_type_code = "putaway".to_string();
    third_request.source_doc_id = Some(Uuid::new_v4());
    third_request.source_doc_no = "SO-MTE-CAPACITY-RELEASE-003".to_string();
    third_request.source_task_key = "M2:SO-MTE-CAPACITY-RELEASE-003:1:putaway".to_string();
    let third = tasks
        .create_task(
            &manager,
            third_request,
            now,
            "mte-capacity-release-create-3",
        )
        .await
        .expect("another task should wait while capacity is full")
        .value;
    assert_eq!(third.status, "pending_release");
    tasks
        .transition_task(
            &manager,
            first.id,
            TransitionWarehouseTaskRequest {
                assignee_user_id: Some(worker_id),
                ..transition(TaskTransitionAction::Assign)
            },
            now,
            "mte-capacity-release-assign-1",
        )
        .await
        .expect("first capacity task should assign");
    assert_eq!(task_release_job::run_once(&pool, now).await.unwrap(), 0);

    tasks
        .transition_task(
            &manager,
            first.id,
            transition(TaskTransitionAction::Dispatch),
            now,
            "mte-capacity-release-dispatch-1",
        )
        .await
        .expect("first capacity task should dispatch");
    tasks
        .transition_task(
            &worker,
            first.id,
            transition(TaskTransitionAction::Start),
            now,
            "mte-capacity-release-start-1",
        )
        .await
        .expect("first capacity task should start");
    tasks
        .transition_task(
            &worker,
            first.id,
            TransitionWarehouseTaskRequest {
                actual_qty: Some(10.into()),
                ..transition(TaskTransitionAction::Complete)
            },
            now,
            "mte-capacity-release-complete-1",
        )
        .await
        .expect("first capacity task should complete");

    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(1))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(2))
            .await
            .unwrap(),
        0
    );
    let statuses: Vec<String> = sqlx::query_scalar(
        "SELECT status FROM warehouse_tasks WHERE owner_id = $1 AND id = ANY($2) ORDER BY id",
    )
    .bind(owner_id)
    .bind(vec![second.id, third.id])
    .fetch_all(&pool)
    .await
    .expect("capacity task statuses should query");
    assert_eq!(
        statuses
            .iter()
            .filter(|status| *status == "pending_assignment")
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| *status == "pending_release")
            .count(),
        1
    );
}
