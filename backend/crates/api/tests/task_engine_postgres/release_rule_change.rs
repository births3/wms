use super::*;

#[sqlx::test(migrations = "../../migrations")]
async fn existing_pending_task_releases_after_strategy_changes_to_immediate(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let warehouse_id = seed_warehouse(&pool, owner_id).await;
    let manager_id = Uuid::new_v4();
    seed_user(&pool, owner_id, manager_id, "释放规则主管").await;
    let manager = ctx(owner_id, manager_id);
    let now = Utc::now();
    let tasks = PgTaskEngineRepository::new(pool.clone());

    tasks
        .upsert_task_group(
            &manager,
            "pick-a",
            UpsertTaskGroupRequest {
                task_group_name: "释放规则组".to_string(),
                warehouse_id,
                zone_ids: vec![],
                task_type_codes: vec!["pick".to_string()],
                member_user_ids: vec![],
                member_qualifications: vec![],
                enabled: true,
            },
            now,
            "mte-release-rule-change-group-1",
        )
        .await
        .expect("release rule group should persist");
    let types = PgTaskTypeRepository::new(pool.clone());
    types
        .upsert(
            &manager,
            "pick",
            release_type_request(TaskReleaseStrategy::Scheduled),
            now,
            "mte-release-rule-change-1",
        )
        .await
        .expect("scheduled release rule should persist");
    let task = tasks
        .create_task(
            &manager,
            create_request(warehouse_id),
            now,
            "mte-release-rule-change-create-1",
        )
        .await
        .expect("scheduled task should create")
        .value;
    assert_eq!(task.status, "pending_release");

    types
        .upsert(
            &manager,
            "pick",
            release_type_request(TaskReleaseStrategy::Immediate),
            now + Duration::minutes(1),
            "mte-release-rule-change-2",
        )
        .await
        .expect("immediate release rule should persist");
    assert_eq!(
        task_release_job::run_once(&pool, now + Duration::minutes(1))
            .await
            .unwrap(),
        1
    );
    let status: String =
        sqlx::query_scalar("SELECT status FROM warehouse_tasks WHERE owner_id = $1 AND id = $2")
            .bind(owner_id)
            .bind(task.id)
            .fetch_one(&pool)
            .await
            .expect("released task should query");
    assert_eq!(status, "pending_assignment");
}

fn release_type_request(strategy: TaskReleaseStrategy) -> UpsertTaskTypeRequest {
    let scheduled = strategy == TaskReleaseStrategy::Scheduled;
    UpsertTaskTypeRequest {
        task_type_name: "拣选".to_string(),
        default_priority: 100,
        estimated_minutes: 15,
        mergeable: true,
        insertable: true,
        enabled: true,
        release_strategy: strategy,
        release_interval_minutes: scheduled.then_some(10),
        release_batch_size: scheduled.then_some(10),
    }
}
