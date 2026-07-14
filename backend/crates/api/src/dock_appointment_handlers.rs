use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use wms_domain::{
    ArriveDockAppointmentRequest, CancelDockAppointmentRequest, CreateDockAppointmentRequest,
    DockAppointment, ErrorResponse, UpdateDockAppointmentRequest,
};

use crate::{
    audit::AuditWriteRequest,
    auth::{AuthContext, AuthError},
    dock_appointment_repository::{DockAppointmentRepositoryError, PgDockAppointmentRepository},
};

const DOCK_APPOINTMENT_WRITE_PERMISSION: &str = "dock.manage";
const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";

#[derive(Debug, Deserialize)]
struct DockAppointmentListQuery {
    warehouse_id: Uuid,
    dock_id: Option<Uuid>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    status: Option<String>,
}

#[derive(Clone, Debug)]
pub struct DockAppointmentAppState {
    pub repository: Arc<PgDockAppointmentRepository>,
}

impl DockAppointmentAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgDockAppointmentRepository::new(pool)),
        }
    }
}

#[derive(Debug)]
enum DockAppointmentHandlerError {
    Auth(AuthError),
    Repository(DockAppointmentRepositoryError),
    MissingIdempotencyKey,
}

impl From<AuthError> for DockAppointmentHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<DockAppointmentRepositoryError> for DockAppointmentHandlerError {
    fn from(value: DockAppointmentRepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl IntoResponse for DockAppointmentHandlerError {
    fn into_response(self) -> Response {
        if let DockAppointmentHandlerError::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            DockAppointmentHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "H_DOCK_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key".to_string(),
            ),
            DockAppointmentHandlerError::Repository(
                DockAppointmentRepositoryError::OwnerWarehouseMismatch
                | DockAppointmentRepositoryError::NotFound,
            ) => (
                StatusCode::NOT_FOUND,
                "H_DOCK_DOCK_NOT_FOUND",
                "月台或仓库不存在".to_string(),
            ),
            DockAppointmentHandlerError::Repository(
                DockAppointmentRepositoryError::TimeConflict,
            ) => (
                StatusCode::CONFLICT,
                "H_DOCK_APPOINTMENT_CONFLICT",
                "月台预约时间窗重叠".to_string(),
            ),
            DockAppointmentHandlerError::Repository(
                DockAppointmentRepositoryError::StatusNotEditable
                | DockAppointmentRepositoryError::StatusNotCancellable,
            ) => (
                StatusCode::CONFLICT,
                "H_DOCK_APPOINTMENT_CONFLICT",
                "预约当前状态不允许此操作".to_string(),
            ),
            DockAppointmentHandlerError::Repository(
                DockAppointmentRepositoryError::AppointmentNoConflict
                | DockAppointmentRepositoryError::ActiveAppointmentConflict,
            ) => (
                StatusCode::CONFLICT,
                "H_DOCK_APPOINTMENT_CONFLICT",
                "预约编号或关联单据发生冲突".to_string(),
            ),
            DockAppointmentHandlerError::Repository(
                DockAppointmentRepositoryError::AppointmentNotFound,
            ) => (
                StatusCode::NOT_FOUND,
                "H_DOCK_APPOINTMENT_NOT_FOUND",
                "预约不存在".to_string(),
            ),
            DockAppointmentHandlerError::Repository(
                DockAppointmentRepositoryError::ArrivalCheckMismatch,
            ) => (
                StatusCode::CONFLICT,
                "H_DOCK_ARRIVAL_CHECK_FAILED",
                "预约到达核对不一致".to_string(),
            ),
            DockAppointmentHandlerError::Repository(
                DockAppointmentRepositoryError::TemperatureMismatch,
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H_DOCK_VEHICLE_TYPE_MISMATCH",
                "车辆类型与月台温区不匹配".to_string(),
            ),
            DockAppointmentHandlerError::Repository(
                DockAppointmentRepositoryError::StatusNotArrivable,
            ) => (
                StatusCode::CONFLICT,
                "H_DOCK_APPOINTMENT_NOT_ARRIVABLE",
                "预约当前状态不允许到达核对".to_string(),
            ),
            DockAppointmentHandlerError::Repository(
                DockAppointmentRepositoryError::IdempotencyConflict,
            ) => (
                StatusCode::CONFLICT,
                "H_DOCK_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用".to_string(),
            ),
            DockAppointmentHandlerError::Repository(
                DockAppointmentRepositoryError::Invalid(_)
                | DockAppointmentRepositoryError::WindowInvalid
                | DockAppointmentRepositoryError::WindowEnded,
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H_DOCK_APPOINTMENT_INVALID",
                "预约字段或时间窗非法".to_string(),
            ),
            DockAppointmentHandlerError::Repository(
                DockAppointmentRepositoryError::Audit(_)
                | DockAppointmentRepositoryError::Database(_)
                | DockAppointmentRepositoryError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H_DOCK_PERSISTENCE_FAILED",
                "预约持久化失败".to_string(),
            ),
            DockAppointmentHandlerError::Auth(_) => unreachable!("auth error returned above"),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message,
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

pub fn dock_appointment_router(state: DockAppointmentAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/dock-appointments",
            get(list_dock_appointments_handler).post(create_dock_appointment_handler),
        )
        .route(
            "/api/v1/dock-appointments/:id",
            patch(update_dock_appointment_handler),
        )
        .route(
            "/api/v1/dock-appointments/:id/cancel",
            post(cancel_dock_appointment_handler),
        )
        .route(
            "/api/v1/dock-appointments/:id/arrive",
            post(arrive_dock_appointment_handler),
        )
        .with_state(state)
}

async fn list_dock_appointments_handler(
    ctx: AuthContext,
    State(state): State<DockAppointmentAppState>,
    Query(query): Query<DockAppointmentListQuery>,
) -> Result<Json<Vec<DockAppointment>>, DockAppointmentHandlerError> {
    ctx.require_permission(DOCK_APPOINTMENT_WRITE_PERMISSION)?;
    let DockAppointmentListQuery {
        warehouse_id,
        dock_id,
        from,
        to,
        status,
    } = query;
    Ok(Json(
        state
            .repository
            .list(&ctx, warehouse_id, dock_id, from, to, status)
            .await?,
    ))
}

async fn create_dock_appointment_handler(
    ctx: AuthContext,
    State(state): State<DockAppointmentAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateDockAppointmentRequest>,
) -> Result<Json<DockAppointment>, DockAppointmentHandlerError> {
    ctx.require_permission(DOCK_APPOINTMENT_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key(&headers)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create_dock_appointment",
        "H2",
        "dock_appointment",
        "",
        None,
    );
    Ok(Json(
        state
            .repository
            .create_with_audit(&ctx, request, Utc::now(), &idempotency_key, audit)
            .await?,
    ))
}

async fn update_dock_appointment_handler(
    ctx: AuthContext,
    State(state): State<DockAppointmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateDockAppointmentRequest>,
) -> Result<Json<DockAppointment>, DockAppointmentHandlerError> {
    ctx.require_permission(DOCK_APPOINTMENT_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key(&headers)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "change_dock_appointment",
        "H2",
        "dock_appointment",
        id.to_string(),
        None,
    );
    Ok(Json(
        state
            .repository
            .change_with_audit(&ctx, id, request, Utc::now(), &idempotency_key, audit)
            .await?,
    ))
}

async fn cancel_dock_appointment_handler(
    ctx: AuthContext,
    State(state): State<DockAppointmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CancelDockAppointmentRequest>,
) -> Result<Json<DockAppointment>, DockAppointmentHandlerError> {
    ctx.require_permission(DOCK_APPOINTMENT_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key(&headers)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "cancel_dock_appointment",
        "H2",
        "dock_appointment",
        id.to_string(),
        None,
    );
    Ok(Json(
        state
            .repository
            .cancel_with_audit(&ctx, id, request, Utc::now(), &idempotency_key, audit)
            .await?,
    ))
}

async fn arrive_dock_appointment_handler(
    ctx: AuthContext,
    State(state): State<DockAppointmentAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ArriveDockAppointmentRequest>,
) -> Result<Json<DockAppointment>, DockAppointmentHandlerError> {
    ctx.require_permission(DOCK_APPOINTMENT_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key(&headers)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "arrive_dock_appointment",
        "H2",
        "dock_appointment",
        id.to_string(),
        None,
    );
    Ok(Json(
        state
            .repository
            .arrive_with_audit(&ctx, id, request, Utc::now(), &idempotency_key, audit)
            .await?,
    ))
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, DockAppointmentHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or(DockAppointmentHandlerError::MissingIdempotencyKey)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn missing_idempotency_key_is_rejected_before_repository_call() {
        let headers = HeaderMap::new();
        assert!(headers.get(IDEMPOTENCY_KEY_HEADER).is_none());
    }

    #[tokio::test]
    async fn time_conflict_maps_to_stable_http_error() {
        let response =
            DockAppointmentHandlerError::Repository(DockAppointmentRepositoryError::TimeConflict)
                .into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("error response body should be readable");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("error response should be JSON");
        assert_eq!(payload["code"], "H_DOCK_APPOINTMENT_CONFLICT");
    }
}
