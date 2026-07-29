//! Wave 4 cross-module closure handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    CompletePickTaskRequest, CreateOutboundOrderRequest, CreateOutboundWaveRequest,
    DisposeTemperatureExcursionRequest, DriverTaskListResponse, ErrorResponse, OutboundOrder,
    OutboundOrderListResponse, OutboundWave, OutboundWaveListResponse, PageMeta,
    ReviewOutboundOrderRequest, ShipOutboundOrderRequest, StoreDashboardResponse,
    TemperatureExcursionDispositionResponse, TemperatureExcursionEventListResponse,
    TraceabilityOutboundReport, TraceabilityOutboundReportRequest,
};

use crate::{
    audit::AuditWriteRequest,
    auth::{AuthContext, AuthError},
    wave4_repository::{
        PgWave4Repository, TemperatureExcursionDisposition, Wave4RepositoryError,
        APPROVAL_SOURCE_TEMPERATURE_EXCURSION,
    },
};

const M4_READ_PERMISSION: &str = "m4.read";

#[derive(Clone, Debug)]
pub struct Wave4AppState {
    pub wave4_repository: Arc<PgWave4Repository>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ListOutboundOrdersQuery {
    status: Option<String>,
    q: Option<String>,
    limit: Option<u32>,
}

impl Wave4AppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            wave4_repository: Arc::new(PgWave4Repository::new(pool)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Wave4HandlerError {
    Auth(AuthError),
    InvalidIdempotencyKey,
    Repository(Wave4RepositoryError),
}

impl From<AuthError> for Wave4HandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<Wave4RepositoryError> for Wave4HandlerError {
    fn from(value: Wave4RepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl IntoResponse for Wave4HandlerError {
    fn into_response(self) -> Response {
        if let Wave4HandlerError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            Wave4HandlerError::InvalidIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "W4-400",
                "缺少或非法 Idempotency-Key",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::NotFound) => {
                (StatusCode::NOT_FOUND, "W4-404", "资源不存在")
            }
            Wave4HandlerError::Repository(Wave4RepositoryError::IdempotencyConflict) => {
                (StatusCode::CONFLICT, "W4-409", "幂等键已用于不同请求")
            }
            Wave4HandlerError::Repository(Wave4RepositoryError::DuplicateCode) => {
                (StatusCode::CONFLICT, "W4-409", "业务单号重复")
            }
            Wave4HandlerError::Repository(Wave4RepositoryError::OrderAlreadyInWave) => {
                (StatusCode::CONFLICT, "W4-409", "订单已加入其他波次")
            }
            Wave4HandlerError::Repository(Wave4RepositoryError::MissingSecondReviewer) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M4_DUAL_PERSON_REQUIRED",
                "M-VR 策略要求第二复核员",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::UnqualifiedSecondReviewer) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M4_SECOND_REVIEWER_UNAUTHORIZED",
                "第二复核员不是当前货主的有效保管员",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::DualPersonApprovalRequired) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M4_DUAL_PERSON_APPROVAL_REQUIRED",
                "M-VR 策略要求先完成主管审批",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::EmptySelection)
            | Wave4HandlerError::Repository(Wave4RepositoryError::BatchNotAffected(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidQuantity)
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidDocumentType)
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidDeliveryAddress)
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidTraceabilityEvent)
            | Wave4HandlerError::Repository(Wave4RepositoryError::ShortPickNotReplenished)
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidDriver)
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidSignatureAttachment)
            | Wave4HandlerError::Repository(Wave4RepositoryError::ReviewValidation(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::ShipmentValidation(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidStatus { .. })
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidStateTransition {
                ..
            }) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "W4-422",
                "业务规则校验失败",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::Audit(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::DocumentNumbering(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::Database(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "W4-500",
                "持久化或审计写入失败",
            ),
            Wave4HandlerError::Auth(_) => unreachable!("auth error returned above"),
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

pub fn wave4_router(state: Wave4AppState) -> Router {
    Router::new()
        .route(
            "/api/v1/outbound/orders",
            get(list_outbound_orders_handler).post(create_outbound_order_handler),
        )
        .route(
            "/api/v1/outbound/orders/:order_id",
            get(get_outbound_order_handler),
        )
        .route(
            "/api/v1/outbound/waves",
            get(list_outbound_waves_handler).post(create_outbound_wave_handler),
        )
        .route(
            "/api/v1/outbound/waves/:wave_id",
            get(get_outbound_wave_handler),
        )
        .route(
            "/api/v1/outbound/waves/:wave_id/cancel",
            post(cancel_outbound_wave_handler),
        )
        .route(
            "/api/v1/outbound/pick-tasks/:id/complete",
            post(complete_pick_task_handler),
        )
        .route(
            "/api/v1/outbound/orders/:id/review",
            get(get_outbound_review_handler).post(review_outbound_order_handler),
        )
        .route(
            "/api/v1/outbound/orders/:id/ship",
            post(ship_outbound_order_handler),
        )
        .route(
            "/api/v1/cold-chain/excursions/pending-disposition",
            get(list_pending_temperature_excursions_handler),
        )
        .route(
            "/api/v1/cold-chain/excursions/:external_event_id/dispose",
            post(dispose_temperature_excursion_handler),
        )
        .route(
            "/api/v1/traceability/outbound-reports",
            post(create_traceability_outbound_report_handler),
        )
        .route(
            "/api/v1/driver/tasks/today",
            get(list_driver_today_tasks_handler),
        )
        .route("/api/v1/store/dashboard", get(get_store_dashboard_handler))
        .with_state(state)
}

pub fn postgres_outbound(pool: PgPool) -> Router {
    wave4_router(Wave4AppState::with_postgres(pool))
}

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
        .wave4_repository
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
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "ship_outbound_order",
        "M4",
        "outbound_order",
        id.to_string(),
        None,
    );
    let outcome = state
        .wave4_repository
        .ship_outbound_order(&ctx, id, req, now, &idempotency_key, Some(audit))
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

async fn list_pending_temperature_excursions_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
) -> Result<Json<TemperatureExcursionEventListResponse>, Wave4HandlerError> {
    ctx.require_permission("m5.write")?;
    let events = state
        .wave4_repository
        .list_pending_temperature_excursions(&ctx)
        .await?;
    let count = events.len() as u32;
    Ok(Json(TemperatureExcursionEventListResponse {
        data: events,
        page: PageMeta {
            next_cursor: None,
            count,
        },
    }))
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, Wave4HandlerError> {
    let value = headers
        .get("idempotency-key")
        .or_else(|| headers.get("Idempotency-Key"))
        .ok_or(Wave4HandlerError::InvalidIdempotencyKey)?;
    let key = value
        .to_str()
        .map_err(|_| Wave4HandlerError::InvalidIdempotencyKey)?
        .trim();
    if key.is_empty() {
        return Err(Wave4HandlerError::InvalidIdempotencyKey);
    }
    Ok(key.to_string())
}

fn require_any_permission(ctx: &AuthContext, permissions: &[&str]) -> Result<(), AuthError> {
    if permissions
        .iter()
        .any(|permission| ctx.has_permission(permission))
    {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied(permissions.join("|")))
    }
}

async fn dispose_temperature_excursion_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    Path(external_event_id): Path<String>,
    Json(req): Json<DisposeTemperatureExcursionRequest>,
) -> Result<Json<TemperatureExcursionDispositionResponse>, Wave4HandlerError> {
    ctx.require_permission("m5.write")?;
    let now = Utc::now();
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "dispose_temperature_excursion",
        "M5",
        "temperature_excursion",
        external_event_id.clone(),
        None,
    );
    let disposition = state
        .wave4_repository
        .dispose_temperature_excursion_and_quarantine_batches(
            &ctx,
            &external_event_id,
            req.selected_batch_ids,
            now,
            Some(audit),
        )
        .await?;

    Ok(Json(disposition_response(disposition)))
}

