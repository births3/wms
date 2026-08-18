impl PgLpnContainerRepository {
    pub async fn apply_quality_lock(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: ApplyContainerQualityLockRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<LpnContainer, LpnContainerRepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "id": id,
            "action": "apply_quality_lock",
            "request": &req,
        }))?;
        let path = QUALITY_LOCK_PATH;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<LpnContainer>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            now,
        )
        .await?
        {
            return Ok(replay);
        }

        let before = lock_container_row_for_update(&mut tx, ctx.owner_id, id).await?;

        if before.status == LPN_CONTAINER_STATUS_DISABLED {
            return Err(LpnContainerRepositoryError::NotFound);
        }
        validate_apply_lock(&before.status, &req, ctx.user_id)?;

        // Validate dictionary reason item
        let is_reason_valid = validate_container_quality_lock_reason_in_tx(
            &mut tx,
            ctx.owner_id,
            &req.lock_category,
            &req.reason_dict_item_code,
            now,
        )
        .await
        .map_err(map_db_error)?;
        if !is_reason_valid {
            return Err(LpnContainerRepositoryError::ReasonInvalid);
        }

        // Validate M-QL existence if provided
        if let Some(mql_id) = req.quality_liaison_id {
            let mql_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM quality_liaison_orders WHERE id = $1 AND owner_id = $2)",
            )
            .bind(mql_id)
            .bind(ctx.owner_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            if !mql_exists {
                return Err(LpnContainerRepositoryError::NotFound);
            }
        }

        // Update container master
        let row = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            UPDATE lpn_containers
               SET current_lock_category = $3,
                   current_lock_reason_item_code = $4,
                   updated_at = $5
             WHERE id = $1 AND owner_id = $2
            RETURNING id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id,
                      current_lock_category, current_lock_reason_item_code, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.lock_category)
        .bind(&req.reason_dict_item_code)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_write_error)?;

        let updated: LpnContainer = row.into();

        // Batch inventory linkage & allocation release
        let target_batch_status = batch_status_for_lock_category(&req.lock_category);
        let approval_id = req
            .quality_liaison_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| id.to_string());

        let batches = sqlx::query_as::<_, (Uuid, i64, Option<Uuid>, Option<Uuid>, String)>(
            r#"
            SELECT id, qty_allocated::BIGINT, warehouse_id, location_id, status
              FROM inventory_batches
             WHERE owner_id = $1 AND container_lpn = $2
               AND status NOT IN ('loss_deducted', 'pending_destruction')
            "#,
        )
        .bind(ctx.owner_id)
        .bind(&before.lpn_code)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let mut first_batch_id: Option<Uuid> = None;
        for (batch_id, qty_allocated, _warehouse_id, _location_id, from_status) in &batches {
            if first_batch_id.is_none() {
                first_batch_id = Some(*batch_id);
            }
            if *qty_allocated > 0 {
                release_batch_allocations_with_outbox(
                    &mut tx,
                    ctx.owner_id,
                    *batch_id,
                    before.id,
                    &before.lpn_code,
                    &req.lock_category,
                    &req.reason_dict_item_code,
                    now,
                )
                .await?;
            }

            // Update batch status and reset qty_allocated to 0
            sqlx::query(
                r#"
                UPDATE inventory_batches
                   SET status = $3,
                       qty_allocated = 0,
                       updated_at = $4
                 WHERE id = $1 AND owner_id = $2
                "#,
            )
            .bind(*batch_id)
            .bind(ctx.owner_id)
            .bind(target_batch_status)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;

            // 库存流水：批次状态联动的前后值（movement + status_change）
            append_quality_lock_movement(
                &mut tx,
                ctx.owner_id,
                id,
                *batch_id,
                "quality_lock",
                &before.lpn_code,
                ctx.user_id,
                &ctx.actor_name,
                req.quality_liaison_id,
                now,
            )
            .await?;
            append_quality_lock_status_change(
                &mut tx,
                ctx.owner_id,
                *batch_id,
                from_status,
                target_batch_status,
                &req.reason_dict_item_code,
                "M-QL",
                &approval_id,
                now,
            )
            .await?;
        }

        // 同事务生成隔离移库任务（lock_move，容器级一条；含未上架容器）
        insert_lock_move_task(
            &mut tx,
            ctx.owner_id,
            &before.lpn_code,
            &req.lock_category,
            &req.reason_dict_item_code,
            first_batch_id,
            before.location_id,
            ctx.user_id,
            now,
        )
        .await?;

        // Pure INSERT into container_quality_lock_events
        sqlx::query(
            r#"
            INSERT INTO container_quality_lock_events (
                id, owner_id, container_id, lpn_code, event_type, lock_category,
                reason_dict_item_code, reason_desc, evidence_urls, quality_liaison_id,
                operated_by, witness_id, occurred_at, note
            ) VALUES (
                gen_random_uuid(), $1, $2, $3, 'lock', $4,
                $5, $6, $7, $8,
                $9, $10, $11, $12
            )
            "#,
        )
        .bind(ctx.owner_id)
        .bind(before.id)
        .bind(&before.lpn_code)
        .bind(&req.lock_category)
        .bind(&req.reason_dict_item_code)
        .bind(&req.reason_desc)
        .bind(json!(req.evidence_urls))
        .bind(req.quality_liaison_id)
        .bind(ctx.user_id)
        .bind(req.witness_id)
        .bind(now)
        .bind(&req.note)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        append_lpn_audit(&mut tx, ctx, "apply_container_quality_lock", &updated, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            "lpn_container",
            &updated.id.to_string(),
            &updated,
            now,
        )
        .await?;

        tx.commit().await.map_err(map_db_error)?;
        Ok(updated)
    }

    pub async fn change_quality_lock_reason(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: ChangeContainerQualityLockReasonRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<LpnContainer, LpnContainerRepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "id": id,
            "action": "change_quality_lock_reason",
            "request": &req,
        }))?;
        let path = QUALITY_LOCK_PATH;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<LpnContainer>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            path,
            now,
        )
        .await?
        {
            return Ok(replay);
        }

        let before = lock_container_row_for_update(&mut tx, ctx.owner_id, id).await?;

        if before.status == LPN_CONTAINER_STATUS_DISABLED {
            return Err(LpnContainerRepositoryError::NotFound);
        }
        validate_change_reason(before.current_lock_category.as_deref(), &req, ctx.user_id)?;
        let current_category = before
            .current_lock_category
            .as_deref()
            .unwrap_or(LPN_LOCK_CATEGORY_QUALIFIED);
        let target_category = req.lock_category.as_deref().unwrap_or(current_category);

        // Validate dictionary reason item
        let is_reason_valid = validate_container_quality_lock_reason_in_tx(
            &mut tx,
            ctx.owner_id,
            target_category,
            &req.reason_dict_item_code,
            now,
        )
        .await
        .map_err(map_db_error)?;
        if !is_reason_valid {
            return Err(LpnContainerRepositoryError::ReasonInvalid);
        }

        let row = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            UPDATE lpn_containers
               SET current_lock_category = $3,
                   current_lock_reason_item_code = $4,
                   updated_at = $5
             WHERE id = $1 AND owner_id = $2
            RETURNING id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id,
                      current_lock_category, current_lock_reason_item_code, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(target_category)
        .bind(&req.reason_dict_item_code)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_write_error)?;

        let updated: LpnContainer = row.into();

        // If category switched between quarantine and rejected, update batch status
        if target_category != current_category {
            let target_batch_status = batch_status_for_lock_category(target_category);
            let batches = sqlx::query_as::<_, (Uuid, String)>(
                r#"
                SELECT id, status
                  FROM inventory_batches
                 WHERE owner_id = $1 AND container_lpn = $2
                   AND status NOT IN ('loss_deducted', 'pending_destruction')
                "#,
            )
            .bind(ctx.owner_id)
            .bind(&before.lpn_code)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_db_error)?;
            let approval_id = updated.id.to_string();
            let first_batch_id = batches.first().map(|(id, _)| *id);
            for (batch_id, from_status) in &batches {
                if from_status == target_batch_status {
                    continue;
                }
                sqlx::query(
                    r#"
                    UPDATE inventory_batches
                       SET status = $3,
                           updated_at = $4
                     WHERE owner_id = $1 AND id = $2
                    "#,
                )
                .bind(ctx.owner_id)
                .bind(batch_id)
                .bind(target_batch_status)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
                append_quality_lock_movement(
                    &mut tx,
                    ctx.owner_id,
                    id,
                    *batch_id,
                    "quality_lock_change",
                    &before.lpn_code,
                    ctx.user_id,
                    &ctx.actor_name,
                    None,
                    now,
                )
                .await?;
                append_quality_lock_status_change(
                    &mut tx,
                    ctx.owner_id,
                    *batch_id,
                    from_status,
                    target_batch_status,
                    &req.reason_dict_item_code,
                    "M-QL",
                    &approval_id,
                    now,
                )
                .await?;
            }
            insert_lock_move_task(
                &mut tx,
                ctx.owner_id,
                &before.lpn_code,
                target_category,
                &req.reason_dict_item_code,
                first_batch_id,
                before.location_id,
                ctx.user_id,
                now,
            )
            .await?;
        }

        // Pure INSERT into container_quality_lock_events
        sqlx::query(
            r#"
            INSERT INTO container_quality_lock_events (
                id, owner_id, container_id, lpn_code, event_type, lock_category,
                reason_dict_item_code, reason_desc, evidence_urls, quality_liaison_id,
                operated_by, witness_id, occurred_at, note
            ) VALUES (
                gen_random_uuid(), $1, $2, $3, 'change_reason', $4,
                $5, $6, $7, NULL,
                $8, $9, $10, $11
            )
            "#,
        )
        .bind(ctx.owner_id)
        .bind(before.id)
        .bind(&before.lpn_code)
        .bind(target_category)
        .bind(&req.reason_dict_item_code)
        .bind(&req.reason_desc)
        .bind(json!(req.evidence_urls))
        .bind(ctx.user_id)
        .bind(req.witness_id)
        .bind(now)
        .bind(&req.note)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        append_lpn_audit(&mut tx, ctx, "change_container_quality_lock_reason", &updated, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            path,
            "lpn_container",
            &updated.id.to_string(),
            &updated,
            now,
        )
        .await?;

        tx.commit().await.map_err(map_db_error)?;
        Ok(updated)
    }

    pub async fn release_quality_lock(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: ReleaseContainerQualityLockRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<LpnContainer, LpnContainerRepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "id": id,
            "action": "release_quality_lock",
            "request": &req,
        }))?;
        let path = QUALITY_LOCK_RELEASE_PATH;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<LpnContainer>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            now,
        )
        .await?
        {
            return Ok(replay);
        }

        let before = lock_container_row_for_update(&mut tx, ctx.owner_id, id).await?;

        if before.status == LPN_CONTAINER_STATUS_DISABLED {
            return Err(LpnContainerRepositoryError::NotFound);
        }
        let current_category = before
            .current_lock_category
            .as_deref()
            .unwrap_or(LPN_LOCK_CATEGORY_QUALIFIED);

        // Determine M-QL quality liaison ID (from request or from latest lock event)
        let mql_id = if let Some(lid) = req.quality_liaison_id {
            Some(lid)
        } else {
            sqlx::query_scalar::<_, Option<Uuid>>(
                r#"
                SELECT quality_liaison_id
                  FROM container_quality_lock_events
                 WHERE owner_id = $1 AND container_id = $2 AND quality_liaison_id IS NOT NULL
                 ORDER BY occurred_at DESC
                 LIMIT 1
                "#,
            )
            .bind(ctx.owner_id)
            .bind(before.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .flatten()
        };

        let mql_status: Option<String> = if let Some(lid) = mql_id {
            sqlx::query_scalar(
                "SELECT status FROM quality_liaison_orders WHERE id = $1 AND owner_id = $2",
            )
            .bind(lid)
            .bind(ctx.owner_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
        } else {
            None
        };
        validate_release_lock(Some(current_category), &req, ctx.user_id, mql_status.as_deref())?;

        // Precise batch write-back: only revert batches that are still in the locked state
        let expected_locked_status = batch_status_for_lock_category(current_category);
        let batches = sqlx::query_as::<_, (Uuid, String)>(
            r#"
            SELECT id, status
              FROM inventory_batches
             WHERE owner_id = $1 AND container_lpn = $2 AND status = $3
            "#,
        )
        .bind(ctx.owner_id)
        .bind(&before.lpn_code)
        .bind(expected_locked_status)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let approval_id = mql_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| before.id.to_string());
        let mut first_batch_id: Option<Uuid> = None;
        for (batch_id, from_status) in &batches {
            if first_batch_id.is_none() {
                first_batch_id = Some(*batch_id);
            }
            sqlx::query(
                r#"
                UPDATE inventory_batches
                   SET status = $4,
                       updated_at = $5
                 WHERE owner_id = $1 AND id = $2 AND status = $3
                "#,
            )
            .bind(ctx.owner_id)
            .bind(batch_id)
            .bind(expected_locked_status)
            .bind(STATUS_QUALIFIED)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            append_quality_lock_movement(
                &mut tx,
                ctx.owner_id,
                id,
                *batch_id,
                "quality_lock_release",
                &before.lpn_code,
                ctx.user_id,
                &ctx.actor_name,
                mql_id,
                now,
            )
            .await?;
            append_quality_lock_status_change(
                &mut tx,
                ctx.owner_id,
                *batch_id,
                from_status,
                STATUS_QUALIFIED,
                "release",
                "M-QL",
                &approval_id,
                now,
            )
            .await?;
        }

        // Update container master back to qualified
        let row = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            UPDATE lpn_containers
               SET current_lock_category = $4,
                   current_lock_reason_item_code = NULL,
                   updated_at = $3
             WHERE id = $1 AND owner_id = $2
            RETURNING id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id,
                      current_lock_category, current_lock_reason_item_code, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .bind(LPN_LOCK_CATEGORY_QUALIFIED)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_write_error)?;

        let updated: LpnContainer = row.into();

        // 同事务生成移回合格区任务（lock_move_back；目标 = 原库位或系统推荐合格位）
        insert_lock_move_back_task(
            &mut tx,
            ctx.owner_id,
            &before.lpn_code,
            before.location_id,
            first_batch_id,
            ctx.user_id,
            now,
        )
        .await?;

        // Pure INSERT into container_quality_lock_events
        sqlx::query(
            r#"
            INSERT INTO container_quality_lock_events (
                id, owner_id, container_id, lpn_code, event_type, lock_category,
                reason_dict_item_code, reason_desc, evidence_urls, quality_liaison_id,
                operated_by, witness_id, occurred_at, note
            ) VALUES (
                gen_random_uuid(), $1, $2, $3, 'release', NULL,
                NULL, $4, '[]'::jsonb, $5,
                $6, $7, $8, $9
            )
            "#,
        )
        .bind(ctx.owner_id)
        .bind(before.id)
        .bind(&before.lpn_code)
        .bind(&req.reason_desc)
        .bind(mql_id)
        .bind(ctx.user_id)
        .bind(req.witness_id)
        .bind(now)
        .bind(&req.note)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        append_lpn_audit(&mut tx, ctx, "release_container_quality_lock", &updated, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            "lpn_container",
            &updated.id.to_string(),
            &updated,
            now,
        )
        .await?;

        tx.commit().await.map_err(map_db_error)?;
        Ok(updated)
    }

    /// 扫描 lock_move 未完成超过阈值的容器：
    /// 任务终态（inventory_relocations.status <> 'completed'）且容器当前所在库区
    /// 与锁类别对应质量区不匹配（即实物未完成物理隔离）时判为超时未移库。
    pub async fn scan_overdue_lock_moves(
        &self,
        owner_id: Uuid,
        threshold_hours: i64,
        now: DateTime<Utc>,
    ) -> Result<Vec<OverdueLockMove>, LpnContainerRepositoryError> {
        let threshold_time = now - chrono::Duration::hours(threshold_hours);
        let rows = sqlx::query_as::<_, (Uuid, String, DateTime<Utc>)>(
            r#"
            SELECT c.id, c.lpn_code, MAX(r.created_at)
              FROM inventory_relocations r
              JOIN lpn_containers c
                ON c.owner_id = r.owner_id
               AND c.lpn_code = r.lpn_code
             WHERE r.owner_id = $1
               AND r.reason LIKE $2
               AND r.status <> 'completed'
               AND r.created_at <= $3
             GROUP BY c.id, c.lpn_code
             ORDER BY MAX(r.created_at) ASC
            "#,
        )
        .bind(owner_id)
        .bind(format!("{LOCK_MOVE_MARKER}%"))
        .bind(threshold_time)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        let mut overdue = Vec::new();
        for (container_id, lpn_code, task_created_at) in rows {
            let lock_category: Option<String> = sqlx::query_scalar(
                "SELECT current_lock_category FROM lpn_containers WHERE id = $1 AND owner_id = $2",
            )
            .bind(container_id)
            .bind(owner_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
            .flatten();
            let Some(category) = lock_category else {
                continue;
            };
            if category != LPN_LOCK_CATEGORY_QUARANTINE
                && category != LPN_LOCK_CATEGORY_REJECTED
            {
                continue;
            }
            let expected_color = quality_color_for_lock_category(&category);
            // 容器当前所在库区 == 锁类别对应质量区 → 实物已到位，视为移库完成。
            let current_matches: bool = sqlx::query_scalar(
                r#"
                SELECT EXISTS (
                    SELECT 1
                      FROM lpn_containers container
                      JOIN warehouse_locations location
                        ON location.id = container.location_id
                       AND location.owner_id = container.owner_id
                      JOIN warehouse_zones zone
                        ON zone.id = location.zone_id
                       AND zone.owner_id = location.owner_id
                     WHERE container.id = $1
                       AND container.owner_id = $2
                       AND zone.quality_color = $3
                )
                "#,
            )
            .bind(container_id)
            .bind(owner_id)
            .bind(expected_color)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_error)?;
            if !current_matches {
                overdue.push(OverdueLockMove {
                    container_id,
                    lpn_code,
                    task_created_at,
                });
            }
        }
        Ok(overdue)
    }
}
