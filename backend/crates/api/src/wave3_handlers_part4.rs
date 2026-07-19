use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct NearExpiryReportQuery {
    as_of: Option<String>,
    warning_days: Option<String>,
}

async fn near_expiry_report_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    axum::extract::Query(query): axum::extract::Query<NearExpiryReportQuery>,
) -> Result<Json<InventoryBatchListResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.read", "m3.write"])?;
    let as_of = match query.as_of {
        Some(value) => chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d")
            .map_err(|_| Wave3HandlerError::Repository(Wave3RepositoryError::InvalidDate(value)))?,
        None => Utc::now().date_naive(),
    };
    let warning_days = query
        .warning_days
        .map(|value| {
            value.parse::<i64>().map_err(|_| {
                Wave3HandlerError::Repository(Wave3RepositoryError::InvalidQuantity)
            })
        })
        .transpose()?;
    let repository = state.wave3_repository.as_ref().ok_or_else(|| {
        Wave3HandlerError::Repository(Wave3RepositoryError::Database(
            "近效期报表需要 PostgreSQL repository".to_string(),
        ))
    })?;
    let data = repository
        .list_near_expiry_batches(&ctx, as_of, warning_days)
        .await?;
    Ok(Json(InventoryBatchListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
        },
        data,
    }))
}

async fn putaway_receiving_order_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<PutawayRequest>,
) -> Result<Json<PutawayRecord>, Wave3HandlerError> {
    ctx.require_permission("m2.putaway.write")?;
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
    Query(query): Query<InventoryBatchQuery>,
) -> Result<Json<InventoryBatchListResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.read", "m3.write"])?;
    let production_from = parse_inventory_batch_date_filter(query.production_from.as_ref())?;
    let production_to = parse_inventory_batch_date_filter(query.production_to.as_ref())?;
    let expiry_from = parse_inventory_batch_date_filter(query.expiry_from.as_ref())?;
    let expiry_to = parse_inventory_batch_date_filter(query.expiry_to.as_ref())?;
    let created_from = parse_inventory_batch_datetime_filter(query.created_from.as_ref())?;
    let created_to = parse_inventory_batch_datetime_filter(query.created_to.as_ref())?;
    if production_from.zip(production_to).is_some_and(|(from, to)| from > to) {
        return Err(Wave3RepositoryError::InvalidDate(
            "production_from_after_production_to".to_string(),
        )
        .into());
    }
    if expiry_from.zip(expiry_to).is_some_and(|(from, to)| from > to) {
        return Err(Wave3RepositoryError::InvalidDate(
            "expiry_from_after_expiry_to".to_string(),
        )
        .into());
    }
    if created_from.zip(created_to).is_some_and(|(from, to)| from > to) {
        return Err(Wave3RepositoryError::InvalidDate(
            "created_from_after_created_to".to_string(),
        )
        .into());
    }
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
        let batches = repository
            .list_inventory_batches_with_query(&ctx, query)
            .await?;
        let count = batches.len() as u32;
        return Ok(Json(InventoryBatchListResponse {
            data: batches,
            page: PageMeta {
                next_cursor: None,
                count,
            },
        }));
    }
    let mut batches = {
        let store = state.inventory_store.lock().await;
        store
            .list_batches(&ctx)
            .into_iter()
            .filter(|batch| {
                inventory_batch_matches_query(
                    batch,
                    &query,
                    production_from,
                    production_to,
                    expiry_from,
                    expiry_to,
                    created_from,
                    created_to,
                )
            })
            .collect::<Vec<_>>()
    };
    if expiry_from.is_some() || expiry_to.is_some() {
        batches.sort_by(|left, right| {
            left.expiry_date
                .cmp(&right.expiry_date)
                .then_with(|| left.product_code.cmp(&right.product_code))
                .then_with(|| left.batch_no.cmp(&right.batch_no))
                .then_with(|| left.id.cmp(&right.id))
        });
    }
    let count = batches.len() as u32;
    Ok(Json(InventoryBatchListResponse {
        data: batches,
        page: PageMeta {
            next_cursor: None,
            count,
        },
    }))
}

