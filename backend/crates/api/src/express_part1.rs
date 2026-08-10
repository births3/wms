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
    idempotency,
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

impl From<crate::idempotency::IdempotencyError> for ExpressError {
    fn from(error: crate::idempotency::IdempotencyError) -> Self {
        match error {
            crate::idempotency::IdempotencyError::Conflict => Self::IdempotencyConflict,
            crate::idempotency::IdempotencyError::Database(error) => {
                Self::Database(error.to_string())
            }
            crate::idempotency::IdempotencyError::Serialize(error) => Self::Serialize(error),
        }
    }
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
            total: None,
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
    let path = "/api/v1/express/carriers";
    let mut tx = state.pool.begin().await.map_err(map_db_error)?;
    lock_idempotency_key(&mut tx, ctx.owner_id, &idempotency_key).await?;
    if let Some(replay) = replay_idempotency(
        &mut tx,
        ctx.owner_id,
        &idempotency_key,
        &request_hash,
        "POST",
        path,
        now,
    )
    .await?
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
        path,
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
            total: None,
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
    let path = "/api/v1/express/routing-rules";
    let mut tx = state.pool.begin().await.map_err(map_db_error)?;
    lock_idempotency_key(&mut tx, ctx.owner_id, &idempotency_key).await?;
    if let Some(replay) = replay_idempotency(
        &mut tx,
        ctx.owner_id,
        &idempotency_key,
        &request_hash,
        "POST",
        path,
        now,
    )
    .await?
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
        path,
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
    let path = "/api/v1/express/waybills";
    let mut tx = state.pool.begin().await.map_err(map_db_error)?;
    lock_idempotency_key(&mut tx, ctx.owner_id, &idempotency_key).await?;
    if let Some(replay) = replay_idempotency(
        &mut tx,
        ctx.owner_id,
        &idempotency_key,
        &request_hash,
        "POST",
        path,
        now,
    )
    .await?
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
        path,
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
