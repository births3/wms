#[derive(Clone, FromRow)]
struct LockedOutboundPickTask {
    id: Uuid,
    outbound_order_id: Uuid,
    line_no: i32,
    batch_id: Uuid,
    planned_qty: wms_domain::Quantity,
    picked_qty: wms_domain::Quantity,
    status: String,
}

#[derive(Clone, FromRow)]
struct LockedWarehousePickTask {
    id: Uuid,
    planned_qty: wms_domain::Quantity,
    assignee_user_id: Option<Uuid>,
    status: String,
}

fn validated_m4_operated_at(
    requested: Option<DateTime<Utc>>,
    server_now: DateTime<Utc>,
) -> Result<DateTime<Utc>, Wave4RepositoryError> {
    let operated_at = requested.unwrap_or(server_now);
    if operated_at > server_now + chrono::Duration::minutes(5)
        || operated_at < server_now - chrono::Duration::hours(24)
    {
        return Err(Wave4RepositoryError::InvalidOperationTime);
    }
    Ok(operated_at)
}

fn normalized_trace_codes(codes: &[String]) -> Result<Vec<String>, Wave4RepositoryError> {
    let mut cleaned: Vec<String> = codes
        .iter()
        .map(|code| code.trim().to_string())
        .filter(|code| !code.is_empty())
        .collect();
    let original_len = cleaned.len();
    cleaned.sort();
    cleaned.dedup();
    if cleaned.len() != original_len {
        return Err(Wave4RepositoryError::DuplicateCode);
    }
    Ok(cleaned)
}

