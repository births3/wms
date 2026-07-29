async fn create_purchase_return_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    headers: HeaderMap,
    Json(req): Json<CreatePurchaseReturnRequest>,
) -> Result<Json<PurchaseReturnOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create_purchase_return",
        "M4",
        "purchase_return_order",
        req.return_no.clone(),
        None,
    );
    let outcome = state
        .wave4_repository
        .create_purchase_return(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn list_purchase_returns_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Query(query): Query<ListOutboundOrdersQuery>,
) -> Result<Json<PurchaseReturnOrderListResponse>, Wave4HandlerError> {
    require_any_permission(&ctx, &[M4_READ_PERMISSION, "m4.write"])?;
    let data = state
        .wave4_repository
        .list_purchase_returns(
            &ctx,
            query.status.as_deref(),
            query.q.as_deref(),
            query.limit,
        )
        .await?;
    Ok(Json(PurchaseReturnOrderListResponse {
        page: PageMeta {
            count: data.len() as u32,
            next_cursor: None,
        },
        data,
    }))
}

async fn get_purchase_return_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PurchaseReturnOrder>, Wave4HandlerError> {
    require_any_permission(&ctx, &[M4_READ_PERMISSION, "m4.write"])?;
    Ok(Json(
        state.wave4_repository.get_purchase_return(&ctx, id).await?,
    ))
}

async fn approve_purchase_return_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PurchaseReturnOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = purchase_return_action_audit(&ctx, "approve_purchase_return", id);
    let outcome = state
        .wave4_repository
        .approve_purchase_return(&ctx, id, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn reject_purchase_return_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<RejectPurchaseReturnRequest>,
) -> Result<Json<PurchaseReturnOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = purchase_return_action_audit(&ctx, "reject_purchase_return", id);
    let outcome = state
        .wave4_repository
        .reject_purchase_return(&ctx, id, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn pick_purchase_return_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PurchaseReturnOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = purchase_return_action_audit(&ctx, "pick_purchase_return", id);
    let outcome = state
        .wave4_repository
        .pick_purchase_return(&ctx, id, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn review_purchase_return_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PurchaseReturnOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = purchase_return_action_audit(&ctx, "review_purchase_return", id);
    let outcome = state
        .wave4_repository
        .review_purchase_return(&ctx, id, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn ship_purchase_return_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PurchaseReturnOrder>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = purchase_return_action_audit(&ctx, "ship_purchase_return", id);
    let outcome = state
        .wave4_repository
        .ship_purchase_return(&ctx, id, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

fn purchase_return_action_audit(ctx: &AuthContext, action: &str, id: Uuid) -> AuditWriteRequest {
    AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M4",
        "purchase_return_order",
        id.to_string(),
        None,
    )
}