async fn get_inventory_batch_trace_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(batch_id): Path<Uuid>,
) -> Result<Json<InventoryBatchTrace>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.read", "m3.write"])?;
    if let Some(repository) = &state.wave3_repository {
        return Ok(Json(
            repository
                .get_inventory_batch_trace(&ctx, batch_id)
                .await?,
        ));
    }
    let trace = state
        .inventory_store
        .lock()
        .await
        .trace_batch(&ctx, batch_id)?;
    Ok(Json(trace))
}

async fn list_location_history_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    axum::extract::Query(query): axum::extract::Query<LocationHistoryQuery>,
) -> Result<Json<LocationHistoryResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.read", "m3.write"])?;
    let repository = state.wave3_repository.as_ref().ok_or_else(|| {
        Wave3HandlerError::Repository(Wave3RepositoryError::Database(
            "库位历史查询需要 PostgreSQL repository".to_string(),
        ))
    })?;
    Ok(Json(repository.list_location_history(&ctx, &query).await?))
}

fn parse_inventory_batch_date_filter(
    value: Option<&String>,
) -> Result<Option<NaiveDate>, Wave3HandlerError> {
    value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| Wave3RepositoryError::InvalidDate(value.to_string()).into())
        })
        .transpose()
}

fn parse_inventory_batch_datetime_filter(
    value: Option<&String>,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, Wave3HandlerError> {
    value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| {
            chrono::DateTime::parse_from_rfc3339(value)
                .map(|value| value.with_timezone(&chrono::Utc))
                .map_err(|_| Wave3RepositoryError::InvalidDate(value.to_string()).into())
        })
        .transpose()
}

