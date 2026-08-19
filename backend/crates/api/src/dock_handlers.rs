use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use wms_domain::{
    CreateDockImportRequest, CreateDockRequest, Dock, ErrorResponse, UpdateDockRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    dock_repository::{DockRepositoryError, PgDockRepository},
};

const DOCK_READ_PERMISSION: &str = "m1.master_data.read";
const DOCK_WRITE_PERMISSION: &str = "dock.manage";
const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";

#[derive(Clone, Debug)]
pub struct DockAppState {
    pub repository: Arc<PgDockRepository>,
}

impl DockAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgDockRepository::new(pool)),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ListDocksQuery {
    warehouse_id: Uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DockHandlerError {
    Auth(AuthError),
    Repository(DockRepositoryError),
    MissingIdempotencyKey,
}

impl From<AuthError> for DockHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<DockRepositoryError> for DockHandlerError {
    fn from(value: DockRepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl IntoResponse for DockHandlerError {
    fn into_response(self) -> Response {
        if let DockHandlerError::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            DockHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "DOCK_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key".to_string(),
            ),
            DockHandlerError::Repository(DockRepositoryError::NotFound) => (
                StatusCode::NOT_FOUND,
                "M1-404",
                "仓库或月台不存在".to_string(),
            ),
            DockHandlerError::Repository(DockRepositoryError::DuplicateCode) => (
                StatusCode::CONFLICT,
                "M1-409",
                "同一仓库的月台编号已存在".to_string(),
            ),
            DockHandlerError::Repository(DockRepositoryError::InUse(count)) => (
                StatusCode::CONFLICT,
                "DOCK_IN_USE",
                format!("该月台有 {count} 笔关联预约，不能删除"),
            ),
            DockHandlerError::Repository(DockRepositoryError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "DOCK_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用".to_string(),
            ),
            DockHandlerError::Repository(DockRepositoryError::Audit(_))
            | DockHandlerError::Repository(DockRepositoryError::Database(_))
            | DockHandlerError::Repository(DockRepositoryError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M1-500",
                "月台档案持久化失败".to_string(),
            ),
            DockHandlerError::Auth(_) => unreachable!("auth error returned above"),
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

pub fn dock_router(state: DockAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/docks",
            get(list_docks_handler).post(create_dock_handler),
        )
        .route(
            "/api/v1/docks/:id",
            patch(update_dock_handler).delete(delete_dock_handler),
        )
        .route("/api/v1/docks/import", post(import_docks_handler))
        .with_state(state)
}

async fn list_docks_handler(
    ctx: AuthContext,
    State(state): State<DockAppState>,
    Query(query): Query<ListDocksQuery>,
) -> Result<Json<Vec<Dock>>, DockHandlerError> {
    require_read_permission(&ctx)?;
    Ok(Json(
        state
            .repository
            .list_docks(&ctx, query.warehouse_id)
            .await?,
    ))
}

async fn create_dock_handler(
    ctx: AuthContext,
    State(state): State<DockAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateDockRequest>,
) -> Result<Json<Dock>, DockHandlerError> {
    ctx.require_permission(DOCK_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .repository
            .create_dock(&ctx, request, Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn update_dock_handler(
    ctx: AuthContext,
    State(state): State<DockAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateDockRequest>,
) -> Result<Json<Dock>, DockHandlerError> {
    ctx.require_permission(DOCK_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .repository
            .update_dock(&ctx, id, request, Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn delete_dock_handler(
    ctx: AuthContext,
    State(state): State<DockAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<StatusCode, DockHandlerError> {
    ctx.require_permission(DOCK_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    state
        .repository
        .delete_dock(&ctx, id, Utc::now(), &idempotency_key)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn import_docks_handler(
    ctx: AuthContext,
    State(state): State<DockAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateDockImportRequest>,
) -> Result<Json<Vec<Dock>>, DockHandlerError> {
    ctx.require_permission(DOCK_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .repository
            .import_docks(&ctx, request, Utc::now(), &idempotency_key)
            .await?,
    ))
}

fn require_read_permission(ctx: &AuthContext) -> Result<(), AuthError> {
    if ctx.has_permission(DOCK_READ_PERMISSION) || ctx.has_permission(DOCK_WRITE_PERMISSION) {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied(
            DOCK_READ_PERMISSION.to_string(),
        ))
    }
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, DockHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or(DockHandlerError::MissingIdempotencyKey)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotency_key_must_be_present_and_non_empty() {
        assert_eq!(
            idempotency_key_from_headers(&HeaderMap::new()),
            Err(DockHandlerError::MissingIdempotencyKey)
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            IDEMPOTENCY_KEY_HEADER,
            "  dock-1  ".parse().expect("test header should parse"),
        );
        assert_eq!(
            idempotency_key_from_headers(&headers),
            Ok("dock-1".to_string())
        );
    }
}
