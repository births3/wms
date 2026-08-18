use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    BindReplenishmentLocationsRequest, BindReplenishmentLocationsResponse,
    CancelReplenishmentTaskRequest, ClaimReplenishmentTaskRequest, ConfirmReplenishmentTaskRequest,
    CreateReplenishmentTaskRequest, ErrorResponse, PickReplenishmentTaskRequest,
    ReassignReplenishmentTaskRequest, ReplenishmentLocationGroup,
    ReplenishmentLocationGroupListResponse, ReplenishmentPreviewResponse, ReplenishmentStrategy,
    ReplenishmentStrategyListResponse, ReplenishmentTask, ReturnReplenishmentTaskRequest,
    UpsertReplenishmentLocationGroupRequest, UpsertReplenishmentStrategyRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    replenishment_repository::PgReplenishmentRepository,
    replenishment_service::{ReplenishmentError, ReplenishmentService},
};

const MANAGE: &str = "m3.replenishment.manage";
const EXECUTE: &str = "m3.replenishment.execute";

#[derive(Clone)]
pub struct ReplenishmentAppState {
    service: Arc<ReplenishmentService>,
}

impl ReplenishmentAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            service: Arc::new(ReplenishmentService::new(PgReplenishmentRepository::new(
                pool,
            ))),
        }
    }
}

pub fn replenishment_router(state: ReplenishmentAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/replenishment/strategies",
            get(list_strategies_handler).post(create_strategy_handler),
        )
        .route(
            "/api/v1/replenishment/strategies/:id",
            get(get_strategy_handler).put(update_strategy_handler),
        )
        .route(
            "/api/v1/replenishment/strategies/:id/disable",
            post(disable_strategy_handler),
        )
        .route(
            "/api/v1/replenishment/strategies/:id/locations",
            put(bind_locations_handler),
        )
        .route(
            "/api/v1/replenishment/strategies/:id/preview",
            get(preview_handler),
        )
        .route(
            "/api/v1/replenishment/location-groups",
            get(list_location_groups_handler).post(create_location_group_handler),
        )
        .route("/api/v1/replenishment/tasks", post(create_task_handler))
        .route(
            "/api/v1/replenishment/tasks/:id/claim",
            post(claim_task_handler),
        )
        .route(
            "/api/v1/replenishment/tasks/:id/pick",
            post(pick_task_handler),
        )
        .route(
            "/api/v1/replenishment/tasks/:id/confirm",
            post(confirm_task_handler),
        )
        .route(
            "/api/v1/replenishment/tasks/:id/cancel",
            post(cancel_task_handler),
        )
        .route(
            "/api/v1/replenishment/tasks/:id/reassign",
            post(reassign_task_handler),
        )
        .route(
            "/api/v1/replenishment/tasks/:id/return",
            post(return_task_handler),
        )
        .with_state(state)
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ListStrategiesQuery {
    keyword: Option<String>,
    enabled: Option<bool>,
    scope_type: Option<String>,
}

async fn list_strategies_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Query(query): Query<ListStrategiesQuery>,
) -> Result<Json<ReplenishmentStrategyListResponse>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    Ok(Json(
        state
            .service
            .list_strategies(
                &ctx,
                query.keyword.as_deref(),
                query.enabled,
                query.scope_type.as_deref(),
            )
            .await?,
    ))
}

async fn get_strategy_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ReplenishmentStrategy>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    Ok(Json(state.service.get_strategy(&ctx, id).await?))
}

async fn update_strategy_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpsertReplenishmentStrategyRequest>,
) -> Result<Json<ReplenishmentStrategy>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state.service.update_strategy(&ctx, id, req, &key).await?,
    ))
}

async fn disable_strategy_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ReplenishmentStrategy>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.disable_strategy(&ctx, id, &key).await?))
}

async fn list_location_groups_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
) -> Result<Json<ReplenishmentLocationGroupListResponse>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    Ok(Json(state.service.list_location_groups(&ctx).await?))
}

async fn create_strategy_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertReplenishmentStrategyRequest>,
) -> Result<Json<ReplenishmentStrategy>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.create_strategy(&ctx, req, &key).await?))
}

async fn bind_locations_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<BindReplenishmentLocationsRequest>,
) -> Result<Json<BindReplenishmentLocationsResponse>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state.service.bind_locations(&ctx, id, req, &key).await?,
    ))
}

async fn preview_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ReplenishmentPreviewResponse>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    Ok(Json(state.service.preview(&ctx, id).await?))
}

async fn claim_task_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ClaimReplenishmentTaskRequest>,
) -> Result<Json<ReplenishmentTask>, ReplenishmentHandlerError> {
    require_execute(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.claim_task(&ctx, id, req, &key).await?))
}

async fn pick_task_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<PickReplenishmentTaskRequest>,
) -> Result<Json<ReplenishmentTask>, ReplenishmentHandlerError> {
    require_execute(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.pick_task(&ctx, id, req, &key).await?))
}

async fn cancel_task_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<CancelReplenishmentTaskRequest>,
) -> Result<Json<ReplenishmentTask>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.cancel_task(&ctx, id, req, &key).await?))
}

async fn reassign_task_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ReassignReplenishmentTaskRequest>,
) -> Result<Json<ReplenishmentTask>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state.service.reassign_task(&ctx, id, req, &key).await?,
    ))
}