fn inventory_batch_matches_query(
    batch: &InventoryBatch,
    query: &InventoryBatchQuery,
    production_from: Option<NaiveDate>,
    production_to: Option<NaiveDate>,
    expiry_from: Option<NaiveDate>,
    expiry_to: Option<NaiveDate>,
    created_from: Option<chrono::DateTime<chrono::Utc>>,
    created_to: Option<chrono::DateTime<chrono::Utc>>,
) -> bool {
    fn contains_filter(value: &str, filter: &Option<String>) -> bool {
        filter.as_ref().is_none_or(|filter| {
            let filter = filter.trim();
            filter.is_empty() || value.to_lowercase().contains(&filter.to_lowercase())
        })
    }

    contains_filter(&batch.product_code, &query.product_code)
        && query.q.as_ref().is_none_or(|filter| {
            let filter = filter.trim().to_lowercase();
            filter.is_empty()
                || batch.product_code.to_lowercase().contains(&filter)
                || batch
                    .product_name
                    .as_ref()
                    .is_some_and(|value| value.to_lowercase().contains(&filter))
                || batch.batch_no.to_lowercase().contains(&filter)
                || batch.location_code.to_lowercase().contains(&filter)
                || batch
                    .container_lpn
                    .as_ref()
                    .is_some_and(|value| value.to_lowercase().contains(&filter))
        })
        && contains_filter(&batch.batch_no, &query.batch_no)
        && contains_filter(&batch.location_code, &query.location_code)
        // 内存库存模型未携带库位主数据；非空元数据条件不能伪造匹配结果。
        && query
            .location_type
            .as_ref()
            .is_none_or(|location_type| location_type.trim().is_empty())
        && query
            .zone_code
            .as_ref()
            .is_none_or(|zone_code| zone_code.trim().is_empty())
        && query
            .temperature_zone
            .as_ref()
            .is_none_or(|temperature_zone| {
                temperature_zone.trim().is_empty()
                    || batch.temperature_zone.as_deref() == Some(temperature_zone.trim())
            })
        && query.quality_status.as_ref().is_none_or(|status| {
            status.trim().is_empty() || batch.quality_status == status.trim()
        })
        && (production_from.is_none() && production_to.is_none()
            || NaiveDate::parse_from_str(&batch.production_date, "%Y-%m-%d")
                .ok()
                .is_some_and(|production_date| {
                    production_from.is_none_or(|from| production_date >= from)
                        && production_to.is_none_or(|to| production_date <= to)
                }))
        && NaiveDate::parse_from_str(&batch.expiry_date, "%Y-%m-%d")
            .ok()
            .is_some_and(|expiry_date| {
                expiry_from.is_none_or(|from| expiry_date >= from)
                    && expiry_to.is_none_or(|to| expiry_date <= to)
            })
        && created_from.is_none_or(|from| batch.created_at >= from)
        && created_to.is_none_or(|to| batch.created_at <= to)
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

async fn mark_inventory_recall_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<MarkInventoryRecallRequest>,
) -> Result<Json<InventoryBatch>, Wave3HandlerError> {
    ctx.require_permission("m3.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let batch_id = req.batch_id;
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "mark_inventory_recall",
            "M3",
            "inventory_batch",
            batch_id.to_string(),
            None,
        );
        let outcome = repository
            .mark_inventory_batch_recalled(&ctx, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let batch = {
        let mut store = state.inventory_store.lock().await;
        store.mark_recall(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "mark_inventory_recall",
        "M3",
        "inventory_batch",
        batch_id.to_string(),
    )
    .await;
    Ok(Json(batch))
}

async fn cancel_inventory_recall_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<CancelInventoryRecallRequest>,
) -> Result<Json<InventoryBatch>, Wave3HandlerError> {
    ctx.require_permission("m3.recall.cancel")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let batch_id = req.batch_id;
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "cancel_inventory_recall",
            "M3",
            "inventory_batch",
            batch_id.to_string(),
            None,
        );
        let outcome = repository
            .cancel_inventory_batch_recall(&ctx, req, now, &idempotency_key, Some(audit))
            .await?;
        return Ok(Json(outcome.value));
    }
    let batch = {
        let mut store = state.inventory_store.lock().await;
        store.cancel_recall(&ctx, req, now)?
    };
    append_audit(
        &state,
        &ctx,
        "cancel_inventory_recall",
        "M3",
        "inventory_batch",
        batch_id.to_string(),
    )
    .await;
    Ok(Json(batch))
}

async fn isolate_expired_inventory_batches_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<ExpireInventoryBatchesRequest>,
) -> Result<Json<InventoryBatchListResponse>, Wave3HandlerError> {
    ctx.require_permission("m3.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let as_of = req
        .as_of
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
                .map_err(|_| Wave3HandlerError::Repository(Wave3RepositoryError::InvalidDate("as_of".to_string())))
        })
        .transpose()?
        .unwrap_or_else(|| now.date_naive());
    if let Some(repository) = &state.wave3_repository {
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "isolate_expired_inventory_batch",
            "M3",
            "inventory_expiry_job",
            format!("{}:{}", ctx.owner_id, as_of),
            None,
        );
        let outcome = repository
            .isolate_expired_inventory_batches(&ctx, as_of, now, &idempotency_key, Some(audit))
            .await?;
        let count = outcome.value.len() as u32;
        return Ok(Json(InventoryBatchListResponse {
            data: outcome.value,
            page: PageMeta { next_cursor: None, count },
        }));
    }
    let batches = {
        let mut store = state.inventory_store.lock().await;
        store.isolate_expired_batches(&ctx, as_of, now)?
    };
    for batch in &batches {
        append_audit(
            &state,
            &ctx,
            "isolate_expired_inventory_batch",
            "M3",
            "inventory_batch",
            batch.id.to_string(),
        )
        .await;
    }
    let count = batches.len() as u32;
    Ok(Json(InventoryBatchListResponse {
        data: batches,
        page: PageMeta { next_cursor: None, count },
    }))
}
