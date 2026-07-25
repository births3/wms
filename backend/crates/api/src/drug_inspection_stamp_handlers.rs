use std::{path::PathBuf, sync::Arc};

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    ApproveDrugInspectionCopyOversizeRequest, CreateDrugInspectionStampVersionRequest,
    DrugInspectionDocumentValidationError, ErrorResponse,
    PublishDrugInspectionProcessingRuleRequest, ReviewDrugInspectionStampVersionRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    drug_inspection_copy_service::{DrugInspectionCopyService, DrugInspectionCopyServiceError},
    drug_inspection_document_repository::{
        DrugInspectionDocumentRepositoryError, PgDrugInspectionStampRepository,
    },
};

pub const M_DI_STAMP_MANAGE_PERMISSION: &str = "m-di.stamp.manage";
pub const M_DI_STAMP_REVIEW_PERMISSION: &str = "m-di.stamp.review";

#[derive(Clone)]
pub struct DrugInspectionStampAppState {
    stamps: Arc<PgDrugInspectionStampRepository>,
    copies: Arc<DrugInspectionCopyService>,
}

impl DrugInspectionStampAppState {
    pub fn with_local_storage(pool: PgPool, storage_root: PathBuf) -> Self {
        Self {
            stamps: Arc::new(PgDrugInspectionStampRepository::new(pool.clone())),
            copies: Arc::new(DrugInspectionCopyService::new(pool, storage_root)),
        }
    }
}

pub fn drug_inspection_stamp_router(state: DrugInspectionStampAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/drug-inspection/stamp-versions",
            get(list_stamp_versions).post(create_stamp_version),
        )
        .route(
            "/api/v1/drug-inspection/stamp-versions/:version_id/submit",
            post(submit_stamp_version),
        )
        .route(
            "/api/v1/drug-inspection/stamp-versions/:version_id/review",
            post(review_stamp_version),
        )
        .route(
            "/api/v1/drug-inspection/customer-copy-jobs",
            get(list_copy_jobs),
        )
        .route(
            "/api/v1/drug-inspection/processing-rule-versions",
            get(list_processing_rules).post(publish_processing_rule),
        )
        .route(
            "/api/v1/drug-inspection/customer-copy-jobs/:job_id/process",
            post(process_copy_job),
        )
        .route(
            "/api/v1/drug-inspection/customer-copy-jobs/:job_id/oversize-approval",
            post(approve_copy_oversize),
        )
        .with_state(state)
}

