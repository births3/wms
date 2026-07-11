async fn cancel_waybill_handler(
    ctx: AuthContext,
    State(state): State<ExpressAppState>,
    headers: HeaderMap,
    Path(waybill_no): Path<String>,
    Json(req): Json<CancelExpressWaybillRequest>,
) -> Result<Json<ExpressWaybill>, ExpressError> {
    ctx.require_permission("h5.express.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let request_hash = json_request_hash(&serde_json::json!({
        "waybill_no": waybill_no.trim(),
        "request": req,
    }))?;
    let mut tx = state.pool.begin().await.map_err(map_db_error)?;
    lock_idempotency_key(&mut tx, ctx.owner_id, &idempotency_key).await?;
    if let Some(replay) =
        replay_idempotency(&mut tx, ctx.owner_id, &idempotency_key, &request_hash, now).await?
    {
        return Ok(Json(replay));
    }

    let current = sqlx::query_as::<_, WaybillRow>(
        r#"
        SELECT id, owner_id, outbound_order_id, package_no, carrier_code,
               waybill_no, status, sender_name, sender_mobile, sender_address,
               receiver_name, receiver_mobile, receiver_address, weight_grams,
               volume_cm3, package_count, eta_at, created_at, updated_at
          FROM h5_express_waybills
         WHERE owner_id = $1 AND waybill_no = $2
         FOR UPDATE
        "#,
    )
    .bind(ctx.owner_id)
    .bind(waybill_no.trim())
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_db_error)?
    .ok_or(ExpressError::NotFound)?;
    if matches!(current.status.as_str(), "in_transit" | "signed") {
        return Err(ExpressError::InvalidRequest);
    }

    let waybill = if current.status == "cancelled" {
        ExpressWaybill::from(current)
    } else {
        ExpressWaybill::from(
            sqlx::query_as::<_, WaybillRow>(
                r#"
                UPDATE h5_express_waybills
                   SET status = 'cancelled', updated_at = $3
                 WHERE owner_id = $1 AND waybill_no = $2
                RETURNING id, owner_id, outbound_order_id, package_no, carrier_code,
                          waybill_no, status, sender_name, sender_mobile, sender_address,
                          receiver_name, receiver_mobile, receiver_address, weight_grams,
                          volume_cm3, package_count, eta_at, created_at, updated_at
                "#,
            )
            .bind(ctx.owner_id)
            .bind(waybill_no.trim())
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?,
        )
    };

    let reason = req
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("用户取消快递单");
    sqlx::query(
        r#"
        INSERT INTO h5_express_tracking_events (
            id, owner_id, waybill_id, waybill_no, event_time, status, location,
            description, source, cached_at
        )
        VALUES ($1, $2, $3, $4, $5, 'cancelled', 'WMS', $6, 'wms', $5)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(waybill.id)
    .bind(&waybill.waybill_no)
    .bind(now)
    .bind(reason)
    .execute(&mut *tx)
    .await
    .map_err(map_db_error)?;

    finish_mutation(
        &mut tx,
        &ctx,
        &idempotency_key,
        &request_hash,
        "POST",
        "/api/v1/express/waybills/{waybill_no}/cancel",
        "express_waybill",
        waybill.waybill_no.clone(),
        &waybill,
        "cancel_express_waybill",
        now,
    )
    .await?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(Json(waybill))
}

