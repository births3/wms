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
    let task_type: Option<(i32, i32, bool)> = sqlx::query_as(
        "SELECT default_priority, estimated_minutes, enabled FROM task_types WHERE owner_id = $1 AND task_type_code = $2",
    )
    .bind(ctx.owner_id)
    .bind(&request.task_type_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let (default_priority, estimated_minutes, task_type_enabled) =
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
    let priority = request.priority.unwrap_or(default_priority);
    if !(0..=1000).contains(&priority) {
        return Err(TaskEngineError::Validation(
            "priority must be between 0 and 1000".to_string(),
        ));
    }
    let id = Uuid::new_v4();
    let task_no = format!(
        "TE-{}-{}",
        now.format("%Y%m%d%H%M%S"),
        &id.simple().to_string()[..8]
    );
    let row = sqlx::query_as::<_, WarehouseTaskRow>(
        r#"
        INSERT INTO warehouse_tasks (
            id, owner_id, task_no, task_type_code, source_module, source_doc_type,
            source_doc_id, source_doc_no, source_line_no, source_task_key,
            warehouse_id, task_group_code, product_id, product_code, batch_id,
            batch_no, planned_qty, source_location_id, source_location_code,
            target_location_id, target_location_code, priority, estimated_minutes,
            status, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19,
            $20, $21, $22, $23, 'pending_assignment', $24, $24
        )
        RETURNING id, owner_id, task_no, task_type_code, source_module, source_doc_type,
                  source_doc_id, source_doc_no, source_line_no, source_task_key,
                  warehouse_id, task_group_code, product_id, product_code, batch_id,
                  batch_no, planned_qty, actual_qty, source_location_id,
                  source_location_code, target_location_id, target_location_code,
                  priority, estimated_minutes, assignee_user_id, status, exception_code,
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
    .bind(estimated_minutes)
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
    status: &'static str,
    assignee_user_id: Option<Uuid>,
    actual_qty: Option<i64>,
    exception_code: Option<String>,
    exception_note: Option<String>,
}

async fn resolve_transition(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    task: &WarehouseTaskRow,
    request: &TransitionWarehouseTaskRequest,
) -> Result<ResolvedTransition, TaskEngineError> {
    let unchanged = || ResolvedTransition {
        status: TASK_STATUS_PENDING_ASSIGNMENT,
        assignee_user_id: None,
        actual_qty: task.actual_qty,
        exception_code: task.exception_code.clone(),
        exception_note: task.exception_note.clone(),
    };
    match request.action {
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
            let assignee = if let Some(assignee) = request.assignee_user_id {
                validate_worker_qualification(tx, task, assignee).await?;
                assignee
            } else {
                select_least_loaded_worker(tx, task).await?
            };
            Ok(ResolvedTransition {
                status: TASK_STATUS_ASSIGNED,
                assignee_user_id: Some(assignee),
                ..unchanged()
            })
        }
        TaskTransitionAction::Dispatch if task.status == TASK_STATUS_ASSIGNED => {
            Ok(ResolvedTransition {
                status: TASK_STATUS_DISPATCHED,
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
                status: TASK_STATUS_IN_PROGRESS,
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
                status: TASK_STATUS_COMPLETED,
                assignee_user_id: task.assignee_user_id,
                actual_qty: Some(actual_qty),
                exception_code: None,
                exception_note: None,
            })
        }
        TaskTransitionAction::ReportException if task.status == TASK_STATUS_IN_PROGRESS => {
            require_assignee(ctx, task)?;
            let exception_code = normalized_optional_text(request.exception_code.as_deref(), 64)?
                .ok_or_else(|| {
                TaskEngineError::Validation("exception_code is required".into())
            })?;
            Ok(ResolvedTransition {
                status: TASK_STATUS_EXCEPTION,
                assignee_user_id: task.assignee_user_id,
                actual_qty: request.actual_qty,
                exception_code: Some(exception_code),
                exception_note: normalized_optional_text(request.exception_note.as_deref(), 500)?,
            })
        }
        TaskTransitionAction::ResolveComplete if task.status == TASK_STATUS_EXCEPTION => {
            Ok(ResolvedTransition {
                status: TASK_STATUS_COMPLETED,
                assignee_user_id: task.assignee_user_id,
                actual_qty: task.actual_qty,
                exception_code: task.exception_code.clone(),
                exception_note: task.exception_note.clone(),
            })
        }
        TaskTransitionAction::Cancel
            if matches!(
                task.status.as_str(),
                TASK_STATUS_PENDING_ASSIGNMENT | TASK_STATUS_EXCEPTION
            ) =>
        {
            Ok(ResolvedTransition {
                status: TASK_STATUS_CANCELLED,
                assignee_user_id: task.assignee_user_id,
                ..unchanged()
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

async fn validate_worker_qualification(
    tx: &mut Transaction<'_, Postgres>,
    task: &WarehouseTaskRow,
    user_id: Uuid,
) -> Result<(), TaskEngineError> {
    let qualified: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM task_groups task_group
              JOIN task_group_memberships membership
                ON membership.task_group_id = task_group.id
               AND membership.owner_id = task_group.owner_id
              JOIN auth_users auth_user ON auth_user.id = membership.user_id
              JOIN auth_user_owner_bindings binding
                ON binding.user_id = membership.user_id
               AND binding.owner_id = membership.owner_id
             WHERE task_group.owner_id = $1
               AND task_group.task_group_code = $2
               AND task_group.warehouse_id = $3
               AND task_group.enabled
               AND $4 = ANY(task_group.task_type_codes)
               AND membership.user_id = $5
               AND auth_user.status = 'active'
               AND binding.is_active
        )
        "#,
    )
    .bind(task.owner_id)
    .bind(&task.task_group_code)
    .bind(task.warehouse_id)
    .bind(&task.task_type_code)
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)?;
    if qualified {
        Ok(())
    } else {
        Err(TaskEngineError::WorkerNotQualified)
    }
}

async fn select_least_loaded_worker(
    tx: &mut Transaction<'_, Postgres>,
    task: &WarehouseTaskRow,
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
           AND active_task.status IN ('assigned', 'dispatched', 'in_progress')
         WHERE task_group.owner_id = $1
           AND task_group.task_group_code = $2
           AND task_group.warehouse_id = $3
           AND task_group.enabled
           AND $4 = ANY(task_group.task_type_codes)
           AND auth_user.status = 'active'
           AND binding.is_active
         GROUP BY membership.user_id
         ORDER BY count(active_task.id), membership.user_id
         LIMIT 1
        "#,
    )
    .bind(task.owner_id)
    .bind(&task.task_group_code)
    .bind(task.warehouse_id)
    .bind(&task.task_type_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?
    .ok_or(TaskEngineError::NoAvailableWorker)
}
