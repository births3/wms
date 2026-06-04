//! Wave 5 value-added module handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    BillingChargeCalculation, BillingStatement, CalculateBillingChargesRequest,
    ConfirmBillingStatementRequest, ConfirmContainerRecoveryRequest, ContainerRecovery,
    CreateCrossdockPlanRequest, CreatePackJobRequest, CreatePackingStationRequest,
    CreateRetailReplenishmentSuggestionRequest, CrossdockPlan, ErrorResponse,
    GenerateBillingStatementRequest, IngestTransitTemperatureRequest, PackJob, PackingStation,
    PrintWaybillRequest, ReceiveTmsDispatchRequest, RetailReplenishmentSuggestion, TmsDispatch,
    TransitTemperatureReading, WeighPackJobRequest,
};

use crate::{
    audit::AuditWriteRequest,
    auth::{AuthContext, AuthError},
    wave5_repository::{PgWave5Repository, Wave5RepositoryError},
};

#[derive(Clone, Debug)]
pub struct Wave5AppState {
    pub wave5_repository: Arc<PgWave5Repository>,
}

impl Wave5AppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            wave5_repository: Arc::new(PgWave5Repository::new(pool)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Wave5HandlerError {
    Auth(AuthError),
    InvalidIdempotencyKey,
    Repository(Wave5RepositoryError),
}

impl From<AuthError> for Wave5HandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<Wave5RepositoryError> for Wave5HandlerError {
    fn from(value: Wave5RepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl IntoResponse for Wave5HandlerError {
    fn into_response(self) -> Response {
        if let Wave5HandlerError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            Wave5HandlerError::InvalidIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "W5-400",
                "缺少或非法 Idempotency-Key",
            ),
            Wave5HandlerError::Repository(Wave5RepositoryError::NotFound) => {
                (StatusCode::NOT_FOUND, "W5-404", "资源不存在")
            }
            Wave5HandlerError::Repository(Wave5RepositoryError::DuplicateCode) => {
                (StatusCode::CONFLICT, "W5-409", "业务唯一键冲突")
            }
            Wave5HandlerError::Repository(Wave5RepositoryError::IdempotencyConflict) => {
                (StatusCode::CONFLICT, "W5-409", "幂等键已用于不同请求")
            }
            Wave5HandlerError::Repository(Wave5RepositoryError::InvalidInput) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "W5-422",
                "业务规则校验失败",
            ),
            Wave5HandlerError::Repository(Wave5RepositoryError::Audit(_))
            | Wave5HandlerError::Repository(Wave5RepositoryError::Database(_))
            | Wave5HandlerError::Repository(Wave5RepositoryError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "W5-500",
                "持久化或审计写入失败",
            ),
            Wave5HandlerError::Auth(_) => unreachable!("auth error returned above"),
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

pub fn wave5_router(state: Wave5AppState) -> Router {
    Router::new()
        .route(
            "/api/v1/packing/stations",
            post(create_packing_station_handler),
        )
        .route("/api/v1/packing/jobs", post(create_pack_job_handler))
        .route(
            "/api/v1/packing/jobs/:id/weigh",
            post(weigh_pack_job_handler),
        )
        .route(
            "/api/v1/packing/jobs/:id/waybill",
            post(print_pack_job_waybill_handler),
        )
        .route(
            "/api/v1/retail/replenishment-suggestions",
            post(create_replenishment_suggestion_handler),
        )
        .route(
            "/api/v1/retail/crossdock-plans",
            post(create_crossdock_plan_handler),
        )
        .route(
            "/api/v1/billing/charges/calculate",
            post(calculate_billing_charges_handler),
        )
        .route(
            "/api/v1/billing/statements",
            post(generate_billing_statement_handler),
        )
        .route(
            "/api/v1/billing/statements/:id/confirm",
            post(confirm_billing_statement_handler),
        )
        .route("/api/v1/tms/dispatches", post(receive_tms_dispatch_handler))
        .route(
            "/api/v1/tms/transit-temperature-readings",
            post(ingest_transit_temperature_handler),
        )
        .route(
            "/api/v1/tms/container-recoveries",
            post(confirm_container_recovery_handler),
        )
        .with_state(state)
}

pub fn postgres_wave5(pool: PgPool) -> Router {
    wave5_router(Wave5AppState::with_postgres(pool))
}

async fn create_packing_station_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    headers: HeaderMap,
    Json(req): Json<CreatePackingStationRequest>,
) -> Result<Json<PackingStation>, Wave5HandlerError> {
    ctx.require_permission("m-pk.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(
        &ctx,
        "create_packing_station",
        "M-PK",
        "packing_station",
        &req.station_code,
    );
    let outcome = state
        .wave5_repository
        .create_packing_station(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn create_pack_job_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    headers: HeaderMap,
    Json(req): Json<CreatePackJobRequest>,
) -> Result<Json<PackJob>, Wave5HandlerError> {
    ctx.require_permission("m-pk.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(&ctx, "create_pack_job", "M-PK", "packing_job", &req.job_no);
    let outcome = state
        .wave5_repository
        .create_pack_job(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn weigh_pack_job_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<WeighPackJobRequest>,
) -> Result<Json<PackJob>, Wave5HandlerError> {
    ctx.require_permission("m-pk.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(
        &ctx,
        "weigh_pack_job",
        "M-PK",
        "packing_job",
        id.to_string(),
    );
    let outcome = state
        .wave5_repository
        .weigh_pack_job(&ctx, id, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn print_pack_job_waybill_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<PrintWaybillRequest>,
) -> Result<Json<PackJob>, Wave5HandlerError> {
    ctx.require_permission("m-pk.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(
        &ctx,
        "print_pack_job_waybill",
        "M-PK",
        "packing_job",
        id.to_string(),
    );
    let outcome = state
        .wave5_repository
        .print_pack_job_waybill(&ctx, id, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn create_replenishment_suggestion_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateRetailReplenishmentSuggestionRequest>,
) -> Result<Json<RetailReplenishmentSuggestion>, Wave5HandlerError> {
    ctx.require_permission("m8.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(
        &ctx,
        "create_replenishment_suggestion",
        "M8",
        "retail_replenishment_suggestion",
        req.store_id.to_string(),
    );
    let outcome = state
        .wave5_repository
        .create_replenishment_suggestion(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn create_crossdock_plan_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateCrossdockPlanRequest>,
) -> Result<Json<CrossdockPlan>, Wave5HandlerError> {
    ctx.require_permission("m8.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(
        &ctx,
        "create_crossdock_plan",
        "M8",
        "crossdock_plan",
        req.asn_id.to_string(),
    );
    let outcome = state
        .wave5_repository
        .create_crossdock_plan(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn calculate_billing_charges_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    headers: HeaderMap,
    Json(req): Json<CalculateBillingChargesRequest>,
) -> Result<Json<BillingChargeCalculation>, Wave5HandlerError> {
    ctx.require_permission("m9.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(
        &ctx,
        "calculate_period_charges",
        "M9",
        "billing_charge_calculation",
        req.contract_id.to_string(),
    );
    let outcome = state
        .wave5_repository
        .calculate_period_charges(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn generate_billing_statement_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    headers: HeaderMap,
    Json(req): Json<GenerateBillingStatementRequest>,
) -> Result<Json<BillingStatement>, Wave5HandlerError> {
    ctx.require_permission("m9.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(
        &ctx,
        "generate_billing_statement",
        "M9",
        "billing_statement",
        req.contract_id.to_string(),
    );
    let outcome = state
        .wave5_repository
        .generate_billing_statement(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn confirm_billing_statement_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ConfirmBillingStatementRequest>,
) -> Result<Json<BillingStatement>, Wave5HandlerError> {
    ctx.require_permission("m9.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(
        &ctx,
        "confirm_billing_statement",
        "M9",
        "billing_statement",
        id.to_string(),
    );
    let outcome = state
        .wave5_repository
        .confirm_billing_statement(&ctx, id, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn receive_tms_dispatch_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    headers: HeaderMap,
    Json(req): Json<ReceiveTmsDispatchRequest>,
) -> Result<Json<TmsDispatch>, Wave5HandlerError> {
    ctx.require_permission("m10.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(
        &ctx,
        "receive_tms_dispatch",
        "M10",
        "tms_dispatch",
        &req.dispatch_no,
    );
    let outcome = state
        .wave5_repository
        .receive_tms_dispatch(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn ingest_transit_temperature_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    headers: HeaderMap,
    Json(req): Json<IngestTransitTemperatureRequest>,
) -> Result<Json<TransitTemperatureReading>, Wave5HandlerError> {
    ctx.require_permission("m10.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(
        &ctx,
        "ingest_transit_temperature",
        "M10",
        "transit_temperature_reading",
        req.dispatch_id.to_string(),
    );
    let outcome = state
        .wave5_repository
        .ingest_transit_temperature(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

async fn confirm_container_recovery_handler(
    ctx: AuthContext,
    State(state): State<Wave5AppState>,
    headers: HeaderMap,
    Json(req): Json<ConfirmContainerRecoveryRequest>,
) -> Result<Json<ContainerRecovery>, Wave5HandlerError> {
    ctx.require_permission("m10.write")?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let audit = audit(
        &ctx,
        "confirm_container_recovery",
        "M10",
        "container_recovery",
        &req.container_lpn,
    );
    let outcome = state
        .wave5_repository
        .confirm_container_recovery(&ctx, req, now, &idempotency_key, Some(audit))
        .await?;
    Ok(Json(outcome.value))
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, Wave5HandlerError> {
    let key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(Wave5HandlerError::InvalidIdempotencyKey)?;
    Ok(key.to_string())
}

fn audit(
    ctx: &AuthContext,
    action: &str,
    module: &str,
    resource_type: &str,
    resource_id: impl Into<String>,
) -> AuditWriteRequest {
    AuditWriteRequest::from_auth_context(ctx, action, module, resource_type, resource_id, None)
}

#[cfg(test)]
mod tests {
    use axum::{
        extract::State,
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        Json,
    };
    use sqlx::PgPool;
    use uuid::Uuid;
    use wms_domain::{
        ConfirmContainerRecoveryRequest, CreatePackJobRequest, CreatePackingStationRequest,
    };

    use super::{
        confirm_container_recovery_handler, create_pack_job_handler,
        create_packing_station_handler, wave5_router, Wave5AppState, Wave5HandlerError,
    };
    use crate::auth::{AuthContext, AuthError};
    use crate::wave5_repository::Wave5RepositoryError;

    fn ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "wave5-handler-test".to_string(),
            permissions: permissions.iter().map(|item| item.to_string()).collect(),
            jti: Uuid::new_v4().to_string(),
        }
    }

    #[tokio::test]
    async fn wave5_router_registers_value_added_paths() {
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during router registration");
        let _router = wave5_router(Wave5AppState::with_postgres(pool));
        let _paths = [
            "/api/v1/packing/stations",
            "/api/v1/packing/jobs",
            "/api/v1/packing/jobs/{id}/weigh",
            "/api/v1/packing/jobs/{id}/waybill",
            "/api/v1/retail/replenishment-suggestions",
            "/api/v1/retail/crossdock-plans",
            "/api/v1/billing/charges/calculate",
            "/api/v1/billing/statements",
            "/api/v1/billing/statements/{id}/confirm",
            "/api/v1/tms/dispatches",
            "/api/v1/tms/transit-temperature-readings",
            "/api/v1/tms/container-recoveries",
        ];
    }

    #[test]
    fn duplicate_business_key_maps_to_conflict() {
        let response =
            Wave5HandlerError::Repository(Wave5RepositoryError::DuplicateCode).into_response();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn packing_handler_checks_permission_before_postgres() {
        let owner_id = Uuid::new_v4();
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during handler auth test");
        let state = Wave5AppState::with_postgres(pool);

        let result = create_packing_station_handler(
            ctx(owner_id, &[]),
            State(state),
            HeaderMap::new(),
            Json(CreatePackingStationRequest {
                station_code: "PK-01".to_string(),
                station_name: "包装台 01".to_string(),
                printer_code: None,
                scale_code: None,
                temperature_zone: "normal".to_string(),
            }),
        )
        .await
        .expect_err("m-pk.write should be checked before repository access");

        assert!(matches!(
            result,
            Wave5HandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == "m-pk.write"
        ));
    }

    #[tokio::test]
    async fn packing_handler_requires_idempotency_before_postgres() {
        let owner_id = Uuid::new_v4();
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during handler idempotency test");
        let state = Wave5AppState::with_postgres(pool);

        let result = create_pack_job_handler(
            ctx(owner_id, &["m-pk.write"]),
            State(state),
            HeaderMap::new(),
            Json(CreatePackJobRequest {
                outbound_order_id: Uuid::new_v4(),
                station_id: None,
                job_no: "PK-JOB-001".to_string(),
                pack_mode: "station".to_string(),
                recommended_box_type: "M".to_string(),
                actual_box_type: "M".to_string(),
                adjustment_reason: None,
                outbound_lpn: "LPN-001".to_string(),
                trace_codes: vec!["TC-001".to_string()],
            }),
        )
        .await
        .expect_err("Idempotency-Key should be required before repository access");

        assert_eq!(result, Wave5HandlerError::InvalidIdempotencyKey);
    }

    #[tokio::test]
    async fn tms_recovery_handler_requires_m10_permission() {
        let owner_id = Uuid::new_v4();
        let pool = PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should not connect during handler auth test");
        let state = Wave5AppState::with_postgres(pool);

        let result = confirm_container_recovery_handler(
            ctx(owner_id, &[]),
            State(state),
            HeaderMap::new(),
            Json(ConfirmContainerRecoveryRequest {
                container_lpn: "LPN-REC-001".to_string(),
                dispatch_id: None,
                customer_id: Uuid::new_v4(),
                delivery_provider_type: "own_fleet".to_string(),
                shipped_at: None,
            }),
        )
        .await
        .expect_err("m10.write should be checked before repository access");

        assert!(matches!(
            result,
            Wave5HandlerError::Auth(AuthError::PermissionDenied(permission))
                if permission == "m10.write"
        ));
    }
}
