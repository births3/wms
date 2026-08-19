const OUTBOUND_WAVE_STATUS_DRAFT: &str = "draft";
const OUTBOUND_WAVE_STATUS_RELEASED: &str = "released";

const OUTBOUND_REVALIDATE_ALLOWED_STATUSES: &str =
    "pending_validation|validation_exception|confirmed";

impl PgWave4Repository {
    /// 重新校验出库订单：批号存在性 + 可用库存充足性。
    ///
    /// 校验通过置 `confirmed`；校验失败置 `validation_exception`，失败明细写入审计 diff。
    pub async fn revalidate_outbound_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundOrder>, Wave4RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "outbound_order_id": order_id,
            "action": "revalidate",
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

        let order_row = lock_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        if !matches!(
            order_row.status.as_str(),
            OUTBOUND_STATUS_PENDING_VALIDATION
                | OUTBOUND_STATUS_VALIDATION_EXCEPTION
                | OUTBOUND_STATUS_CONFIRMED
        ) {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: OUTBOUND_REVALIDATE_ALLOWED_STATUSES.to_string(),
                actual: order_row.status,
            });
        }

        let order = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        let failures = collect_outbound_validation_failures(&mut tx, ctx.owner_id, &order).await?;
        let next_status = if failures.is_empty() {
            OUTBOUND_STATUS_CONFIRMED
        } else {
            OUTBOUND_STATUS_VALIDATION_EXCEPTION
        };
        sqlx::query(
            r#"
            UPDATE outbound_orders
               SET status = $3, updated_at = $4, version = version + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(next_status)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let updated = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;

        let audit = audit.map(|mut audit| {
            audit.diff = Some(AuditDiff::compute(
                serde_json::json!({ "status": &order.status }),
                serde_json::json!({
                    "status": &updated.status,
                    "validation_failures": failures,
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
            "/api/v1/outbound/orders/{id}/revalidate",
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
            "revalidate_outbound_order",
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

    /// 作废申请：未进波次的订单（待校验/校验异常/已确认）置 `void_requested`。
    pub async fn request_void_outbound_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundOrder>, Wave4RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "outbound_order_id": order_id,
            "action": "void_request",
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

        let order_row = lock_outbound_order(&mut tx, ctx.owner_id, order_id).await?;
        if !matches!(
            order_row.status.as_str(),
            OUTBOUND_STATUS_PENDING_VALIDATION
                | OUTBOUND_STATUS_VALIDATION_EXCEPTION
                | OUTBOUND_STATUS_CONFIRMED
        ) {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: OUTBOUND_REVALIDATE_ALLOWED_STATUSES.to_string(),
                actual: order_row.status,
            });
        }

        sqlx::query(
            r#"
            UPDATE outbound_orders
               SET status = $3, updated_at = $4, version = version + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(OUTBOUND_STATUS_VOID_REQUESTED)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let updated = load_outbound_order(&mut tx, ctx.owner_id, order_id).await?;

        let audit = audit.map(|mut audit| {
            audit.diff = Some(AuditDiff::compute(
                serde_json::json!({ "status": &order_row.status }),
                serde_json::json!({ "status": &updated.status }),
            ));
            audit
        });
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/orders/{id}/void-request",
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
            "request_void_outbound_order",
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

    /// 波次下发：`draft` → `released`，锁定订单库存并生成拣选任务（复用创建波次的语义）。
    pub async fn release_outbound_wave(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundWave>, Wave4RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let result = self
            .release_outbound_wave_in_tx(&mut tx, ctx, id, now, idempotency_key, audit)
            .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(result)
    }

    pub async fn release_outbound_wave_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<OutboundWave>, Wave4RepositoryError> {
        let request_hash =
            request_hash(&serde_json::json!({ "wave_id": id, "action": "release" }))?;

        lock_idempotency_key(&mut *tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut *tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let mut wave_row = sqlx::query_as::<_, OutboundWaveRow>(
            r#"
            SELECT id, owner_id, wave_no, status, created_at, updated_at
              FROM outbound_waves
             WHERE owner_id = $1 AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave4RepositoryError::NotFound)?;
        if wave_row.status != OUTBOUND_WAVE_STATUS_DRAFT {
            return Err(Wave4RepositoryError::InvalidStatus {
                expected: OUTBOUND_WAVE_STATUS_DRAFT.to_string(),
                actual: wave_row.status,
            });
        }

        let order_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT outbound_order_id
              FROM outbound_wave_orders
             WHERE owner_id = $1 AND wave_id = $2
             ORDER BY created_at ASC, outbound_order_id ASC
            "#,
        )
        .bind(ctx.owner_id)
        .bind(id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if order_ids.is_empty() {
            return Err(Wave4RepositoryError::EmptySelection);
        }

        let mut task_drafts = Vec::new();
        for order_id in &order_ids {
            let order = lock_outbound_order(&mut *tx, ctx.owner_id, *order_id).await?;
            if order.status != OUTBOUND_STATUS_CONFIRMED {
                return Err(Wave4RepositoryError::InvalidStatus {
                    expected: OUTBOUND_STATUS_CONFIRMED.to_string(),
                    actual: order.status,
                });
            }
            let order_with_lines = load_outbound_order(&mut *tx, ctx.owner_id, order.id).await?;
            for line in &order_with_lines.lines {
                allocate_inventory_for_outbound(&mut *tx, ctx.owner_id, order.id, line, now).await?;
            }
            task_drafts.extend(
                collect_locked_allocation_task_drafts(
                    &mut *tx,
                    ctx.owner_id,
                    order.id,
                    &order.wms_order_no,
                )
                .await?,
            );
            sqlx::query(
                r#"
                UPDATE outbound_orders
                   SET status = $3, updated_at = $4, version = version + 1
                 WHERE owner_id = $1 AND id = $2
                "#,
            )
            .bind(ctx.owner_id)
            .bind(order.id)
            .bind(OUTBOUND_STATUS_IN_WAVE)
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        }

        insert_wave_pick_tasks(&mut *tx, ctx, wave_row.id, task_drafts, now).await?;

        sqlx::query(
            r#"
            UPDATE outbound_waves
               SET status = $3, updated_at = $4, version = version + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(id)
        .bind(OUTBOUND_WAVE_STATUS_RELEASED)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        wave_row.status = OUTBOUND_WAVE_STATUS_RELEASED.to_string();
        wave_row.updated_at = now;
        let released = map_outbound_wave(wave_row, order_ids);

        store_idempotency_success(
            &mut *tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/outbound/waves/{wave_id}/release",
            "outbound_wave",
            released.id.to_string(),
            &released,
            now,
        )
        .await?;
        append_outbound_audit(tx, ctx, audit, "release_outbound_wave", id, now).await?;
        Ok(IdempotentMutation {
            value: released,
            replayed: false,
        })
    }
}

/// 参照创建出库订单时的库存约束：批号必须存在于合格且未召回的库存中，且可用量覆盖计划量。
async fn collect_outbound_validation_failures(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order: &OutboundOrder,
) -> Result<Vec<serde_json::Value>, Wave4RepositoryError> {
    let mut failures = Vec::new();
    if order.lines.is_empty() {
        failures.push(serde_json::json!({ "reason": "empty_lines" }));
        return Ok(failures);
    }
    for line in &order.lines {
        let (batch_count, available_qty): (i64, wms_domain::Quantity) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::BIGINT,
                   COALESCE(SUM(qty_on_hand - qty_frozen), 0)
              FROM inventory_batches
             WHERE owner_id = $1
               AND product_code = $2
               AND batch_no = $3
               AND status = $4
               AND recall_flag = FALSE
            "#,
        )
        .bind(owner_id)
        .bind(&line.product_code)
        .bind(&line.batch_no)
        .bind(STATUS_QUALIFIED)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if batch_count == 0 {
            failures.push(serde_json::json!({
                "line_no": line.line_no,
                "product_code": line.product_code,
                "batch_no": line.batch_no,
                "reason": "batch_not_found",
            }));
        } else if available_qty < line.planned_qty {
            failures.push(serde_json::json!({
                "line_no": line.line_no,
                "product_code": line.product_code,
                "batch_no": line.batch_no,
                "planned_qty": line.planned_qty,
                "available_qty": available_qty,
                "reason": "insufficient_inventory",
            }));
        }
    }
    Ok(failures)
}

async fn collect_locked_allocation_task_drafts(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
    order_no: &str,
) -> Result<Vec<PickTaskDraft>, Wave4RepositoryError> {
    let allocated_tasks =
        sqlx::query_as::<_, (Uuid, i32, Uuid, String, String, Uuid, String, Uuid, wms_domain::Quantity)>(
            r#"
            SELECT allocation.batch_id,
                   allocation.line_no,
                   allocation.outbound_order_id,
                   batch.product_code,
                   batch.batch_no,
                   batch.location_id,
                   batch.location_code,
                   location.warehouse_id,
                   allocation.allocated_qty
              FROM inventory_allocations allocation
              JOIN inventory_batches batch
                ON batch.owner_id = allocation.owner_id
               AND batch.id = allocation.batch_id
              JOIN warehouse_locations location
                ON location.owner_id = batch.owner_id
               AND location.id = batch.location_id
             WHERE allocation.owner_id = $1
               AND allocation.outbound_order_id = $2
               AND allocation.status = 'locked'
             ORDER BY batch.location_code ASC, allocation.line_no ASC, allocation.batch_id ASC
            "#,
        )
        .bind(owner_id)
        .bind(order_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(allocated_tasks
        .into_iter()
        .map(
            |(
                batch_id,
                line_no,
                order_id,
                product_code,
                batch_no,
                location_id,
                location_code,
                warehouse_id,
                planned_qty,
            )| PickTaskDraft {
                order_id,
                order_no: order_no.to_string(),
                warehouse_id,
                line_no,
                batch_id,
                product_code,
                batch_no,
                location_id,
                location_code,
                planned_qty,
            },
        )
        .collect())
}

async fn insert_wave_pick_tasks(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    wave_id: Uuid,
    mut task_drafts: Vec<PickTaskDraft>,
    now: DateTime<Utc>,
) -> Result<(), Wave4RepositoryError> {
    task_drafts.sort_by(|left, right| {
        left.location_code
            .cmp(&right.location_code)
            .then_with(|| left.order_id.cmp(&right.order_id))
            .then_with(|| left.line_no.cmp(&right.line_no))
            .then_with(|| left.batch_id.cmp(&right.batch_id))
    });
    for (index, task) in task_drafts.into_iter().enumerate() {
        let route_sequence =
            i32::try_from(index + 1).map_err(|_| Wave4RepositoryError::InvalidQuantity)?;
        let pick_task_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO outbound_pick_tasks (
                id, owner_id, wave_id, outbound_order_id, line_no, batch_id,
                product_code, batch_no, location_id, location_code, planned_qty,
                status, route_sequence, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'pending_assignment', $12, $13, $13)
            "#,
        )
        .bind(pick_task_id)
        .bind(ctx.owner_id)
        .bind(wave_id)
        .bind(task.order_id)
        .bind(task.line_no)
        .bind(task.batch_id)
        .bind(&task.product_code)
        .bind(&task.batch_no)
        .bind(task.location_id)
        .bind(&task.location_code)
        .bind(task.planned_qty)
        .bind(route_sequence)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        crate::task_engine::create_task_in_tx(
            tx,
            ctx,
            &wms_domain::CreateWarehouseTaskRequest {
                task_type_code: wms_domain::TASK_TYPE_PICK.to_string(),
                source_module: "M4".to_string(),
                source_doc_type: "outbound_order".to_string(),
                source_doc_id: Some(task.order_id),
                source_doc_no: task.order_no,
                source_line_no: Some(task.line_no),
                source_task_key: format!(
                    "M4:pick:{}:{}:{}:{}",
                    wave_id, task.order_id, task.line_no, task.batch_id
                ),
                warehouse_id: task.warehouse_id,
                task_group_code: crate::task_engine::default_task_group_code(task.warehouse_id),
                product_id: None,
                product_code: task.product_code,
                batch_id: Some(task.batch_id),
                batch_no: Some(task.batch_no),
                planned_qty: task.planned_qty,
                source_location_id: Some(task.location_id),
                source_location_code: Some(task.location_code),
                target_location_id: None,
                target_location_code: None,
                priority: None,
                urgent_order: false,
                predecessor_task_id: None,
            },
            now,
        )
        .await
        .map_err(|error| {
            Wave4RepositoryError::Database(format!("M-TE 拣选任务创建失败: {error:?}"))
        })?;
    }
    Ok(())
}
