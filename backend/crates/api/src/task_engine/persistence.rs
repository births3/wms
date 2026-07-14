fn normalize_group_request(
    mut request: UpsertTaskGroupRequest,
) -> Result<UpsertTaskGroupRequest, TaskEngineError> {
    request.task_group_name = required_text(&request.task_group_name, "task_group_name", 128)?;
    if request.task_type_codes.is_empty() {
        return Err(TaskEngineError::Validation(
            "task_type_codes is required".to_string(),
        ));
    }
    request.task_type_codes = request
        .task_type_codes
        .iter()
        .map(|code| {
            normalize_task_type_code(code)
                .map_err(|error| TaskEngineError::Validation(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    request.task_type_codes.sort();
    request.task_type_codes.dedup();
    request.zone_ids.sort();
    request.zone_ids.dedup();
    request.member_user_ids.sort();
    request.member_user_ids.dedup();
    Ok(request)
}

fn normalize_create_request(
    mut request: CreateWarehouseTaskRequest,
) -> Result<CreateWarehouseTaskRequest, TaskEngineError> {
    request.task_type_code = normalize_task_type_code(&request.task_type_code)
        .map_err(|error| TaskEngineError::Validation(error.to_string()))?;
    request.task_group_code = normalize_code(&request.task_group_code)?;
    request.source_module = required_text(&request.source_module, "source_module", 32)?;
    request.source_doc_type = required_text(&request.source_doc_type, "source_doc_type", 64)?;
    request.source_doc_no = required_text(&request.source_doc_no, "source_doc_no", 128)?;
    request.source_task_key = required_text(&request.source_task_key, "source_task_key", 256)?;
    request.product_code = required_text(&request.product_code, "product_code", 128)?;
    request.batch_no = normalized_optional_text(request.batch_no.as_deref(), 128)?;
    request.source_location_code =
        normalized_optional_text(request.source_location_code.as_deref(), 128)?;
    request.target_location_code =
        normalized_optional_text(request.target_location_code.as_deref(), 128)?;
    if request.planned_qty <= 0 {
        return Err(TaskEngineError::Validation(
            "planned_qty must be positive".to_string(),
        ));
    }
    Ok(request)
}

async fn validate_task_group_references(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    request: &UpsertTaskGroupRequest,
) -> Result<(), TaskEngineError> {
    let warehouse_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM warehouses WHERE owner_id = $1 AND id = $2 AND status = 'active')",
    )
    .bind(owner_id)
    .bind(request.warehouse_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)?;
    if !warehouse_exists {
        return Err(TaskEngineError::WarehouseNotFound);
    }
    let type_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM task_types WHERE owner_id = $1 AND task_type_code = ANY($2) AND enabled",
    )
    .bind(owner_id)
    .bind(&request.task_type_codes)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)?;
    if type_count != request.task_type_codes.len() as i64 {
        return Err(TaskEngineError::TaskTypeNotFound);
    }
    if !request.zone_ids.is_empty() {
        let zone_count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM warehouse_zones WHERE owner_id = $1 AND warehouse_id = $2 AND id = ANY($3) AND status = 'active'",
        )
        .bind(owner_id)
        .bind(request.warehouse_id)
        .bind(&request.zone_ids)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_database_error)?;
        if zone_count != request.zone_ids.len() as i64 {
            return Err(TaskEngineError::ZoneNotFound);
        }
    }
    if !request.member_user_ids.is_empty() {
        let user_count: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)
              FROM auth_users auth_user
              JOIN auth_user_owner_bindings binding ON binding.user_id = auth_user.id
             WHERE binding.owner_id = $1
               AND auth_user.id = ANY($2)
               AND auth_user.status = 'active'
               AND binding.is_active
            "#,
        )
        .bind(owner_id)
        .bind(&request.member_user_ids)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_database_error)?;
        if user_count != request.member_user_ids.len() as i64 {
            return Err(TaskEngineError::UserNotFound);
        }
    }
    Ok(())
}

async fn load_task_group_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    task_group_code: &str,
) -> Result<Option<TaskGroupRow>, TaskEngineError> {
    sqlx::query_as::<_, TaskGroupRow>(
        r#"
        SELECT task_group.id, task_group.owner_id, task_group.task_group_code,
               task_group.task_group_name, task_group.warehouse_id, task_group.zone_ids,
               task_group.task_type_codes,
               COALESCE(array_agg(membership.user_id ORDER BY membership.user_id)
                   FILTER (WHERE membership.user_id IS NOT NULL), '{}') AS member_user_ids,
               task_group.enabled, task_group.created_at, task_group.updated_at, task_group.version
          FROM task_groups task_group
          LEFT JOIN task_group_memberships membership ON membership.task_group_id = task_group.id
         WHERE task_group.owner_id = $1 AND task_group.task_group_code = $2
         GROUP BY task_group.id
        "#,
    )
    .bind(owner_id)
    .bind(task_group_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)
}