async fn list_stamp_versions(
    ctx: AuthContext,
    State(state): State<DrugInspectionStampAppState>,
) -> Result<Json<Vec<wms_domain::DrugInspectionStampVersion>>, StampHandlerError> {
    ctx.require_permission(M_DI_STAMP_MANAGE_PERMISSION)?;
    state
        .stamps
        .list_versions(ctx.owner_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn create_stamp_version(
    ctx: AuthContext,
    State(state): State<DrugInspectionStampAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateDrugInspectionStampVersionRequest>,
) -> Result<Json<wms_domain::DrugInspectionStampVersion>, StampHandlerError> {
    ctx.require_permission(M_DI_STAMP_MANAGE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .stamps
        .create_version(&ctx, request, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn submit_stamp_version(
    ctx: AuthContext,
    State(state): State<DrugInspectionStampAppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<wms_domain::DrugInspectionStampVersion>, StampHandlerError> {
    ctx.require_permission(M_DI_STAMP_MANAGE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .stamps
        .submit_version(&ctx, version_id, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn review_stamp_version(
    ctx: AuthContext,
    State(state): State<DrugInspectionStampAppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ReviewDrugInspectionStampVersionRequest>,
) -> Result<Json<wms_domain::DrugInspectionStampVersion>, StampHandlerError> {
    ctx.require_permission(M_DI_STAMP_REVIEW_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .stamps
        .review_version(&ctx, version_id, request, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn list_copy_jobs(
    ctx: AuthContext,
    State(state): State<DrugInspectionStampAppState>,
) -> Result<Json<Vec<wms_domain::DrugInspectionCustomerCopyJob>>, StampHandlerError> {
    ctx.require_permission(M_DI_STAMP_REVIEW_PERMISSION)?;
    state
        .copies
        .list_jobs(ctx.owner_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn list_processing_rules(
    ctx: AuthContext,
    State(state): State<DrugInspectionStampAppState>,
) -> Result<Json<Vec<wms_domain::DrugInspectionProcessingRuleVersion>>, StampHandlerError> {
    ctx.require_permission(M_DI_STAMP_MANAGE_PERMISSION)?;
    state
        .stamps
        .list_processing_rules(ctx.owner_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn publish_processing_rule(
    ctx: AuthContext,
    State(state): State<DrugInspectionStampAppState>,
    headers: HeaderMap,
    Json(request): Json<PublishDrugInspectionProcessingRuleRequest>,
) -> Result<Json<wms_domain::DrugInspectionProcessingRuleVersion>, StampHandlerError> {
    ctx.require_permission(M_DI_STAMP_MANAGE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .stamps
        .publish_processing_rule(&ctx, request, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn process_copy_job(
    ctx: AuthContext,
    State(state): State<DrugInspectionStampAppState>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<wms_domain::DrugInspectionCustomerCopyJob>, StampHandlerError> {
    ctx.require_permission(M_DI_STAMP_REVIEW_PERMISSION)?;
    state
        .copies
        .process_job(ctx.owner_id, job_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn approve_copy_oversize(
    ctx: AuthContext,
    State(state): State<DrugInspectionStampAppState>,
    Path(job_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ApproveDrugInspectionCopyOversizeRequest>,
) -> Result<Json<wms_domain::DrugInspectionCustomerCopyJob>, StampHandlerError> {
    ctx.require_permission(M_DI_STAMP_REVIEW_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .copies
        .approve_oversize(&ctx, job_id, request, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, StampHandlerError> {
    headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 200)
        .map(str::to_string)
        .ok_or(StampHandlerError::IdempotencyRequired)
}

enum StampHandlerError {
    Auth(AuthError),
    IdempotencyRequired,
    Repository(DrugInspectionDocumentRepositoryError),
    Copy(DrugInspectionCopyServiceError),
}

impl From<AuthError> for StampHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<DrugInspectionDocumentRepositoryError> for StampHandlerError {
    fn from(value: DrugInspectionDocumentRepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl From<DrugInspectionCopyServiceError> for StampHandlerError {
    fn from(value: DrugInspectionCopyServiceError) -> Self {
        Self::Copy(value)
    }
}

impl IntoResponse for StampHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::IdempotencyRequired => (
                StatusCode::BAD_REQUEST,
                "M_DI_IDEMPOTENCY_REQUIRED",
                "缺少或非法 Idempotency-Key",
            ),
            Self::Repository(DrugInspectionDocumentRepositoryError::Invalid(error))
            | Self::Copy(DrugInspectionCopyServiceError::Invalid(error)) => validation_error(error),
            Self::Repository(DrugInspectionDocumentRepositoryError::NotFound)
            | Self::Copy(DrugInspectionCopyServiceError::NotFound) => (
                StatusCode::NOT_FOUND,
                "M_DI_STAMP_OR_COPY_NOT_FOUND",
                "图章版本或客户副本任务不存在",
            ),
            Self::Repository(DrugInspectionDocumentRepositoryError::Conflict(
                "reviewer_is_configurer",
            )) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_DI_STAMP_REVIEWER_IS_CONFIGURER",
                "图章配置人与发布审核人必须是不同用户",
            ),
            Self::Repository(DrugInspectionDocumentRepositoryError::Conflict(_))
            | Self::Copy(DrugInspectionCopyServiceError::Conflict(_)) => (
                StatusCode::CONFLICT,
                "M_DI_STAMP_OR_COPY_CONFLICT",
                "当前图章或客户副本状态不允许此操作",
            ),
            Self::Copy(DrugInspectionCopyServiceError::Processing(_)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_DI_COPY_PROCESSING_FAILED",
                "客户副本生成失败，请检查原件、图章或文件大小",
            ),
            Self::Repository(DrugInspectionDocumentRepositoryError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "M_DI_IDEMPOTENCY_CONFLICT",
                "幂等键已用于不同请求",
            ),
            Self::Copy(DrugInspectionCopyServiceError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "M_DI_IDEMPOTENCY_CONFLICT",
                "幂等键已用于不同请求",
            ),
            Self::Repository(DrugInspectionDocumentRepositoryError::Audit(_))
            | Self::Repository(DrugInspectionDocumentRepositoryError::Database(_))
            | Self::Repository(DrugInspectionDocumentRepositoryError::Serialize(_))
            | Self::Copy(DrugInspectionCopyServiceError::Storage(_))
            | Self::Copy(DrugInspectionCopyServiceError::Database(_))
            | Self::Copy(DrugInspectionCopyServiceError::Audit(_))
            | Self::Copy(DrugInspectionCopyServiceError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M_DI_STAMP_OR_COPY_PERSISTENCE_FAILED",
                "图章或客户副本处理失败",
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

fn validation_error(
    error: DrugInspectionDocumentValidationError,
) -> (StatusCode, &'static str, &'static str) {
    let (code, message) = match error {
        DrugInspectionDocumentValidationError::InvalidStampGeometry => {
            ("M_DI_STAMP_GEOMETRY_INVALID", "图章位置或尺寸超出页面")
        }
        DrugInspectionDocumentValidationError::ReviewCommentRequired => {
            ("M_DI_REVIEW_COMMENT_REQUIRED", "退回必须填写审核意见")
        }
        DrugInspectionDocumentValidationError::InvalidDecision => {
            ("M_DI_REVIEW_DECISION_INVALID", "审核结论非法")
        }
        DrugInspectionDocumentValidationError::FieldRequired(_) => {
            ("M_DI_FIELD_REQUIRED", "必填字段缺失")
        }
        DrugInspectionDocumentValidationError::FieldTooLong(_) => {
            ("M_DI_FIELD_TOO_LONG", "字段超长")
        }
        _ => ("M_DI_REQUEST_INVALID", "请求参数非法"),
    };
    (StatusCode::UNPROCESSABLE_ENTITY, code, message)
}
