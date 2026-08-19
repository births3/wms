async fn revalidate_outbound_order_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<OutboundOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "revalidate_outbound_order",
        "M4",
        "outbound_order",
        id.to_string(),
        None,
    );
    let outcome = state
        .wave4_repository
        .revalidate_outbound_order(&ctx, id, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn void_request_outbound_order_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<OutboundOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "request_void_outbound_order",
        "M4",
        "outbound_order",
        id.to_string(),
        None,
    );
    let outcome = state
        .wave4_repository
        .request_void_outbound_order(&ctx, id, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}