async fn load_task_group(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    task_group_code: &str,
) -> Result<Option<TaskGroupRow>, TaskEngineError> {
    sqlx::query_as::<_, TaskGroupRow>(
        r#"
        SELECT task_group.id, task_group.owner_id, task_group.task_group_code,
               task_group.task_group_name, task_group.warehouse_id, task_group.zone_ids,
               task_group.task_type_codes,
               COALESCE(array_agg(membership.user_id ORDER BY membership.user_id)
                   FILTER (WHERE membership.user_id IS NOT NULL), '{}') AS member_user_ids,
               task_group.enabled, task_group.created_at, task_group.updated_at, task_group.version
          FROM task_groups task_group
          LEFT JOIN task_group_memberships membership ON membership.task_group_id = task_group.id
         WHERE task_group.owner_id = $1 AND task_group.task_group_code = $2
         GROUP BY task_group.id
        "#,
    )
    .bind(owner_id)
    .bind(task_group_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)
}

async fn load_task_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    task_id: Uuid,
) -> Result<Option<WarehouseTaskRow>, TaskEngineError> {
    sqlx::query_as::<_, WarehouseTaskRow>(
        r#"
        SELECT id, owner_id, task_no, task_type_code, source_module, source_doc_type,
               source_doc_id, source_doc_no, source_line_no, source_task_key,
               warehouse_id, task_group_code, product_id, product_code, batch_id,
               batch_no, planned_qty, actual_qty, source_location_id,
               source_location_code, target_location_id, target_location_code,
               priority, estimated_minutes, assignee_user_id, status, exception_code,
               exception_note, assigned_at, dispatched_at, started_at, completed_at,
               created_at, updated_at, version
          FROM warehouse_tasks
         WHERE owner_id = $1 AND id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(task_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)
}

async fn load_task_by_source_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    source_task_key: &str,
) -> Result<Option<WarehouseTaskRow>, TaskEngineError> {
    sqlx::query_as::<_, WarehouseTaskRow>(
        r#"
        SELECT id, owner_id, task_no, task_type_code, source_module, source_doc_type,
               source_doc_id, source_doc_no, source_line_no, source_task_key,
               warehouse_id, task_group_code, product_id, product_code, batch_id,
               batch_no, planned_qty, actual_qty, source_location_id,
               source_location_code, target_location_id, target_location_code,
               priority, estimated_minutes, assignee_user_id, status, exception_code,
               exception_note, assigned_at, dispatched_at, started_at, completed_at,
               created_at, updated_at, version
          FROM warehouse_tasks
         WHERE owner_id = $1 AND source_task_key = $2
        "#,
    )
    .bind(owner_id)
    .bind(source_task_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)
}

async fn load_task_by_source_identity(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    request: &CreateWarehouseTaskRequest,
) -> Result<Option<WarehouseTaskRow>, TaskEngineError> {
    sqlx::query_as::<_, WarehouseTaskRow>(
        r#"
        SELECT id, owner_id, task_no, task_type_code, source_module, source_doc_type,
               source_doc_id, source_doc_no, source_line_no, source_task_key,
               warehouse_id, task_group_code, product_id, product_code, batch_id,
               batch_no, planned_qty, actual_qty, source_location_id,
               source_location_code, target_location_id, target_location_code,
               priority, estimated_minutes, assignee_user_id, status, exception_code,
               exception_note, assigned_at, dispatched_at, started_at, completed_at,
               created_at, updated_at, version
          FROM warehouse_tasks
         WHERE owner_id = $1
           AND source_module = $2
           AND source_doc_type = $3
           AND source_doc_id IS NOT DISTINCT FROM $4
           AND ($4::UUID IS NOT NULL OR source_doc_no = $5)
           AND source_line_no IS NOT DISTINCT FROM $6
           AND task_type_code = $7
           AND batch_id IS NOT DISTINCT FROM $8
        "#,
    )
    .bind(owner_id)
    .bind(&request.source_module)
    .bind(&request.source_doc_type)
    .bind(request.source_doc_id)
    .bind(&request.source_doc_no)
    .bind(request.source_line_no)
    .bind(&request.task_type_code)
    .bind(request.batch_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)
}

