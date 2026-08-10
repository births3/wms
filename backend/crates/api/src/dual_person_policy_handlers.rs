use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use wms_domain::{
    DualPersonPolicyResponse, DualPersonPolicyRule, DualPersonPolicyRuleListQuery,
    DualPersonPolicyRuleListResponse, DualPersonPolicyScope, ErrorResponse, PageMeta,
    ResolveDualPersonPolicyQuery, UpsertDualPersonPolicyRuleRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    dual_person_policy::{DualPersonPolicyError, PgDualPersonPolicyRepository},
};

const READ_PERMISSION: &str = "mvr.dual_person.read";
const WRITE_PERMISSION: &str = "mvr.dual_person.write";
const GLOBAL_WRITE_PERMISSION: &str = "mvr.dual_person.global.write";

#[derive(Clone, Debug)]
pub struct DualPersonPolicyAppState {
    repository: PgDualPersonPolicyRepository,
}

#[derive(Debug)]
pub enum DualPersonPolicyHandlerError {
    Auth(AuthError),
    Policy(DualPersonPolicyError),
    MissingIdempotencyKey,
}

impl DualPersonPolicyAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: PgDualPersonPolicyRepository::new(pool),
        }
    }

    pub fn with_postgres_and_redis(pool: PgPool, cache: redis::aio::MultiplexedConnection) -> Self {
        Self {
            repository: PgDualPersonPolicyRepository::with_redis_cache(pool, cache),
        }
    }
}

impl From<AuthError> for DualPersonPolicyHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<DualPersonPolicyError> for DualPersonPolicyHandlerError {
    fn from(value: DualPersonPolicyError) -> Self {
        Self::Policy(value)
    }
}

impl IntoResponse for DualPersonPolicyHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "M_VR_DUAL_PERSON_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::Policy(DualPersonPolicyError::CrossOwner) => (
                StatusCode::FORBIDDEN,
                "M_VR_DUAL_PERSON_CROSS_OWNER",
                "禁止跨货主查询双人策略",
            ),
            Self::Policy(DualPersonPolicyError::InvalidProcessNode) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_VR_DUAL_PERSON_POLICY_INVALID",
                "流程与节点不匹配",
            ),
            Self::Policy(DualPersonPolicyError::InvalidRule) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_VR_DUAL_PERSON_POLICY_INVALID",
                "双人策略规则参数非法",
            ),
            Self::Policy(DualPersonPolicyError::SamePerson) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_VR_DUAL_PERSON_SAME_PERSON",
                "矩阵变更确认人不能与操作人相同",
            ),
            Self::Policy(DualPersonPolicyError::UnqualifiedConfirmer) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_VR_DUAL_PERSON_UNQUALIFIED",
                "矩阵变更确认人无对应资格",
            ),
            Self::Policy(DualPersonPolicyError::ProductNotFound) => (
                StatusCode::NOT_FOUND,
                "M_VR_DUAL_PERSON_REFERENCE_NOT_FOUND",
                "商品不存在或不属于当前货主",
            ),
            Self::Policy(DualPersonPolicyError::WarehouseNotFound) => (
                StatusCode::NOT_FOUND,
                "M_VR_DUAL_PERSON_REFERENCE_NOT_FOUND",
                "仓库不存在或不属于当前货主",
            ),
            Self::Policy(DualPersonPolicyError::CategoryNotFound) => (
                StatusCode::NOT_FOUND,
                "M_VR_DUAL_PERSON_REFERENCE_NOT_FOUND",
                "特殊药品分类不存在",
            ),
            Self::Policy(DualPersonPolicyError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "M_VR_DUAL_PERSON_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            Self::Policy(
                DualPersonPolicyError::Audit(_)
                | DualPersonPolicyError::Database(_)
                | DualPersonPolicyError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M_VR_DUAL_PERSON_INTERNAL",
                "双人策略处理失败",
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

pub fn dual_person_policy_router(state: DualPersonPolicyAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/m-vr/dual-person-policy",
            get(resolve_dual_person_policy_handler),
        )
        .route(
            "/api/v1/m-vr/dual-person-policy/rules",
            get(list_dual_person_policy_rules_handler).put(upsert_dual_person_policy_rule_handler),
        )
        .with_state(state)
}

async fn list_dual_person_policy_rules_handler(
    ctx: AuthContext,
    State(state): State<DualPersonPolicyAppState>,
    Query(query): Query<DualPersonPolicyRuleListQuery>,
) -> Result<Json<DualPersonPolicyRuleListResponse>, DualPersonPolicyHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    let data = state.repository.list(&ctx, query.warehouse_id).await?;
    Ok(Json(DualPersonPolicyRuleListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
            total: None,
        },
        data,
    }))
}

async fn resolve_dual_person_policy_handler(
    ctx: AuthContext,
    State(state): State<DualPersonPolicyAppState>,
    Query(query): Query<ResolveDualPersonPolicyQuery>,
) -> Result<Json<DualPersonPolicyResponse>, DualPersonPolicyHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(state.repository.resolve(&ctx, &query).await?))
}

async fn upsert_dual_person_policy_rule_handler(
    ctx: AuthContext,
    State(state): State<DualPersonPolicyAppState>,
    headers: HeaderMap,
    Json(request): Json<UpsertDualPersonPolicyRuleRequest>,
) -> Result<Json<DualPersonPolicyRule>, DualPersonPolicyHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    if request.scope == DualPersonPolicyScope::Global {
        ctx.require_permission(GLOBAL_WRITE_PERMISSION)?;
    }
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(DualPersonPolicyHandlerError::MissingIdempotencyKey)?;
    Ok(Json(
        state
            .repository
            .upsert(&ctx, request, Utc::now(), idempotency_key)
            .await?
            .value,
    ))
}
