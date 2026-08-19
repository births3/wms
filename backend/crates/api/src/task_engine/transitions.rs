pub(crate) async fn create_task_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    request: &CreateWarehouseTaskRequest,
    now: DateTime<Utc>,
) -> Result<WarehouseTask, TaskEngineError> {
    let request = normalize_create_request(request.clone())?;
    create_normalized_task_in_tx(tx, ctx, &request, now).await
}

async fn create_normalized_task_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    request: &CreateWarehouseTaskRequest,
    now: DateTime<Utc>,
) -> Result<WarehouseTask, TaskEngineError> {
    lock_key(
        tx,
        "mte-source-task",
        ctx.owner_id,
        &source_identity_key(request),
    )
    .await?;
    if let Some(existing) =
        load_task_by_source_key(tx, ctx.owner_id, &request.source_task_key).await?
    {
        if task_matches_request(&existing, request) {
            return Ok(existing.into());
        }
        return Err(TaskEngineError::SourceTaskConflict);
    }
    if let Some(existing) = load_task_by_source_identity(tx, ctx.owner_id, request).await? {
        if task_matches_request(&existing, request) {
            return Ok(existing.into());
        }
        return Err(TaskEngineError::SourceTaskConflict);
    }
    let task_type: Option<(i32, i32, bool, String, Option<i32>)> = sqlx::query_as(
        "SELECT default_priority, estimated_minutes, enabled, release_strategy, release_interval_minutes FROM task_types WHERE owner_id = $1 AND task_type_code = $2",
    )
    .bind(ctx.owner_id)
    .bind(&request.task_type_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let (default_priority, estimated_minutes, task_type_enabled, release_strategy, release_interval) =
        task_type.ok_or(TaskEngineError::TaskTypeNotFound)?;
    if !task_type_enabled {
        return Err(TaskEngineError::TaskTypeNotFound);
    }
    if request.task_group_code == default_task_group_code(request.warehouse_id) {
        ensure_default_task_group_in_tx(tx, ctx.owner_id, request.warehouse_id, now).await?;
    }
    let group: Option<(Uuid, bool, Vec<String>)> = sqlx::query_as(
        "SELECT warehouse_id, enabled, task_type_codes FROM task_groups WHERE owner_id = $1 AND task_group_code = $2",
    )
    .bind(ctx.owner_id)
    .bind(&request.task_group_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let (group_warehouse_id, group_enabled, task_type_codes) =
        group.ok_or(TaskEngineError::TaskGroupNotFound)?;
    if !group_enabled
        || group_warehouse_id != request.warehouse_id
        || !task_type_codes.contains(&request.task_type_code)
    {
        return Err(TaskEngineError::TaskGroupNotFound);
    }
    let capacity_available = release_strategy == "capacity"
        && has_available_worker(
            tx,
            ctx.owner_id,
            &request.task_group_code,
            request.warehouse_id,
            &request.task_type_code,
            now,
        )
        .await?;
    let base_priority = request.priority.unwrap_or(default_priority);
    if !(0..=1000).contains(&base_priority) {
        return Err(TaskEngineError::Validation(
            "priority must be between 0 and 1000".to_string(),
        ));
    }
    let rule: (i32, i32) = sqlx::query_as(
        "SELECT urgent_order_bonus, cold_chain_bonus FROM task_priority_rules WHERE owner_id = $1",
    )
    .bind(ctx.owner_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let cold_chain: bool = sqlx::query_scalar(
        r#"
        SELECT COALESCE((
            SELECT storage_condition IN ('freeze_le_minus_20', 'cold_2_8', 'cool_le_20')
              FROM products
             WHERE owner_id = $1
               AND (id = $2 OR ($2::UUID IS NULL AND product_code = $3))
             LIMIT 1
        ), FALSE)
        "#,
    )
    .bind(ctx.owner_id)
    .bind(request.product_id)
    .bind(&request.product_code)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let priority = (base_priority
        + if request.urgent_order { rule.0 } else { 0 }
        + if cold_chain { rule.1 } else { 0 })
        .min(1000);
    let id = Uuid::new_v4();
    let task_no = format!(
        "TE-{}-{}",
        now.format("%Y%m%d%H%M%S"),
        &id.simple().to_string()[..8]
    );
    let predecessor_completed = match request.predecessor_task_id {
        Some(predecessor_id) => sqlx::query_scalar::<_, String>(
            "SELECT status FROM warehouse_tasks WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(predecessor_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_database_error)?
        .map(|status| status == TASK_STATUS_COMPLETED)
        .ok_or(TaskEngineError::ReleaseConditionNotMet)?,
        None => false,
    };
    if release_strategy == "conditional" && request.predecessor_task_id.is_none() {
        return Err(TaskEngineError::Validation(
            "predecessor_task_id is required for conditional release".to_string(),
        ));
    }
    if release_strategy != "conditional" && request.predecessor_task_id.is_some() {
        return Err(TaskEngineError::Validation(
            "predecessor_task_id is only allowed for conditional release".to_string(),
        ));
    }
    let (status, release_due_at, released_at) = match release_strategy.as_str() {
        "scheduled" => (
            TASK_STATUS_PENDING_RELEASE,
            Some(now + Duration::minutes(i64::from(release_interval.unwrap_or(1)))),
            None,
        ),
        "conditional" if !predecessor_completed => (TASK_STATUS_PENDING_RELEASE, None, None),
        "capacity" if !capacity_available => (TASK_STATUS_PENDING_RELEASE, None, None),
        _ => (TASK_STATUS_PENDING_ASSIGNMENT, None, Some(now)),
    };
    let row = sqlx::query_as::<_, WarehouseTaskRow>(
        r#"
        INSERT INTO warehouse_tasks (
            id, owner_id, task_no, task_type_code, source_module, source_doc_type,
            source_doc_id, source_doc_no, source_line_no, source_task_key,
            warehouse_id, task_group_code, product_id, product_code, batch_id,
            batch_no, planned_qty, source_location_id, source_location_code,
            target_location_id, target_location_code, priority, urgent_order,
            cold_chain, estimated_minutes, predecessor_task_id, release_due_at,
            released_at, status,
            created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19,
            $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $30
        )
        RETURNING id, owner_id, task_no, task_type_code, source_module, source_doc_type,
                  source_doc_id, source_doc_no, source_line_no, source_task_key,
                  warehouse_id, task_group_code, product_id, product_code, batch_id,
                  batch_no, planned_qty, actual_qty, source_location_id,
                  source_location_code, target_location_id, target_location_code,
                  priority, urgent_order, cold_chain, manually_expedited,
                  estimated_minutes, predecessor_task_id, release_due_at, released_at,
                  assignee_user_id, status, exception_code,
                  exception_note, assigned_at, dispatched_at, started_at, completed_at,
                  created_at, updated_at, version
        "#,
    )
    .bind(id)
    .bind(ctx.owner_id)
    .bind(task_no)
    .bind(&request.task_type_code)
    .bind(&request.source_module)
    .bind(&request.source_doc_type)
    .bind(request.source_doc_id)
    .bind(&request.source_doc_no)
    .bind(request.source_line_no)
    .bind(&request.source_task_key)
    .bind(request.warehouse_id)
    .bind(&request.task_group_code)
    .bind(request.product_id)
    .bind(&request.product_code)
    .bind(request.batch_id)
    .bind(&request.batch_no)
    .bind(request.planned_qty)
    .bind(request.source_location_id)
    .bind(&request.source_location_code)
    .bind(request.target_location_id)
    .bind(&request.target_location_code)
    .bind(priority)
    .bind(request.urgent_order)
    .bind(cold_chain)
    .bind(estimated_minutes)
    .bind(request.predecessor_task_id)
    .bind(release_due_at)
    .bind(released_at)
    .bind(status)
    .bind(now)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let value = WarehouseTask::from(row);
    append_task_event(
        tx,
        ctx,
        value.id,
        "create_task",
        None,
        &value.status,
        None,
        None,
        None,
        None,
        now,
    )
    .await?;
    append_audit(
        tx,
        ctx,
        "create_task",
        "warehouse_task",
        value.id,
        None,
        &value,
        now,
    )
    .await?;
    Ok(value)
}

fn source_identity_key(request: &CreateWarehouseTaskRequest) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        request.source_module,
        request.source_doc_type,
        request
            .source_doc_id
            .map_or_else(|| "none".to_string(), |id| id.to_string()),
        request
            .source_doc_id
            .map_or_else(|| request.source_doc_no.clone(), |_| "by-id".to_string()),
        request
            .source_line_no
            .map_or_else(|| "none".to_string(), |line| line.to_string()),
        request.task_type_code,
        request
            .batch_id
            .map_or_else(|| "none".to_string(), |id| id.to_string()),
    )
}

async fn ensure_default_task_group_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    warehouse_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), TaskEngineError> {
    let code = default_task_group_code(warehouse_id);
    let inserted = sqlx::query(
        r#"
        INSERT INTO task_groups (
            id, owner_id, task_group_code, task_group_name, warehouse_id,
            zone_ids, task_type_codes, enabled, created_at, updated_at
        )
        SELECT $1, $2, $3, warehouse_name || '默认任务组', id,
               '{}',
               ARRAY['inventory_count', 'loading', 'pick', 'putaway', 'relocation', 'replenish', 'return_putaway'],
               TRUE, $5, $5
          FROM warehouses
         WHERE owner_id = $2 AND id = $4 AND status = 'active'
        ON CONFLICT (owner_id, task_group_code) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(code)
    .bind(warehouse_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_database_error)?;
    if inserted.rows_affected() == 0 {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM task_groups WHERE owner_id = $1 AND task_group_code = $2 AND warehouse_id = $3)",
        )
        .bind(owner_id)
        .bind(default_task_group_code(warehouse_id))
        .bind(warehouse_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_database_error)?;
        if !exists {
            return Err(TaskEngineError::WarehouseNotFound);
        }
    }
    Ok(())
}

