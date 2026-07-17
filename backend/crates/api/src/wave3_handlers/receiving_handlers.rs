use super::*;

pub(super) fn apply_receiving_order_routes() -> Router<Wave3AppState> {
    Router::new()
        .route(
            "/api/v1/inbound/receiving-orders",
            get(super::list_receiving_orders_handler).post(super::create_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-dashboard",
            get(super::list_receiving_dashboard_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id",
            get(get_receiving_order_handler)
                .patch(update_receiving_order_handler)
                .delete(delete_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/print-data",
            get(get_receiving_order_print_data_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/release",
            post(release_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/receive",
            post(receive_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/reject",
            post(reject_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/inspect",
            post(super::inspect_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/sign",
            post(super::sign_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/putaway",
            post(super::putaway_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/cancel",
            post(cancel_receiving_order_handler),
        )
        .route(
            "/api/v1/inbound/receiving-orders/:id/force-close-shortage",
            post(force_close_shortage_handler),
        )
        .route(
            "/api/v1/inbound/putaway-strategy-profiles",
            get(list_putaway_strategy_profiles_handler)
                .put(upsert_putaway_strategy_profile_handler),
        )
}

pub(super) async fn get_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ReceivingOrder>, Wave3HandlerError> {
    let order = if let Some(repository) = &state.wave3_repository {
        repository.get_receiving_order(&ctx, id).await?
    } else {
        let store = state.inbound_store.lock().await;
        store.get(&ctx, id)?
    };
    Ok(Json(order))
}

pub(super) async fn get_receiving_order_print_data_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ReceivingOrderPrintData>, Wave3HandlerError> {
    let data = if let Some(repository) = &state.wave3_repository {
        repository.get_receiving_order_print_data(&ctx, id).await?
    } else {
        let store = state.inbound_store.lock().await;
        store.get_print_data(&ctx, id)?
    };
    Ok(Json(data))
}

pub(super) async fn delete_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ReceivingOrder>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let order = if let Some(repository) = &state.wave3_repository {
        repository
            .delete_receiving_order(&ctx, id, Utc::now())
            .await?
    } else {
        let mut store = state.inbound_store.lock().await;
        let order = store.delete(&ctx, id)?;
        append_audit(
            &state,
            &ctx,
            "delete",
            "M2",
            "receiving_order",
            id.to_string(),
        )
        .await;
        order
    };
    Ok(Json(order))
}

pub(super) async fn update_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpdateReceivingOrderRequest>,
) -> Result<Json<ReceivingOrder>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let (order, audit_diff) = if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "update",
            "M2",
            "receiving_order",
            id.to_string(),
            None,
        );
        (
            repository
                .update_receiving_order(&ctx, id, req, now, &idempotency_key, audit)
                .await?,
            None,
        )
    } else {
        let mut store = state.inbound_store.lock().await;
        let before = store.get(&ctx, id)?;
        let after = store.update(&ctx, id, req, now)?;
        let diff = AuditDiff::compute(serde_json::json!(before), serde_json::json!(after));
        (after, Some(diff))
    };
    if let Some(diff) = audit_diff {
        append_audit_with_diff(
            &state,
            &ctx,
            "update",
            "M2",
            "receiving_order",
            id.to_string(),
            Some(diff),
        )
        .await;
    }
    Ok(Json(order))
}

pub(super) async fn cancel_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<wms_domain::CancelReceivingOrderRequest>,
) -> Result<Json<ReceivingOrder>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let repository = state.wave3_repository.as_ref().ok_or_else(|| {
        Wave3HandlerError::Repository(crate::wave3_repository::Wave3RepositoryError::Database(
            "ASN 作废需要 PostgreSQL repository".to_string(),
        ))
    })?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "cancel",
        "M2",
        "receiving_order",
        id.to_string(),
        None,
    );
    let result = repository
        .cancel_receiving_order_with_audit(&ctx, id, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(result.value))
}

pub(super) async fn force_close_shortage_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<wms_domain::ForceCloseShortageRequest>,
) -> Result<Json<ReceivingOrder>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let repository = state.wave3_repository.as_ref().ok_or_else(|| {
        Wave3HandlerError::Repository(crate::wave3_repository::Wave3RepositoryError::Database(
            "短少强制关闭需要 PostgreSQL repository".to_string(),
        ))
    })?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "force_close_shortage",
        "M2",
        "receiving_order",
        id.to_string(),
        None,
    );
    let result = repository
        .force_close_shortage_with_audit(&ctx, id, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(result.value))
}

