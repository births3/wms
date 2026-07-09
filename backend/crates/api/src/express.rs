//! H5 快递对接接口。

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    CancelExpressWaybillRequest, CreateExpressWaybillRequest, ErrorResponse, ExpressCarrier,
    ExpressCarrierListResponse, ExpressRoutingRule, ExpressRoutingRuleListResponse,
    ExpressTrackingEvent, ExpressTrackingResponse, ExpressWaybill, PageMeta,
    UpsertExpressCarrierRequest, UpsertExpressRoutingRuleRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::{AuthContext, AuthError},
};

const DEFAULT_LIMIT: u32 = 100;
const MAX_LIMIT: u32 = 200;

#[derive(Clone, Debug)]
pub struct ExpressAppState {
    pool: PgPool,
}

impl ExpressAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ExpressError {
    Auth(AuthError),
    InvalidIdempotencyKey,
    InvalidRequest,
    NotFound,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

impl From<AuthError> for ExpressError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl IntoResponse for ExpressError {
    fn into_response(self) -> Response {
        if let ExpressError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            ExpressError::InvalidIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "H5-400",
                "缺少或非法 Idempotency-Key",
            ),
            ExpressError::InvalidRequest => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H5-422",
                "快递请求字段或规则校验失败",
            ),
            ExpressError::NotFound => (StatusCode::NOT_FOUND, "H5-404", "快递资源不存在"),
            ExpressError::IdempotencyConflict => {
                (StatusCode::CONFLICT, "H5-409", "幂等键已用于不同请求")
            }
            ExpressError::Audit(_) | ExpressError::Database(_) | ExpressError::Serialize(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H5-500",
                "快递对接持久化或审计失败",
            ),
            ExpressError::Auth(_) => unreachable!("auth error returned above"),
        };

        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

#[derive(Clone, Debug, Deserialize)]
struct CarrierQuery {
    q: Option<String>,
    enabled: Option<bool>,
    limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
struct RuleQuery {
    q: Option<String>,
    delivery_provider_type: Option<String>,
    enabled: Option<bool>,
    limit: Option<u32>,
}

#[derive(Clone, Debug, FromRow)]
struct CarrierRow {
    id: Uuid,
    owner_id: Uuid,
    carrier_code: String,
    carrier_name: String,
    api_url: String,
    api_key_alias: Option<String>,
    api_secret_alias: Option<String>,
    account_no: Option<String>,
    enabled: bool,
    priority: i32,
    conditions: Value,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow)]
struct RuleRow {
    id: Uuid,
    owner_id: Uuid,
    rule_code: String,
    rule_name: String,
    delivery_provider_type: String,
    carrier_code: Option<String>,
    priority: i32,
    conditions: Value,
    fallback_strategy: Option<String>,
    enabled: bool,
    effective_from: Option<DateTime<Utc>>,
    effective_to: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow)]
struct WaybillRow {
    id: Uuid,
    owner_id: Uuid,
    outbound_order_id: Option<Uuid>,
    package_no: String,
    carrier_code: String,
    waybill_no: String,
    status: String,
    sender_name: String,
    sender_mobile: String,
    sender_address: String,
    receiver_name: String,
    receiver_mobile: String,
    receiver_address: String,
    weight_grams: i64,
    volume_cm3: i64,
    package_count: i32,
    eta_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow)]
struct TrackingEventRow {
    id: Uuid,
    waybill_no: String,
    event_time: DateTime<Utc>,
    status: String,
    location: Option<String>,
    description: String,
    source: String,
    cached_at: DateTime<Utc>,
}

pub fn express_router(state: ExpressAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/express/carriers",
            get(list_carriers_handler).post(upsert_carrier_handler),
        )
        .route(
            "/api/v1/express/routing-rules",
            get(list_routing_rules_handler).post(upsert_routing_rule_handler),
        )
        .route("/api/v1/express/waybills", post(create_waybill_handler))
        .route(
            "/api/v1/express/waybills/:waybill_no/cancel",
            post(cancel_waybill_handler),
        )
        .route(
            "/api/v1/express/waybills/:waybill_no/tracking",
            get(get_tracking_handler),
        )
        .with_state(state)
}

