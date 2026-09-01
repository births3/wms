use super::*;

impl PgLpnContainerRepository {
    pub async fn get(
        &self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<LpnContainer, LpnContainerRepositoryError> {
        let row = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            SELECT id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id,
                   current_lock_category, current_lock_reason_item_code, created_at, updated_at
              FROM lpn_containers
             WHERE id = $1 AND owner_id = $2
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(LpnContainerRepositoryError::NotFound)?;
        Ok(row.into())
    }

    pub async fn update(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateLpnContainerRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<LpnContainer, LpnContainerRepositoryError> {
        req.validate()?;
        let request_hash = request_hash(&serde_json::json!({
            "id": id,
            "request": &req,
        }))?;
        let path = "/api/v1/master-data/lpn-containers/{id}";
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
        let before = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            SELECT id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id,
                   current_lock_category, current_lock_reason_item_code, created_at, updated_at
              FROM lpn_containers
             WHERE id = $1 AND owner_id = $2
             FOR UPDATE
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(LpnContainerRepositoryError::NotFound)?;
        if before.status == LPN_CONTAINER_STATUS_DISABLED {
            return Err(LpnContainerRepositoryError::NotFound);
        }
        if req
            .status
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| value == LPN_CONTAINER_STATUS_DISABLED)
        {
            return Err(LpnContainerRepositoryError::StatusInvalid);
        }
        let status = req
            .status
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&before.status);
        let location_id = req.location_id.or(before.location_id);
        let capacity_cm3 = req.capacity_cm3.or(before.capacity_cm3);
        let row = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            UPDATE lpn_containers
               SET status = $3,
                   location_id = $4,
                   capacity_cm3 = $5,
                   updated_at = $6
             WHERE id = $1 AND owner_id = $2
            RETURNING id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id,
                      current_lock_category, current_lock_reason_item_code, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(status)
        .bind(location_id)
        .bind(capacity_cm3)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_write_error)?;
        let updated: LpnContainer = row.into();
        append_lpn_audit(&mut tx, ctx, "update_lpn_container", &updated, now).await?;
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

    pub async fn delete(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<LpnContainer, LpnContainerRepositoryError> {
        let request_hash = request_hash(&serde_json::json!({ "id": id, "action": "delete" }))?;
        let path = "/api/v1/master-data/lpn-containers/{id}";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<LpnContainer>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "DELETE",
            path,
            now,
        )
        .await?
        {
            return Ok(replay);
        }
        let before = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            SELECT id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id,
                   current_lock_category, current_lock_reason_item_code, created_at, updated_at
              FROM lpn_containers
             WHERE id = $1 AND owner_id = $2
             FOR UPDATE
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(LpnContainerRepositoryError::NotFound)?;
        if !lpn_status_allows_soft_delete(&before.status) {
            return Err(LpnContainerRepositoryError::NotDeletable);
        }
        let has_stock: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                  FROM inventory_batches
                 WHERE owner_id = $1
                   AND container_lpn = $2
                   AND qty_on_hand > 0
            )
            "#,
        )
        .bind(ctx.owner_id)
        .bind(&before.lpn_code)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if has_stock {
            return Err(LpnContainerRepositoryError::NotDeletable);
        }
        if before.status == LPN_CONTAINER_STATUS_DISABLED {
            let current = LpnContainer::from(before);
            store_idempotency_success(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &request_hash,
                "DELETE",
                path,
                "lpn_container",
                &current.id.to_string(),
                &current,
                now,
            )
            .await?;
            tx.commit().await.map_err(map_db_error)?;
            return Ok(current);
        }
        let row = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            UPDATE lpn_containers
               SET status = $3,
                   updated_at = $4
             WHERE id = $1 AND owner_id = $2
            RETURNING id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id,
                      current_lock_category, current_lock_reason_item_code, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(LPN_CONTAINER_STATUS_DISABLED)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_write_error)?;
        let deleted: LpnContainer = row.into();
        append_lpn_audit(&mut tx, ctx, "delete_lpn_container", &deleted, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "DELETE",
            path,
            "lpn_container",
            &deleted.id.to_string(),
            &deleted,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(deleted)
    }

    pub async fn batch_create(
        &self,
        ctx: &AuthContext,
        req: BatchCreateLpnContainerRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<LpnContainerListResponse, LpnContainerRepositoryError> {
        let req = BatchCreateLpnContainerRequest {
            container_type: req.container_type.trim().to_string(),
            capacity_cm3: req.capacity_cm3,
            count: req.count,
        };
        req.validate()?;
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
        let path = "/api/v1/master-data/lpn-containers/batch-create";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<LpnContainerListResponse>(
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
        let item = req.item_request();
        let mut created = Vec::with_capacity(req.count as usize);
        for _ in 0..req.count {
            created.push(insert_new_container_in_tx(&mut tx, ctx, &item, now).await?);
        }
        let response = LpnContainerListResponse { data: created };
        let resource_id = response
            .data
            .first()
            .map(|row| row.id.to_string())
            .ok_or(LpnContainerRepositoryError::BatchCountInvalid)?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            "lpn_container_batch",
            &resource_id,
            &response,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(response)
    }

    pub async fn upsert_type_policy_idempotent(
        &self,
        ctx: &AuthContext,
        req: UpsertLpnContainerTypePolicyRequest,
        idempotency_key: &str,
    ) -> Result<LpnContainerTypePolicy, LpnContainerRepositoryError> {
        if !wms_domain::is_valid_lpn_container_type(req.container_type.trim()) {
            return Err(LpnContainerRepositoryError::TypeInvalid);
        }
        let now = Utc::now();
        let container_type = req.container_type.trim().to_string();
        let request_hash = request_hash(&serde_json::json!({
            "container_type": &container_type,
            "allow_mix_batch": req.allow_mix_batch,
            "allow_mix_sku": req.allow_mix_sku,
        }))?;
        let path = "/api/v1/master-data/lpn-container-type-policies";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<LpnContainerTypePolicy>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PUT",
            path,
            now,
        )
        .await?
        {
            return Ok(replay);
        }
        let policy = LpnContainerTypePolicy {
            owner_id: ctx.owner_id,
            container_type,
            allow_mix_batch: req.allow_mix_batch,
            allow_mix_sku: req.allow_mix_sku,
        };
        sqlx::query(
            r#"
            INSERT INTO lpn_container_type_policies (
                owner_id, container_type, allow_mix_batch, allow_mix_sku, updated_at
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (owner_id, container_type)
            DO UPDATE SET
                allow_mix_batch = EXCLUDED.allow_mix_batch,
                allow_mix_sku = EXCLUDED.allow_mix_sku,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(policy.owner_id)
        .bind(&policy.container_type)
        .bind(policy.allow_mix_batch)
        .bind(policy.allow_mix_sku)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            "upsert_lpn_type_policy",
            "M1",
            "lpn_container_type_policy",
            policy.container_type.clone(),
            None,
        );
        audit.occurred_at = now;
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| LpnContainerRepositoryError::Audit(format!("{error:?}")))?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PUT",
            path,
            "lpn_container_type_policy",
            &policy.container_type,
            &policy,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(policy)
    }
}
