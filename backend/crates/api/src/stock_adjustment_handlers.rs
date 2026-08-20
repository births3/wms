use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    CreateStockLossOrderRequest, CreateStockSurplusOrderRequest, ErrorResponse,
    ExecuteStockLossOrderRequest, ExecuteStockSurplusOrderRequest, StockLossOrder,
    StockLossQualityApprovalRequest, StockSurplusOrder, StockSurplusQualityApprovalRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    stock_adjustment::{PgStockAdjustmentRepository, StockAdjustmentError},
};

const READ_PERMISSION: &str = "msa.stock-adjustment.read";
const WRITE_PERMISSION: &str = "msa.stock-adjustment.write";
const EXECUTE_PERMISSION: &str = "msa.stock-adjustment.execute";
const QUALITY_APPROVE_PERMISSION: &str = "msa.stock-adjustment.quality-approve";

#[derive(Clone, Debug)]
pub struct StockAdjustmentAppState {
    repository: PgStockAdjustmentRepository,
}

#[derive(Debug)]
pub enum StockAdjustmentHandlerError {
    Auth(AuthError),
    StockAdjustment(StockAdjustmentError),
    MissingIdempotencyKey,
}

impl StockAdjustmentAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: PgStockAdjustmentRepository::new(pool),
        }
    }
}

impl From<AuthError> for StockAdjustmentHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<StockAdjustmentError> for StockAdjustmentHandlerError {
    fn from(value: StockAdjustmentError) -> Self {
        Self::StockAdjustment(value)
    }
}

impl IntoResponse for StockAdjustmentHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "SA_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::StockAdjustment(StockAdjustmentError::NotFound) => (
                StatusCode::NOT_FOUND,
                "SA_ORDER_NOT_FOUND",
                "库存调整单、仓库、商品或库存批次不存在",
            ),
            Self::StockAdjustment(StockAdjustmentError::CrossOwner) => (
                StatusCode::FORBIDDEN,
                "SA_CROSS_OWNER",
                "禁止跨货主访问库存调整单",
            ),
            Self::StockAdjustment(StockAdjustmentError::InvalidRequest) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "SA_REQUEST_INVALID",
                "库存调整请求参数不完整",
            ),
            Self::StockAdjustment(StockAdjustmentError::InvalidStatus { .. }) => (
                StatusCode::CONFLICT,
                "SA_STATUS_CONFLICT",
                "库存调整单当前状态不允许执行该操作",
            ),
            Self::StockAdjustment(StockAdjustmentError::QuantityExceeded) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "SA_QTY_EXCEEDED",
                "报损数量超过可用库存",
            ),
            Self::StockAdjustment(StockAdjustmentError::LocationUnreachable) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LOCATION_UNREACHABLE",
                "格口处于 AGV 搬运不可达期",
            ),
            Self::StockAdjustment(StockAdjustmentError::InvalidPutawayTarget) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "SA_PUTAWAY_TARGET_INVALID",
                "报溢目标库位不符合温区、色标或容量规则",
            ),
            Self::StockAdjustment(StockAdjustmentError::MissingSecondOperator) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "SA_SECOND_OPERATOR_REQUIRED",
                "当前策略要求第二操作人扫码",
            ),
            Self::StockAdjustment(StockAdjustmentError::SameOperator) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "SA_SAME_OPERATOR",
                "两名操作人不能相同",
            ),
            Self::StockAdjustment(StockAdjustmentError::UnqualifiedOperator) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "SA_OPERATOR_UNQUALIFIED",
                "操作人无有效保管员资格",
            ),
            Self::StockAdjustment(StockAdjustmentError::DifferentFirstOperator) => (
                StatusCode::CONFLICT,
                "SA_FIRST_OPERATOR_CHANGED",
                "开始与完成库存调整的第一操作人必须一致",
            ),
            Self::StockAdjustment(StockAdjustmentError::DualPersonApprovalRequired) => (
                StatusCode::CONFLICT,
                "SA_DUAL_PERSON_APPROVAL_REQUIRED",
                "当前双人策略要求仓库主管审批",
            ),
            Self::StockAdjustment(StockAdjustmentError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "SA_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            Self::StockAdjustment(
                StockAdjustmentError::DocumentNumbering(_)
                | StockAdjustmentError::Audit(_)
                | StockAdjustmentError::Database(_)
                | StockAdjustmentError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SA_INTERNAL",
                "库存调整操作处理失败",
            ),
            Self::Auth(_) => unreachable!("auth error returned above"),
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

