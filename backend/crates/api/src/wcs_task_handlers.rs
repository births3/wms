//! T03：指令任务 HTTP 层（事件上报 / 任务列表 / 重发 / 作废）。

use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection, QueryRejection},
        Path, Query, State,
    },
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::device_platform_error::{
    idempotency_key, require_manage, require_monitor, DevicePlatformHandlerError,
};
use crate::wcs_task_service::{
    ConfirmSkipRequest, CreateWcsTaskRequest, DeviceDashboardSummary, DeviceEventLog,
    DeviceEventRequest, ReceiptRequest, ResendRequest, VoidRequest, WcsTaskResponse,
    WcsTaskService,
};

#[derive(Clone)]
pub struct WcsTaskAppState {
    pub service: WcsTaskService,
}

impl WcsTaskAppState {
    pub fn with_postgres(pool: sqlx::PgPool) -> Self {
        Self {
            service: WcsTaskService::new(pool),
        }
    }
}

pub fn wcs_task_router(state: WcsTaskAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/wcs-tasks",
            get(list_tasks_handler).post(create_task_handler),
        )
        .route("/api/v1/wcs-tasks/:id", get(get_task_handler))
        .route("/api/v1/wcs-tasks/:id/resend", post(resend_task_handler))
        .route(
            "/api/v1/wcs-tasks/:id/dispatch",
            post(dispatch_task_handler),
        )
        .route("/api/v1/wcs-tasks/:id/receipt", post(receipt_task_handler))
        .route("/api/v1/wcs-tasks/:id/void", post(void_task_handler))
        .route(
            "/api/v1/wcs-tasks/:id/confirm-skip",
            post(confirm_skip_handler),
        )
        .route("/api/v1/iot-devices/:id/events", post(device_event_handler))
        .route("/api/v1/iot-events", get(list_events_handler))
        .route("/api/v1/device-dashboard", get(dashboard_handler))
        .with_state(state)
}

#[derive(Deserialize)]
pub struct TaskListQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub task_type: Option<String>,
}

#[derive(Deserialize)]
pub struct EventListQuery {
    pub warehouse_id: Uuid,
    #[serde(default)]
    pub device_id: Option<Uuid>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct DashboardQuery {
    pub warehouse_id: Uuid,
}

async fn dashboard_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    request: Result<Query<DashboardQuery>, QueryRejection>,
) -> Result<Json<DeviceDashboardSummary>, DevicePlatformHandlerError> {
    let Query(query) = request?;
    require_monitor(&ctx)?;
    Ok(Json(
        state
            .service
            .dashboard_summary(&ctx, query.warehouse_id)
            .await?,
    ))
}

async fn list_events_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    request: Result<Query<EventListQuery>, QueryRejection>,
) -> Result<Json<Vec<DeviceEventLog>>, DevicePlatformHandlerError> {
    let Query(query) = request?;
    require_monitor(&ctx)?;
    Ok(Json(
        state
            .service
            .list_events(
                &ctx,
                query.warehouse_id,
                query.device_id,
                query.event_type,
                query.limit,
            )
            .await?,
    ))
}

async fn create_task_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    headers: HeaderMap,
    request: Result<Json<CreateWcsTaskRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<WcsTaskResponse>), DevicePlatformHandlerError> {
    let Json(req) = request?;
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    let created = state.service.create_task(&ctx, req, &key).await?;
    let status = if created.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(created.task)))
}

async fn list_tasks_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    request: Result<Query<TaskListQuery>, QueryRejection>,
) -> Result<Json<Vec<WcsTaskResponse>>, DevicePlatformHandlerError> {
    let Query(query) = request?;
    require_monitor(&ctx)?;
    Ok(Json(
        state
            .service
            .list(&ctx, query.status, query.task_type)
            .await?,
    ))
}

async fn get_task_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    request: Result<Path<Uuid>, PathRejection>,
) -> Result<Json<WcsTaskResponse>, DevicePlatformHandlerError> {
    let Path(id) = request?;
    require_monitor(&ctx)?;
    Ok(Json(state.service.get(&ctx, id).await?))
}

async fn resend_task_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    request: Result<Json<ResendRequest>, JsonRejection>,
) -> Result<Json<WcsTaskResponse>, DevicePlatformHandlerError> {
    let Path(id) = path?;
    let Json(req) = request?;
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .service
            .resend(&ctx, id, req.reason.clone(), &key)
            .await?,
    ))
}

async fn dispatch_task_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    request: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> Result<Json<WcsTaskResponse>, DevicePlatformHandlerError> {
    let Path(id) = request?;
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.dispatch_command(&ctx, id, &key).await?))
}

async fn receipt_task_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    request: Result<Json<ReceiptRequest>, JsonRejection>,
) -> Result<Json<WcsTaskResponse>, DevicePlatformHandlerError> {
    let Path(id) = path?;
    let Json(req) = request?;
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .service
            .apply_receipt_command(&ctx, id, req, &key)
            .await?,
    ))
}

async fn void_task_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    request: Result<Json<VoidRequest>, JsonRejection>,
) -> Result<Json<WcsTaskResponse>, DevicePlatformHandlerError> {
    let Path(id) = path?;
    let Json(req) = request?;
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.void(&ctx, id, req, &key).await?))
}

async fn confirm_skip_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    request: Result<Json<ConfirmSkipRequest>, JsonRejection>,
) -> Result<Json<WcsTaskResponse>, DevicePlatformHandlerError> {
    let Path(id) = path?;
    let Json(req) = request?;
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.confirm_skip(&ctx, id, req, &key).await?))
}

async fn device_event_handler(
    ctx: AuthContext,
    State(state): State<WcsTaskAppState>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    request: Result<Json<DeviceEventRequest>, JsonRejection>,
) -> Result<StatusCode, DevicePlatformHandlerError> {
    let Path(id) = path?;
    let Json(req) = request?;
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    state
        .service
        .handle_event_command(&ctx, id, req, &key)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
