use super::*;

impl PgWave3Repository {
    pub async fn create_cold_chain_device_with_audit(
        &self,
        ctx: &AuthContext,
        req: CreateColdChainDeviceRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: AuditWriteRequest,
    ) -> Result<IdempotentMutation<ColdChainDevice>, Wave3RepositoryError> {
        if !crate::cold_chain::is_supported_device_type(&req.device_type) {
            return Err(Wave3RepositoryError::InvalidDeviceType);
        }
        if req.device_code.trim().is_empty() {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let row = sqlx::query_as::<_, ColdChainDeviceRow>(
            r#"
            INSERT INTO cold_chain_devices (
                id, owner_id, device_code, device_type,
                installed_at_location_code, calibration_due_at, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'active', $7, $7)
            RETURNING id, owner_id, device_code, device_type,
                      installed_at_location_code, calibration_due_at, status, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(&req.device_code)
        .bind(&req.device_type)
        .bind(&req.installed_at_location_code)
        .bind(req.calibration_due_at)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let device = map_cold_chain_device(row);

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/cold-chain/devices",
            "cold_chain_device",
            device.id.to_string(),
            &device,
            now,
        )
        .await?;
        let mut audit = audit;
        audit.resource_id = device.id.to_string();
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: device,
            replayed: false,
        })
    }

    pub async fn list_cold_chain_devices(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<ColdChainDevice>, Wave3RepositoryError> {
        let rows = sqlx::query_as::<_, ColdChainDeviceRow>(
            "SELECT id, owner_id, device_code, device_type, installed_at_location_code, calibration_due_at, status, created_at FROM cold_chain_devices WHERE owner_id = $1 ORDER BY device_code",
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(map_cold_chain_device).collect())
    }

    pub async fn update_cold_chain_device_with_audit(
        &self,
        ctx: &AuthContext,
        device_code: &str,
        req: UpdateColdChainDeviceRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: AuditWriteRequest,
    ) -> Result<IdempotentMutation<ColdChainDevice>, Wave3RepositoryError> {
        if let Some(device_type) = &req.device_type {
            if !crate::cold_chain::is_supported_device_type(device_type) {
                return Err(Wave3RepositoryError::InvalidDeviceType);
            }
        }
        let request_hash = request_hash(&serde_json::json!({
            "device_code": device_code,
            "request": &req,
        }))?;
        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }
        let row = sqlx::query_as::<_, ColdChainDeviceRow>(
            r#"
            UPDATE cold_chain_devices
               SET device_type = COALESCE($3, device_type),
                   installed_at_location_code = COALESCE($4, installed_at_location_code),
                   calibration_due_at = COALESCE($5, calibration_due_at),
                   updated_at = $6,
                   version = version + 1
             WHERE owner_id = $1 AND device_code = $2
            RETURNING id, owner_id, device_code, device_type,
                      installed_at_location_code, calibration_due_at, status, created_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(device_code)
        .bind(&req.device_type)
        .bind(&req.installed_at_location_code)
        .bind(req.calibration_due_at)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let device = map_cold_chain_device(row);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            "/api/v1/cold-chain/devices/{device_code}",
            "cold_chain_device",
            device.id.to_string(),
            &device,
            now,
        )
        .await?;
        let mut audit = audit;
        audit.resource_id = device.id.to_string();
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: device,
            replayed: false,
        })
    }

    pub async fn disable_cold_chain_device_with_audit(
        &self,
        ctx: &AuthContext,
        device_code: &str,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: AuditWriteRequest,
    ) -> Result<IdempotentMutation<ColdChainDevice>, Wave3RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({ "device_code": device_code }))?;
        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM cold_chain_devices WHERE owner_id = $1 AND device_code = $2 FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(device_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let Some(current_status) = current_status else {
            return Err(Wave3RepositoryError::NotFound);
        };
        if current_status == "monitoring" {
            return Err(Wave3RepositoryError::ActiveMonitoring);
        }
        let row = sqlx::query_as::<_, ColdChainDeviceRow>(
            r#"
            UPDATE cold_chain_devices
               SET status = 'inactive', updated_at = $3, version = version + 1
             WHERE owner_id = $1 AND device_code = $2
            RETURNING id, owner_id, device_code, device_type,
                      installed_at_location_code, calibration_due_at, status, created_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(device_code)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let device = map_cold_chain_device(row);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/cold-chain/devices/{device_code}/disable",
            "cold_chain_device",
            device.id.to_string(),
            &device,
            now,
        )
        .await?;
        let mut audit = audit;
        audit.resource_id = device.id.to_string();
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: device,
            replayed: false,
        })
    }
}