async fn create_traceability_outbound_report_handler(
    ctx: AuthContext,
    State(state): State<Wave4AppState>,
    headers: HeaderMap,
    Json(req): Json<TraceabilityOutboundReportRequest>,
) -> Result<Json<TraceabilityOutboundReport>, Wave4HandlerError> {
    ctx.require_permission("m-tc.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create_traceability_outbound_report",
        "M-TC",
        "traceability_outbound_report",
        "pending".to_string(),
        None,
    );
    let outcome = state
        .wave4_repository
        .create_traceability_outbound_report(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

fn disposition_response(
    disposition: TemperatureExcursionDisposition,
) -> TemperatureExcursionDispositionResponse {
    TemperatureExcursionDispositionResponse {
        event: disposition.event,
        quarantined_batches: disposition.quarantined_batches,
        approval_source: APPROVAL_SOURCE_TEMPERATURE_EXCURSION.to_string(),
    }
}

async fn list_driver_today_tasks_handler(
    ctx: AuthContext,
    State(_state): State<Wave4AppState>,
) -> Result<Json<DriverTaskListResponse>, Wave4HandlerError> {
    ctx.require_permission("h-driver.read")?;
    Ok(Json(DriverTaskListResponse {
        data: Vec::new(),
        page: PageMeta {
            next_cursor: None,
            count: 0,
        },
    }))
}

async fn get_store_dashboard_handler(
    ctx: AuthContext,
    State(_state): State<Wave4AppState>,
) -> Result<Json<StoreDashboardResponse>, Wave4HandlerError> {
    ctx.require_permission("h-store.read")?;
    Ok(Json(StoreDashboardResponse {
        store_id: None,
        pending_receipt_orders: 0,
        in_transit_orders: 0,
        signed_orders_last_7_days: 0,
        inventory_alert_count: 0,
        returns_this_month: 0,
        exceptions_this_month: 0,
        generated_at: Utc::now(),
    }))
}

include!("wave4_handlers_waves.rs");

#[cfg(test)]
#[path = "wave4_handlers_review_tests.rs"]
mod review_tests;

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;
    use axum::{extract::State, Json};
    use chrono::{NaiveDate, TimeZone, Utc};
    use sqlx::PgPool;
    use uuid::Uuid;
    use wms_domain::{
        CreateOutboundOrderRequest, DisposeTemperatureExcursionRequest,
        TraceabilityOutboundReportRequest, TraceabilityStatusChangeEvent,
    };

    use super::{
        create_outbound_order_handler, create_traceability_outbound_report_handler,
        dispose_temperature_excursion_handler, get_store_dashboard_handler,
        list_driver_today_tasks_handler, list_pending_temperature_excursions_handler, wave4_router,
        Wave4AppState, Wave4HandlerError,
    };
    use crate::{
        auth::{AuthContext, AuthError},
        inventory::{STATUS_QUALIFIED, STATUS_QUARANTINED},
    };

    fn ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "wave4-handler-test".to_string(),
            permissions: permissions
                .iter()
                .map(|permission| permission.to_string())
                .collect(),
            jti: Uuid::new_v4().to_string(),
            warehouse_scope: None,
        }
    }

    async fn seed_inventory_batch(
        pool: &PgPool,
        owner_id: Uuid,
        batch_no: &str,
        now: chrono::DateTime<Utc>,
    ) -> Uuid {
        let batch_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO inventory_batches (
                id, owner_id, product_code, batch_no, production_date, expiry_date,
                qty_on_hand, qty_locked, quality_status, location_id, location_code,
                recall_flag, created_at, updated_at
            )
            VALUES ($1, $2, 'P-COLD-001', $3, $4, $5, 10, 0, $6, $7, 'COLD-A-01', FALSE, $8, $8)
            "#,
        )
        .bind(batch_id)
        .bind(owner_id)
        .bind(batch_no)
        .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
        .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid date"))
        .bind(STATUS_QUALIFIED)
        .bind(Uuid::new_v4())
        .bind(now)
        .execute(pool)
        .await
        .expect("seed inventory batch");
        batch_id
    }

    async fn seed_temperature_excursion(
        pool: &PgPool,
        owner_id: Uuid,
        external_event_id: &str,
        affected_batch_ids: Vec<Uuid>,
        now: chrono::DateTime<Utc>,
    ) -> Uuid {
        let event_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO temperature_excursion_events (
                id, owner_id, external_event_id, device_code, location_code,
                started_at, ended_at, min_temperature_celsius, max_temperature_celsius,
                affected_batch_ids, status, created_at
            )
            VALUES ($1, $2, $3, 'TEMP-W4-HANDLER-001', 'COLD-A', $4, $5, 1.0, 12.0, $6, 'pending_disposition', $5)
            "#,
        )
        .bind(event_id)
        .bind(owner_id)
        .bind(external_event_id)
        .bind(now - chrono::Duration::minutes(20))
        .bind(now)
        .bind(&affected_batch_ids)
        .execute(pool)
        .await
        .expect("seed temperature excursion");
        event_id
    }

    #[tokio::test]
    async fn wave4_router_registers_cold_chain_disposition_handlers() {
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during router registration");
        let _router = wave4_router(Wave4AppState::with_postgres(pool));
    }

    #[tokio::test]
    async fn outbound_write_handler_requires_m4_permission_before_postgres() {
        let owner_id = Uuid::new_v4();
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during handler auth test");
        let state = Wave4AppState::with_postgres(pool);

        let result = create_outbound_order_handler(
            ctx(owner_id, &[]),
            State(state),
            HeaderMap::new(),
            Json(CreateOutboundOrderRequest {
                document_type: "sales_outbound".to_string(),
                wms_order_no: "WMS-R-20260604-001".to_string(),
                erp_order_no: Some("ERP-SO-001".to_string()),
                customer_id: Uuid::new_v4(),
                delivery_address_id: Uuid::new_v4(),
                warehouse_id: Uuid::new_v4(),
                required_ship_at: None,
                lines: vec![],
            }),
        )
        .await
        .expect_err("m4.write should be checked before repository access");

        assert!(matches!(
            result,
            Wave4HandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == "m4.write"
        ));
    }

    #[tokio::test]
    async fn outbound_write_handler_requires_idempotency_key_before_postgres() {
        let owner_id = Uuid::new_v4();
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during handler idempotency test");
        let state = Wave4AppState::with_postgres(pool);

        let result = create_outbound_order_handler(
            ctx(owner_id, &["m4.write"]),
            State(state),
            HeaderMap::new(),
            Json(CreateOutboundOrderRequest {
                document_type: "sales_outbound".to_string(),
                wms_order_no: "WMS-R-20260604-002".to_string(),
                erp_order_no: Some("ERP-SO-002".to_string()),
                customer_id: Uuid::new_v4(),
                delivery_address_id: Uuid::new_v4(),
                warehouse_id: Uuid::new_v4(),
                required_ship_at: None,
                lines: vec![],
            }),
        )
        .await
        .expect_err("Idempotency-Key should be required before repository access");

        assert_eq!(result, Wave4HandlerError::InvalidIdempotencyKey);
    }

    #[tokio::test]
    async fn traceability_report_handler_requires_permission_and_idempotency_before_postgres() {
        let owner_id = Uuid::new_v4();
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during handler traceability test");
        let state = Wave4AppState::with_postgres(pool);
        let req = TraceabilityOutboundReportRequest {
            events: vec![TraceabilityStatusChangeEvent {
                event_id: Uuid::new_v4(),
                trace_code: "TC-W4-HANDLER-001".to_string(),
                status_change_type: "已入库→已出库".to_string(),
                occurred_at: Utc
                    .with_ymd_and_hms(2026, 6, 5, 9, 0, 0)
                    .single()
                    .expect("valid time"),
            }],
        };

        let denied = create_traceability_outbound_report_handler(
            ctx(owner_id, &[]),
            State(state.clone()),
            HeaderMap::new(),
            Json(req.clone()),
        )
        .await
        .expect_err("m-tc.write should be checked before repository access");
        assert!(matches!(
            denied,
            Wave4HandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == "m-tc.write"
        ));

        let missing_key = create_traceability_outbound_report_handler(
            ctx(owner_id, &["m-tc.write"]),
            State(state),
            HeaderMap::new(),
            Json(req),
        )
        .await
        .expect_err("Idempotency-Key should be required before repository access");
        assert_eq!(missing_key, Wave4HandlerError::InvalidIdempotencyKey);
    }

    #[tokio::test]
    async fn driver_and_store_read_handlers_require_actor_permissions() {
        let owner_id = Uuid::new_v4();
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during handler test");
        let state = Wave4AppState::with_postgres(pool);

        let driver_denied =
            list_driver_today_tasks_handler(ctx(owner_id, &[]), State(state.clone()))
                .await
                .expect_err("driver task list should require h-driver.read");
        assert!(matches!(
            driver_denied,
            Wave4HandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == "h-driver.read"
        ));

        let Json(tasks) = list_driver_today_tasks_handler(
            ctx(owner_id, &["h-driver.read"]),
            State(state.clone()),
        )
        .await
        .expect("authorized driver should read tasks");
        assert_eq!(tasks.page.count, 0);

        let store_denied = get_store_dashboard_handler(ctx(owner_id, &[]), State(state.clone()))
            .await
            .expect_err("store dashboard should require h-store.read");
        assert!(matches!(
            store_denied,
            Wave4HandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == "h-store.read"
        ));

        let Json(dashboard) =
            get_store_dashboard_handler(ctx(owner_id, &["h-store.read"]), State(state))
                .await
                .expect("authorized store user should read dashboard");
        assert_eq!(dashboard.pending_receipt_orders, 0);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn postgres_cold_chain_disposition_handler_lists_and_quarantines(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let authorized = ctx(owner_id, &["m5.write"]);
        let denied = ctx(owner_id, &[]);
        let state = Wave4AppState::with_postgres(pool.clone());
        let now = Utc
            .with_ymd_and_hms(2026, 6, 5, 10, 0, 0)
            .single()
            .expect("valid time");
        let batch_id = seed_inventory_batch(&pool, owner_id, "B-W4-HANDLER-001", now).await;
        let event_id = seed_temperature_excursion(
            &pool,
            owner_id,
            "TEMP-EXT-W4-HANDLER-001",
            vec![batch_id],
            now,
        )
        .await;

        let denied_result =
            list_pending_temperature_excursions_handler(denied, State(state.clone()))
                .await
                .expect_err("m5.write should be required");
        assert!(matches!(
            denied_result,
            Wave4HandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == "m5.write"
        ));

        let Json(pending) =
            list_pending_temperature_excursions_handler(authorized.clone(), State(state.clone()))
                .await
                .expect("pending list should load");
        assert_eq!(pending.page.count, 1);
        assert_eq!(pending.data[0].id, event_id);

        let Json(disposition) = dispose_temperature_excursion_handler(
            authorized,
            State(state),
            axum::extract::Path("TEMP-EXT-W4-HANDLER-001".to_string()),
            Json(DisposeTemperatureExcursionRequest {
                selected_batch_ids: vec![batch_id],
            }),
        )
        .await
        .expect("disposition should quarantine batch");

        assert_eq!(disposition.event.status, "disposed");
        assert_eq!(disposition.quarantined_batches.len(), 1);
        assert_eq!(
            disposition.quarantined_batches[0].quality_status,
            STATUS_QUARANTINED
        );

        let counts: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM temperature_excursion_events
                  WHERE owner_id = $1 AND status = 'pending_disposition'),
                (SELECT COUNT(*) FROM inventory_status_changes WHERE owner_id = $1),
                (SELECT COUNT(*) FROM audit_event
                  WHERE owner_id = $1 AND action = 'dispose_temperature_excursion')
            "#,
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("counts");
        assert_eq!(counts, (0, 1, 1));
    }
}
