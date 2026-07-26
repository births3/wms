//! H9 print orchestration HTTP handlers.

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
    CreateCutoffPlanRequest, CutoffPlan, CutoffPlanListResponse, DeliveryNoteCandidateListResponse,
    DeliveryNoteGroup, DeliveryNoteGroupListResponse, ErrorResponse,
    ManualDeliveryNoteCutoffRequest, PublishRouteBindingRequest, RouteBinding,
    RouteBindingListResponse,
};

use crate::{
    auth::{AuthContext, AuthError},
    document_numbering::DocumentNumberingError,
    print_orchestration::{PrintOrchestrationError, PrintOrchestrationService},
};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const READ_PERMISSION: &str = "h9.print_orchestration.read";
const WRITE_PERMISSION: &str = "h9.print_orchestration.write";

/// H9 print orchestration HTTP state.
#[derive(Clone, Debug)]
pub struct PrintOrchestrationAppState {
    service: PrintOrchestrationService,
}

#[derive(Debug)]
enum PrintOrchestrationHandlerError {
    Auth(AuthError),
    Orchestration(PrintOrchestrationError),
    MissingIdempotencyKey,
}

#[derive(Debug, Deserialize)]
struct WarehouseFilter {
    warehouse_id: Option<Uuid>,
}

impl PrintOrchestrationAppState {
    /// Builds the H9 print orchestration HTTP state.
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            service: PrintOrchestrationService::with_postgres(pool),
        }
    }
}

impl From<AuthError> for PrintOrchestrationHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<PrintOrchestrationError> for PrintOrchestrationHandlerError {
    fn from(value: PrintOrchestrationError) -> Self {
        Self::Orchestration(value)
    }
}

impl IntoResponse for PrintOrchestrationHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Auth(error) => return error.into_response(),
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "H9_PRINT_ORCHESTRATION_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::Orchestration(PrintOrchestrationError::InvalidRequest) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_PRINT_ORCHESTRATION_INVALID",
                "打印编排参数非法",
            ),
            Self::Orchestration(PrintOrchestrationError::EffectivePeriodOverlap) => (
                StatusCode::CONFLICT,
                "H9_PRINT_ORCHESTRATION_PERIOD_OVERLAP",
                "同级配置的有效期重叠",
            ),
            Self::Orchestration(PrintOrchestrationError::CutoffPlanNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_CUTOFF_PLAN_NOT_FOUND",
                "截单计划不存在",
            ),
            Self::Orchestration(PrintOrchestrationError::InvalidState) => (
                StatusCode::CONFLICT,
                "H9_CUTOFF_PLAN_STATE_INVALID",
                "截单计划状态不允许当前操作",
            ),
            Self::Orchestration(PrintOrchestrationError::RouteBindingNotFound) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_ROUTE_BINDING_NOT_FOUND",
                "送货地址没有有效线路绑定",
            ),
            Self::Orchestration(PrintOrchestrationError::OrderNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_DELIVERY_NOTE_ORDER_NOT_FOUND",
                "出库订单不存在或尚未冻结线路",
            ),
            Self::Orchestration(PrintOrchestrationError::OrderNotEligibleForCutoff) => (
                StatusCode::CONFLICT,
                "H9_DELIVERY_NOTE_ORDER_STATE_INVALID",
                "只有已确认的出库订单可以截单",
            ),
            Self::Orchestration(PrintOrchestrationError::AggregationBoundaryMismatch) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_DELIVERY_NOTE_BOUNDARY_MISMATCH",
                "订单不属于同一货主、仓库和送货地址",
            ),
            Self::Orchestration(PrintOrchestrationError::OrderAlreadyCutoff) => (
                StatusCode::CONFLICT,
                "H9_DELIVERY_NOTE_ORDER_ALREADY_CUTOFF",
                "订单已归入随货同行单",
            ),
            Self::Orchestration(PrintOrchestrationError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "H9_PRINT_ORCHESTRATION_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            Self::Orchestration(PrintOrchestrationError::DocumentNumbering(
                DocumentNumberingError::RuleNotFound,
            )) => (
                StatusCode::CONFLICT,
                "H9_DELIVERY_NOTE_NUMBERING_RULE_MISSING",
                "随货同行单编号规则未配置",
            ),
            Self::Orchestration(PrintOrchestrationError::DocumentNumbering(
                DocumentNumberingError::DocumentTypeInvalid,
            )) => (
                StatusCode::CONFLICT,
                "H9_DELIVERY_NOTE_CATEGORY_INVALID",
                "随货同行单分类未启用",
            ),
            Self::Orchestration(PrintOrchestrationError::DocumentNumbering(_))
            | Self::Orchestration(PrintOrchestrationError::Audit(_))
            | Self::Orchestration(PrintOrchestrationError::Database(_))
            | Self::Orchestration(PrintOrchestrationError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H9_DELIVERY_NOTE_CUTOFF_FAILED",
                "随货同行单截单失败",
            ),
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