pub fn stock_adjustment_router(state: StockAdjustmentAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/stock-adjustments/loss-orders",
            post(create_loss_order_handler),
        )
        .route(
            "/api/v1/stock-adjustments/loss-orders/:id",
            get(get_loss_order_handler),
        )
        .route(
            "/api/v1/stock-adjustments/loss-orders/:id/quality-approval",
            post(record_quality_approval_handler),
        )
        .route(
            "/api/v1/stock-adjustments/loss-orders/:id/start",
            post(start_loss_order_handler),
        )
        .route(
            "/api/v1/stock-adjustments/loss-orders/:id/execute",
            post(execute_loss_order_handler),
        )
        .route(
            "/api/v1/stock-adjustments/surplus-orders",
            post(create_surplus_order_handler),
        )
        .route(
            "/api/v1/stock-adjustments/surplus-orders/:id",
            get(get_surplus_order_handler),
        )
        .route(
            "/api/v1/stock-adjustments/surplus-orders/:id/quality-approval",
            post(record_surplus_quality_approval_handler),
        )
        .route(
            "/api/v1/stock-adjustments/surplus-orders/:id/start",
            post(start_surplus_order_handler),
        )
        .route(
            "/api/v1/stock-adjustments/surplus-orders/:id/execute",
            post(execute_surplus_order_handler),
        )
        .with_state(state)
}

async fn create_loss_order_handler(
    ctx: AuthContext,
    State(state): State<StockAdjustmentAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateStockLossOrderRequest>,
) -> Result<(StatusCode, Json<StockLossOrder>), StockAdjustmentHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    let result = state
        .repository
        .create_loss_order(&ctx, request, Utc::now(), key)
        .await?;
    Ok((StatusCode::CREATED, Json(result.value)))
}

async fn get_loss_order_handler(
    ctx: AuthContext,
    State(state): State<StockAdjustmentAppState>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<StockLossOrder>, StockAdjustmentHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(state.repository.get_loss_order(&ctx, order_id).await?))
}

async fn record_quality_approval_handler(
    ctx: AuthContext,
    State(state): State<StockAdjustmentAppState>,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<StockLossQualityApprovalRequest>,
) -> Result<Json<StockLossOrder>, StockAdjustmentHandlerError> {
    ctx.require_permission(QUALITY_APPROVE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .record_quality_approval(
                &ctx,
                order_id,
                &request.quality_liaison_id,
                request.approved,
                Utc::now(),
                key,
            )
            .await?
            .value,
    ))
}

async fn start_loss_order_handler(
    ctx: AuthContext,
    State(state): State<StockAdjustmentAppState>,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<StockLossOrder>, StockAdjustmentHandlerError> {
    ctx.require_permission(EXECUTE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .start_loss_order(&ctx, order_id, Utc::now(), key)
            .await?
            .value,
    ))
}

async fn execute_loss_order_handler(
    ctx: AuthContext,
    State(state): State<StockAdjustmentAppState>,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ExecuteStockLossOrderRequest>,
) -> Result<Json<StockLossOrder>, StockAdjustmentHandlerError> {
    ctx.require_permission(EXECUTE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .execute_loss_order(&ctx, order_id, request.second_operator_id, Utc::now(), key)
            .await?
            .value,
    ))
}

async fn create_surplus_order_handler(
    ctx: AuthContext,
    State(state): State<StockAdjustmentAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateStockSurplusOrderRequest>,
) -> Result<(StatusCode, Json<StockSurplusOrder>), StockAdjustmentHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    let result = state
        .repository
        .create_surplus_order(&ctx, request, Utc::now(), key)
        .await?;
    Ok((StatusCode::CREATED, Json(result.value)))
}

async fn get_surplus_order_handler(
    ctx: AuthContext,
    State(state): State<StockAdjustmentAppState>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<StockSurplusOrder>, StockAdjustmentHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state.repository.get_surplus_order(&ctx, order_id).await?,
    ))
}

async fn record_surplus_quality_approval_handler(
    ctx: AuthContext,
    State(state): State<StockAdjustmentAppState>,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<StockSurplusQualityApprovalRequest>,
) -> Result<Json<StockSurplusOrder>, StockAdjustmentHandlerError> {
    ctx.require_permission(QUALITY_APPROVE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .record_surplus_quality_approval(
                &ctx,
                order_id,
                &request.quality_liaison_id,
                request.approved,
                Utc::now(),
                key,
            )
            .await?
            .value,
    ))
}

async fn start_surplus_order_handler(
    ctx: AuthContext,
    State(state): State<StockAdjustmentAppState>,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<StockSurplusOrder>, StockAdjustmentHandlerError> {
    ctx.require_permission(EXECUTE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .start_surplus_order(&ctx, order_id, Utc::now(), key)
            .await?
            .value,
    ))
}

async fn execute_surplus_order_handler(
    ctx: AuthContext,
    State(state): State<StockAdjustmentAppState>,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ExecuteStockSurplusOrderRequest>,
) -> Result<Json<StockSurplusOrder>, StockAdjustmentHandlerError> {
    ctx.require_permission(EXECUTE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .execute_surplus_order(&ctx, order_id, request.second_operator_id, Utc::now(), key)
            .await?
            .value,
    ))
}

fn idempotency_key(headers: &HeaderMap) -> Result<&str, StockAdjustmentHandlerError> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(StockAdjustmentHandlerError::MissingIdempotencyKey)
}