#[derive(Debug)]
struct ResolvedTransition {
    status: String,
    assignee_user_id: Option<Uuid>,
    actual_qty: Option<wms_domain::Quantity>,
    exception_code: Option<String>,
    exception_note: Option<String>,
    priority: i32,
    manually_expedited: bool,
}

async fn resolve_transition(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    task: &WarehouseTaskRow,
    request: &TransitionWarehouseTaskRequest,
    now: DateTime<Utc>,
) -> Result<ResolvedTransition, TaskEngineError> {
    let unchanged = || ResolvedTransition {
        status: TASK_STATUS_PENDING_ASSIGNMENT.to_string(),
        assignee_user_id: None,
        actual_qty: task.actual_qty,
        exception_code: task.exception_code.clone(),
        exception_note: task.exception_note.clone(),
        priority: task.priority,
        manually_expedited: task.manually_expedited,
    };
    match request.action {
        TaskTransitionAction::Release if task.status == TASK_STATUS_PENDING_RELEASE => {
            let strategy: String = sqlx::query_scalar(
                "SELECT release_strategy FROM task_types WHERE owner_id = $1 AND task_type_code = $2",
            )
            .bind(task.owner_id)
            .bind(&task.task_type_code)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_database_error)?;
            let condition_met = match strategy.as_str() {
                "scheduled" => task.release_due_at.is_some_and(|due_at| now >= due_at),
                "conditional" => sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM warehouse_tasks WHERE owner_id = $1 AND id = $2 AND status = 'completed')",
                )
                .bind(task.owner_id)
                .bind(task.predecessor_task_id)
                .fetch_one(&mut **tx)
                .await
                .map_err(map_database_error)?,
                "capacity" => {
                    has_available_worker(
                        tx,
                        task.owner_id,
                        &task.task_group_code,
                        task.warehouse_id,
                        &task.task_type_code,
                        now,
                    )
                    .await?
                }
                _ => true,
            };
            if !condition_met {
                return Err(TaskEngineError::ReleaseConditionNotMet);
            }
            Ok(unchanged())
        }
        TaskTransitionAction::Assign | TaskTransitionAction::Reassign => {
            let valid_status = match request.action {
                TaskTransitionAction::Assign => task.status == TASK_STATUS_PENDING_ASSIGNMENT,
                TaskTransitionAction::Reassign => {
                    matches!(
                        task.status.as_str(),
                        TASK_STATUS_ASSIGNED | TASK_STATUS_DISPATCHED
                    )
                }
                _ => false,
            };
            if !valid_status {
                return Err(TaskEngineError::InvalidTransition);
            }
            lock_key(
                tx,
                "mte-task-group-assignment",
                task.owner_id,
                &task.task_group_code,
            )
            .await?;
            let assignee = if let Some(assignee) = request.assignee_user_id {
                validate_worker_qualification(tx, task, assignee, now).await?;
                assignee
            } else {
                select_least_loaded_worker(tx, task, now).await?
            };
            Ok(ResolvedTransition {
                status: TASK_STATUS_ASSIGNED.to_string(),
                assignee_user_id: Some(assignee),
                ..unchanged()
            })
        }
        TaskTransitionAction::Dispatch if task.status == TASK_STATUS_ASSIGNED => {
            Ok(ResolvedTransition {
                status: TASK_STATUS_DISPATCHED.to_string(),
                assignee_user_id: task.assignee_user_id,
                ..unchanged()
            })
        }
        TaskTransitionAction::Recall
            if matches!(
                task.status.as_str(),
                TASK_STATUS_ASSIGNED | TASK_STATUS_DISPATCHED
            ) =>
        {
            Ok(unchanged())
        }
        TaskTransitionAction::Start if task.status == TASK_STATUS_DISPATCHED => {
            require_assignee(ctx, task)?;
            Ok(ResolvedTransition {
                status: TASK_STATUS_IN_PROGRESS.to_string(),
                assignee_user_id: task.assignee_user_id,
                ..unchanged()
            })
        }
        TaskTransitionAction::Complete if task.status == TASK_STATUS_IN_PROGRESS => {
            require_assignee(ctx, task)?;
            let actual_qty = request
                .actual_qty
                .ok_or_else(|| TaskEngineError::Validation("actual_qty is required".into()))?;
            if actual_qty != task.planned_qty {
                return Err(TaskEngineError::QuantityDifferenceRequiresException);
            }
            Ok(ResolvedTransition {
                status: TASK_STATUS_COMPLETED.to_string(),
                assignee_user_id: task.assignee_user_id,
                actual_qty: Some(actual_qty),
                exception_code: None,
                exception_note: None,
                priority: task.priority,
                manually_expedited: task.manually_expedited,
            })
        }
        TaskTransitionAction::ReportException if task.status == TASK_STATUS_IN_PROGRESS => {
            require_assignee(ctx, task)?;
            let exception_code = normalized_optional_text(request.exception_code.as_deref(), 64)?
                .ok_or_else(|| {
                TaskEngineError::Validation("exception_code is required".into())
            })?;
            Ok(ResolvedTransition {
                status: TASK_STATUS_EXCEPTION.to_string(),
                assignee_user_id: task.assignee_user_id,
                actual_qty: request.actual_qty,
                exception_code: Some(exception_code),
                exception_note: normalized_optional_text(request.exception_note.as_deref(), 500)?,
                priority: task.priority,
                manually_expedited: task.manually_expedited,
            })
        }
        TaskTransitionAction::ResolveComplete if task.status == TASK_STATUS_EXCEPTION => {
            Ok(ResolvedTransition {
                status: TASK_STATUS_COMPLETED.to_string(),
                assignee_user_id: task.assignee_user_id,
                actual_qty: task.actual_qty,
                exception_code: task.exception_code.clone(),
                exception_note: task.exception_note.clone(),
                priority: task.priority,
                manually_expedited: task.manually_expedited,
            })
        }
        TaskTransitionAction::Cancel
            if matches!(
                task.status.as_str(),
                TASK_STATUS_PENDING_RELEASE | TASK_STATUS_PENDING_ASSIGNMENT | TASK_STATUS_EXCEPTION
            ) =>
        {
            Ok(ResolvedTransition {
                status: TASK_STATUS_CANCELLED.to_string(),
                assignee_user_id: task.assignee_user_id,
                ..unchanged()
            })
        }
        TaskTransitionAction::Expedite
            if !task.manually_expedited
                && !matches!(
                    task.status.as_str(),
                    TASK_STATUS_COMPLETED | TASK_STATUS_CANCELLED
                ) =>
        {
            let bonus: i32 = sqlx::query_scalar(
                "SELECT manual_expedite_bonus FROM task_priority_rules WHERE owner_id = $1",
            )
            .bind(task.owner_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_database_error)?;
            Ok(ResolvedTransition {
                status: task.status.clone(),
                assignee_user_id: task.assignee_user_id,
                actual_qty: task.actual_qty,
                exception_code: task.exception_code.clone(),
                exception_note: task.exception_note.clone(),
                priority: (task.priority + bonus).min(1000),
                manually_expedited: true,
            })
        }
        _ => Err(TaskEngineError::InvalidTransition),
    }
}

