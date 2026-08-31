impl PgWave5Repository {
pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            packing: PackingStationService,
            retail: RetailChainService,
            tms: TmsPlusService,
        }
    }

    pub async fn create_packing_station(
        &self,
        ctx: &AuthContext,
        req: CreatePackingStationRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PackingStation>, Wave5RepositoryError> {
        self.packing.validate_station(&req)?;
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

        let id = Uuid::new_v4();
        let station = map_packing_station(
            sqlx::query_as::<_, PackingStationRow>(
                r#"
            INSERT INTO packing_stations (
                id, owner_id, station_code, station_name, printer_code, scale_code,
                temperature_zone, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'idle', $8, $8)
            RETURNING id, owner_id, station_code, station_name, printer_code, scale_code,
                      temperature_zone, status, created_at, updated_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(&req.station_code)
            .bind(&req.station_name)
            .bind(&req.printer_code)
            .bind(&req.scale_code)
            .bind(&req.temperature_zone)
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
            "/api/v1/packing/stations",
            "packing_station",
            station.id,
            &station,
            audit,
            "create_packing_station",
            "M-PK",
            "packing_station",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: station,
            replayed: false,
        })
    }

    pub async fn create_pack_job(
        &self,
        ctx: &AuthContext,
        req: CreatePackJobRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PackJob>, Wave5RepositoryError> {
        self.packing.validate_pack_job(&req)?;
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
        if let Some(station_id) = req.station_id {
            ensure_packing_station(&mut tx, ctx.owner_id, station_id).await?;
        }

        let id = Uuid::new_v4();
        let job = map_pack_job(
            sqlx::query_as::<_, PackJobRow>(
                r#"
            INSERT INTO packing_jobs (
                id, owner_id, outbound_order_id, station_id, job_no, pack_mode,
                recommended_box_type, actual_box_type, adjustment_reason,
                outbound_lpn, trace_codes, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'packed', $12, $12)
            RETURNING id, owner_id, outbound_order_id, station_id, job_no, pack_mode,
                      recommended_box_type, actual_box_type, adjustment_reason,
                      outbound_lpn, trace_codes, status, weight_grams, waybill_no,
                      created_at, updated_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(req.outbound_order_id)
            .bind(req.station_id)
            .bind(&req.job_no)
            .bind(&req.pack_mode)
            .bind(&req.recommended_box_type)
            .bind(&req.actual_box_type)
            .bind(&req.adjustment_reason)
            .bind(&req.outbound_lpn)
            .bind(&req.trace_codes)
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
            "/api/v1/packing/jobs",
            "packing_job",
            job.id,
            &job,
            audit,
            "create_pack_job",
            "M-PK",
            "packing_job",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: job,
            replayed: false,
        })
    }

    pub async fn weigh_pack_job(
        &self,
        ctx: &AuthContext,
        job_id: Uuid,
        req: WeighPackJobRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PackJob>, Wave5RepositoryError> {
        self.packing.validate_weight(&req)?;
        let request_hash = request_hash(&serde_json::json!({ "job_id": job_id, "request": req }))?;
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
        let job = map_pack_job(
            sqlx::query_as::<_, PackJobRow>(
                r#"
            UPDATE packing_jobs
               SET weight_grams = $3,
                   status = 'weighed',
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            RETURNING id, owner_id, outbound_order_id, station_id, job_no, pack_mode,
                      recommended_box_type, actual_box_type, adjustment_reason,
                      outbound_lpn, trace_codes, status, weight_grams, waybill_no,
                      created_at, updated_at
            "#,
            )
            .bind(ctx.owner_id)
            .bind(job_id)
            .bind(req.actual_weight_grams)
            .bind(now)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .ok_or(Wave5RepositoryError::NotFound)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/packing/jobs/{id}/weigh",
            "packing_job",
            job.id,
            &job,
            audit,
            "weigh_pack_job",
            "M-PK",
            "packing_job",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: job,
            replayed: false,
        })
    }

    pub async fn print_pack_job_waybill(
        &self,
        ctx: &AuthContext,
        job_id: Uuid,
        req: PrintWaybillRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PackJob>, Wave5RepositoryError> {
        self.packing.validate_waybill(&req)?;
        let request_hash = request_hash(&serde_json::json!({ "job_id": job_id, "request": req }))?;
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
        let waybill_no = req
            .waybill_no
            .unwrap_or_else(|| format!("{}-{}", req.carrier_code, job_id.simple()));
        let job = map_pack_job(
            sqlx::query_as::<_, PackJobRow>(
                r#"
            UPDATE packing_jobs
               SET waybill_no = $3,
                   status = 'waybill_printed',
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            RETURNING id, owner_id, outbound_order_id, station_id, job_no, pack_mode,
                      recommended_box_type, actual_box_type, adjustment_reason,
                      outbound_lpn, trace_codes, status, weight_grams, waybill_no,
                      created_at, updated_at
            "#,
            )
            .bind(ctx.owner_id)
            .bind(job_id)
            .bind(&waybill_no)
            .bind(now)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .ok_or(Wave5RepositoryError::NotFound)?,
        );
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/packing/jobs/{id}/waybill",
            "packing_job",
            job.id,
            &job,
            audit,
            "print_pack_job_waybill",
            "M-PK",
            "packing_job",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: job,
            replayed: false,
        })
    }

    pub async fn create_replenishment_suggestion(
        &self,
        ctx: &AuthContext,
        req: CreateRetailReplenishmentSuggestionRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<RetailReplenishmentSuggestion>, Wave5RepositoryError> {
        let suggested_qty = self.retail.suggested_qty(&req)?;
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
        let id = Uuid::new_v4();
        let suggestion = map_replenishment(sqlx::query_as::<_, RetailReplenishmentSuggestionRow>(
            r#"
            INSERT INTO retail_replenishment_suggestions (
                id, owner_id, store_id, product_code, period_key, min_qty, max_qty,
                current_qty, in_transit_qty, daily_sales_avg, suggested_qty, status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'pending_approval', $12)
            RETURNING id, owner_id, store_id, product_code, period_key, min_qty, max_qty,
                      current_qty, in_transit_qty, daily_sales_avg, suggested_qty, status, created_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(req.store_id)
        .bind(&req.product_code)
        .bind(&req.period_key)
        .bind(req.min_qty)
        .bind(req.max_qty)
        .bind(req.current_qty)
        .bind(req.in_transit_qty)
        .bind(req.daily_sales_avg)
        .bind(suggested_qty)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?);
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/retail/replenishment-suggestions",
            "retail_replenishment_suggestion",
            suggestion.id,
            &suggestion,
            audit,
            "create_replenishment_suggestion",
            "M8",
            "retail_replenishment_suggestion",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: suggestion,
            replayed: false,
        })
    }

    pub async fn create_crossdock_plan(
        &self,
        ctx: &AuthContext,
        req: CreateCrossdockPlanRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<CrossdockPlan>, Wave5RepositoryError> {
        self.retail.validate_crossdock(&req)?;
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
        let plan = map_crossdock_plan(
            sqlx::query_as::<_, CrossdockPlanRow>(
                r#"
            INSERT INTO crossdock_plans (
                id, owner_id, asn_id, outbound_order_id, store_id, product_code,
                qty, status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'planned', $8)
            RETURNING id, owner_id, asn_id, outbound_order_id, store_id, product_code,
                      qty, status, created_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(req.asn_id)
            .bind(req.outbound_order_id)
            .bind(req.store_id)
            .bind(&req.product_code)
            .bind(req.qty)
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
            "/api/v1/retail/crossdock-plans",
            "crossdock_plan",
            plan.id,
            &plan,
            audit,
            "create_crossdock_plan",
            "M8",
            "crossdock_plan",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: plan,
            replayed: false,
        })
    }

    pub async fn calculate_period_charges(
        &self,
        ctx: &AuthContext,
        req: wms_domain::CalculateBillingChargesRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<BillingChargeCalculation>, Wave5RepositoryError> {
        if req.quantity < wms_domain::Quantity::ZERO || req.period_start.is_empty() || req.period_end.is_empty() {
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
        let unit_price: wms_domain::Quantity = sqlx::query_scalar(
            r#"
            SELECT unit_price_cents
              FROM billing_rules
             WHERE owner_id = $1
               AND contract_id = $2
               AND charge_item = $3
               AND effective_from <= $4
               AND effective_to >= $5
             ORDER BY created_at DESC
             LIMIT 1
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.contract_id)
        .bind(&req.charge_item)
        .bind(period_end)
        .bind(period_start)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave5RepositoryError::NotFound)?;
        let amount_cents = unit_price
            .checked_mul(req.quantity)
            .ok_or(Wave5RepositoryError::InvalidInput)?;
        let id = Uuid::new_v4();
        let charge = map_charge(
            sqlx::query_as::<_, BillingChargeCalculationRow>(
                r#"
            INSERT INTO billing_charge_calculations (
                id, owner_id, contract_id, period_start, period_end, charge_item,
                quantity, amount_cents, source_refs, status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'calculated', $10)
            RETURNING id, owner_id, contract_id, period_start, period_end, charge_item,
                      quantity, amount_cents, source_refs, status, created_at
            "#,
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(req.contract_id)
            .bind(period_start)
            .bind(period_end)
            .bind(&req.charge_item)
            .bind(req.quantity)
            .bind(amount_cents)
            .bind(&req.source_refs)
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
            "/api/v1/billing/charges/calculate",
            "billing_charge_calculation",
            charge.id,
            &charge,
            audit,
            "calculate_period_charges",
            "M9",
            "billing_charge_calculation",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: charge,
            replayed: false,
        })
    }

    pub async fn get_tote_status(
        &self,
        ctx: &AuthContext,
        tote_code: &str,
    ) -> Result<wms_domain::ToteStatusResponse, Wave5RepositoryError> {
        let (tote_id, canonical_code, raw_status): (Uuid, String, String) =
            sqlx::query_as(
                r#"
                SELECT id, lpn_code, status
                  FROM lpn_containers
                 WHERE owner_id = $1
                   AND lower(lpn_code) = lower($2)
                   AND container_type = 'tote'
                 LIMIT 1
                "#,
            )
            .bind(ctx.owner_id)
            .bind(tote_code.trim())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
            .ok_or(Wave5RepositoryError::NotFound)?;

        let status = match raw_status.as_str() {
            "idle" => "AVAILABLE",
            "in_use" => "IN_USE",
            "in_transit" | "shipped" => "SEALED",
            "disabled" => "DISABLED",
            _ => "IN_USE",
        }
        .to_string();

        let current_order_id = if raw_status == "in_use" {
            sqlx::query_scalar(
                r#"
                SELECT outbound_order_id
                  FROM outbound_pick_tote_bindings
                 WHERE owner_id = $1
                   AND tote_id = $2
                   AND status = 'active'
                 LIMIT 1
                "#,
            )
            .bind(ctx.owner_id)
            .bind(tote_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
        } else {
            None
        };

        let loaded_sku_count = if let Some(order_id) = current_order_id {
            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(DISTINCT product_code)::BIGINT
                  FROM outbound_order_lines
                 WHERE owner_id = $1
                   AND outbound_order_id = $2
                   AND picked_qty > 0
                "#,
            )
            .bind(ctx.owner_id)
            .bind(order_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_error)?;
            u32::try_from(count).map_err(|_| Wave5RepositoryError::InvalidInput)?
        } else {
            0
        };

        Ok(wms_domain::ToteStatusResponse {
            tote_code: canonical_code,
            status,
            current_order_id,
            loaded_sku_count,
        })
    }
}
