//! M-RC HTTP handlers。

use std::sync::Arc;

use axum::{
    extract::{rejection::JsonRejection, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;
use wms_domain::ErrorResponse;

use crate::{
    auth::{AuthContext, AuthError},
    reconciliation::{
        ErpInventorySnapshotItem, PgReconciliationRepository, ReconciliationDisposition,
        ReconciliationError, ReconciliationInventoryAllocation, ReconciliationItem,
        ReconciliationRun, RunReconciliationRequest,
    },
    reconciliation_query::{
        ClaimReconciliationRequest, FailReconciliationClaimRequest, ReconciliationClaimMutation,
        ReconciliationClaimResponse, ReconciliationItemListResponse, ReconciliationItemQuery,
        ReconciliationRule, RenewReconciliationClaimRequest, UpsertReconciliationRuleRequest,
    },
};

const READ: &str = "rc.reconciliation.read";
const EXECUTE: &str = "rc.reconciliation.execute";
const RESOLVE: &str = "rc.reconciliation.resolve";
const INGEST: &str = "rc.reconciliation.ingest";

#[derive(Clone)]
pub struct ReconciliationAppState {
    repository: Arc<PgReconciliationRepository>,
}

impl ReconciliationAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgReconciliationRepository::new(pool)),
        }
    }
}

#[derive(Debug)]
pub enum ReconciliationHandlerError {
    Auth(AuthError),
    Domain(ReconciliationError),
    MissingIdempotencyKey,
    InvalidJson,
}

impl From<AuthError> for ReconciliationHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<ReconciliationError> for ReconciliationHandlerError {
    fn from(value: ReconciliationError) -> Self {
        Self::Domain(value)
    }
}

impl IntoResponse for ReconciliationHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "RC_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::InvalidJson => (
                StatusCode::BAD_REQUEST,
                "RC_INVALID_REQUEST",
                "对账请求 JSON 缺少必填字段或格式错误",
            ),
            Self::Domain(ReconciliationError::InvalidRequest) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "RC_INVALID_REQUEST",
                "对账请求或当前状态非法",
            ),
            Self::Domain(ReconciliationError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "RC_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            Self::Domain(ReconciliationError::ClaimInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "RC_CLAIM_INVALID",
                "对账调度认领无效或状态不允许",
            ),
            Self::Domain(ReconciliationError::ClaimExpired) => (
                StatusCode::CONFLICT,
                "RC_CLAIM_EXPIRED",
                "对账调度认领租约已过期",
            ),
            Self::Domain(
                ReconciliationError::Database(_)
                | ReconciliationError::Serialize(_)
                | ReconciliationError::Audit(_)
                | ReconciliationError::StockAdjustment(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "RC_INTERNAL",
                "库存对账处理失败",
            ),
            Self::Auth(_) => unreachable!(),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.into(),
                message: message.into(),
                severity: "error".into(),
                details: json!({}),
                trace_id: "unavailable".into(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct SetIsolationRequest {
    pub item_ids: Vec<Uuid>,
    pub isolate: bool,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ResolveReconciliationRequest {
    pub disposition: ReconciliationDisposition,
    #[serde(default)]
    pub allocations: Vec<ReconciliationInventoryAllocation>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct SubmitReconciliationRunRequest {
    pub claim_id: Uuid,
    pub claim_token: Uuid,
    pub window_key: String,
    pub snapshot_at: DateTime<Utc>,
    pub items: Vec<ErpInventorySnapshotItem>,
}

impl From<SubmitReconciliationRunRequest> for RunReconciliationRequest {
    fn from(value: SubmitReconciliationRunRequest) -> Self {
        Self {
            claim_id: value.claim_id,
            claim_token: value.claim_token,
            window_key: value.window_key,
            snapshot_at: value.snapshot_at,
            items: value.items,
        }
    }
}

pub fn reconciliation_router(state: ReconciliationAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/reconciliation/rule",
            get(get_rule).put(upsert_rule),
        )
        .route("/api/v1/reconciliation/claims", post(claim_due_window))
        .route("/api/v1/reconciliation/claims/:id/renew", post(renew_claim))
        .route("/api/v1/reconciliation/claims/:id/failed", post(fail_claim))
        .route("/api/v1/reconciliation/runs", post(run_reconciliation))
        .route(
            "/api/v1/reconciliation/items",
            get(list_reconciliation_items),
        )
        .route(
            "/api/v1/reconciliation/items/isolation",
            post(set_isolation),
        )
        .route(
            "/api/v1/reconciliation/items/:id/resolve",
            post(resolve_item),
        )
        .with_state(state)
}

async fn claim_due_window(
    ctx: AuthContext,
    State(state): State<ReconciliationAppState>,
    headers: HeaderMap,
    Json(req): Json<ClaimReconciliationRequest>,
) -> Result<Json<ReconciliationClaimResponse>, ReconciliationHandlerError> {
    ctx.require_permission(INGEST)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .claim_due_window(&ctx, req, Utc::now(), &key)
            .await?
            .value,
    ))
}

async fn renew_claim(
    ctx: AuthContext,
    State(state): State<ReconciliationAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<RenewReconciliationClaimRequest>,
) -> Result<Json<ReconciliationClaimMutation>, ReconciliationHandlerError> {
    ctx.require_permission(INGEST)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .renew_claim(&ctx, id, req, Utc::now(), &key)
            .await?
            .value,
    ))
}

