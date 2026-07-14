impl PgTaskEngineRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_worker_candidates(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<wms_domain::TaskWorker>, TaskEngineError> {
        sqlx::query_as::<_, (Uuid, String, String)>(
            r#"
            SELECT auth_user.id, auth_user.username, auth_user.display_name
              FROM auth_users auth_user
              JOIN auth_user_owner_bindings binding ON binding.user_id = auth_user.id
             WHERE binding.owner_id = $1
               AND binding.is_active
               AND auth_user.status = 'active'
             ORDER BY auth_user.display_name, auth_user.username, auth_user.id
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(user_id, username, display_name)| wms_domain::TaskWorker {
                    user_id,
                    username,
                    display_name,
                })
                .collect()
        })
        .map_err(map_database_error)
    }

    pub async fn list_task_groups(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<TaskGroup>, TaskEngineError> {
        let rows = sqlx::query_as::<_, TaskGroupRow>(
            r#"
            SELECT task_group.id, task_group.owner_id, task_group.task_group_code,
                   task_group.task_group_name, task_group.warehouse_id, task_group.zone_ids,
                   task_group.task_type_codes,
                   COALESCE(array_agg(membership.user_id ORDER BY membership.user_id)
                       FILTER (WHERE membership.user_id IS NOT NULL AND
                           (membership.qualification_valid_until IS NULL OR
                            membership.qualification_valid_until > now())), '{}') AS member_user_ids,
                   COALESCE(array_agg(membership.qualification_valid_until ORDER BY membership.user_id)
                       FILTER (WHERE membership.user_id IS NOT NULL AND
                           (membership.qualification_valid_until IS NULL OR
                            membership.qualification_valid_until > now())), '{}')
                       AS member_qualification_valid_until,
                   COALESCE(array_agg(membership.max_active_tasks ORDER BY membership.user_id)
                       FILTER (WHERE membership.user_id IS NOT NULL AND
                           (membership.qualification_valid_until IS NULL OR
                            membership.qualification_valid_until > now())), '{}')
                       AS member_max_active_tasks,
                   task_group.enabled, task_group.created_at, task_group.updated_at, task_group.version
              FROM task_groups task_group
              LEFT JOIN task_group_memberships membership ON membership.task_group_id = task_group.id
             WHERE task_group.owner_id = $1
               AND ($2 OR EXISTS (
                    SELECT 1
                      FROM task_group_memberships own_membership
                     WHERE own_membership.task_group_id = task_group.id
                       AND own_membership.owner_id = task_group.owner_id
                       AND own_membership.user_id = $3
                       AND (own_membership.qualification_valid_until IS NULL OR
                            own_membership.qualification_valid_until > now())
               ))
             GROUP BY task_group.id
             ORDER BY task_group.task_group_code
            "#,
        )
        .bind(ctx.owner_id)
        .bind(ctx.permissions.iter().any(|permission| permission == "mte.task_group.write"))
        .bind(ctx.user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_database_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn upsert_task_group(
        &self,
        ctx: &AuthContext,
        task_group_code: &str,
        request: UpsertTaskGroupRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentTaskMutation<TaskGroup>, TaskEngineError> {
        let task_group_code = normalize_code(task_group_code)?;
        let request = normalize_group_request(request)?;
        let request_hash = request_hash(&serde_json::json!({
            "operation": "upsert_task_group",
            "task_group_code": task_group_code,
            "request": request,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_key(&mut tx, "mte-idempotency", ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<TaskGroup>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            return Ok(IdempotentTaskMutation {
                value,
                replayed: true,
            });
        }
        lock_key(&mut tx, "mte-task-group", ctx.owner_id, &task_group_code).await?;
        validate_task_group_references(&mut tx, ctx.owner_id, &request).await?;
        let before = load_task_group_for_update(&mut tx, ctx.owner_id, &task_group_code).await?;
        let id = before.as_ref().map_or_else(Uuid::new_v4, |item| item.id);
        sqlx::query(
            r#"
            INSERT INTO task_groups (
                id, owner_id, task_group_code, task_group_name, warehouse_id,
                zone_ids, task_type_codes, enabled, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
            ON CONFLICT (owner_id, task_group_code) DO UPDATE
               SET task_group_name = EXCLUDED.task_group_name,
                   warehouse_id = EXCLUDED.warehouse_id,
                   zone_ids = EXCLUDED.zone_ids,
                   task_type_codes = EXCLUDED.task_type_codes,
                   enabled = EXCLUDED.enabled,
                   updated_at = EXCLUDED.updated_at,
                   version = task_groups.version + 1
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&task_group_code)
        .bind(&request.task_group_name)
        .bind(request.warehouse_id)
        .bind(&request.zone_ids)
        .bind(&request.task_type_codes)
        .bind(request.enabled)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_database_error)?;
        sqlx::query("DELETE FROM task_group_memberships WHERE task_group_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(map_database_error)?;
        for user_id in &request.member_user_ids {
            let qualification = request
                .member_qualifications
                .iter()
                .find(|qualification| qualification.user_id == *user_id);
            sqlx::query(
                r#"
                INSERT INTO task_group_memberships (
                    task_group_id, owner_id, user_id, qualification_valid_until,
                    max_active_tasks, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(user_id)
            .bind(qualification.and_then(|value| value.valid_until))
            .bind(qualification.and_then(|value| value.max_active_tasks))
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_database_error)?;
        }
        let value = load_task_group(&mut tx, ctx.owner_id, &task_group_code)
            .await?
            .ok_or(TaskEngineError::TaskGroupNotFound)?;
        append_audit(
            &mut tx,
            ctx,
            "upsert_task_group",
            "task_group",
            value.id,
            before.map(TaskGroup::from),
            &TaskGroup::from(value.clone()),
            now,
        )
        .await?;
        let value = TaskGroup::from(value);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PUT",
            &format!("/api/v1/task-engine/task-groups/{task_group_code}"),
            "task_group",
            value.id,
            &value,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentTaskMutation {
            value,
            replayed: false,
        })
    }

    pub async fn create_task(
        &self,
        ctx: &AuthContext,
        request: CreateWarehouseTaskRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentTaskMutation<WarehouseTask>, TaskEngineError> {
        let request = normalize_create_request(request)?;
        let request_hash = request_hash(&serde_json::json!({
            "operation": "create_task",
            "request": request,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_key(&mut tx, "mte-idempotency", ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<WarehouseTask>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            return Ok(IdempotentTaskMutation {
                value,
                replayed: true,
            });
        }
        let value = create_task_in_tx(&mut tx, ctx, &request, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/task-engine/tasks",
            "warehouse_task",
            value.id,
            &value,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentTaskMutation {
            value,
            replayed: false,
        })
    }

    pub async fn list_tasks(
        &self,
        ctx: &AuthContext,
        query: TaskListQuery,
    ) -> Result<Vec<WarehouseTask>, TaskEngineError> {
        let limit = query.limit.unwrap_or(100).clamp(1, 500);
        let rows = sqlx::query_as::<_, WarehouseTaskRow>(
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
               AND (NOT $2 OR assignee_user_id = $3)
               AND ($4::TEXT IS NULL OR status = $4)
               AND ($5::TEXT IS NULL OR task_type_code = $5)
               AND ($6::UUID IS NULL OR warehouse_id = $6)
             ORDER BY priority DESC, created_at, id
             LIMIT $7
            "#,
        )
        .bind(ctx.owner_id)
        .bind(query.mine_only)
        .bind(ctx.user_id)
        .bind(query.status)
        .bind(query.task_type_code)
        .bind(query.warehouse_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_database_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn transition_task(
        &self,
        ctx: &AuthContext,
        task_id: Uuid,
        request: TransitionWarehouseTaskRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentTaskMutation<WarehouseTask>, TaskEngineError> {
        let request_hash = request_hash(&serde_json::json!({
            "operation": "transition_task",
            "task_id": task_id,
            "request": request,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_key(&mut tx, "mte-idempotency", ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency::<WarehouseTask>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            return Ok(IdempotentTaskMutation {
                value,
                replayed: true,
            });
        }
        let before = load_task_for_update(&mut tx, ctx.owner_id, task_id)
            .await?
            .ok_or(TaskEngineError::TaskNotFound)?;
        let transition = resolve_transition(&mut tx, ctx, &before, &request, now).await?;
        let row = sqlx::query_as::<_, WarehouseTaskRow>(
            r#"
            UPDATE warehouse_tasks
               SET status = $1,
                   assignee_user_id = $2,
                   actual_qty = $3,
                   exception_code = $4,
                   exception_note = $5,
                   assigned_at = CASE WHEN $1 = 'assigned' THEN $6 ELSE assigned_at END,
                   dispatched_at = CASE WHEN $1 = 'dispatched' THEN $6 ELSE dispatched_at END,
                   started_at = CASE WHEN $1 = 'in_progress' THEN $6 ELSE started_at END,
                   completed_at = CASE WHEN $1 = 'completed' THEN $6 ELSE completed_at END,
                   updated_at = $6,
                   version = version + 1
             WHERE id = $7 AND owner_id = $8
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
        .bind(transition.status)
        .bind(transition.assignee_user_id)
        .bind(transition.actual_qty)
        .bind(&transition.exception_code)
        .bind(&transition.exception_note)
        .bind(now)
        .bind(task_id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_database_error)?;
        let value = WarehouseTask::from(row);
        append_task_event(
            &mut tx,
            ctx,
            task_id,
            action_name(&request.action),
            Some(&before.status),
            &value.status,
            value.assignee_user_id,
            value.actual_qty,
            value.exception_code.as_deref(),
            value.exception_note.as_deref(),
            now,
        )
        .await?;
        append_audit(
            &mut tx,
            ctx,
            action_name(&request.action),
            "warehouse_task",
            value.id,
            Some(WarehouseTask::from(before)),
            &value,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &format!("/api/v1/task-engine/tasks/{task_id}/transitions"),
            "warehouse_task",
            value.id,
            &value,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentTaskMutation {
            value,
            replayed: false,
        })
    }
}
