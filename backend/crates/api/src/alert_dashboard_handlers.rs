use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    AlertChangeListResponse, AlertExportJob, AlertInstanceListQuery, AlertInstanceListResponse,
    AlertStatisticsResponse, CreateAlertExportRequest, ErrorResponse, GspAlertLifecycleReport,
    PageMeta,
};

use crate::{
    alert_dashboard::{AlertDashboardError, PgAlertDashboardService},
    auth::{AuthContext, AuthError},
};

#[derive(Clone, Debug)]
pub struct AlertDashboardAppState {
    service: PgAlertDashboardService,
}

impl AlertDashboardAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            service: PgAlertDashboardService::new(pool),
        }
    }
}

#[derive(Debug)]
pub enum AlertDashboardHandlerError {
    Auth(AuthError),
    Dashboard(AlertDashboardError),
}

impl From<AuthError> for AlertDashboardHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<AlertDashboardError> for AlertDashboardHandlerError {
    fn from(value: AlertDashboardError) -> Self {
        Self::Dashboard(value)
    }
}

impl IntoResponse for AlertDashboardHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::Dashboard(AlertDashboardError::NotFound) => (
                StatusCode::NOT_FOUND,
                "HAL_EXPORT_NOT_FOUND",
                "导出文件不存在或已过期",
            ),
            Self::Dashboard(AlertDashboardError::RangeTooLarge) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_QUERY_RANGE_TOO_LARGE",
                "查询范围超过一年，请缩小时间范围或使用异步导出",
            ),
            Self::Dashboard(AlertDashboardError::WarehouseScopeRequired) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_WAREHOUSE_SCOPE_REQUIRED",
                "仓库主管必须选择已授权仓库",
            ),
            Self::Dashboard(AlertDashboardError::WarehouseScopeDenied) => (
                StatusCode::FORBIDDEN,
                "HAL_WAREHOUSE_SCOPE_DENIED",
                "无权查询该仓库告警",
            ),
            Self::Dashboard(AlertDashboardError::InvalidExportFormat) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "HAL_EXPORT_FORMAT_INVALID",
                "导出格式仅支持 excel 或 pdf",
            ),
            Self::Dashboard(
                AlertDashboardError::Database(_)
                | AlertDashboardError::Audit(_)
                | AlertDashboardError::Serialize(_)
                | AlertDashboardError::Notification(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "HAL_DASHBOARD_INTERNAL",
                "告警看板处理失败",
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

pub fn alert_dashboard_router(state: AlertDashboardAppState) -> Router {
    Router::new()
        .route("/api/v1/alerts/active", get(active_handler))
        .route("/api/v1/alerts/statistics", get(statistics_handler))
        .route("/api/v1/alerts/gsp-report", get(gsp_report_handler))
        .route("/api/v1/alerts/changes", get(changes_handler))
        .route("/api/v1/alerts/exports", post(create_export_handler))
        .route("/api/v1/alerts/exports/:id", get(get_export_handler))
        .route(
            "/api/v1/alerts/exports/:token/download",
            get(download_handler),
        )
        .with_state(state)
}

async fn active_handler(
    ctx: AuthContext,
    State(state): State<AlertDashboardAppState>,
    Query(query): Query<AlertInstanceListQuery>,
) -> Result<Json<AlertInstanceListResponse>, AlertDashboardHandlerError> {
    ctx.require_permission("hal.alert.read")?;
    let data = state.service.list_active(&ctx, query, Utc::now()).await?;
    Ok(Json(AlertInstanceListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len().min(u32::MAX as usize) as u32,
        },
        data,
    }))
}

async fn statistics_handler(
    ctx: AuthContext,
    State(state): State<AlertDashboardAppState>,
    Query(query): Query<AlertInstanceListQuery>,
) -> Result<Json<AlertStatisticsResponse>, AlertDashboardHandlerError> {
    ctx.require_permission("hal.alert.report")?;
    Ok(Json(
        state.service.statistics(&ctx, query, Utc::now()).await?,
    ))
}

async fn gsp_report_handler(
    ctx: AuthContext,
    State(state): State<AlertDashboardAppState>,
    Query(query): Query<AlertInstanceListQuery>,
) -> Result<Json<GspAlertLifecycleReport>, AlertDashboardHandlerError> {
    ctx.require_permission("hal.alert.report")?;
    Ok(Json(
        state.service.gsp_report(&ctx, query, Utc::now()).await?,
    ))
}

#[derive(Deserialize)]
struct ChangesQuery {
    since: Option<DateTime<Utc>>,
}

async fn changes_handler(
    ctx: AuthContext,
    State(state): State<AlertDashboardAppState>,
    Query(query): Query<ChangesQuery>,
) -> Result<Json<AlertChangeListResponse>, AlertDashboardHandlerError> {
    ctx.require_permission("hal.alert.read")?;
    let server_time = Utc::now();
    let since = query.since.unwrap_or(server_time - Duration::seconds(5));
    let data = state.service.changes_since(&ctx, since).await?;
    Ok(Json(AlertChangeListResponse { data, server_time }))
}

async fn create_export_handler(
    ctx: AuthContext,
    State(state): State<AlertDashboardAppState>,
    Json(request): Json<CreateAlertExportRequest>,
) -> Result<(StatusCode, Json<AlertExportJob>), AlertDashboardHandlerError> {
    ctx.require_permission("hal.alert.report")?;
    let job = state
        .service
        .create_export(&ctx, request, Utc::now())
        .await?;
    let status = if job.status == "queued" {
        StatusCode::ACCEPTED
    } else {
        StatusCode::CREATED
    };
    Ok((status, Json(job)))
}

async fn get_export_handler(
    ctx: AuthContext,
    State(state): State<AlertDashboardAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AlertExportJob>, AlertDashboardHandlerError> {
    ctx.require_permission("hal.alert.report")?;
    Ok(Json(state.service.get_export(ctx.owner_id, id).await?))
}

async fn download_handler(
    ctx: AuthContext,
    State(state): State<AlertDashboardAppState>,
    Path(token): Path<Uuid>,
) -> Result<Response, AlertDashboardHandlerError> {
    ctx.require_permission("hal.alert.report")?;
    let (content, content_type, filename) = state
        .service
        .download(ctx.owner_id, token, Utc::now())
        .await?;
    let mut response = Response::new(Body::from(content));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&content_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .unwrap_or(HeaderValue::from_static("attachment")),
    );
    Ok(response)
}