async fn fail_claim(
    ctx: AuthContext,
    State(state): State<ReconciliationAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<FailReconciliationClaimRequest>,
) -> Result<Json<ReconciliationClaimMutation>, ReconciliationHandlerError> {
    ctx.require_permission(INGEST)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .fail_claim(&ctx, id, req, Utc::now(), &key)
            .await?
            .value,
    ))
}

async fn get_rule(
    ctx: AuthContext,
    State(state): State<ReconciliationAppState>,
) -> Result<Json<ReconciliationRule>, ReconciliationHandlerError> {
    ctx.require_permission(READ)?;
    Ok(Json(state.repository.get_rule(&ctx, Utc::now()).await?))
}

async fn upsert_rule(
    ctx: AuthContext,
    State(state): State<ReconciliationAppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertReconciliationRuleRequest>,
) -> Result<Json<ReconciliationRule>, ReconciliationHandlerError> {
    ctx.require_permission(EXECUTE)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .upsert_rule(&ctx, req, Utc::now(), &key)
            .await?
            .value,
    ))
}

async fn run_reconciliation(
    ctx: AuthContext,
    State(state): State<ReconciliationAppState>,
    headers: HeaderMap,
    payload: Result<Json<SubmitReconciliationRunRequest>, JsonRejection>,
) -> Result<Json<ReconciliationRun>, ReconciliationHandlerError> {
    ctx.require_permission(INGEST)?;
    let Json(req) = payload.map_err(|_| ReconciliationHandlerError::InvalidJson)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .run(&ctx, req.into(), Utc::now(), &key)
            .await?
            .value,
    ))
}

async fn list_reconciliation_items(
    ctx: AuthContext,
    State(state): State<ReconciliationAppState>,
    Query(query): Query<ReconciliationItemQuery>,
) -> Result<Json<ReconciliationItemListResponse>, ReconciliationHandlerError> {
    ctx.require_permission(READ)?;
    Ok(Json(state.repository.list_items(&ctx, query).await?))
}

async fn set_isolation(
    ctx: AuthContext,
    State(state): State<ReconciliationAppState>,
    headers: HeaderMap,
    Json(req): Json<SetIsolationRequest>,
) -> Result<Json<i64>, ReconciliationHandlerError> {
    ctx.require_permission(RESOLVE)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .set_isolation(&ctx, &req.item_ids, req.isolate, Utc::now(), &key)
            .await?
            .value,
    ))
}

async fn resolve_item(
    ctx: AuthContext,
    State(state): State<ReconciliationAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ResolveReconciliationRequest>,
) -> Result<Json<ReconciliationItem>, ReconciliationHandlerError> {
    ctx.require_permission(RESOLVE)?;
    let key = idempotency_key(&headers)?;
    Ok(Json(
        state
            .repository
            .resolve(&ctx, id, req.disposition, req.allocations, Utc::now(), &key)
            .await?
            .value,
    ))
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, ReconciliationHandlerError> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(ReconciliationHandlerError::MissingIdempotencyKey)
}

#[cfg(test)]
mod tests {
    use super::{EXECUTE, INGEST, READ, RESOLVE};
    use crate::auth::AuthContext;
    use uuid::Uuid;

    fn ctx(permission: &str) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            actor_name: "rc-handler-test".into(),
            permissions: vec![permission.into()],
            jti: "rc-handler-test".into(),
            warehouse_scope: None,
        }
    }

    #[test]
    fn permissions_are_separated_by_action() {
        assert!(ctx(READ).require_permission(EXECUTE).is_err());
        assert!(ctx(EXECUTE).require_permission(READ).is_err());
        assert!(ctx(EXECUTE).require_permission(INGEST).is_err());
        assert!(ctx(INGEST).require_permission(INGEST).is_ok());
        assert!(ctx(RESOLVE).require_permission(RESOLVE).is_ok());
    }
}