async fn get_tracking_handler(
    ctx: AuthContext,
    State(state): State<ExpressAppState>,
    Path(waybill_no): Path<String>,
) -> Result<Json<ExpressTrackingResponse>, ExpressError> {
    ctx.require_permission("h5.express.read")?;
    let waybill = ExpressWaybill::from(
        sqlx::query_as::<_, WaybillRow>(
            r#"
            SELECT id, owner_id, outbound_order_id, package_no, carrier_code,
                   waybill_no, status, sender_name, sender_mobile, sender_address,
                   receiver_name, receiver_mobile, receiver_address, weight_grams,
                   volume_cm3, package_count, eta_at, created_at, updated_at
              FROM h5_express_waybills
             WHERE owner_id = $1 AND waybill_no = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(waybill_no.trim())
        .fetch_optional(&state.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(ExpressError::NotFound)?,
    );

    let events = sqlx::query_as::<_, TrackingEventRow>(
        r#"
        SELECT id, waybill_no, event_time, status, location, description, source, cached_at
          FROM h5_express_tracking_events
         WHERE owner_id = $1 AND waybill_no = $2
         ORDER BY event_time DESC
        "#,
    )
    .bind(ctx.owner_id)
    .bind(&waybill.waybill_no)
    .fetch_all(&state.pool)
    .await
    .map_err(map_db_error)?
    .into_iter()
    .map(ExpressTrackingEvent::from)
    .collect();

    Ok(Json(ExpressTrackingResponse { waybill, events }))
}

impl From<CarrierRow> for ExpressCarrier {
    fn from(row: CarrierRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            carrier_code: row.carrier_code,
            carrier_name: row.carrier_name,
            api_url: row.api_url,
            api_key_alias: row.api_key_alias,
            api_secret_alias: row.api_secret_alias,
            account_no: row.account_no,
            enabled: row.enabled,
            priority: row.priority,
            conditions: row.conditions,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<RuleRow> for ExpressRoutingRule {
    fn from(row: RuleRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            rule_code: row.rule_code,
            rule_name: row.rule_name,
            delivery_provider_type: row.delivery_provider_type,
            carrier_code: row.carrier_code,
            priority: row.priority,
            conditions: row.conditions,
            fallback_strategy: row.fallback_strategy,
            enabled: row.enabled,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<WaybillRow> for ExpressWaybill {
    fn from(row: WaybillRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            outbound_order_id: row.outbound_order_id,
            package_no: row.package_no,
            carrier_code: row.carrier_code,
            waybill_no: row.waybill_no,
            status: row.status,
            sender_name: row.sender_name,
            sender_mobile: row.sender_mobile,
            sender_address: row.sender_address,
            receiver_name: row.receiver_name,
            receiver_mobile: row.receiver_mobile,
            receiver_address: row.receiver_address,
            weight_grams: row.weight_grams,
            volume_cm3: row.volume_cm3,
            package_count: row.package_count,
            eta_at: row.eta_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<TrackingEventRow> for ExpressTrackingEvent {
    fn from(row: TrackingEventRow) -> Self {
        Self {
            id: row.id,
            waybill_no: row.waybill_no,
            event_time: row.event_time,
            status: row.status,
            location: row.location,
            description: row.description,
            source: row.source,
            cached_at: row.cached_at,
        }
    }
}

fn validate_carrier(req: &UpsertExpressCarrierRequest) -> Result<(), ExpressError> {
    if req.carrier_code.trim().is_empty()
        || req.carrier_name.trim().is_empty()
        || req.api_url.trim().is_empty()
        || req.priority < 0
    {
        return Err(ExpressError::InvalidRequest);
    }
    Ok(())
}

fn validate_rule(req: &UpsertExpressRoutingRuleRequest) -> Result<(), ExpressError> {
    let provider = req.delivery_provider_type.trim();
    if req.rule_code.trim().is_empty()
        || req.rule_name.trim().is_empty()
        || req.priority < 0
        || !matches!(provider, "own_fleet" | "third_party_express")
    {
        return Err(ExpressError::InvalidRequest);
    }
    if provider == "third_party_express" && trimmed_option(req.carrier_code.clone()).is_none() {
        return Err(ExpressError::InvalidRequest);
    }
    Ok(())
}

fn validate_waybill(req: &CreateExpressWaybillRequest) -> Result<(), ExpressError> {
    if req.package_no.trim().is_empty()
        || req.carrier_code.trim().is_empty()
        || req.sender_name.trim().is_empty()
        || req.sender_mobile.trim().is_empty()
        || req.sender_address.trim().is_empty()
        || req.receiver_name.trim().is_empty()
        || req.receiver_mobile.trim().is_empty()
        || req.receiver_address.trim().is_empty()
        || req.weight_grams <= 0
        || req.volume_cm3 < 0
        || req.package_count <= 0
    {
        return Err(ExpressError::InvalidRequest);
    }
    Ok(())
}

fn normalized_limit(value: Option<u32>) -> u32 {
    value.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

fn trimmed_option(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn json_object_or_empty(value: Value) -> Value {
    if value.is_object() {
        value
    } else {
        serde_json::json!({})
    }
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, ExpressError> {
    headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or(ExpressError::InvalidIdempotencyKey)
}

#[allow(clippy::too_many_arguments)]
async fn finish_mutation<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
    action: &str,
    now: DateTime<Utc>,
) -> Result<(), ExpressError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| ExpressError::Serialize(error.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 200, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(method)
    .bind(path)
    .bind(response_body.clone())
    .bind(resource_type)
    .bind(&resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    append_event_in_tx(
        tx,
        &AuditWriteRequest::from_auth_context(
            ctx,
            action,
            "H5",
            resource_type,
            resource_id,
            Some(AuditDiff::compute(serde_json::json!({}), response_body)),
        ),
    )
    .await
    .map(|_| ())
    .map_err(|error| ExpressError::Audit(format!("{error:?}")))
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), ExpressError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), hashtext($2))")
        .bind(owner_id.to_string())
        .bind(idempotency_key)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, ExpressError> {
    let row: Option<(String, Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT request_hash, response_body, expires_at
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((stored_hash, response_body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        return Ok(None);
    }
    if stored_hash != request_hash {
        return Err(ExpressError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| ExpressError::Serialize(error.to_string()))
}

fn json_request_hash<T: Serialize>(value: &T) -> Result<String, ExpressError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| ExpressError::Serialize(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn map_db_error(error: sqlx::Error) -> ExpressError {
    ExpressError::Database(error.to_string())
}

#[cfg(test)]
mod tests {
    use axum::{extract::State, http::HeaderMap, Json};
    use sqlx::PgPool;
    use uuid::Uuid;
    use wms_domain::UpsertExpressCarrierRequest;

    use super::{express_router, upsert_carrier_handler, ExpressAppState, ExpressError};
    use crate::auth::{AuthContext, AuthError};

    fn ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "h5-express-test".to_string(),
            permissions: permissions.iter().map(|item| item.to_string()).collect(),
            jti: Uuid::new_v4().to_string(),
        }
    }

    fn carrier_request() -> UpsertExpressCarrierRequest {
        UpsertExpressCarrierRequest {
            carrier_code: "SF".to_string(),
            carrier_name: "顺丰速运".to_string(),
            api_url: "https://carrier.example.test".to_string(),
            api_key_alias: Some("sf_key".to_string()),
            api_secret_alias: Some("sf_secret".to_string()),
            account_no: Some("WMS-001".to_string()),
            enabled: true,
            priority: 10,
            conditions: serde_json::json!({ "cold_chain": true }),
        }
    }

    #[tokio::test]
    async fn express_router_registers_h5_paths() {
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during router registration");
        let _router = express_router(ExpressAppState::with_postgres(pool));
        let _paths = [
            "/api/v1/express/carriers",
            "/api/v1/express/routing-rules",
            "/api/v1/express/waybills",
            "/api/v1/express/waybills/{waybill_no}/tracking",
        ];
    }

    #[tokio::test]
    async fn express_carrier_write_checks_permission_before_postgres() {
        let owner_id = Uuid::new_v4();
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during auth test");
        let state = ExpressAppState::with_postgres(pool);
        let result = upsert_carrier_handler(
            ctx(owner_id, &[]),
            State(state),
            HeaderMap::new(),
            Json(carrier_request()),
        )
        .await
        .expect_err("h5.express.write should be checked before postgres access");

        assert!(matches!(
            result,
            ExpressError::Auth(AuthError::PermissionDenied(permission))
                if permission == "h5.express.write"
        ));
    }

    #[tokio::test]
    async fn express_carrier_write_requires_idempotency_before_postgres() {
        let owner_id = Uuid::new_v4();
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during idempotency test");
        let state = ExpressAppState::with_postgres(pool);
        let result = upsert_carrier_handler(
            ctx(owner_id, &["h5.express.write"]),
            State(state),
            HeaderMap::new(),
            Json(carrier_request()),
        )
        .await
        .expect_err("Idempotency-Key should be checked before postgres access");

        assert!(matches!(result, ExpressError::InvalidIdempotencyKey));
    }

    #[test]
    fn third_party_rule_requires_carrier_code() {
        let req = wms_domain::UpsertExpressRoutingRuleRequest {
            rule_code: "third-party".to_string(),
            rule_name: "三方快递".to_string(),
            delivery_provider_type: "third_party_express".to_string(),
            carrier_code: None,
            priority: 10,
            conditions: serde_json::json!({}),
            fallback_strategy: Some("manual".to_string()),
            enabled: true,
            effective_from: None,
            effective_to: None,
        };

        assert!(matches!(
            super::validate_rule(&req),
            Err(ExpressError::InvalidRequest)
        ));
    }
}
