impl PgWave3Repository {
pub async fn inspect_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: InspectReceivingOrderRequest,
        today: NaiveDate,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<ReceivingInspectionRecord>, Wave3RepositoryError> {
        if req.accepted_qty < 0 || req.rejected_qty < 0 {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let production_date = parse_date(&req.production_date)?;
        let expiry_date = parse_date(&req.expiry_date)?;
        if expiry_date < today {
            return Err(Wave3RepositoryError::BatchExpired);
        }
        let request_hash = request_hash(&serde_json::json!({
            "receiving_order_id": id,
            "request": req,
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

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if order.status != "inspecting" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "inspecting".to_string(),
                actual: order.status,
            });
        }

        let inspection = ReceivingInspectionRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            batch_no: req.batch_no.clone(),
            accepted_qty: req.accepted_qty,
            rejected_qty: req.rejected_qty,
            quality_status: req.quality_status.clone(),
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_inspections (
                id, receiving_order_id, owner_id, batch_no, accepted_qty,
                rejected_qty, production_date, expiry_date, quality_status,
                trace_codes, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(inspection.id)
        .bind(inspection.receiving_order_id)
        .bind(inspection.owner_id)
        .bind(&inspection.batch_no)
        .bind(inspection.accepted_qty)
        .bind(inspection.rejected_qty)
        .bind(production_date)
        .bind(expiry_date)
        .bind(&inspection.quality_status)
        .bind(&req.trace_codes)
        .bind(inspection.occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/inspect",
            "receiving_inspection",
            inspection.id.to_string(),
            &inspection,
            now,
        )
        .await?;
        if let Some(audit) = audit {
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: inspection,
            replayed: false,
        })
    }

    pub async fn sign_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: SignInspectionRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<InspectionSignatureRecord>, Wave3RepositoryError> {
        if req.dual_required && req.second_signer_id.is_none() {
            return Err(Wave3RepositoryError::MissingSecondSigner);
        }
        if let Some(second_signer_id) = req.second_signer_id {
            if second_signer_id == req.first_signer_id {
                return Err(Wave3RepositoryError::SameSigner);
            }
        }
        let request_hash = request_hash(&serde_json::json!({
            "receiving_order_id": id,
            "request": req,
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

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if order.status != "inspecting" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "inspecting".to_string(),
                actual: order.status,
            });
        }

        let signature = InspectionSignatureRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            first_signer_id: req.first_signer_id,
            second_signer_id: req.second_signer_id,
            signed_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_inspection_signatures (
                id, receiving_order_id, owner_id, dual_required,
                first_signer_id, second_signer_id, strategy_rule_id, signed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NULL, $7)
            "#,
        )
        .bind(signature.id)
        .bind(signature.receiving_order_id)
        .bind(signature.owner_id)
        .bind(req.dual_required)
        .bind(signature.first_signer_id)
        .bind(signature.second_signer_id)
        .bind(signature.signed_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        sqlx::query(
            "UPDATE receiving_orders SET status = 'putaway', updated_at = $3, version = version + 1 WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/sign",
            "receiving_inspection_signature",
            signature.id.to_string(),
            &signature,
            now,
        )
        .await?;
        if let Some(audit) = audit {
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: signature,
            replayed: false,
        })
    }