fn task_matches_request(row: &WarehouseTaskRow, request: &CreateWarehouseTaskRequest) -> bool {
    row.task_type_code == request.task_type_code
        && row.source_module == request.source_module
        && row.source_doc_type == request.source_doc_type
        && row.source_doc_id == request.source_doc_id
        && row.source_doc_no == request.source_doc_no
        && row.source_line_no == request.source_line_no
        && row.warehouse_id == request.warehouse_id
        && row.task_group_code == request.task_group_code
        && row.product_id == request.product_id
        && row.product_code == request.product_code
        && row.batch_id == request.batch_id
        && row.batch_no == request.batch_no
        && row.planned_qty == request.planned_qty
        && row.source_location_id == request.source_location_id
        && row.source_location_code == request.source_location_code
        && row.target_location_id == request.target_location_id
        && row.target_location_code == request.target_location_code
}

async fn append_task_event(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    task_id: Uuid,
    action: &str,
    from_status: Option<&str>,
    to_status: &str,
    assignee_user_id: Option<Uuid>,
    actual_qty: Option<i64>,
    exception_code: Option<&str>,
    exception_note: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), TaskEngineError> {
    sqlx::query(
        r#"
        INSERT INTO task_execution_events (
            id, owner_id, task_id, action, from_status, to_status, actor_user_id,
            assignee_user_id, actual_qty, exception_code, exception_note, occurred_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(task_id)
    .bind(action)
    .bind(from_status)
    .bind(to_status)
    .bind(ctx.user_id)
    .bind(assignee_user_id)
    .bind(actual_qty)
    .bind(exception_code)
    .bind(exception_note)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_database_error)?;
    Ok(())
}

async fn append_audit<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
    before: Option<T>,
    after: &T,
    now: DateTime<Utc>,
) -> Result<(), TaskEngineError> {
    let before = before.map_or_else(|| Ok(serde_json::json!({})), |value| json_value(&value))?;
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M-TE",
        resource_type,
        resource_id.to_string(),
        Some(AuditDiff::compute(before, json_value(after)?)),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| TaskEngineError::Audit(format!("{error:?}")))
}

async fn replay_idempotency<T: serde::de::DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, TaskEngineError> {
    let row: Option<(String, Value, DateTime<Utc>)> = sqlx::query_as(
        "SELECT request_hash, response_body, expires_at FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2 FOR UPDATE",
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let Some((stored_hash, response, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await
            .map_err(map_database_error)?;
        return Ok(None);
    }
    if stored_hash != request_hash {
        return Err(TaskEngineError::IdempotencyConflict);
    }
    serde_json::from_value(response)
        .map(Some)
        .map_err(|error| TaskEngineError::Serialize(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: Uuid,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), TaskEngineError> {
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, 200, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(method)
    .bind(path)
    .bind(json_value(response)?)
    .bind(resource_type)
    .bind(resource_id.to_string())
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_database_error)?;
    Ok(())
}

async fn lock_key(
    tx: &mut Transaction<'_, Postgres>,
    namespace: &str,
    owner_id: Uuid,
    key: &str,
) -> Result<(), TaskEngineError> {
    let digest = Sha256::digest(format!("{namespace}:{owner_id}:{key}").as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(i64::from_be_bytes(bytes))
        .execute(&mut **tx)
        .await
        .map_err(map_database_error)?;
    Ok(())
}

fn normalize_code(value: &str) -> Result<String, TaskEngineError> {
    normalize_task_type_code(value).map_err(|error| TaskEngineError::Validation(error.to_string()))
}

fn required_text(value: &str, field: &str, max: usize) -> Result<String, TaskEngineError> {
    normalized_optional_text(Some(value), max)?
        .ok_or_else(|| TaskEngineError::Validation(format!("{field} is required")))
}

fn normalized_optional_text(
    value: Option<&str>,
    max: usize,
) -> Result<Option<String>, TaskEngineError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > max || value.chars().any(char::is_control) {
        return Err(TaskEngineError::Validation(
            "text field is invalid".to_string(),
        ));
    }
    Ok(Some(value.to_string()))
}

fn request_hash(value: &Value) -> Result<String, TaskEngineError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| TaskEngineError::Serialize(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn json_value<T: Serialize>(value: &T) -> Result<Value, TaskEngineError> {
    serde_json::to_value(value).map_err(|error| TaskEngineError::Serialize(error.to_string()))
}

fn map_database_error(error: sqlx::Error) -> TaskEngineError {
    TaskEngineError::Database(format!("{error:?}"))
}

fn action_name(action: &TaskTransitionAction) -> &'static str {
    match action {
        TaskTransitionAction::Assign => "assign_task",
        TaskTransitionAction::Dispatch => "dispatch_task",
        TaskTransitionAction::Reassign => "reassign_task",
        TaskTransitionAction::Recall => "recall_task",
        TaskTransitionAction::Start => "start_task",
        TaskTransitionAction::Complete => "complete_task",
        TaskTransitionAction::ReportException => "report_task_exception",
        TaskTransitionAction::ResolveComplete => "resolve_task_complete",
        TaskTransitionAction::Cancel => "cancel_task",
    }
}
