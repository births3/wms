async fn create_outbound_order_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateOutboundOrderRequest>,
) -> Result<Json<OutboundOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create_outbound_order",
        "M4",
        "outbound_order",
        req.wms_order_no.clone(),
        None,
    );
    let outcome = state
        .wave4_repository
        .create_outbound_order(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn create_outbound_wave_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateOutboundWaveRequest>,
) -> Result<Json<OutboundWave>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create_outbound_wave",
        "M4",
        "outbound_wave",
        req.wave_no.clone(),
        None,
    );
    let outcome = state
        .wave_replenish
        .create_outbound_wave(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn complete_pick_task_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<CompletePickTaskRequest>,
) -> Result<Json<OutboundOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "complete_pick_task",
        "M4",
        "outbound_order",
        id.to_string(),
        None,
    );
    let outcome = state
        .wave4_repository
        .complete_pick_task(&ctx, id, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn review_outbound_order_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ReviewOutboundOrderRequest>,
) -> Result<Json<OutboundOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "review_outbound_order",
        "M4",
        "outbound_order",
        id.to_string(),
        None,
    );
    let outcome = state
        .wave4_repository
        .review_outbound_order(&ctx, id, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn get_outbound_review_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<OutboundOrder>, Wave4HandlerError> {
    require_any_permission(&ctx, &[M4_READ_PERMISSION, "m4.write"])?;
    Ok(Json(
        state.wave4_repository.get_outbound_order(&ctx, id).await?,
    ))
}

async fn ship_outbound_order_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ShipOutboundOrderRequest>,
) -> Result<Json<OutboundOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let outcome = state
        .shipping_service
        .ship_outbound_order(&ctx, id, req, now, &idempotency_key)
        .await?;
    Ok(Json(outcome.value))
}

async fn list_outbound_orders_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Query(query): Query<ListOutboundOrdersQuery>,
) -> Result<Json<OutboundOrderListResponse>, Wave4HandlerError> {
    require_any_permission(&ctx, &[M4_READ_PERMISSION, "m4.write"])?;
    let data = state
        .wave4_repository
        .list_outbound_orders(
            &ctx,
            query.status.as_deref(),
            query.q.as_deref(),
            query.limit,
        )
        .await?;
    Ok(Json(OutboundOrderListResponse {
        page: PageMeta {
            count: data.len() as u32,
            next_cursor: None,
            total: None,
        },
        data,
    }))
}

async fn get_outbound_order_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<OutboundOrder>, Wave4HandlerError> {
    require_any_permission(&ctx, &[M4_READ_PERMISSION, "m4.write"])?;
    let order = state
        .wave4_repository
        .get_outbound_order(&ctx, order_id)
        .await?;
    Ok(Json(order))
}
