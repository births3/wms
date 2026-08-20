use super::*;

impl LpnContainerQualityLockService {
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

        let mut req = req;
        if req.quality_liaison_id.is_none() && req.create_liaison {
            req.quality_liaison_id =
                Some(create_liaison_for_lock(&mut tx, ctx, &before.lpn_code, now).await?);
        }
        if let Some(mql_id) = req.quality_liaison_id {
            if !quality_liaison_exists(&mut tx, ctx.owner_id, mql_id).await? {
                return Err(LpnContainerRepositoryError::NotFound);
            }
            bind_liaison_to_container(&mut tx, ctx.owner_id, mql_id, &before.lpn_code, now).await?;
        }

        let updated = update_container_lock_fields(
            &mut tx,
            ctx.owner_id,
            id,
            &req.lock_category,
            &req.reason_dict_item_code,
            now,
        )
        .await?;
        let target_batch_status = batch_status_for_lock_category(&req.lock_category);
        let approval_id = id.to_string();
        let batches =
            list_container_batches_for_lock(&mut tx, ctx.owner_id, &before.lpn_code).await?;
        let mut first_batch_id: Option<Uuid> = None;
        for (batch_id, qty_allocated, from_status) in &batches {
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
            update_batch_lock_status(
                &mut tx,
                ctx.owner_id,
                *batch_id,
                target_batch_status,
                now,
                true,
            )
            .await?;
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
        insert_quality_lock_event(
            &mut tx,
            ctx.owner_id,
            before.id,
            &before.lpn_code,
            "lock",
            Some(&req.lock_category),
            Some(&req.reason_dict_item_code),
            req.reason_desc.as_deref(),
            &json!(req.evidence_urls),
            req.quality_liaison_id,
            ctx.user_id,
            req.witness_id,
            now,
            req.note.as_deref(),
        )
        .await?;
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
        if let Some(mql_id) = req.quality_liaison_id {
            if !quality_liaison_exists(&mut tx, ctx.owner_id, mql_id).await? {
                return Err(LpnContainerRepositoryError::NotFound);
            }
            bind_liaison_to_container(&mut tx, ctx.owner_id, mql_id, &before.lpn_code, now).await?;
        }

        let updated = update_container_lock_fields(
            &mut tx,
            ctx.owner_id,
            id,
            target_category,
            &req.reason_dict_item_code,
            now,
        )
        .await?;
        if target_category != current_category {
            let target_batch_status = batch_status_for_lock_category(target_category);
            let batches =
                list_container_batch_statuses(&mut tx, ctx.owner_id, &before.lpn_code).await?;
            let approval_id = id.to_string();
            let first_batch_id = batches.first().map(|(batch_id, _)| *batch_id);
            for (batch_id, from_status) in &batches {
                if from_status == target_batch_status {
                    continue;
                }
                update_batch_lock_status(
                    &mut tx,
                    ctx.owner_id,
                    *batch_id,
                    target_batch_status,
                    now,
                    false,
                )
                .await?;
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
        insert_quality_lock_event(
            &mut tx,
            ctx.owner_id,
            before.id,
            &before.lpn_code,
            "change_reason",
            Some(target_category),
            Some(&req.reason_dict_item_code),
            req.reason_desc.as_deref(),
            &json!(req.evidence_urls),
            req.quality_liaison_id,
            ctx.user_id,
            req.witness_id,
            now,
            req.note.as_deref(),
        )
        .await?;
        append_lpn_audit(
            &mut tx,
            ctx,
            "change_container_quality_lock_reason",
            &updated,
            now,
        )
        .await?;
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
    ) -> Result<ReleaseContainerQualityLockResponse, LpnContainerRepositoryError> {
        let request_hash = request_hash(&serde_json::json!({
            "id": id,
            "action": "release_quality_lock",
            "request": &req,
        }))?;
        let path = QUALITY_LOCK_RELEASE_PATH;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<ReleaseContainerQualityLockResponse>(
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
        let mql_id = if let Some(lid) = req.quality_liaison_id {
            Some(lid)
        } else {
            latest_container_liaison_id(&mut tx, ctx.owner_id, before.id).await?
        };
        let mql_status = if let Some(lid) = mql_id {
            quality_liaison_status(&mut tx, ctx.owner_id, lid).await?
        } else {
            None
        };
        validate_release_lock(
            Some(current_category),
            &req,
            ctx.user_id,
            mql_status.as_deref(),
        )?;
        let expected_locked_status = batch_status_for_lock_category(current_category);
        let (batches, skipped_batches) = classify_unlock_batches(
            &mut tx,
            ctx.owner_id,
            &before.lpn_code,
            expected_locked_status,
            before.id,
        )
        .await?;
        let approval_id = mql_id
            .map(|liaison_id| liaison_id.to_string())
            .unwrap_or_else(|| before.id.to_string());
        let mut first_batch_id: Option<Uuid> = None;
        for (batch_id, from_status) in &batches {
            if first_batch_id.is_none() {
                first_batch_id = Some(*batch_id);
            }
            rewrite_batch_qualified(
                &mut tx,
                ctx.owner_id,
                *batch_id,
                expected_locked_status,
                now,
            )
            .await?;
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
        let updated = clear_container_lock_fields(&mut tx, ctx.owner_id, id, now).await?;
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
        insert_quality_lock_event(
            &mut tx,
            ctx.owner_id,
            before.id,
            &before.lpn_code,
            "release",
            None,
            None,
            req.reason_desc.as_deref(),
            &json!([]),
            mql_id,
            ctx.user_id,
            req.witness_id,
            now,
            req.note.as_deref(),
        )
        .await?;
        append_lpn_audit(
            &mut tx,
            ctx,
            "release_container_quality_lock",
            &updated,
            now,
        )
        .await?;
        let response = ReleaseContainerQualityLockResponse {
            container: updated,
            skipped_batches,
        };
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            "lpn_container",
            &response.container.id.to_string(),
            &response,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(response)
    }
}