async fn lock_pick_task_by_id(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    pick_task_id: Uuid,
) -> Result<LockedOutboundPickTask, Wave4RepositoryError> {
    sqlx::query_as::<_, LockedOutboundPickTask>(
        r#"
        SELECT id, outbound_order_id, line_no, batch_id,
               planned_qty, picked_qty, status
          FROM outbound_pick_tasks
         WHERE owner_id = $1 AND id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(pick_task_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave4RepositoryError::NotFound)
}

async fn lock_single_pick_task_for_line(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
    line_no: i32,
) -> Result<LockedOutboundPickTask, Wave4RepositoryError> {
    let tasks = sqlx::query_as::<_, LockedOutboundPickTask>(
        r#"
        SELECT id, outbound_order_id, line_no, batch_id,
               planned_qty, picked_qty, status
          FROM outbound_pick_tasks
         WHERE owner_id = $1
           AND outbound_order_id = $2
           AND line_no = $3
           AND status <> 'cancelled'
         ORDER BY route_sequence, id
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(order_id)
    .bind(line_no)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;

    match tasks.as_slice() {
        [] => Err(Wave4RepositoryError::NotFound),
        [task] => Ok(task.clone()),
        _ => Err(Wave4RepositoryError::AmbiguousPickTask),
    }
}

async fn complete_locked_pick_task(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    task: &LockedOutboundPickTask,
    picked_qty: wms_domain::Quantity,
    trace_codes: &[String],
    operated_at: DateTime<Utc>,
    server_now: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    if picked_qty < wms_domain::Quantity::ZERO || picked_qty > task.planned_qty {
        return Err(Wave4RepositoryError::InvalidQuantity);
    }
    if task.status == "exception" && picked_qty <= task.picked_qty {
        return Err(Wave4RepositoryError::InvalidQuantity);
    }
    if !matches!(
        task.status.as_str(),
        "pending_assignment" | "assigned" | "dispatched" | "in_progress" | "exception"
    ) {
        return Err(Wave4RepositoryError::InvalidStatus {
            expected: "pending_assignment|assigned|dispatched|in_progress|exception".to_string(),
            actual: task.status.clone(),
        });
    }

    let warehouse_task = sqlx::query_as::<_, LockedWarehousePickTask>(
        r#"
        SELECT id, planned_qty, assignee_user_id, status
          FROM warehouse_tasks
         WHERE owner_id = $1
           AND task_type_code = 'pick'
           AND source_module = 'M4'
           AND source_doc_type = 'outbound_order'
           AND source_doc_id = $2
           AND source_line_no = $3
           AND batch_id = $4
         FOR UPDATE
        "#,
    )
    .bind(ctx.owner_id)
    .bind(task.outbound_order_id)
    .bind(task.line_no)
    .bind(task.batch_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave4RepositoryError::NotFound)?;

    if warehouse_task.planned_qty != task.planned_qty {
        return Err(Wave4RepositoryError::InvalidQuantity);
    }
    if warehouse_task.assignee_user_id.is_some()
        && warehouse_task.assignee_user_id != Some(ctx.user_id)
    {
        return Err(Wave4RepositoryError::InvalidStatus {
            expected: "unassigned or assigned to current operator".to_string(),
            actual: warehouse_task.status,
        });
    }
    if !matches!(
        warehouse_task.status.as_str(),
        "pending_assignment" | "assigned" | "dispatched" | "in_progress" | "exception"
    ) {
        return Err(Wave4RepositoryError::InvalidStatus {
            expected: "pending_assignment|assigned|dispatched|in_progress|exception".to_string(),
            actual: warehouse_task.status,
        });
    }

    let completed = picked_qty == task.planned_qty;
    let next_status = if completed { "completed" } else { "exception" };
    let exception_code = (!completed).then(|| "SHORT_PICK".to_string());
    let exception_note = (!completed)
        .then(|| format!("planned {}, picked {}", task.planned_qty, picked_qty));

    sqlx::query(
        r#"
        UPDATE outbound_pick_tasks
           SET picked_qty = $3,
               status = $4,
               updated_at = $5
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(ctx.owner_id)
    .bind(task.id)
    .bind(picked_qty)
    .bind(next_status)
    .bind(server_now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    sqlx::query(
        r#"
        UPDATE warehouse_tasks
           SET assignee_user_id = COALESCE(assignee_user_id, $3),
               actual_qty = $4,
               status = $5,
               exception_code = $6,
               exception_note = $7,
               assigned_at = COALESCE(assigned_at, $8),
               dispatched_at = COALESCE(dispatched_at, $8),
               started_at = COALESCE(started_at, $8),
               completed_at = CASE WHEN $5 = 'completed' THEN $8 ELSE NULL END,
               updated_at = $9,
               version = version + 1
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(ctx.owner_id)
    .bind(warehouse_task.id)
    .bind(ctx.user_id)
    .bind(picked_qty)
    .bind(next_status)
    .bind(&exception_code)
    .bind(&exception_note)
    .bind(operated_at)
    .bind(server_now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    sqlx::query(
        r#"
        INSERT INTO task_execution_events (
            id, owner_id, task_id, action, from_status, to_status,
            actor_user_id, assignee_user_id, actual_qty,
            exception_code, exception_note, occurred_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $7, $8, $9, $10, $11
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(warehouse_task.id)
    .bind(if completed {
        "complete"
    } else {
        "report_exception"
    })
    .bind(&warehouse_task.status)
    .bind(next_status)
    .bind(ctx.user_id)
    .bind(picked_qty)
    .bind(&exception_code)
    .bind(&exception_note)
    .bind(operated_at)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    for trace_code in normalized_trace_codes(trace_codes)? {
        sqlx::query(
            r#"
            INSERT INTO outbound_pick_trace_codes (
                id, owner_id, pick_task_id, outbound_order_id, line_no,
                trace_code, scanned_by, operated_at, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(task.id)
        .bind(task.outbound_order_id)
        .bind(task.line_no)
        .bind(trace_code)
        .bind(ctx.user_id)
        .bind(operated_at)
        .bind(server_now)
        .execute(&mut **tx)
        .await
        .map_err(map_insert_error)?;
    }

    Ok(())
}

async fn refresh_order_line_from_pick_tasks(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
    line_no: i32,
) -> Result<(), Wave4RepositoryError> {
    let (picked_qty, pending_count): (wms_domain::Quantity, i64) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(picked_qty), 0),
               COUNT(*) FILTER (
                   WHERE status NOT IN ('completed', 'exception', 'cancelled')
               )::BIGINT
          FROM outbound_pick_tasks
         WHERE owner_id = $1
           AND outbound_order_id = $2
           AND line_no = $3
           AND status <> 'cancelled'
        "#,
    )
    .bind(owner_id)
    .bind(order_id)
    .bind(line_no)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let planned_qty: wms_domain::Quantity = sqlx::query_scalar(
        r#"
        SELECT planned_qty
          FROM outbound_order_lines
         WHERE owner_id = $1 AND outbound_order_id = $2 AND line_no = $3
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(order_id)
    .bind(line_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave4RepositoryError::NotFound)?;

    if picked_qty > planned_qty {
        return Err(Wave4RepositoryError::InvalidQuantity);
    }
    let short_pick_qty = if pending_count == 0 {
        planned_qty - picked_qty
    } else {
        wms_domain::Quantity::ZERO
    };

    sqlx::query(
        r#"
        UPDATE outbound_order_lines
           SET picked_qty = $4,
               short_pick_qty = $5
         WHERE owner_id = $1 AND outbound_order_id = $2 AND line_no = $3
        "#,
    )
    .bind(owner_id)
    .bind(order_id)
    .bind(line_no)
    .bind(picked_qty)
    .bind(short_pick_qty)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

async fn finalize_order_pick_state(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    order: &OutboundOrderRow,
    server_now: DateTime<Utc>,
) -> Result<OutboundOrder, Wave4RepositoryError> {
    let pending_task_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
          FROM outbound_pick_tasks
         WHERE owner_id = $1
           AND outbound_order_id = $2
           AND status NOT IN ('completed', 'exception', 'cancelled')
        "#,
    )
    .bind(ctx.owner_id)
    .bind(order.id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let updated = load_outbound_order(tx, ctx.owner_id, order.id).await?;
    let next_status = if pending_task_count == 0 {
        status_after_pick(&updated.lines)
    } else {
        "inventory_locked"
    };

    if order.status != next_status {
        let event_code = if next_status == "inventory_locked" {
            "start_picking"
        } else {
            crate::outbound_state_rules::pick_transition_event(&order.status, next_status)
        };
        crate::outbound_state_rules::validate_outbound_transition(
            &order.status,
            next_status,
            event_code,
        )
        .map_err(|_| Wave4RepositoryError::InvalidStateTransition {
            from: order.status.clone(),
            to: next_status.to_string(),
            approval_source: event_code.to_string(),
        })?;
        sqlx::query(
            r#"
            UPDATE outbound_orders
               SET status = $3,
                   short_pick = $4,
                   updated_at = $5,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order.id)
        .bind(next_status)
        .bind(
            pending_task_count == 0
                && updated
                    .lines
                    .iter()
                    .any(|line| line.short_pick_qty > wms_domain::Quantity::ZERO),
        )
        .bind(server_now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }

    load_outbound_order(tx, ctx.owner_id, order.id).await
}

async fn bind_outbound_tote(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    order_id: Uuid,
    requested_code: Option<&str>,
    server_now: DateTime<Utc>,
) -> Result<Option<String>, Wave4RepositoryError> {
    let Some(requested_code) = requested_code
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    let (tote_id, tote_code, tote_status): (Uuid, String, String) = sqlx::query_as(
        r#"
        SELECT id, lpn_code, status
          FROM lpn_containers
         WHERE owner_id = $1
           AND lower(lpn_code) = lower($2)
           AND container_type = 'tote'
         FOR UPDATE
        "#,
    )
    .bind(ctx.owner_id)
    .bind(requested_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave4RepositoryError::NotFound)?;

    if !matches!(tote_status.as_str(), "idle" | "in_use") {
        return Err(Wave4RepositoryError::InvalidStatus {
            expected: "idle|in_use".to_string(),
            actual: tote_status,
        });
    }

    let existing_tote_order: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT outbound_order_id
          FROM outbound_pick_tote_bindings
         WHERE owner_id = $1 AND tote_id = $2 AND status = 'active'
         FOR UPDATE
        "#,
    )
    .bind(ctx.owner_id)
    .bind(tote_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;

    if tote_status == "idle" {
        if existing_tote_order.is_some() {
            sqlx::query(
                r#"
                UPDATE outbound_pick_tote_bindings
                   SET status = 'released', released_at = $3, updated_at = $3
                 WHERE owner_id = $1 AND tote_id = $2 AND status = 'active'
                "#,
            )
            .bind(ctx.owner_id)
            .bind(tote_id)
            .bind(server_now)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        }
    } else if existing_tote_order != Some(order_id) {
        return Err(Wave4RepositoryError::InvalidStatus {
            expected: "tote available or already bound to this order".to_string(),
            actual: "in_use".to_string(),
        });
    }

    let existing_order_tote: Option<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT tote_id, tote_code
          FROM outbound_pick_tote_bindings
         WHERE owner_id = $1 AND outbound_order_id = $2 AND status = 'active'
         FOR UPDATE
        "#,
    )
    .bind(ctx.owner_id)
    .bind(order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;

    if let Some((existing_tote_id, existing_tote_code)) = existing_order_tote {
        if existing_tote_id != tote_id {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: format!("order already bound to {existing_tote_code}"),
                actual: tote_code,
            });
        }
    } else {
        sqlx::query(
            r#"
            INSERT INTO outbound_pick_tote_bindings (
                id, owner_id, outbound_order_id, tote_id, tote_code,
                status, bound_by, bound_at, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, 'active', $6, $7, $7, $7)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(tote_id)
        .bind(&tote_code)
        .bind(ctx.user_id)
        .bind(server_now)
        .execute(&mut **tx)
        .await
        .map_err(map_insert_error)?;
    }

    sqlx::query(
        r#"
        UPDATE lpn_containers
           SET status = 'in_use', updated_at = $3
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(ctx.owner_id)
    .bind(tote_id)
    .bind(server_now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    Ok(Some(tote_code))
}

impl PgWave4Repository {
    pub async fn complete_pick_task(
        &self,
        ctx: &AuthContext,
        pick_task_id: Uuid,
        req: CompletePickTaskRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundOrder>, Wave4RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "pick_task_id": pick_task_id,
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }
        let operated_at = validated_m4_operated_at(req.operated_at, now)?;
        let task = lock_pick_task_by_id(&mut tx, ctx.owner_id, pick_task_id).await?;
        if task.line_no
            != i32::try_from(req.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?
        {
            return Err(Wave4RepositoryError::InvalidQuantity);
        }
        let order = lock_outbound_order(&mut tx, ctx.owner_id, task.outbound_order_id).await?;
        if !matches!(
            order.status.as_str(),
            OUTBOUND_STATUS_IN_WAVE | "inventory_locked" | "picked_short" | OUTBOUND_STATUS_REVIEWED_SHORT
        ) {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "in_wave|inventory_locked|picked_short|reviewed_short".to_string(),
                actual: order.status,
            });
        }

        complete_locked_pick_task(
            &mut tx,
            ctx,
            &task,
            req.picked_qty,
            &[],
            operated_at,
            now,
        )
        .await?;
        refresh_order_line_from_pick_tasks(&mut tx, ctx.owner_id, order.id, task.line_no).await?;
        let updated = finalize_order_pick_state(&mut tx, ctx, &order, now).await?;

        let audit = audit.map(|mut audit| {
            audit.resource_id = pick_task_id.to_string();
            audit.diff = Some(AuditDiff::compute(
                serde_json::json!({
                    "task_status": task.status,
                    "picked_qty": task.picked_qty,
                }),
                serde_json::json!({
                    "task_status": if req.picked_qty == task.planned_qty {
                        "completed"
                    } else {
                        "exception"
                    },
                    "picked_qty": req.picked_qty,
                    "operated_at": operated_at,
                    "order_status": &updated.status,
                }),
            ));
            audit
        });

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/pick-tasks/{id}/complete",
            "outbound_order",
            updated.id.to_string(),
            &updated,
            now,
        )
        .await?;
        append_outbound_audit(
            &mut tx,
            ctx,
            audit,
            "complete_pick_task",
            updated.id,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: updated,
            replayed: false,
        })
    }

    pub async fn batch_complete_pick_tasks(
        &self,
        ctx: &AuthContext,
        req: wms_domain::BatchCompletePickTaskRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<
        IdempotentMutation<wms_domain::BatchCompletePickTaskResponse>,
        Wave4RepositoryError,
    > {
        if req.items.is_empty() {
            return Err(Wave4RepositoryError::EmptySelection);
        }
        let request_hash = request_hash(&serde_json::json!({
            "outbound_order_id": req.order_id,
            "request": &req,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }
        let operated_at = validated_m4_operated_at(req.operated_at, now)?;
        let order = lock_outbound_order(&mut tx, ctx.owner_id, req.order_id).await?;
        if !matches!(order.status.as_str(), OUTBOUND_STATUS_IN_WAVE | "inventory_locked") {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: "in_wave|inventory_locked".to_string(),
                actual: order.status,
            });
        }

        let mut seen_lines = HashSet::with_capacity(req.items.len());
        let mut completed_task_ids = Vec::with_capacity(req.items.len());
        for item in &req.items {
            let line_no =
                i32::try_from(item.line_no).map_err(|_| Wave4RepositoryError::InvalidQuantity)?;
            if !seen_lines.insert(line_no) {
                return Err(Wave4RepositoryError::DuplicateCode);
            }
            let task =
                lock_single_pick_task_for_line(&mut tx, ctx.owner_id, req.order_id, line_no).await?;
            complete_locked_pick_task(
                &mut tx,
                ctx,
                &task,
                item.picked_qty,
                &item.trace_codes,
                operated_at,
                now,
            )
            .await?;
            refresh_order_line_from_pick_tasks(&mut tx, ctx.owner_id, order.id, line_no).await?;
            completed_task_ids.push(task.id);
        }

        let outbound_lpn = bind_outbound_tote(
            &mut tx,
            ctx,
            order.id,
            req.outbound_lpn.as_deref(),
            now,
        )
        .await?;
        let updated = finalize_order_pick_state(&mut tx, ctx, &order, now).await?;

        let response = wms_domain::BatchCompletePickTaskResponse {
            order_id: order.id,
            completed_lines: u32::try_from(req.items.len())
                .map_err(|_| Wave4RepositoryError::InvalidQuantity)?,
            outbound_lpn,
            status: updated.status.clone(),
        };
        let audit = audit.map(|mut audit| {
            audit.diff = Some(AuditDiff::compute(
                serde_json::json!({ "status": &order.status }),
                serde_json::json!({
                    "status": &response.status,
                    "pick_task_ids": completed_task_ids,
                    "completed_lines": response.completed_lines,
                    "outbound_lpn": &response.outbound_lpn,
                    "operated_at": operated_at,
                }),
            ));
            audit
        });

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/pick-tasks/batch-complete",
            "outbound_order",
            order.id.to_string(),
            &response,
            now,
        )
        .await?;
        append_outbound_audit(
            &mut tx,
            ctx,
            audit,
            "batch_complete_pick_task",
            order.id,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: response,
            replayed: false,
        })
    }
}
