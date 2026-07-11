async fn putaway_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<PutawayRequest>,
) -> Result<Json<PutawayRecord>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "putaway",
            "M2",
            "receiving_order",
            id.to_string(),
            None,
        );
        let outcome = repository
            .putaway_receiving_order_and_inventory_with_audit(
                &ctx,
                id,
                req,
                now,
                &idempotency_key,
                Some(audit),
            )
            .await?;
        return Ok(Json(outcome.value.putaway));
    }
    let putaway = {
        let mut store = state.inbound_store.lock().await;
        store.putaway(&ctx, id, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "putaway",
        "M2",
        "receiving_order",
        id.to_string(),
    )
    .await;
    Ok(Json(putaway))
}
async fn list_inventory_batches_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
) -> Result<Json<InventoryBatchListResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.read", "m3.write"])?;
    if let Some(config_center_state) = &state.config_center_state {
        if !config_center_state
            .is_feature_enabled(INVENTORY_BATCHES_SMOKE_FLAG)
            .await?
        {
            return Err(
                ConfigCenterError::DisabledFlag(INVENTORY_BATCHES_SMOKE_FLAG.to_string()).into(),
            );
        }
    }
    if let Some(repository) = &state.wave3_repository {
        let batches = repository.list_inventory_batches(&ctx).await?;
        let count = batches.len() as u32;
        return Ok(Json(InventoryBatchListResponse {
            data: batches,
            page: PageMeta {
                next_cursor: None,
                count,
            },
        }));
    }
    let batches = {
        let store = state.inventory_store.lock().await;
        store.list_batches(&ctx)
    };
    let count = batches.len() as u32;
    Ok(Json(InventoryBatchListResponse {
        data: batches,
        page: PageMeta {
            next_cursor: None,
            count,
        },
    }))
}

async fn putaway_inventory_batch_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<PutawayInventoryRequest>,
) -> Result<Json<InventoryBatch>, Wave3HandlerError> {
    ctx.require_permission("m3.write")?;
    let now = Utc::now();
    let source_id = req.source_receiving_order_id;
    let batch = {
        let mut store = state.inventory_store.lock().await;
        store.putaway_from_inbound(&ctx, req, now.date_naive(), now)?
    };
    append_audit(
        &state,
        &ctx,
        "putaway",
        "M3",
        "inventory_batch",
        source_id.to_string(),
    )
    .await;
    Ok(Json(batch))
}

async fn change_inventory_batch_status_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<ChangeInventoryStatusRequest>,
) -> Result<Json<InventoryBatch>, Wave3HandlerError> {
    ctx.require_permission("m3.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let batch_id = req.batch_id;
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "change_status",
            "M3",
            "inventory_batch",
            batch_id.to_string(),
            None,
        );
        let outcome = repository
            .change_inventory_status_with_audit(&ctx, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let batch = {
        let mut store = state.inventory_store.lock().await;
        store.change_status(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "change_status",
        "M3",
        "inventory_batch",
        batch_id.to_string(),
    )
    .await;
    Ok(Json(batch))
}

async fn create_cold_chain_device_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<CreateColdChainDeviceRequest>,
) -> Result<Json<ColdChainDevice>, Wave3HandlerError> {
    ctx.require_permission("m5.write")?;
    let now = Utc::now();
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
    Json(req): Json<CreateBillingContractRequest>,
) -> Result<Json<BillingContract>, Wave3HandlerError> {
    ctx.require_permission("m9.write")?;
    let now = Utc::now();
    let contract = {
        let mut store = state.billing_store.lock().await;
        store.create_contract(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "create_contract",
        "M9",
        "billing_contract",
        contract.id.to_string(),
    )
    .await;
    Ok(Json(contract))
}

async fn create_billing_rule_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<CreateBillingRuleRequest>,
) -> Result<Json<BillingRule>, Wave3HandlerError> {
    ctx.require_permission("m9.write")?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "create_rule",
            "M9",
            "billing_rule",
            "",
            None,
        );
        let rule = repository
            .create_billing_rule_with_audit(&ctx, req, now, audit)
            .await?;
        return Ok(Json(rule));
    }
    let rule = {
        let mut store = state.billing_store.lock().await;
        store.create_rule(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "create_rule",
        "M9",
        "billing_rule",
        rule.id.to_string(),
    )
    .await;
    Ok(Json(rule))
}

fn require_any_permission(ctx: &AuthContext, permissions: &[&str]) -> Result<(), AuthError> {
    if permissions
        .iter()
        .any(|permission| ctx.has_permission(permission))
    {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied(permissions.join("|")))
    }
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, Wave3HandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(Wave3HandlerError::MissingIdempotencyKey)
}

fn cold_chain_external_context(
    state: &Wave3AppState,
    headers: &HeaderMap,
) -> Result<(AuthContext, String), Wave3HandlerError> {
    let idempotency_key = idempotency_key_from_headers(headers)?;
    let config = state
        .cold_chain_api_key
        .as_ref()
        .ok_or(Wave3HandlerError::ExternalAuthConfigMissing)?;
    let configured_hash = config.key_sha256.trim();
    if configured_hash.len() != 64
        || !configured_hash
            .chars()
            .all(|value| value.is_ascii_hexdigit())
    {
        return Err(Wave3HandlerError::ExternalAuthConfigInvalid);
    }

    let api_key = headers
        .get(EXTERNAL_API_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(Wave3HandlerError::ExternalAuthMissing)?;
    let provided_hash = sha256_hex(api_key.as_bytes());
    if !constant_time_eq(
        provided_hash.as_bytes(),
        configured_hash.to_ascii_lowercase().as_bytes(),
    ) {
        return Err(Wave3HandlerError::ExternalAuthInvalid);
    }

    Ok((
        AuthContext {
            user_id: Uuid::nil(),
            owner_id: config.owner_id,
            actor_name: config.actor_name.clone(),
            permissions: vec!["m5.write".to_string()],
            jti: format!("m5-cold-chain:{idempotency_key}"),
        },
        idempotency_key,
    ))
}

fn sha256_hex(value: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    hex::encode(hasher.finalize())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut diff = left.len() ^ right.len();
    let max_len = left.len().max(right.len());
    for index in 0..max_len {
        let left_byte = left.get(index).copied().unwrap_or_default();
        let right_byte = right.get(index).copied().unwrap_or_default();
        diff |= (left_byte ^ right_byte) as usize;
    }
    diff == 0
}

async fn append_audit(
    state: &Wave3AppState,
    ctx: &AuthContext,
    action: &'static str,
    module: &'static str,
    resource_type: &'static str,
    resource_id: String,
) {
    append_audit_with_diff(state, ctx, action, module, resource_type, resource_id, None).await;
}

async fn append_audit_with_diff(
    state: &Wave3AppState,
    ctx: &AuthContext,
    action: &'static str,
    module: &'static str,
    resource_type: &'static str,
    resource_id: String,
    diff: Option<AuditDiff>,
) {
    let mut audit_log = state.audit_log.lock().await;
    audit_log.append_event(AuditWriteRequest::from_auth_context(
        ctx,
        action,
        module,
        resource_type,
        resource_id,
        diff,
    ));
}
