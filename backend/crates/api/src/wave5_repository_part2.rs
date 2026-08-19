impl PgWave5Repository {
    pub async fn receive_tms_route_plan(
        &self,
        ctx: &AuthContext,
        req: ReceiveTmsRoutePlanRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<TmsRoutePlan>, Wave5RepositoryError> {
        self.tms.validate_route_plan(&req)?;
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<TmsRoutePlan>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let existing = sqlx::query_as::<_, TmsRoutePlanRow>(
            r#"
            SELECT id, owner_id, dispatch_result_id, delivery_date, vehicle_no, plate_no,
                   driver_user_id, status, planning_version AS version, payload_hash,
                   created_at, updated_at
              FROM tms_route_plans
             WHERE owner_id = $1 AND dispatch_result_id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(&req.dispatch_result_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if let Some(row) = existing {
            if row.payload_hash != request_hash {
                return Err(Wave5RepositoryError::IdempotencyConflict);
            }
            let route_plan = load_tms_route_plan(&mut tx, row).await?;
            store_idempotency_success(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &request_hash,
                "POST",
                "/api/v1/tms/route-plans",
                "tms_route_plan",
                route_plan.id.to_string(),
                &route_plan,
                now,
            )
            .await?;
            tx.commit().await.map_err(map_db_error)?;
            return Ok(IdempotentMutation {
                value: route_plan,
                replayed: true,
            });
        }

        ensure_route_orders(&mut tx, ctx.owner_id, &req.outbound_order_ids).await?;
        ensure_route_driver(&mut tx, ctx.owner_id, req.driver_user_id).await?;
        let route_plan_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO tms_route_plans (
                id, owner_id, dispatch_result_id, delivery_date, vehicle_no, plate_no,
                driver_user_id, status, planning_version, payload_hash, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'received', $8, $9, $10, $10)
            "#,
        )
        .bind(route_plan_id)
        .bind(ctx.owner_id)
        .bind(&req.dispatch_result_id)
        .bind(req.delivery_date)
        .bind(&req.vehicle_no)
        .bind(&req.plate_no)
        .bind(req.driver_user_id)
        .bind(req.version)
        .bind(&request_hash)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let mut stops = Vec::with_capacity(req.stops.len());
        for stop in req.stops {
            let stop_id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO tms_route_stops (
                    id, owner_id, route_plan_id, store_id, stop_sequence,
                    estimated_arrival_at, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(stop_id)
            .bind(ctx.owner_id)
            .bind(route_plan_id)
            .bind(stop.store_id)
            .bind(stop.sequence)
            .bind(stop.estimated_arrival_at)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            for order_id in &stop.outbound_order_ids {
                sqlx::query(
                    r#"
                    INSERT INTO tms_route_orders (
                        id, owner_id, route_plan_id, route_stop_id,
                        outbound_order_id, created_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6)
                    "#,
                )
                .bind(Uuid::new_v4())
                .bind(ctx.owner_id)
                .bind(route_plan_id)
                .bind(stop_id)
                .bind(order_id)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
            }
            stops.push(TmsRouteStop {
                id: stop_id,
                store_id: stop.store_id,
                sequence: stop.sequence,
                estimated_arrival_at: stop.estimated_arrival_at,
                outbound_order_ids: stop.outbound_order_ids,
            });
        }
        let route_plan = TmsRoutePlan {
            id: route_plan_id,
            owner_id: ctx.owner_id,
            dispatch_result_id: req.dispatch_result_id,
            delivery_date: req.delivery_date,
            vehicle_no: req.vehicle_no,
            plate_no: req.plate_no,
            driver_user_id: req.driver_user_id,
            status: "received".to_string(),
            version: req.version,
            outbound_order_ids: req.outbound_order_ids,
            stops,
            created_at: now,
            updated_at: now,
        };
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/tms/route-plans",
            "tms_route_plan",
            route_plan.id,
            &route_plan,
            audit,
            "receive_tms_route_plan",
            "M10",
            "tms_route_plan",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: route_plan,
            replayed: false,
        })
    }

    pub async fn generate_billing_statement(
        &self,
        ctx: &AuthContext,
        req: GenerateBillingStatementRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<BillingStatement>, Wave5RepositoryError> {
        if req.charge_ids.is_empty()
            || has_duplicate_uuids(&req.charge_ids)
            || req.period_start.is_empty()
            || req.period_end.is_empty()
        {
            return Err(Wave5RepositoryError::InvalidInput);
        }
        let period_start = parse_billing_date(&req.period_start)?;
        let period_end = parse_billing_date(&req.period_end)?;
        if period_end < period_start {
            return Err(Wave5RepositoryError::InvalidInput);
        }
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
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
        let (selected_count, period_count, total): (i64, i64, Option<wms_domain::Quantity>) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*)::BIGINT,
                COUNT(*) FILTER (WHERE period_start = $4 AND period_end = $5)::BIGINT,
                SUM(amount_cents) FILTER (WHERE period_start = $4 AND period_end = $5)
              FROM billing_charge_calculations
             WHERE owner_id = $1 AND contract_id = $2 AND id = ANY($3)
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.contract_id)
        .bind(&req.charge_ids)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if usize::try_from(selected_count).ok() != Some(req.charge_ids.len()) {
            return Err(Wave5RepositoryError::NotFound);
        }
        if usize::try_from(period_count).ok() != Some(req.charge_ids.len()) {
            return Err(Wave5RepositoryError::InvalidInput);
        }
        let id = Uuid::new_v4();
        let statement = map_statement(
            sqlx::query_as::<_, BillingStatementRow>(
                r#"
                INSERT INTO billing_statements (
                    id, owner_id, contract_id, period_start, period_end, status,
                    total_amount_cents, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, 'pending_confirmation', $6, $7, $7)
                RETURNING id, owner_id, contract_id, period_start, period_end, status,
                          total_amount_cents, created_at, updated_at
                "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(req.contract_id)
            .bind(period_start)
            .bind(period_end)
            .bind(total.unwrap_or(wms_domain::Quantity::ZERO))
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
            req.charge_ids.clone(),
        );
        for charge_id in &req.charge_ids {
            sqlx::query(
                r#"
                INSERT INTO billing_statement_charges (id, owner_id, statement_id, charge_id, created_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(statement.id)
            .bind(charge_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/billing/statements",
            "billing_statement",
            statement.id,
            &statement,
            audit,
            "generate_billing_statement",
            "M9",
            "billing_statement",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: statement,
            replayed: false,
        })
    }

    pub async fn confirm_billing_statement(
        &self,
        ctx: &AuthContext,
        statement_id: Uuid,
        req: ConfirmBillingStatementRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<BillingStatement>, Wave5RepositoryError> {
        let request_hash =
            request_hash(&serde_json::json!({ "statement_id": statement_id, "request": req }))?;
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
        let current = sqlx::query_as::<_, BillingStatementRow>(
            r#"
            SELECT id, owner_id, contract_id, period_start, period_end, status,
                   total_amount_cents, created_at, updated_at
              FROM billing_statements
             WHERE owner_id = $1 AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(statement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave5RepositoryError::NotFound)?;
        let charge_ids = load_statement_charge_ids(&mut tx, ctx.owner_id, statement_id).await?;
        if current.status == "confirmed" {
            let statement = map_statement(current, charge_ids);
            store_idempotency_success(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &request_hash,
                "POST",
                "/api/v1/billing/statements/{id}/confirm",
                "billing_statement",
                statement.id.to_string(),
                &statement,
                now,
            )
            .await?;
            tx.commit().await.map_err(map_db_error)?;
            return Ok(IdempotentMutation {
                value: statement,
                replayed: false,
            });
        }
        if current.status != "pending_confirmation" {
            return Err(Wave5RepositoryError::InvalidInput);
        }
        let row = sqlx::query_as::<_, BillingStatementRow>(
            r#"
            UPDATE billing_statements
               SET status = 'confirmed',
                   updated_at = $3,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            RETURNING id, owner_id, contract_id, period_start, period_end, status,
                      total_amount_cents, created_at, updated_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(statement_id)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave5RepositoryError::NotFound)?;
        let statement = map_statement(row, charge_ids);
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/billing/statements/{id}/confirm",
            "billing_statement",
            statement.id,
            &statement,
            audit,
            "confirm_billing_statement",
            "M9",
            "billing_statement",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: statement,
            replayed: false,
        })
    }

    pub async fn receive_tms_dispatch(
        &self,
        ctx: &AuthContext,
        req: ReceiveTmsDispatchRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<TmsDispatch>, Wave5RepositoryError> {
        self.tms.validate_dispatch(&req)?;
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
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
        ensure_outbound_order(&mut tx, ctx.owner_id, req.outbound_order_id).await?;
        let id = Uuid::new_v4();
        let dispatch = map_tms_dispatch(
            sqlx::query_as::<_, TmsDispatchRow>(
                r#"
            INSERT INTO tms_dispatches (
                id, owner_id, dispatch_no, outbound_order_id, delivery_provider_type,
                vehicle_no, plate_no, driver_user_id, carrier_code, waybill_no,
                status, dispatch_version, scheduled_load_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'received', $11, $12, $13, $13)
            RETURNING id, owner_id, dispatch_no, outbound_order_id, delivery_provider_type,
                      vehicle_no, plate_no, driver_user_id, carrier_code, waybill_no,
                      status, dispatch_version AS version, scheduled_load_at, created_at, updated_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(&req.dispatch_no)
            .bind(req.outbound_order_id)
            .bind(&req.delivery_provider_type)
            .bind(&req.vehicle_no)
            .bind(&req.plate_no)
            .bind(req.driver_user_id)
            .bind(&req.carrier_code)
            .bind(&req.waybill_no)
            .bind(req.version)
            .bind(req.scheduled_load_at)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/tms/dispatches",
            "tms_dispatch",
            dispatch.id,
            &dispatch,
            audit,
            "receive_tms_dispatch",
            "M10",
            "tms_dispatch",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: dispatch,
            replayed: false,
        })
    }

    pub async fn ingest_transit_temperature(
        &self,
        ctx: &AuthContext,
        req: IngestTransitTemperatureRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<TransitTemperatureReading>, Wave5RepositoryError> {
        self.tms.validate_temperature(&req, now)?;
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
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
        ensure_dispatch(&mut tx, ctx.owner_id, req.dispatch_id).await?;
        let id = Uuid::new_v4();
        let reading = map_transit_temperature(
            sqlx::query_as::<_, TransitTemperatureReadingRow>(
                r#"
            INSERT INTO transit_temperature_readings (
                id, owner_id, dispatch_id, device_code, plate_no, measured_at,
                temperature_celsius, humidity_percent, is_exceeded, external_trace_url, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, owner_id, dispatch_id, device_code, plate_no, measured_at,
                      temperature_celsius, humidity_percent, is_exceeded, external_trace_url,
                      created_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(req.dispatch_id)
            .bind(&req.device_code)
            .bind(&req.plate_no)
            .bind(req.measured_at)
            .bind(req.temperature_celsius)
            .bind(req.humidity_percent)
            .bind(req.is_exceeded)
            .bind(&req.external_trace_url)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/tms/transit-temperature-readings",
            "transit_temperature_reading",
            reading.id,
            &reading,
            audit,
            "ingest_transit_temperature",
            "M10",
            "transit_temperature_reading",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: reading,
            replayed: false,
        })
    }

    pub async fn confirm_container_recovery(
        &self,
        ctx: &AuthContext,
        req: ConfirmContainerRecoveryRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<ContainerRecovery>, Wave5RepositoryError> {
        self.tms.validate_recovery(&req)?;
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
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
        if let Some(dispatch_id) = req.dispatch_id {
            ensure_dispatch(&mut tx, ctx.owner_id, dispatch_id).await?;
        }
        let shipped_at = req.shipped_at.unwrap_or(now);
        let id = Uuid::new_v4();
        let recovery = map_container_recovery(
            sqlx::query_as::<_, ContainerRecoveryRow>(
                r#"
            INSERT INTO container_recoveries (
                id, owner_id, container_lpn, dispatch_id, customer_id,
                delivery_provider_type, status, shipped_at, recovered_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'recovered', $7, $8, $8, $8)
            RETURNING id, owner_id, container_lpn, dispatch_id, customer_id,
                      delivery_provider_type, status, shipped_at, recovered_at,
                      created_at, updated_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(&req.container_lpn)
            .bind(req.dispatch_id)
            .bind(req.customer_id)
            .bind(&req.delivery_provider_type)
            .bind(shipped_at)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/tms/container-recoveries",
            "container_recovery",
            recovery.id,
            &recovery,
            audit,
            "confirm_container_recovery",
            "M10",
            "container_recovery",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: recovery,
            replayed: false,
        })
    }
}
