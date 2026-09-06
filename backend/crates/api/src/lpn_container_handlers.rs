use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use wms_domain::{
    ApplyContainerQualityLockRequest, BatchCreateLpnContainerRequest,
    ChangeContainerQualityLockReasonRequest, CreateLpnContainerRequest, ErrorResponse,
    LpnContainer, LpnContainerListResponse, LpnContainerTypePolicy,
    ReleaseContainerQualityLockRequest, ReleaseContainerQualityLockResponse,
    UpdateLpnContainerRequest, UpsertLpnContainerTypePolicyRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    lpn_container_quality_lock_service::LpnContainerQualityLockService,
    lpn_container_repository::{LpnContainerRepositoryError, PgLpnContainerRepository},
};

const READ_PERMISSION: &str = "m1.master_data.read";
const WRITE_PERMISSION: &str = "m1.master_data.write";
const QUALITY_LOCK_PERMISSION: &str = "m1.quality-lock.manage";
const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";

#[derive(Clone, Debug)]
pub struct LpnContainerAppState {
    pub repository: Arc<PgLpnContainerRepository>,
}

impl LpnContainerAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgLpnContainerRepository::new(pool)),
        }
    }

    pub fn quality_lock(&self) -> LpnContainerQualityLockService {
        LpnContainerQualityLockService::from_repository(self.repository.as_ref())
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ListLpnContainersQuery {
    keyword: Option<String>,
    #[serde(rename = "type")]
    container_type: Option<String>,
    status: Option<String>,
    lock_category: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum LpnContainerHandlerError {
    Auth(AuthError),
    Repository(LpnContainerRepositoryError),
    MissingIdempotencyKey,
}

impl From<AuthError> for LpnContainerHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<LpnContainerRepositoryError> for LpnContainerHandlerError {
    fn from(value: LpnContainerRepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl IntoResponse for LpnContainerHandlerError {
    fn into_response(self) -> Response {
        if let LpnContainerHandlerError::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            LpnContainerHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "M1_LPN_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::NotFound) => (
                StatusCode::NOT_FOUND,
                "M1_LPN_NOT_FOUND",
                "LPN 容器不存在".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::DuplicateCode) => (
                StatusCode::CONFLICT,
                "M1_LPN_DUPLICATE",
                "同一货主的 LPN 码已存在".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::CodeEmpty) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LPN_CODE_EMPTY",
                "LPN 码不能为空".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::CodeTooLong) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LPN_CODE_TOO_LONG",
                "LPN 码长度必须为 1-64".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::TypeInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LPN_TYPE_INVALID",
                "LPN 容器类型非法".to_string(),
            ),
            LpnContainerHandlerError::Repository(
                LpnContainerRepositoryError::BatchCountInvalid,
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LPN_BATCH_COUNT_INVALID",
                "批量新增数量必须为 1-100".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::StatusInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LPN_STATUS_INVALID",
                "LPN 容器状态非法".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::NotDeletable) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LPN_NOT_DELETABLE",
                "在用或作业中的容器不能删除，请先解绑".to_string(),
            ),
            LpnContainerHandlerError::Repository(
                LpnContainerRepositoryError::NumberingUnavailable,
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_LPN_NUMBERING_UNAVAILABLE",
                "该容器类型未配置可用编号规则".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::StateInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_QUALITY_LOCK_STATE_INVALID",
                "容器状态非 in_use，禁止加锁".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::WitnessInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_QUALITY_LOCK_WITNESS_INVALID",
                "见证人缺失或与操作人相同，双人见证不成立".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::MqlNotFinal) => (
                StatusCode::CONFLICT,
                "M1_QUALITY_LOCK_MQL_NOT_FINAL",
                "关联 M-QL 未办结（closed/rejected），禁止解锁".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::ZoneMismatch) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_QUALITY_LOCK_ZONE_MISMATCH",
                "目标库区质量分区与锁类别不匹配".to_string(),
            ),
            LpnContainerHandlerError::Repository(
                LpnContainerRepositoryError::LocationTypeInvalid,
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_QUALITY_LOCK_LOCATION_TYPE_INVALID",
                "加锁容器目标位非存储位（禁上箱拣/零拣位）".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::ReasonInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_QUALITY_LOCK_REASON_INVALID",
                "质量锁原因字典项无效或已停用".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::MqlRequired) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_QUALITY_LOCK_MQL_REQUIRED",
                "不合格质量锁必须关联 M-QL 质量联系单".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::NotLocked) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_QUALITY_LOCK_NOT_LOCKED",
                "当前容器未处于加锁状态".to_string(),
            ),
            LpnContainerHandlerError::Repository(
                LpnContainerRepositoryError::IdempotencyConflict,
            ) => (
                StatusCode::CONFLICT,
                "M1_LPN_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用".to_string(),
            ),
            LpnContainerHandlerError::Repository(LpnContainerRepositoryError::Audit(_))
            | LpnContainerHandlerError::Repository(LpnContainerRepositoryError::Database(_))
            | LpnContainerHandlerError::Repository(LpnContainerRepositoryError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M1_LPN_PERSIST_FAILED",
                "LPN 容器持久化失败".to_string(),
            ),
            LpnContainerHandlerError::Auth(error) => return error.into_response(),
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

