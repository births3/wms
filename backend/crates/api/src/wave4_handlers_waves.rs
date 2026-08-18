async fn list_outbound_waves_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Query(query): Query<ListOutboundOrdersQuery>,
) -> Result<Json<OutboundWaveListResponse>, Wave4HandlerError> {
    require_any_permission(&ctx, &[M4_READ_PERMISSION, "m4.write"])?;
    let data = state
        .wave4_repository
        .list_outbound_waves(
            &ctx,
            query.status.as_deref(),
            query.q.as_deref(),
            query.limit,
        )
        .await?;
    Ok(Json(OutboundWaveListResponse {
        page: PageMeta {
            count: data.len() as u32,
            next_cursor: None,
            total: None,
        },
        data,
    }))
}

async fn get_outbound_wave_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(wave_id): Path<Uuid>,
) -> Result<Json<OutboundWave>, Wave4HandlerError> {
    require_any_permission(&ctx, &[M4_READ_PERMISSION, "m4.write"])?;
    Ok(Json(
        state
            .wave4_repository
            .get_outbound_wave(&ctx, wave_id)
            .await?,
    ))
}

async fn release_outbound_wave_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(wave_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<OutboundWave>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "release_outbound_wave",
        "M4",
        "outbound_wave",
        wave_id.to_string(),
        None,
    );
    let outcome = state
        .wave4_repository
        .release_outbound_wave(&ctx, wave_id, Utc::now(), &idempotency_key, Some(audit))
        .await?;
    let _ = state
        .replenishment
        .fill_wave_pick_gaps(ctx.owner_id, wave_id)
        .await;
    Ok(Json(outcome.value))
}

async fn cancel_outbound_wave_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(wave_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<OutboundWave>, Wave4HandlerError> {
    ctx.require_permission("m4.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "cancel_outbound_wave",
        "M4",
        "outbound_wave",
        wave_id.to_string(),
        None,
    );
    let outcome = state
        .wave4_repository
        .cancel_outbound_wave(&ctx, wave_id, Utc::now(), &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}