fn require_assignee(ctx: &AuthContext, task: &WarehouseTaskRow) -> Result<(), TaskEngineError> {
    if task.assignee_user_id == Some(ctx.user_id) {
        Ok(())
    } else {
        Err(TaskEngineError::NotAssignee)
    }
}

async fn has_available_worker(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    task_group_code: &str,
    warehouse_id: Uuid,
    task_type_code: &str,
    now: DateTime<Utc>,
) -> Result<bool, TaskEngineError> {
    sqlx::query_scalar(
        r#"
        SELECT COALESCE(sum(worker_capacity.free_slots), 0) > (
            SELECT count(*)
              FROM warehouse_tasks queued_task
             WHERE queued_task.owner_id = $1
               AND queued_task.task_group_code = $2
               AND queued_task.warehouse_id = $3
               AND queued_task.status = 'pending_assignment'
        )
          FROM (
            SELECT GREATEST(
                       COALESCE(membership.max_active_tasks, 2147483647)::BIGINT
                       - count(active_task.id),
                       0
                   ) AS free_slots
              FROM task_groups task_group
              JOIN task_group_memberships membership
                ON membership.task_group_id = task_group.id
               AND membership.owner_id = task_group.owner_id
              JOIN auth_users auth_user ON auth_user.id = membership.user_id
              JOIN auth_user_owner_bindings binding
                ON binding.user_id = membership.user_id
               AND binding.owner_id = membership.owner_id
              LEFT JOIN warehouse_tasks active_task
                ON active_task.owner_id = membership.owner_id
               AND active_task.assignee_user_id = membership.user_id
               AND active_task.status IN ('assigned', 'dispatched', 'in_progress')
             WHERE task_group.owner_id = $1
               AND task_group.task_group_code = $2
               AND task_group.warehouse_id = $3
               AND task_group.enabled
               AND $4 = ANY(task_group.task_type_codes)
               AND auth_user.status = 'active'
               AND binding.is_active
               AND (membership.qualification_valid_until IS NULL OR
                    membership.qualification_valid_until > $5)
             GROUP BY membership.user_id, membership.max_active_tasks
        ) worker_capacity
        "#,
    )
    .bind(owner_id)
    .bind(task_group_code)
    .bind(warehouse_id)
    .bind(task_type_code)
    .bind(now)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)
}