    pub async fn list_inventory_batches(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<InventoryBatch>, Wave3RepositoryError> {
        let rows = sqlx::query_as::<_, InventoryBatchRow>(
            r#"
            SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                   qty_on_hand, qty_locked, quality_status, location_id, location_code,
                   recall_flag, created_at, updated_at
              FROM inventory_batches
             WHERE owner_id = $1
             ORDER BY updated_at DESC, id
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        Ok(rows.into_iter().map(map_inventory_batch).collect())
    }

    pub async fn change_inventory_status_with_audit(
        &self,
        ctx: &AuthContext,
        req: ChangeInventoryStatusRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<InventoryBatch>, Wave3RepositoryError> {
        if req.approval_source.trim().is_empty() || req.approval_id.trim().is_empty() {
            return Err(Wave3RepositoryError::MissingApprovalSource);
        }
        let request_hash = request_hash(&serde_json::json!({
            "request": req,
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

        let batch_row = sqlx::query_as::<_, InventoryBatchRow>(
            r#"
            SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                   qty_on_hand, qty_locked, quality_status, location_id, location_code,
                   recall_flag, created_at, updated_at
              FROM inventory_batches
             WHERE id = $1 AND owner_id = $2
             FOR UPDATE
            "#,
        )
        .bind(req.batch_id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let from_status = batch_row.quality_status.clone();

        let batch = if from_status == req.target_status {
            map_inventory_batch(batch_row)
        } else {
            if !allowed_transition(&from_status, &req.target_status, &req.approval_source) {
                return Err(Wave3RepositoryError::InvalidStateTransition {
                    from: from_status,
                    to: req.target_status,
                    approval_source: req.approval_source,
                });
            }

            sqlx::query(
                r#"
                INSERT INTO inventory_status_changes (
                    id, owner_id, batch_id, from_status, to_status,
                    reason, approval_source, approval_id, occurred_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(req.batch_id)
            .bind(&from_status)
            .bind(&req.target_status)
            .bind(&req.reason)
            .bind(&req.approval_source)
            .bind(&req.approval_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;

            let updated = sqlx::query_as::<_, InventoryBatchRow>(
                r#"
                UPDATE inventory_batches
                   SET quality_status = $3,
                       updated_at = $4,
                       version = version + 1
                 WHERE id = $1 AND owner_id = $2
                RETURNING id, owner_id, product_code, batch_no, production_date, expiry_date,
                          qty_on_hand, qty_locked, quality_status, location_id, location_code,
                          recall_flag, created_at, updated_at
                "#,
            )
            .bind(req.batch_id)
            .bind(ctx.owner_id)
            .bind(&req.target_status)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;

            let batch = map_inventory_batch(updated);
            if let Some(audit) = audit {
                append_event_in_tx(&mut tx, &audit)
                    .await
                    .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
            }
            batch
        };

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inventory/batches/status",
            "inventory_batch",
            batch.id.to_string(),
            &batch,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: batch,
            replayed: false,
        })
    }

    pub async fn ingest_temperature_reading_with_audit(
        &self,
        ctx: &AuthContext,
        req: IngestTemperatureReadingRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<TemperatureReading>, Wave3RepositoryError> {
        if req.captured_at > now {
            return Err(Wave3RepositoryError::FutureTimestamp);
        }
        let request_hash = request_hash(&serde_json::json!({
            "request": req,
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
        ensure_cold_chain_device_active(&mut tx, ctx.owner_id, &req.device_code).await?;

        let existing =
            load_temperature_reading(&mut tx, ctx.owner_id, &req.device_code, req.captured_at)
                .await?;
        let (reading, inserted) = if let Some(existing) = existing {
            (existing, false)
        } else {
            let row = sqlx::query_as::<_, TemperatureReadingRow>(
                r#"
                INSERT INTO temperature_readings (
                    id, owner_id, device_code, temperature_celsius, humidity_percent,
                    captured_at, external_report_url, out_of_range, source_system,
                    external_reading_id, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'external_cold_chain', NULL, $9)
                RETURNING id, owner_id, device_code, temperature_celsius, humidity_percent,
                          captured_at, external_report_url, out_of_range
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(&req.device_code)
            .bind(req.temperature_celsius)
            .bind(req.humidity_percent)
            .bind(req.captured_at)
            .bind(&req.external_report_url)
            .bind(req.out_of_range)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            (map_temperature_reading(row), true)
        };

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/cold-chain/readings",
            "temperature_reading",
            reading.id.to_string(),
            &reading,
            now,
        )
        .await?;
        if inserted {
            let mut audit = audit.unwrap_or_else(|| {
                AuditWriteRequest::from_auth_context(
                    ctx,
                    "ingest_reading",
                    "M5",
                    "temperature_reading",
                    reading.id.to_string(),
                    None,
                )
            });
            audit.resource_id = reading.id.to_string();
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: reading,
            replayed: false,
        })
    }

    pub async fn ingest_temperature_excursion_with_audit(
        &self,
        ctx: &AuthContext,
        req: IngestTemperatureExcursionRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<TemperatureExcursionEvent>, Wave3RepositoryError> {
        if req.started_at > now {
            return Err(Wave3RepositoryError::FutureTimestamp);
        }
        let request_hash = request_hash(&serde_json::json!({
            "request": req,
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
        ensure_cold_chain_device_active(&mut tx, ctx.owner_id, &req.device_code).await?;

        let existing =
            load_temperature_excursion(&mut tx, ctx.owner_id, &req.external_event_id).await?;
        let (event, inserted) = if let Some(existing) = existing {
            (existing, false)
        } else {
            let row = sqlx::query_as::<_, TemperatureExcursionEventRow>(
                r#"
                INSERT INTO temperature_excursion_events (
                    id, owner_id, external_event_id, device_code, location_code,
                    started_at, ended_at, min_temperature_celsius,
                    max_temperature_celsius, affected_batch_ids, status, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'pending_disposition', $11)
                RETURNING id, owner_id, external_event_id, device_code, location_code,
                          started_at, ended_at, min_temperature_celsius,
                          max_temperature_celsius, affected_batch_ids, status, created_at
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(&req.external_event_id)
            .bind(&req.device_code)
            .bind(&req.location_code)
            .bind(req.started_at)
            .bind(req.ended_at)
            .bind(req.min_temperature_celsius)
            .bind(req.max_temperature_celsius)
            .bind(&req.affected_batch_ids)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            (map_temperature_excursion(row), true)
        };

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/cold-chain/excursions",
            "temperature_excursion",
            event.id.to_string(),
            &event,
            now,
        )
        .await?;
        if inserted {
            let mut audit = audit.unwrap_or_else(|| {
                AuditWriteRequest::from_auth_context(
                    ctx,
                    "ingest_excursion",
                    "M5",
                    "temperature_excursion",
                    event.id.to_string(),
                    None,
                )
            });
            audit.resource_id = event.id.to_string();
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: event,
            replayed: false,
        })
    }
}