pub fn lpn_container_router(state: LpnContainerAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/master-data/lpn-containers",
            get(list_lpn_containers_handler).post(create_lpn_container_handler),
        )
        .route(
            "/api/v1/master-data/lpn-containers/batch-create",
            axum::routing::post(batch_create_lpn_containers_handler),
        )
        .route(
            "/api/v1/master-data/lpn-containers/:id",
            get(get_lpn_container_handler)
                .patch(update_lpn_container_handler)
                .delete(delete_lpn_container_handler),
        )
        .route(
            "/api/v1/master-data/lpn-containers/:id/quality-lock",
            axum::routing::post(apply_quality_lock_handler)
                .patch(change_quality_lock_reason_handler),
        )
        .route(
            "/api/v1/master-data/lpn-containers/:id/quality-lock/release",
            axum::routing::post(release_quality_lock_handler),
        )
        .route(
            "/api/v1/master-data/lpn-container-type-policies",
            get(list_lpn_type_policies_handler).put(upsert_lpn_type_policy_handler),
        )
        .with_state(state)
}

async fn list_lpn_containers_handler(
    ctx: AuthContext,
    State(state): State<LpnContainerAppState>,
    Query(query): Query<ListLpnContainersQuery>,
) -> Result<Json<LpnContainerListResponse>, LpnContainerHandlerError> {
    require_read_permission(&ctx)?;
    Ok(Json(LpnContainerListResponse {
        data: state
            .repository
            .list(
                &ctx,
                query.keyword.as_deref(),
                query.container_type.as_deref(),
                query.status.as_deref(),
                query.lock_category.as_deref(),
            )
            .await?,
    }))
}

async fn get_lpn_container_handler(
    ctx: AuthContext,
    State(state): State<LpnContainerAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<LpnContainer>, LpnContainerHandlerError> {
    require_read_permission(&ctx)?;
    Ok(Json(state.repository.get(&ctx, id).await?))
}

async fn batch_create_lpn_containers_handler(
    ctx: AuthContext,
    State(state): State<LpnContainerAppState>,
    headers: HeaderMap,
    Json(request): Json<BatchCreateLpnContainerRequest>,
) -> Result<Json<LpnContainerListResponse>, LpnContainerHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .repository
            .batch_create(&ctx, request, Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn create_lpn_container_handler(
    ctx: AuthContext,
    State(state): State<LpnContainerAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateLpnContainerRequest>,
) -> Result<Json<LpnContainer>, LpnContainerHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .repository
            .create(&ctx, request, Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn update_lpn_container_handler(
    ctx: AuthContext,
    State(state): State<LpnContainerAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateLpnContainerRequest>,
) -> Result<Json<LpnContainer>, LpnContainerHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .repository
            .update(&ctx, id, request, Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn delete_lpn_container_handler(
    ctx: AuthContext,
    State(state): State<LpnContainerAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<LpnContainer>, LpnContainerHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .repository
            .delete(&ctx, id, Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn apply_quality_lock_handler(
    ctx: AuthContext,
    State(state): State<LpnContainerAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ApplyContainerQualityLockRequest>,
) -> Result<Json<LpnContainer>, LpnContainerHandlerError> {
    ctx.require_permission(QUALITY_LOCK_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .quality_lock()
            .apply_quality_lock(&ctx, id, request, Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn change_quality_lock_reason_handler(
    ctx: AuthContext,
    State(state): State<LpnContainerAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ChangeContainerQualityLockReasonRequest>,
) -> Result<Json<LpnContainer>, LpnContainerHandlerError> {
    ctx.require_permission(QUALITY_LOCK_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .quality_lock()
            .change_quality_lock_reason(&ctx, id, request, Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn release_quality_lock_handler(
    ctx: AuthContext,
    State(state): State<LpnContainerAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ReleaseContainerQualityLockRequest>,
) -> Result<Json<ReleaseContainerQualityLockResponse>, LpnContainerHandlerError> {
    ctx.require_permission(QUALITY_LOCK_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .quality_lock()
            .release_quality_lock(&ctx, id, request, Utc::now(), &idempotency_key)
            .await?,
    ))
}

async fn list_lpn_type_policies_handler(
    ctx: AuthContext,
    State(state): State<LpnContainerAppState>,
) -> Result<Json<Vec<LpnContainerTypePolicy>>, LpnContainerHandlerError> {
    require_read_permission(&ctx)?;
    Ok(Json(state.repository.list_type_policies(&ctx).await?))
}

async fn upsert_lpn_type_policy_handler(
    ctx: AuthContext,
    State(state): State<LpnContainerAppState>,
    headers: HeaderMap,
    Json(request): Json<UpsertLpnContainerTypePolicyRequest>,
) -> Result<Json<LpnContainerTypePolicy>, LpnContainerHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    Ok(Json(
        state
            .repository
            .upsert_type_policy_idempotent(&ctx, request, &idempotency_key)
            .await?,
    ))
}

fn require_read_permission(ctx: &AuthContext) -> Result<(), AuthError> {
    if ctx.has_permission(READ_PERMISSION) || ctx.has_permission(WRITE_PERMISSION) {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied(READ_PERMISSION.to_string()))
    }
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, LpnContainerHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or(LpnContainerHandlerError::MissingIdempotencyKey)
}
