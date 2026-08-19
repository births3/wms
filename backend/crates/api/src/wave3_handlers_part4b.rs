async fn create_cold_chain_device_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateColdChainDeviceRequest>,
) -> Result<Json<ColdChainDevice>, Wave3HandlerError> {
    ctx.require_permission("m5.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "create_device",
            "M5",
            "cold_chain_device",
            "",
            None,
        );
        let outcome = repository
            .create_cold_chain_device_with_audit(
                &ctx,
                req,
                now,
                &idempotency_key,
                audit,
            )
            .await?;
        return Ok(Json(outcome.value));
    }
    let device = {
        let mut service = state.cold_chain_service.lock().await;
        service.create_device(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "create_device",
        "M5",
        "cold_chain_device",
        device.id.to_string(),
    )
    .await;
    Ok(Json(device))
}

async fn list_cold_chain_devices_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
) -> Result<Json<Vec<ColdChainDevice>>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m5.read", "m5.write"])?;
    if let Some(repository) = &state.wave3_repository {
        return Ok(Json(repository.list_cold_chain_devices(&ctx).await?));
    }
    Ok(Json(state.cold_chain_service.lock().await.list_devices(&ctx)))
}

async fn update_cold_chain_device_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(device_code): Path<String>,
    headers: HeaderMap,
    Json(req): Json<UpdateColdChainDeviceRequest>,
) -> Result<Json<ColdChainDevice>, Wave3HandlerError> {
    ctx.require_permission("m5.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "update_device",
            "M5",
            "cold_chain_device",
            "",
            None,
        );
        return Ok(Json(
            repository
                .update_cold_chain_device_with_audit(
                    &ctx,
                    &device_code,
                    req,
                    now,
                    &idempotency_key,
                    audit,
                )
                .await?
                .value,
        ));
    }
    let device = state
        .cold_chain_service
        .lock()
        .await
        .update_device(&ctx, &device_code, req)?;
    append_audit(
        &state,
        &ctx,
        "update_device",
        "M5",
        "cold_chain_device",
        device.id.to_string(),
    )
    .await;
    Ok(Json(device))
}

async fn disable_cold_chain_device_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(device_code): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ColdChainDevice>, Wave3HandlerError> {
    ctx.require_permission("m5.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "disable_device",
            "M5",
            "cold_chain_device",
            "",
            None,
        );
        return Ok(Json(
            repository
                .disable_cold_chain_device_with_audit(
                    &ctx,
                    &device_code,
                    now,
                    &idempotency_key,
                    audit,
                )
                .await?
                .value,
        ));
    }
    let device = state
        .cold_chain_service
        .lock()
        .await
        .disable_device(&ctx, &device_code)?;
    append_audit(
        &state,
        &ctx,
        "disable_device",
        "M5",
        "cold_chain_device",
        device.id.to_string(),
    )
    .await;
    Ok(Json(device))
}

async fn ingest_temperature_reading_handler(
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<IngestTemperatureReadingRequest>,
) -> Result<Json<TemperatureReading>, Wave3HandlerError> {
    let (ctx, idempotency_key) = cold_chain_external_context(&state, &headers)?;
    ctx.require_permission("m5.write")?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "ingest_reading",
            "M5",
            "temperature_reading",
            "",
            None,
        );
        let outcome = repository
            .ingest_temperature_reading_with_audit(&ctx, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let reading = {
        let mut service = state.cold_chain_service.lock().await;
        service.ingest_reading(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "ingest_reading",
        "M5",
        "temperature_reading",
        reading.id.to_string(),
    )
    .await;
    Ok(Json(reading))
}

async fn ingest_temperature_excursion_handler(
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<IngestTemperatureExcursionRequest>,
) -> Result<Json<TemperatureExcursionEvent>, Wave3HandlerError> {
    let (ctx, idempotency_key) = cold_chain_external_context(&state, &headers)?;
    ctx.require_permission("m5.write")?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "ingest_excursion",
            "M5",
            "temperature_excursion",
            "",
            None,
        );
        let outcome = repository
            .ingest_temperature_excursion_with_audit(&ctx, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let event = {
        let mut service = state.cold_chain_service.lock().await;
        service.ingest_excursion(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "ingest_excursion",
        "M5",
        "temperature_excursion",
        event.id.to_string(),
    )
    .await;
    Ok(Json(event))
}

async fn create_billing_account_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<CreateBillingAccountRequest>,
) -> Result<Json<BillingAccount>, Wave3HandlerError> {
    ctx.require_permission("m9.write")?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "create_account",
            "M9",
            "billing_account",
            "",
            None,
        );
        let account = repository
            .create_billing_account_with_audit(&ctx, req, now, audit)
            .await?;
        return Ok(Json(account));
    }
    let account = {
        let mut store = state.billing_store.lock().await;
        store.create_account(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "create_account",
        "M9",
        "billing_account",
        account.id.to_string(),
    )
    .await;
    Ok(Json(account))
}

async fn create_billing_contract_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateBillingContractRequest>,
) -> Result<Json<BillingContract>, Wave3HandlerError> {
    ctx.require_permission("m9.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let Some(repository) = &state.wave3_repository else {
        return Err(Wave3HandlerError::Repository(Wave3RepositoryError::Database(
            "M9 合同创建需要 PostgreSQL repository 才能保证幂等".to_string(),
        )));
    };
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create_contract",
        "M9",
        "billing_contract",
        "",
        None,
    );
    let outcome = repository
        .create_billing_contract_with_audit(&ctx, req, now, &idempotency_key, audit)
        .await?;
    Ok(Json(outcome.value))
}

async fn create_billing_rule_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateBillingRuleRequest>,
) -> Result<Json<BillingRule>, Wave3HandlerError> {
    ctx.require_permission("m9.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let Some(repository) = &state.wave3_repository else {
        return Err(Wave3HandlerError::Repository(Wave3RepositoryError::Database(
            "M9 规则创建需要 PostgreSQL repository 才能保证幂等".to_string(),
        )));
    };
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create_rule",
        "M9",
        "billing_rule",
        "",
        None,
    );
    let outcome = repository
        .create_billing_rule_with_audit(&ctx, req, now, &idempotency_key, audit)
        .await?;
    Ok(Json(outcome.value))
}
