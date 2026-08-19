use super::*;

#[sqlx::test(migrations = "../../migrations")]
async fn scheduled_release_respects_due_time_batch_size_idempotency_and_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let warehouse_id = seed_warehouse(&pool, owner_id).await;
    let manager_id = Uuid::new_v4();
    seed_user(&pool, owner_id, manager_id, "释放主管").await;
    let manager = ctx(owner_id, manager_id);
    let now = Utc::now();
    let tasks = PgTaskEngineRepository::new(pool.clone());

    tasks
        .upsert_task_group(
            &manager,
            "pick-a",
            UpsertTaskGroupRequest {
                task_group_name: "定时释放组".to_string(),
                warehouse_id,
                zone_ids: vec![],
                task_type_codes: vec!["pick".to_string()],
                member_user_ids: vec![],
                member_qualifications: vec![],
                enabled: true,
            },
            now,
            "mte-release-group-1",
        )
        .await
        .expect("scheduled release group should persist");
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
                release_batch_size: Some(1),
            },
            now,
            "mte-release-rule-1",
        )
        .await
        .expect("scheduled release rule should persist");

    let first = tasks
        .create_task(
            &manager,
            create_request(warehouse_id),
            now,
            "mte-release-create-1",
        )
        .await
        .expect("first scheduled task should create")
        .value;
    let mut second_request = create_request(warehouse_id);
    second_request.source_doc_id = Some(Uuid::new_v4());
    second_request.source_doc_no = "SO-MTE-RELEASE-002".to_string();
    second_request.source_task_key = "M4:SO-MTE-RELEASE-002:1:pick".to_string();
    let second = tasks
        .create_task(&manager, second_request, now, "mte-release-create-2")
        .await
        .expect("second scheduled task should create")
        .value;

    assert_eq!(first.status, "pending_release");
    assert_eq!(second.status, "pending_release");
    assert_eq!(
        first.release_due_at.map(|value| value.timestamp_micros()),
        Some((now + Duration::minutes(10)).timestamp_micros())
    );
    assert_eq!(first.released_at, None);
    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(9))
            .await
            .unwrap(),
        0
    );

    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(10))
            .await
            .unwrap(),
        1
    );
    let statuses: Vec<(Uuid, String, Option<chrono::DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, status, release_due_at FROM warehouse_tasks WHERE owner_id = $1 ORDER BY id",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("scheduled task statuses should query");
    assert_eq!(
        statuses
            .iter()
            .filter(|(_, status, _)| status == "pending_assignment")
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|(_, status, _)| status == "pending_release")
            .count(),
        1
    );
    assert!(statuses
        .iter()
        .filter(|(_, status, _)| status == "pending_release")
        .all(
            |(_, _, due_at)| due_at.map(|value| value.timestamp_micros())
                == Some((now + Duration::minutes(20)).timestamp_micros())
        ));

    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(10))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(20))
            .await
            .unwrap(),
        1
    );
    let released: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM warehouse_tasks WHERE owner_id = $1 AND status = 'pending_assignment' AND released_at IS NOT NULL",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("released tasks should count");
    assert_eq!(released, 2);
    let release_audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event WHERE owner_id = $1 AND module = 'M-TE' AND action = 'release_task'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("release audits should count");
    assert_eq!(release_audits, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn conditional_release_waits_for_predecessor_completion(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let warehouse_id = seed_warehouse(&pool, owner_id).await;
    let manager_id = Uuid::new_v4();
    let worker_id = Uuid::new_v4();
    seed_user(&pool, owner_id, manager_id, "条件释放主管").await;
    seed_user(&pool, owner_id, worker_id, "条件释放执行人").await;
    let manager = ctx(owner_id, manager_id);
    let worker = worker_ctx(owner_id, worker_id);
    let now = Utc::now();
    let tasks = PgTaskEngineRepository::new(pool.clone());

    tasks
        .upsert_task_group(
            &manager,
            "pick-a",
            UpsertTaskGroupRequest {
                task_group_name: "条件释放组".to_string(),
                warehouse_id,
                zone_ids: vec![],
                task_type_codes: vec!["pick".to_string(), "putaway".to_string()],
                member_user_ids: vec![worker_id],
                member_qualifications: vec![],
                enabled: true,
            },
            now,
            "mte-condition-group-1",
        )
        .await
        .expect("conditional release group should persist");
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
                release_strategy: TaskReleaseStrategy::Conditional,
                release_interval_minutes: None,
                release_batch_size: None,
            },
            now,
            "mte-condition-rule-1",
        )
        .await
        .expect("conditional release rule should persist");

    let mut predecessor_request = create_request(warehouse_id);
    predecessor_request.task_type_code = "putaway".to_string();
    predecessor_request.source_doc_id = Some(Uuid::new_v4());
    predecessor_request.source_doc_no = "ASN-MTE-CONDITION-001".to_string();
    predecessor_request.source_task_key = "M2:ASN-MTE-CONDITION-001:1:putaway".to_string();
    let predecessor = tasks
        .create_task(
            &manager,
            predecessor_request,
            now,
            "mte-condition-predecessor-1",
        )
        .await
        .expect("predecessor should create")
        .value;
    let mut dependent_request = create_request(warehouse_id);
    dependent_request.source_doc_id = Some(Uuid::new_v4());
    dependent_request.source_doc_no = "SO-MTE-CONDITION-001".to_string();
    dependent_request.source_task_key = "M4:SO-MTE-CONDITION-001:1:pick".to_string();
    dependent_request.predecessor_task_id = Some(predecessor.id);
    let dependent = tasks
        .create_task(
            &manager,
            dependent_request,
            now,
            "mte-condition-dependent-1",
        )
        .await
        .expect("dependent task should create")
        .value;
    assert_eq!(dependent.status, "pending_release");
    assert_eq!(dependent.predecessor_task_id, Some(predecessor.id));

    let early = tasks
        .transition_task(
            &manager,
            dependent.id,
            transition(TaskTransitionAction::Release),
            now,
            "mte-condition-early-release-1",
        )
        .await
        .expect_err("unfinished predecessor must block release");
    assert_eq!(early, TaskEngineError::ReleaseConditionNotMet);
    tasks
        .transition_task(
            &manager,
            predecessor.id,
            TransitionWarehouseTaskRequest {
                assignee_user_id: Some(worker_id),
                ..transition(TaskTransitionAction::Assign)
            },
            now,
            "mte-condition-assign-1",
        )
        .await
        .expect("predecessor should assign");
    tasks
        .transition_task(
            &manager,
            predecessor.id,
            transition(TaskTransitionAction::Dispatch),
            now,
            "mte-condition-dispatch-1",
        )
        .await
        .expect("predecessor should dispatch");
    tasks
        .transition_task(
            &worker,
            predecessor.id,
            transition(TaskTransitionAction::Start),
            now,
            "mte-condition-start-1",
        )
        .await
        .expect("predecessor should start");
    tasks
        .transition_task(
            &worker,
            predecessor.id,
            TransitionWarehouseTaskRequest {
                actual_qty: Some(10.into()),
                ..transition(TaskTransitionAction::Complete)
            },
            now,
            "mte-condition-complete-1",
        )
        .await
        .expect("predecessor should complete");

    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(1))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(1))
            .await
            .unwrap(),
        0
    );
    let released: (String, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
        "SELECT status, released_at FROM warehouse_tasks WHERE id = $1 AND owner_id = $2",
    )
    .bind(dependent.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("released dependent should query");
    assert_eq!(released.0, "pending_assignment");
    assert!(released.1.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn dispatched_task_timeout_is_reassigned_once(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let warehouse_id = seed_warehouse(&pool, owner_id).await;
    let manager_id = Uuid::new_v4();
    let first_worker_id = Uuid::from_u128(0x10000000000000000000000000000001);
    let second_worker_id = Uuid::from_u128(0x10000000000000000000000000000002);
    seed_user(&pool, owner_id, manager_id, "超时释放主管").await;
    seed_user(&pool, owner_id, first_worker_id, "超时执行人一").await;
    seed_user(&pool, owner_id, second_worker_id, "超时执行人二").await;
    let manager = ctx(owner_id, manager_id);
    let now = Utc::now();
    let tasks = PgTaskEngineRepository::new(pool.clone());

    tasks
        .upsert_task_group(
            &manager,
            "pick-a",
            UpsertTaskGroupRequest {
                task_group_name: "超时释放组".to_string(),
                warehouse_id,
                zone_ids: vec![],
                task_type_codes: vec!["pick".to_string()],
                member_user_ids: vec![first_worker_id, second_worker_id],
                member_qualifications: vec![],
                enabled: true,
            },
            now,
            "mte-timeout-release-group-1",
        )
        .await
        .expect("timeout release group should persist");
    let task = tasks
        .create_task(
            &manager,
            create_request(warehouse_id),
            now,
            "mte-timeout-release-create-1",
        )
        .await
        .expect("timeout task should create")
        .value;
    tasks
        .transition_task(
            &manager,
            task.id,
            TransitionWarehouseTaskRequest {
                assignee_user_id: Some(first_worker_id),
                ..transition(TaskTransitionAction::Assign)
            },
            now,
            "mte-timeout-release-assign-1",
        )
        .await
        .expect("timeout task should assign");
    tasks
        .transition_task(
            &manager,
            task.id,
            transition(TaskTransitionAction::Dispatch),
            now,
            "mte-timeout-release-dispatch-1",
        )
        .await
        .expect("timeout task should dispatch");

    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(14))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(15))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(15))
            .await
            .unwrap(),
        0
    );
    let reassigned: (String, Option<Uuid>, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
        "SELECT status, assignee_user_id, assigned_at FROM warehouse_tasks WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(task.id)
    .fetch_one(&pool)
    .await
    .expect("reassigned task should query");
    assert_eq!(reassigned.0, "assigned");
    assert_eq!(reassigned.1, Some(second_worker_id));
    assert_eq!(
        reassigned.2.map(|value| value.timestamp_micros()),
        Some((now + Duration::minutes(15)).timestamp_micros())
    );
    let reassign_audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event WHERE owner_id = $1 AND module = 'M-TE' AND action = 'reassign_task'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("timeout reassign audit should count");
    assert_eq!(reassign_audits, 1);
}