pub(super) async fn list_putaway_strategy_profiles_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
) -> Result<Json<wms_domain::PutawayStrategyProfileListResponse>, Wave3HandlerError> {
    ctx.require_permission("m2.putaway.write")?;
    let repository = state.wave3_repository.as_ref().ok_or_else(|| {
        Wave3HandlerError::Repository(crate::wave3_repository::Wave3RepositoryError::Database(
            "上架策略方案需要 PostgreSQL repository".to_string(),
        ))
    })?;
    Ok(Json(repository.list_putaway_strategy_profiles(&ctx).await?))
}

pub(super) async fn upsert_putaway_strategy_profile_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<wms_domain::UpsertPutawayStrategyProfileRequest>,
) -> Result<Json<wms_domain::PutawayStrategyProfile>, Wave3HandlerError> {
    ctx.require_permission("m2.putaway.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let repository = state.wave3_repository.as_ref().ok_or_else(|| {
        Wave3HandlerError::Repository(crate::wave3_repository::Wave3RepositoryError::Database(
            "上架策略方案需要 PostgreSQL repository".to_string(),
        ))
    })?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "upsert",
        "M2",
        "putaway_strategy_profile",
        "pending".to_string(),
        None,
    );
    let result = repository
        .upsert_putaway_strategy_profile_with_audit(&ctx, req, now, &idempotency_key, audit)
        .await?;
    Ok(Json(result.value))
}

pub(super) async fn release_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ReceivingOrder>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "release",
            "M2",
            "receiving_order",
            id.to_string(),
            None,
        );
        let order = repository
            .release_receiving_order_with_audit(&ctx, id, now, Some(&idempotency_key), Some(audit))
            .await?;
        return Ok(Json(order));
    }
    let order = {
        let mut store = state.inbound_store.lock().await;
        store.release(&ctx, id, now)?
    };
    append_audit(
        &state,
        &ctx,
        "release",
        "M2",
        "receiving_order",
        id.to_string(),
    )
    .await;
    Ok(Json(order))
}

pub(super) async fn receive_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ReceiveReceivingOrderRequest>,
) -> Result<Json<ReceivingOrderReceipt>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "receive",
            "M2",
            "receiving_order",
            id.to_string(),
            None,
        );
        let outcome = repository
            .receive_receiving_order_with_audit(&ctx, id, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let receipt = {
        let mut store = state.inbound_store.lock().await;
        store.receive(&ctx, id, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "receive",
        "M2",
        "receiving_order",
        id.to_string(),
    )
    .await;
    Ok(Json(receipt))
}

pub(super) async fn reject_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<RejectReceivingOrderRequest>,
) -> Result<Json<ReceivingOrderReceipt>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let reason = req.reason.trim().to_string();
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "reject",
            "M2",
            "receiving_order",
            id.to_string(),
            None,
        );
        let outcome = repository
            .reject_receiving_order_with_audit(&ctx, id, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let receipt = {
        let mut store = state.inbound_store.lock().await;
        store.reject(&ctx, id, req, now)?
    };
    append_audit_with_diff(
        &state,
        &ctx,
        "reject",
        "M2",
        "receiving_order",
        id.to_string(),
        Some(AuditDiff::compute(
            serde_json::json!({}),
            serde_json::json!({
                "status": "closed_rejected",
                "reason": reason,
                "rejected_qty": receipt.rejected_qty,
            }),
        )),
    )
    .await;
    Ok(Json(receipt))
}
