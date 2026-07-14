use super::*;

async fn seed_product(pool: &PgPool, owner_id: Uuid, storage_condition: &str) -> (Uuid, String) {
    let product_id = Uuid::new_v4();
    let product_code = format!("P-{}", &product_id.simple().to_string()[..8]);
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification,
            storage_condition, special_drug_category
        ) VALUES ($1, $2, $3, '优先级测试商品', '1 盒', $4, 'normal')
        "#,
    )
    .bind(product_id)
    .bind(owner_id)
    .bind(&product_code)
    .bind(storage_condition)
    .execute(pool)
    .await
    .expect("priority product should insert");
    (product_id, product_code)
}

#[sqlx::test(migrations = "../../migrations")]
async fn priority_rules_apply_configured_factors_sort_waiting_and_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let warehouse_id = seed_warehouse(&pool, owner_id).await;
    let manager_id = Uuid::new_v4();
    seed_user(&pool, owner_id, manager_id, "优先级主管").await;
    let repository = PgTaskEngineRepository::new(pool.clone());
    let manager = ctx(owner_id, manager_id);
    let now = Utc::now();
    repository
        .upsert_task_group(
            &manager,
            "pick-a",
            UpsertTaskGroupRequest {
                task_group_name: "优先级测试组".to_string(),
                warehouse_id,
                zone_ids: vec![],
                task_type_codes: vec!["pick".to_string()],
                member_user_ids: vec![],
                member_qualifications: vec![],
                enabled: true,
            },
            now,
            "mte-priority-group-1",
        )
        .await
        .expect("priority task group should persist");

    let invalid = repository
        .upsert_priority_rule(
            &manager,
            UpsertTaskPriorityRuleRequest {
                urgent_order_bonus: 30,
                waiting_minutes_per_point: 0,
                cold_chain_bonus: 20,
                manual_expedite_bonus: 40,
            },
            now,
            "mte-priority-invalid-1",
        )
        .await
        .expect_err("zero waiting interval must be rejected");
    assert_eq!(invalid, TaskEngineError::PriorityRuleInvalid);

    let request = UpsertTaskPriorityRuleRequest {
        urgent_order_bonus: 30,
        waiting_minutes_per_point: 5,
        cold_chain_bonus: 20,
        manual_expedite_bonus: 40,
    };
    let saved = repository
        .upsert_priority_rule(&manager, request.clone(), now, "mte-priority-rule-1")
        .await
        .expect("priority rule should persist");
    assert_eq!(saved.value.urgent_order_bonus, 30);
    assert_eq!(saved.value.waiting_minutes_per_point, 5);
    assert!(!saved.replayed);
    let replay = repository
        .upsert_priority_rule(&manager, request, now, "mte-priority-rule-1")
        .await
        .expect("same priority rule request should replay");
    assert!(replay.replayed);
    assert_eq!(
        repository.get_priority_rule(&manager).await.unwrap(),
        saved.value
    );

    let (normal_product_id, normal_product_code) = seed_product(&pool, owner_id, "normal").await;
    let (_cold_product_id, cold_product_code) = seed_product(&pool, owner_id, "cold").await;
    let mut urgent_request = create_request(warehouse_id);
    urgent_request.source_doc_no = "SO-PRIORITY-URGENT".to_string();
    urgent_request.source_task_key = "M4:SO-PRIORITY-URGENT:1:pick".to_string();
    urgent_request.product_id = Some(normal_product_id);
    urgent_request.product_code = normal_product_code.clone();
    urgent_request.urgent_order = true;
    let urgent = repository
        .create_task(&manager, urgent_request, now, "mte-priority-urgent-1")
        .await
        .expect("urgent task should create")
        .value;
    assert_eq!(urgent.priority, 130, "type default plus urgent bonus");
    assert!(urgent.urgent_order);
    assert!(!urgent.cold_chain);

    let mut cold_request = create_request(warehouse_id);
    cold_request.source_doc_no = "SO-PRIORITY-COLD".to_string();
    cold_request.source_task_key = "M4:SO-PRIORITY-COLD:1:pick".to_string();
    cold_request.product_id = None;
    cold_request.product_code = cold_product_code;
    let cold = repository
        .create_task(&manager, cold_request, now, "mte-priority-cold-1")
        .await
        .expect("cold task should create")
        .value;
    assert_eq!(cold.priority, 120, "type default plus cold-chain bonus");
    assert!(cold.cold_chain);

    let mut waiting_request = create_request(warehouse_id);
    waiting_request.source_doc_no = "SO-PRIORITY-WAITING".to_string();
    waiting_request.source_task_key = "M4:SO-PRIORITY-WAITING:1:pick".to_string();
    waiting_request.product_id = Some(normal_product_id);
    waiting_request.product_code = normal_product_code;
    let waiting = repository
        .create_task(&manager, waiting_request, now, "mte-priority-waiting-1")
        .await
        .expect("waiting task should create")
        .value;
    sqlx::query("UPDATE warehouse_tasks SET created_at = $1 WHERE id = $2")
        .bind(now - Duration::minutes(200))
        .bind(waiting.id)
        .execute(&pool)
        .await
        .expect("waiting time should be controlled");

    let listed = repository
        .list_tasks(
            &manager,
            TaskListQuery {
                mine_only: false,
                status: None,
                task_type_code: None,
                warehouse_id: Some(warehouse_id),
                limit: Some(20),
            },
        )
        .await
        .expect("priority queue should list");
    assert_eq!(listed[0].id, waiting.id);
    assert!(
        listed[0].priority >= 140,
        "waiting time should raise priority"
    );
    assert_eq!(listed[1].id, urgent.id);
    assert_eq!(listed[2].id, cold.id);

    let expedited = repository
        .transition_task(
            &manager,
            cold.id,
            transition(TaskTransitionAction::Expedite),
            now,
            "mte-priority-expedite-1",
        )
        .await
        .expect("supervisor should expedite an active task")
        .value;
    assert_eq!(expedited.priority, 160);
    assert!(expedited.manually_expedited);
    let repeated = repository
        .transition_task(
            &manager,
            cold.id,
            transition(TaskTransitionAction::Expedite),
            now,
            "mte-priority-expedite-2",
        )
        .await
        .expect_err("manual expedite is a one-time action");
    assert_eq!(repeated, TaskEngineError::InvalidTransition);

    let priority_audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event WHERE owner_id = $1 AND module = 'M-TE' AND action IN ('upsert_priority_rule', 'expedite_task')",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("priority audit should query");
    assert_eq!(
        priority_audits, 2,
        "rule change and expedite must be audited"
    );
}
