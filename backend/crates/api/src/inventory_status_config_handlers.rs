use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, put},
    Json, Router,
};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use wms_domain::{ErrorResponse, UpsertInventoryStatusTransitionRequest};

use crate::{
    auth::{AuthContext, AuthError},
    inventory_status_config::{InventoryStatusConfigError, PgInventoryStatusConfigRepository},
};

const READ_PERMISSION: &str = "m3.read";
const WRITE_PERMISSION: &str = "m3.write";
const GLOBAL_WRITE_PERMISSION: &str = "m3.inventory_status.global.write";
const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";

#[derive(Clone, Debug)]
pub struct InventoryStatusConfigAppState {
    repository: Arc<PgInventoryStatusConfigRepository>,
}

impl InventoryStatusConfigAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgInventoryStatusConfigRepository::new(pool)),
        }
    }
}

#[derive(Debug)]
pub enum InventoryStatusConfigHandlerError {
    Auth(AuthError),
    Config(InventoryStatusConfigError),
    MissingIdempotencyKey,
}

impl From<AuthError> for InventoryStatusConfigHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<InventoryStatusConfigError> for InventoryStatusConfigHandlerError {
    fn from(value: InventoryStatusConfigError) -> Self {
        Self::Config(value)
    }
}

impl IntoResponse for InventoryStatusConfigHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::Config(InventoryStatusConfigError::CrossOwnerAccess) => (
                StatusCode::FORBIDDEN,
                "M3_INVENTORY_STATUS_CROSS_OWNER",
                "库存状态规则不能跨货主维护",
            ),
            Self::Config(InventoryStatusConfigError::InvalidStatus) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_INVENTORY_STATUS_INVALID_STATUS",
                "状态必须是已启用的库存质量状态",
            ),
            Self::Config(InventoryStatusConfigError::InvalidTransition) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_INVENTORY_STATUS_INVALID_TRANSITION",
                "状态转换的起点和终点不能相同且不能为空",
            ),
            Self::Config(InventoryStatusConfigError::InvalidApprovalSources) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_INVENTORY_STATUS_APPROVAL_SOURCE_REQUIRED",
                "至少配置一个审批源",
            ),
            Self::Config(InventoryStatusConfigError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "M3_INVENTORY_STATUS_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "M3_INVENTORY_STATUS_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::Config(
                InventoryStatusConfigError::Audit(_)
                | InventoryStatusConfigError::Database(_)
                | InventoryStatusConfigError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M3_INVENTORY_STATUS_INTERNAL",
                "库存状态规则处理失败",
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

pub fn inventory_status_config_router(state: InventoryStatusConfigAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/inventory/status-transitions",
            get(list_inventory_status_transitions_handler),
        )
        .route(
            "/api/v1/inventory/status-transitions/:from_status/:to_status",
            put(upsert_inventory_status_transition_handler),
        )
        .with_state(state)
}

async fn list_inventory_status_transitions_handler(
    ctx: AuthContext,
    State(state): State<InventoryStatusConfigAppState>,
) -> Result<
    Json<wms_domain::InventoryStatusTransitionListResponse>,
    InventoryStatusConfigHandlerError,
> {
    require_read_permission(&ctx)?;
    Ok(Json(state.repository.list_effective(&ctx).await?))
}

async fn upsert_inventory_status_transition_handler(
    ctx: AuthContext,
    State(state): State<InventoryStatusConfigAppState>,
    Path((from_status, to_status)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<UpsertInventoryStatusTransitionRequest>,
) -> Result<Json<wms_domain::InventoryStatusTransition>, InventoryStatusConfigHandlerError> {
    require_write_permission(&ctx, req.owner_id)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .repository
        .upsert(
            &ctx,
            &from_status,
            &to_status,
            req,
            Utc::now(),
            &idempotency_key,
        )
        .await?;
    Ok(Json(result.value))
}

fn require_read_permission(ctx: &AuthContext) -> Result<(), AuthError> {
    if ctx.has_permission(READ_PERMISSION) || ctx.has_permission(WRITE_PERMISSION) {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied(format!(
            "{READ_PERMISSION}|{WRITE_PERMISSION}"
        )))
    }
}

fn require_write_permission(
    ctx: &AuthContext,
    owner_id: Option<uuid::Uuid>,
) -> Result<(), AuthError> {
    match owner_id {
        Some(owner_id) => {
            ctx.require_owner(owner_id)?;
            ctx.require_permission(WRITE_PERMISSION)
        }
        None => ctx.require_permission(GLOBAL_WRITE_PERMISSION),
    }
}

fn idempotency_key_from_headers(
    headers: &HeaderMap,
) -> Result<String, InventoryStatusConfigHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(InventoryStatusConfigHandlerError::MissingIdempotencyKey)
}

#[cfg(test)]
mod tests {
    use super::{require_read_permission, require_write_permission};
    use crate::auth::AuthContext;
    use uuid::Uuid;

    fn ctx(permissions: &[&str]) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            actor_name: "status-config-test".to_string(),
            permissions: permissions.iter().map(|value| value.to_string()).collect(),
            jti: "status-config-test-jti".to_string(),
        }
    }

    #[test]
    fn status_config_requires_m3_read_or_write() {
        assert!(require_read_permission(&ctx(&[])).is_err());
        assert!(require_read_permission(&ctx(&["m3.read"])).is_ok());
        assert!(require_read_permission(&ctx(&["m3.write"])).is_ok());
    }

    #[test]
    fn global_write_requires_dedicated_permission() {
        assert!(require_write_permission(&ctx(&["m3.write"]), None).is_err());
        assert!(
            require_write_permission(&ctx(&["m3.inventory_status.global.write"]), None).is_ok()
        );
    }
}