async fn return_task_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ReturnReplenishmentTaskRequest>,
) -> Result<Json<ReplenishmentTask>, ReplenishmentHandlerError> {
    require_execute(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.return_task(&ctx, id, req, &key).await?))
}

async fn confirm_task_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ConfirmReplenishmentTaskRequest>,
) -> Result<Json<ReplenishmentTask>, ReplenishmentHandlerError> {
    require_execute(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.confirm_task(&ctx, id, req, &key).await?))
}

async fn create_task_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateReplenishmentTaskRequest>,
) -> Result<Json<ReplenishmentTask>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.create_task(&ctx, req, &key).await?))
}

async fn create_location_group_handler(
    ctx: AuthContext,
    State(state): State<ReplenishmentAppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertReplenishmentLocationGroupRequest>,
) -> Result<Json<ReplenishmentLocationGroup>, ReplenishmentHandlerError> {
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state.service.upsert_location_group(&ctx, req, &key).await?,
    ))
}

fn require_manage(ctx: &AuthContext) -> Result<(), ReplenishmentHandlerError> {
    ctx.require_permission(MANAGE)
        .map_err(ReplenishmentHandlerError::Auth)
}

fn require_execute(ctx: &AuthContext) -> Result<(), ReplenishmentHandlerError> {
    ctx.require_permission(EXECUTE)
        .map_err(ReplenishmentHandlerError::Auth)
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, ReplenishmentHandlerError> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(ReplenishmentHandlerError::MissingIdempotencyKey)
}

enum ReplenishmentHandlerError {
    Auth(AuthError),
    Service(ReplenishmentError),
    MissingIdempotencyKey,
}

impl From<ReplenishmentError> for ReplenishmentHandlerError {
    fn from(value: ReplenishmentError) -> Self {
        Self::Service(value)
    }
}

impl IntoResponse for ReplenishmentHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::Service(ReplenishmentError::PermissionDenied) => (
                StatusCode::FORBIDDEN,
                "M3_REPLENISH_PERMISSION_DENIED",
                "补货权限不足",
            ),
            Self::Service(ReplenishmentError::StrategyInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_REPLENISH_STRATEGY_INVALID",
                "补货策略动线、范围或水位非法",
            ),
            Self::Service(ReplenishmentError::ScopeNotFound) => (
                StatusCode::NOT_FOUND,
                "M3_REPLENISH_SCOPE_NOT_FOUND",
                "补货策略范围引用不存在或不属于本货主",
            ),
            Self::Service(ReplenishmentError::LocationBound) => (
                StatusCode::CONFLICT,
                "M3_REPLENISH_LOCATION_BOUND",
                "库位已挂其他补货策略",
            ),
            Self::Service(ReplenishmentError::TaskNotFound) => (
                StatusCode::NOT_FOUND,
                "M3_REPLENISH_TASK_NOT_FOUND",
                "补货策略不存在",
            ),
            Self::Service(ReplenishmentError::ClaimConflict) => (
                StatusCode::CONFLICT,
                "M3_REPLENISH_CLAIM_CONFLICT",
                "补货任务领取冲突",
            ),
            Self::Service(ReplenishmentError::QtyExceeded) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_REPLENISH_QTY_EXCEEDED",
                "补货下架或确认数量超过剩余量",
            ),
            Self::Service(ReplenishmentError::SourceMismatch) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_REPLENISH_SOURCE_MISMATCH",
                "扫描来源库位或容器与任务不符",
            ),
            Self::Service(ReplenishmentError::TargetMismatch) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_REPLENISH_TARGET_MISMATCH",
                "扫描目标库位与任务不符",
            ),
            Self::Service(ReplenishmentError::StateInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_REPLENISH_STATE_INVALID",
                "补货任务状态不允许该动作",
            ),
            Self::Service(ReplenishmentError::CancelBlocked) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_REPLENISH_CANCEL_BLOCKED",
                "已下架或已送达的补货任务不可取消",
            ),
            Self::Service(ReplenishmentError::ReturnBlocked) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_REPLENISH_RETURN_BLOCKED",
                "已下架的补货任务不可退回",
            ),
            Self::Service(ReplenishmentError::SourceUnavailable) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_REPLENISH_SOURCE_UNAVAILABLE",
                "补货来源可下架量不足",
            ),
            Self::Service(ReplenishmentError::NumberingUnavailable) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_REPLENISH_NUMBERING_UNAVAILABLE",
                "补货任务编号规则不可用",
            ),
            Self::Service(ReplenishmentError::PutawayBlocked) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_REPLENISH_PUTAWAY_BLOCKED",
                "补货目标上架校验未通过",
            ),
            Self::Service(ReplenishmentError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "M3_REPLENISH_IDEMPOTENCY_CONFLICT",
                "补货写请求幂等键冲突",
            ),
            Self::MissingIdempotencyKey
            | Self::Service(ReplenishmentError::IdempotencyRequired) => (
                StatusCode::BAD_REQUEST,
                "M3_REPLENISH_IDEMPOTENCY_REQUIRED",
                "补货写请求缺少幂等键",
            ),
            Self::Service(ReplenishmentError::Database(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M3_REPLENISH_STATE_INVALID",
                "补货策略处理失败",
            ),
            Self::Auth(_) => unreachable!("auth error returned above"),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details: json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}