async fn validate_worker_qualification(
    tx: &mut Transaction<'_, Postgres>,
    task: &WarehouseTaskRow,
    user_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), TaskEngineError> {
    let qualification: Option<(Option<DateTime<Utc>>, Option<i32>, i64)> = sqlx::query_as(
        r#"
        SELECT membership.qualification_valid_until,
               membership.max_active_tasks,
               count(active_task.id)
          FROM task_groups task_group
          JOIN task_group_memberships membership
            ON membership.task_group_id = task_group.id
           AND membership.owner_id = task_group.owner_id
          JOIN auth_users auth_user ON auth_user.id = membership.user_id
          JOIN auth_user_owner_bindings binding
            ON binding.user_id = membership.user_id
           AND binding.owner_id = membership.owner_id
          LEFT JOIN warehouse_tasks active_task
            ON active_task.owner_id = membership.owner_id
           AND active_task.assignee_user_id = membership.user_id
           AND active_task.id <> $6
           AND active_task.status IN ('assigned', 'dispatched', 'in_progress')
         WHERE task_group.owner_id = $1
           AND task_group.task_group_code = $2
           AND task_group.warehouse_id = $3
           AND task_group.enabled
           AND $4 = ANY(task_group.task_type_codes)
           AND membership.user_id = $5
           AND auth_user.status = 'active'
           AND binding.is_active
         GROUP BY membership.qualification_valid_until, membership.max_active_tasks
        "#,
    )
    .bind(task.owner_id)
    .bind(&task.task_group_code)
    .bind(task.warehouse_id)
    .bind(&task.task_type_code)
    .bind(user_id)
    .bind(task.id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let Some((valid_until, max_active_tasks, active_tasks)) = qualification else {
        return Err(TaskEngineError::WorkerNotQualified);
    };
    if valid_until.is_some_and(|value| value <= now) {
        return Err(TaskEngineError::WorkerQualificationExpired);
    }
    if max_active_tasks.is_some_and(|value| active_tasks >= i64::from(value)) {
        return Err(TaskEngineError::WorkerAtCapacity);
    }
    Ok(())
}

async fn select_least_loaded_worker(
    tx: &mut Transaction<'_, Postgres>,
    task: &WarehouseTaskRow,
    now: DateTime<Utc>,
) -> Result<Uuid, TaskEngineError> {
    sqlx::query_scalar(
        r#"
        SELECT membership.user_id
          FROM task_groups task_group
          JOIN task_group_memberships membership
            ON membership.task_group_id = task_group.id
           AND membership.owner_id = task_group.owner_id
          JOIN auth_users auth_user ON auth_user.id = membership.user_id
          JOIN auth_user_owner_bindings binding
            ON binding.user_id = membership.user_id
           AND binding.owner_id = membership.owner_id
          LEFT JOIN warehouse_tasks active_task
            ON active_task.owner_id = membership.owner_id
           AND active_task.assignee_user_id = membership.user_id
           AND active_task.id <> $6
           AND active_task.status IN ('assigned', 'dispatched', 'in_progress')
         WHERE task_group.owner_id = $1
           AND task_group.task_group_code = $2
           AND task_group.warehouse_id = $3
           AND task_group.enabled
           AND $4 = ANY(task_group.task_type_codes)
           AND auth_user.status = 'active'
           AND binding.is_active
           AND ($7::UUID IS NULL OR membership.user_id <> $7)
           AND (membership.qualification_valid_until IS NULL OR
                membership.qualification_valid_until > $5)
         GROUP BY membership.user_id, membership.max_active_tasks
        HAVING count(active_task.id) < COALESCE(membership.max_active_tasks, 2147483647)
         ORDER BY count(active_task.id), membership.user_id
         LIMIT 1
        "#,
    )
    .bind(task.owner_id)
    .bind(&task.task_group_code)
    .bind(task.warehouse_id)
    .bind(&task.task_type_code)
    .bind(now)
    .bind(task.id)
    .bind(task.assignee_user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?
    .ok_or(TaskEngineError::NoAvailableWorker)
}