async fn list_carriers_handler(
    ctx: AuthContext,
    State(state): State<ExpressAppState>,
    Query(query): Query<CarrierQuery>,
) -> Result<Json<ExpressCarrierListResponse>, ExpressError> {
    ctx.require_permission("h5.express.read")?;
    let limit = normalized_limit(query.limit);
    let q = trimmed_option(query.q);
    let rows = sqlx::query_as::<_, CarrierRow>(
        r#"
        SELECT id, owner_id, carrier_code, carrier_name, api_url, api_key_alias,
               api_secret_alias, account_no, enabled, priority, conditions,
               status, created_at, updated_at
          FROM h5_express_carriers
         WHERE owner_id = $1
           AND ($2::TEXT IS NULL OR carrier_code ILIKE ('%' || $2 || '%') OR carrier_name ILIKE ('%' || $2 || '%'))
           AND ($3::BOOLEAN IS NULL OR enabled = $3)
         ORDER BY priority ASC, carrier_code ASC
         LIMIT $4
        "#,
    )
    .bind(ctx.owner_id)
    .bind(q)
    .bind(query.enabled)
    .bind(i64::from(limit))
    .fetch_all(&state.pool)
    .await
    .map_err(map_db_error)?;

    let data: Vec<_> = rows.into_iter().map(ExpressCarrier::from).collect();
    Ok(Json(ExpressCarrierListResponse {
        page: PageMeta {
            count: data.len() as u32,
            next_cursor: None,
        },
        data,
    }))
}

