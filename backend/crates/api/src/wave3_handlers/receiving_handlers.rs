use super::*;

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

pub(super) async fn update_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateReceivingOrderRequest>,
) -> Result<Json<ReceivingOrder>, Wave3HandlerError> {
    ctx.require_permission("m2.write")?;
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
                .update_receiving_order(&ctx, id, req, now, audit)
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
    append_audit(
        &state,
        &ctx,
        "reject",
        "M2",
        "receiving_order",
        id.to_string(),
    )
    .await;
    Ok(Json(receipt))
}