/// Builds the H9 print orchestration routes.
pub fn print_orchestration_router(state: PrintOrchestrationAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/print-orchestration/delivery-note-candidates",
            get(list_delivery_note_candidates_handler),
        )
        .route(
            "/api/v1/print-orchestration/delivery-note-groups",
            get(list_delivery_note_groups_handler),
        )
        .route(
            "/api/v1/print-orchestration/delivery-note-groups/manual-cutoff",
            post(manual_delivery_note_cutoff_handler),
        )
        .route(
            "/api/v1/print-orchestration/route-bindings",
            get(list_route_bindings_handler).post(publish_route_binding_handler),
        )
        .route(
            "/api/v1/print-orchestration/cutoff-plans",
            get(list_cutoff_plans_handler).post(create_cutoff_plan_handler),
        )
        .route(
            "/api/v1/print-orchestration/cutoff-plans/:plan_id/publish",
            post(publish_cutoff_plan_handler),
        )
        .with_state(state)
}

async fn list_delivery_note_candidates_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Query(query): Query<WarehouseFilter>,
) -> Result<Json<DeliveryNoteCandidateListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state
            .service
            .list_delivery_note_candidates(&ctx, query.warehouse_id)
            .await?,
    ))
}

async fn list_delivery_note_groups_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Query(query): Query<WarehouseFilter>,
) -> Result<Json<DeliveryNoteGroupListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state
            .service
            .list_delivery_note_groups(&ctx, query.warehouse_id)
            .await?,
    ))
}

async fn manual_delivery_note_cutoff_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    headers: HeaderMap,
    Json(request): Json<ManualDeliveryNoteCutoffRequest>,
) -> Result<Json<DeliveryNoteGroup>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .manual_cutoff(&ctx, request, Utc::now(), idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn publish_route_binding_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    headers: HeaderMap,
    Json(request): Json<PublishRouteBindingRequest>,
) -> Result<Json<RouteBinding>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .publish_route_binding(&ctx, request, Utc::now(), idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn list_route_bindings_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Query(query): Query<WarehouseFilter>,
) -> Result<Json<RouteBindingListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state
            .service
            .list_route_bindings(&ctx, query.warehouse_id)
            .await?,
    ))
}

async fn create_cutoff_plan_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateCutoffPlanRequest>,
) -> Result<Json<CutoffPlan>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .create_cutoff_plan(&ctx, request, Utc::now(), idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn list_cutoff_plans_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Query(query): Query<WarehouseFilter>,
) -> Result<Json<CutoffPlanListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state
            .service
            .list_cutoff_plans(&ctx, query.warehouse_id)
            .await?,
    ))
}

async fn publish_cutoff_plan_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(plan_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<CutoffPlan>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .publish_cutoff_plan(&ctx, plan_id, Utc::now(), idempotency_key)
        .await?;
    Ok(Json(result.value))
}

fn idempotency_key_from_headers(
    headers: &HeaderMap,
) -> Result<&str, PrintOrchestrationHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(PrintOrchestrationHandlerError::MissingIdempotencyKey)
}