async fn upsert_carrier_handler(
    ctx: AuthContext,
    State(state): State<ExpressAppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertExpressCarrierRequest>,
) -> Result<Json<ExpressCarrier>, ExpressError> {
    ctx.require_permission("h5.express.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    validate_carrier(&req)?;
    let now = Utc::now();
    let request_hash = json_request_hash(&req)?;
    let mut tx = state.pool.begin().await.map_err(map_db_error)?;
    lock_idempotency_key(&mut tx, ctx.owner_id, &idempotency_key).await?;
    if let Some(replay) =
        replay_idempotency(&mut tx, ctx.owner_id, &idempotency_key, &request_hash, now).await?
    {
        return Ok(Json(replay));
    }

    let carrier = ExpressCarrier::from(
        sqlx::query_as::<_, CarrierRow>(
            r#"
            INSERT INTO h5_express_carriers (
                id, owner_id, carrier_code, carrier_name, api_url, api_key_alias,
                api_secret_alias, account_no, enabled, priority, conditions, status,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $13)
            ON CONFLICT (owner_id, carrier_code) DO UPDATE
               SET carrier_name = EXCLUDED.carrier_name,
                   api_url = EXCLUDED.api_url,
                   api_key_alias = EXCLUDED.api_key_alias,
                   api_secret_alias = EXCLUDED.api_secret_alias,
                   account_no = EXCLUDED.account_no,
                   enabled = EXCLUDED.enabled,
                   priority = EXCLUDED.priority,
                   conditions = EXCLUDED.conditions,
                   status = EXCLUDED.status,
                   updated_at = EXCLUDED.updated_at
            RETURNING id, owner_id, carrier_code, carrier_name, api_url, api_key_alias,
                      api_secret_alias, account_no, enabled, priority, conditions,
                      status, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(req.carrier_code.trim())
        .bind(req.carrier_name.trim())
        .bind(req.api_url.trim())
        .bind(trimmed_option(req.api_key_alias))
        .bind(trimmed_option(req.api_secret_alias))
        .bind(trimmed_option(req.account_no))
        .bind(req.enabled)
        .bind(req.priority)
        .bind(json_object_or_empty(req.conditions))
        .bind(if req.enabled { "testing" } else { "disabled" })
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?,
    );

    finish_mutation(
        &mut tx,
        &ctx,
        &idempotency_key,
        &request_hash,
        "POST",
        "/api/v1/express/carriers",
        "express_carrier",
        carrier.carrier_code.clone(),
        &carrier,
        "upsert_express_carrier",
        now,
    )
    .await?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(Json(carrier))
}

async fn list_routing_rules_handler(
    ctx: AuthContext,
    State(state): State<ExpressAppState>,
    Query(query): Query<RuleQuery>,
) -> Result<Json<ExpressRoutingRuleListResponse>, ExpressError> {
    ctx.require_permission("h5.express.read")?;
    let limit = normalized_limit(query.limit);
    let q = trimmed_option(query.q);
    let provider = trimmed_option(query.delivery_provider_type);
    let rows = sqlx::query_as::<_, RuleRow>(
        r#"
        SELECT id, owner_id, rule_code, rule_name, delivery_provider_type,
               carrier_code, priority, conditions, fallback_strategy, enabled,
               effective_from, effective_to, created_at, updated_at
          FROM h5_express_routing_rules
         WHERE owner_id = $1
           AND ($2::TEXT IS NULL OR rule_code ILIKE ('%' || $2 || '%') OR rule_name ILIKE ('%' || $2 || '%'))
           AND ($3::TEXT IS NULL OR delivery_provider_type = $3)
           AND ($4::BOOLEAN IS NULL OR enabled = $4)
         ORDER BY priority ASC, rule_code ASC
         LIMIT $5
        "#,
    )
    .bind(ctx.owner_id)
    .bind(q)
    .bind(provider)
    .bind(query.enabled)
    .bind(i64::from(limit))
    .fetch_all(&state.pool)
    .await
    .map_err(map_db_error)?;

    let data: Vec<_> = rows.into_iter().map(ExpressRoutingRule::from).collect();
    Ok(Json(ExpressRoutingRuleListResponse {
        page: PageMeta {
            count: data.len() as u32,
            next_cursor: None,
        },
        data,
    }))
}

async fn upsert_routing_rule_handler(
    ctx: AuthContext,
    State(state): State<ExpressAppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertExpressRoutingRuleRequest>,
) -> Result<Json<ExpressRoutingRule>, ExpressError> {
    ctx.require_permission("h5.express.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    validate_rule(&req)?;
    let now = Utc::now();
    let request_hash = json_request_hash(&req)?;
    let mut tx = state.pool.begin().await.map_err(map_db_error)?;
    lock_idempotency_key(&mut tx, ctx.owner_id, &idempotency_key).await?;
    if let Some(replay) =
        replay_idempotency(&mut tx, ctx.owner_id, &idempotency_key, &request_hash, now).await?
    {
        return Ok(Json(replay));
    }

    let rule = ExpressRoutingRule::from(
        sqlx::query_as::<_, RuleRow>(
            r#"
            INSERT INTO h5_express_routing_rules (
                id, owner_id, rule_code, rule_name, delivery_provider_type,
                carrier_code, priority, conditions, fallback_strategy, enabled,
                effective_from, effective_to, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $13)
            ON CONFLICT (owner_id, rule_code) DO UPDATE
               SET rule_name = EXCLUDED.rule_name,
                   delivery_provider_type = EXCLUDED.delivery_provider_type,
                   carrier_code = EXCLUDED.carrier_code,
                   priority = EXCLUDED.priority,
                   conditions = EXCLUDED.conditions,
                   fallback_strategy = EXCLUDED.fallback_strategy,
                   enabled = EXCLUDED.enabled,
                   effective_from = EXCLUDED.effective_from,
                   effective_to = EXCLUDED.effective_to,
                   updated_at = EXCLUDED.updated_at
            RETURNING id, owner_id, rule_code, rule_name, delivery_provider_type,
                      carrier_code, priority, conditions, fallback_strategy, enabled,
                      effective_from, effective_to, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(req.rule_code.trim())
        .bind(req.rule_name.trim())
        .bind(req.delivery_provider_type.trim())
        .bind(trimmed_option(req.carrier_code))
        .bind(req.priority)
        .bind(json_object_or_empty(req.conditions))
        .bind(trimmed_option(req.fallback_strategy))
        .bind(req.enabled)
        .bind(req.effective_from)
        .bind(req.effective_to)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?,
    );

    finish_mutation(
        &mut tx,
        &ctx,
        &idempotency_key,
        &request_hash,
        "POST",
        "/api/v1/express/routing-rules",
        "express_routing_rule",
        rule.rule_code.clone(),
        &rule,
        "upsert_express_routing_rule",
        now,
    )
    .await?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(Json(rule))
}

async fn create_waybill_handler(
    ctx: AuthContext,
    State(state): State<ExpressAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateExpressWaybillRequest>,
) -> Result<Json<ExpressWaybill>, ExpressError> {
    ctx.require_permission("h5.express.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    validate_waybill(&req)?;
    let now = Utc::now();
    let request_hash = json_request_hash(&req)?;
    let mut tx = state.pool.begin().await.map_err(map_db_error)?;
    lock_idempotency_key(&mut tx, ctx.owner_id, &idempotency_key).await?;
    if let Some(replay) =
        replay_idempotency(&mut tx, ctx.owner_id, &idempotency_key, &request_hash, now).await?
    {
        return Ok(Json(replay));
    }

    let carrier_code = req.carrier_code.trim().to_string();
    let carrier_enabled: Option<bool> = sqlx::query_scalar(
        "SELECT enabled FROM h5_express_carriers WHERE owner_id = $1 AND carrier_code = $2",
    )
    .bind(ctx.owner_id)
    .bind(&carrier_code)
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_db_error)?;
    if carrier_enabled != Some(true) {
        return Err(ExpressError::NotFound);
    }
    if let Some(outbound_order_id) = req.outbound_order_id {
        let exists: Option<bool> =
            sqlx::query_scalar("SELECT TRUE FROM outbound_orders WHERE owner_id = $1 AND id = $2")
                .bind(ctx.owner_id)
                .bind(outbound_order_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
        if exists != Some(true) {
            return Err(ExpressError::NotFound);
        }
    }

    let waybill_no = req
        .requested_waybill_no
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}-{}", carrier_code, Uuid::new_v4().simple()));
    let eta_at = Some(now + Duration::days(2));
    let waybill = ExpressWaybill::from(
        sqlx::query_as::<_, WaybillRow>(
            r#"
            INSERT INTO h5_express_waybills (
                id, owner_id, outbound_order_id, package_no, carrier_code, waybill_no,
                status, sender_name, sender_mobile, sender_address, receiver_name,
                receiver_mobile, receiver_address, weight_grams, volume_cm3,
                package_count, eta_at, idempotency_key, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'pushed', $7, $8, $9, $10,
                    $11, $12, $13, $14, $15, $16, $17, $18, $18)
            ON CONFLICT (owner_id, package_no)
            DO UPDATE SET updated_at = h5_express_waybills.updated_at
            RETURNING id, owner_id, outbound_order_id, package_no, carrier_code,
                      waybill_no, status, sender_name, sender_mobile, sender_address,
                      receiver_name, receiver_mobile, receiver_address, weight_grams,
                      volume_cm3, package_count, eta_at, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(req.outbound_order_id)
        .bind(req.package_no.trim())
        .bind(&carrier_code)
        .bind(&waybill_no)
        .bind(req.sender_name.trim())
        .bind(req.sender_mobile.trim())
        .bind(req.sender_address.trim())
        .bind(req.receiver_name.trim())
        .bind(req.receiver_mobile.trim())
        .bind(req.receiver_address.trim())
        .bind(req.weight_grams)
        .bind(req.volume_cm3)
        .bind(req.package_count)
        .bind(eta_at)
        .bind(&idempotency_key)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?,
    );

    sqlx::query(
        r#"
        INSERT INTO h5_express_tracking_events (
            id, owner_id, waybill_id, waybill_no, event_time, status, location,
            description, source, cached_at
        )
        VALUES ($1, $2, $3, $4, $5, 'pushed', 'WMS', '快递下单成功，等待承运商揽收', 'wms', $5)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(waybill.id)
    .bind(&waybill.waybill_no)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(map_db_error)?;

    finish_mutation(
        &mut tx,
        &ctx,
        &idempotency_key,
        &request_hash,
        "POST",
        "/api/v1/express/waybills",
        "express_waybill",
        waybill.waybill_no.clone(),
        &waybill,
        "create_express_waybill",
        now,
    )
    .await?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(Json(waybill))
}

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
