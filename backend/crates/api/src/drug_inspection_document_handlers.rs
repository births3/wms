use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::NaiveDate;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use wms_domain::{
    CreateDrugInspectionCorrectionRequest, CreateDrugInspectionVersionRequest,
    CreateUpstreamDeliveryVersionRequest, DrugInspectionDocumentValidationError, ErrorResponse,
    ReuseDrugInspectionReportRequest, ReviewDrugInspectionVersionRequest,
    UpdateDrugInspectionDraftRequest, UpsertDrugInspectionRequirementRuleRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    drug_inspection_document_repository::{
        DrugInspectionDocumentRepositoryError, PgDrugInspectionDocumentRepository,
    },
};

pub const M_DI_DOCUMENT_READ_PERMISSION: &str = "m-di.document.read";
pub const M_DI_DOCUMENT_WRITE_PERMISSION: &str = "m-di.document.write";
pub const M_DI_DOCUMENT_REVIEW_PERMISSION: &str = "m-di.document.review";
pub const M_DI_REQUIREMENT_RULE_MANAGE_PERMISSION: &str = "m-di.requirement-rule.manage";

#[derive(Clone)]
pub struct DrugInspectionDocumentAppState {
    repository: Arc<PgDrugInspectionDocumentRepository>,
}

impl DrugInspectionDocumentAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgDrugInspectionDocumentRepository::new(pool)),
        }
    }
}

pub fn drug_inspection_document_router(state: DrugInspectionDocumentAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/drug-inspection/inbound-documents",
            get(list_inbound_documents),
        )
        .route(
            "/api/v1/drug-inspection/report-versions",
            post(create_version),
        )
        .route(
            "/api/v1/drug-inspection/report-versions/editable",
            get(find_editable_version),
        )
        .route(
            "/api/v1/drug-inspection/report-versions/:version_id",
            put(update_draft_version),
        )
        .route(
            "/api/v1/drug-inspection/reports/reusable",
            get(find_reusable_report),
        )
        .route(
            "/api/v1/drug-inspection/review-queue",
            get(list_review_queue),
        )
        .route(
            "/api/v1/drug-inspection/reports/:report_id/versions",
            get(list_report_versions),
        )
        .route(
            "/api/v1/drug-inspection/report-versions/:version_id/submit",
            post(submit_version),
        )
        .route(
            "/api/v1/drug-inspection/report-versions/:version_id/review",
            post(review_version),
        )
        .route(
            "/api/v1/drug-inspection/reports/:report_id/corrections",
            post(create_correction),
        )
        .route(
            "/api/v1/drug-inspection/reports/:report_id/reuse",
            post(reuse_report),
        )
        .route(
            "/api/v1/drug-inspection/upstream-delivery-document-versions",
            post(create_upstream_delivery_version),
        )
        .route(
            "/api/v1/drug-inspection/upstream-delivery-documents/:document_id/versions",
            get(list_upstream_delivery_versions),
        )
        .route(
            "/api/v1/drug-inspection/requirement-rules",
            get(list_requirement_rules),
        )
        .route(
            "/api/v1/drug-inspection/requirement-rules/current",
            put(upsert_requirement_rule),
        )
        .with_state(state)
}

async fn list_requirement_rules(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
) -> Result<Json<Vec<wms_domain::DrugInspectionRequirementRule>>, DrugInspectionDocumentHandlerError>
{
    ctx.require_permission(M_DI_REQUIREMENT_RULE_MANAGE_PERMISSION)?;
    state
        .repository
        .list_requirement_rules(ctx.owner_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn upsert_requirement_rule(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    headers: HeaderMap,
    Json(request): Json<UpsertDrugInspectionRequirementRuleRequest>,
) -> Result<Json<wms_domain::DrugInspectionRequirementRule>, DrugInspectionDocumentHandlerError> {
    ctx.require_permission(M_DI_REQUIREMENT_RULE_MANAGE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .repository
        .upsert_requirement_rule(&ctx, request, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

#[derive(Deserialize)]
struct InboundDocumentListQuery {
    received_from: Option<NaiveDate>,
    received_to: Option<NaiveDate>,
    #[serde(default)]
    missing_drug_inspection: bool,
    #[serde(default)]
    missing_upstream_delivery: bool,
}

#[derive(Deserialize)]
struct ReusableReportQuery {
    product_id: Uuid,
    batch_no: String,
    asn_id: Option<Uuid>,
}

#[derive(Deserialize)]
struct EditableVersionQuery {
    product_id: Uuid,
    batch_no: String,
    asn_id: Uuid,
}

async fn list_inbound_documents(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    Query(query): Query<InboundDocumentListQuery>,
) -> Result<Json<wms_domain::InboundDocumentEntryListResponse>, DrugInspectionDocumentHandlerError>
{
    ctx.require_permission(M_DI_DOCUMENT_READ_PERMISSION)?;
    state
        .repository
        .list_inbound_documents(
            ctx.owner_id,
            crate::drug_inspection_document_repository::InboundDocumentQuery {
                received_from: query.received_from,
                received_to: query.received_to,
                missing_drug_inspection: query.missing_drug_inspection,
                missing_upstream_delivery: query.missing_upstream_delivery,
            },
        )
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn create_version(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateDrugInspectionVersionRequest>,
) -> Result<Json<wms_domain::DrugInspectionReportVersion>, DrugInspectionDocumentHandlerError> {
    ctx.require_permission(M_DI_DOCUMENT_WRITE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .repository
        .create_version(&ctx, request, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn find_editable_version(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    Query(query): Query<EditableVersionQuery>,
) -> Result<Json<wms_domain::DrugInspectionReportVersion>, DrugInspectionDocumentHandlerError> {
    ctx.require_permission(M_DI_DOCUMENT_READ_PERMISSION)?;
    state
        .repository
        .find_editable_version(
            ctx.owner_id,
            ctx.user_id,
            query.asn_id,
            query.product_id,
            &query.batch_no,
        )
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn update_draft_version(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateDrugInspectionDraftRequest>,
) -> Result<Json<wms_domain::DrugInspectionReportVersion>, DrugInspectionDocumentHandlerError> {
    ctx.require_permission(M_DI_DOCUMENT_WRITE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .repository
        .update_draft_version(&ctx, version_id, request, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn find_reusable_report(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    Query(query): Query<ReusableReportQuery>,
) -> Result<
    Json<wms_domain::ReusableDrugInspectionReportResponse>,
    DrugInspectionDocumentHandlerError,
> {
    ctx.require_permission(M_DI_DOCUMENT_READ_PERMISSION)?;
    state
        .repository
        .find_reusable_report(
            ctx.owner_id,
            query.product_id,
            &query.batch_no,
            query.asn_id,
        )
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn list_review_queue(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
) -> Result<Json<Vec<wms_domain::DrugInspectionReviewQueueEntry>>, DrugInspectionDocumentHandlerError>
{
    ctx.require_permission(M_DI_DOCUMENT_REVIEW_PERMISSION)?;
    state
        .repository
        .list_review_queue(ctx.owner_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn list_report_versions(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    Path(report_id): Path<Uuid>,
) -> Result<Json<Vec<wms_domain::DrugInspectionReportVersion>>, DrugInspectionDocumentHandlerError>
{
    ctx.require_permission(M_DI_DOCUMENT_READ_PERMISSION)?;
    state
        .repository
        .list_report_versions(ctx.owner_id, report_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn submit_version(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<wms_domain::DrugInspectionReportVersion>, DrugInspectionDocumentHandlerError> {
    ctx.require_permission(M_DI_DOCUMENT_WRITE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .repository
        .submit_version(&ctx, version_id, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn review_version(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ReviewDrugInspectionVersionRequest>,
) -> Result<Json<wms_domain::DrugInspectionReportVersion>, DrugInspectionDocumentHandlerError> {
    ctx.require_permission(M_DI_DOCUMENT_REVIEW_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .repository
        .review_version(&ctx, version_id, request, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn create_correction(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    Path(report_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateDrugInspectionCorrectionRequest>,
) -> Result<Json<wms_domain::DrugInspectionReportVersion>, DrugInspectionDocumentHandlerError> {
    ctx.require_permission(M_DI_DOCUMENT_WRITE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .repository
        .create_correction(&ctx, report_id, request, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn reuse_report(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    Path(report_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ReuseDrugInspectionReportRequest>,
) -> Result<Json<wms_domain::ReuseDrugInspectionReportResponse>, DrugInspectionDocumentHandlerError>
{
    ctx.require_permission(M_DI_DOCUMENT_WRITE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .repository
        .reuse_report(&ctx, report_id, request, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn create_upstream_delivery_version(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateUpstreamDeliveryVersionRequest>,
) -> Result<Json<wms_domain::UpstreamDeliveryDocumentVersion>, DrugInspectionDocumentHandlerError> {
    ctx.require_permission(M_DI_DOCUMENT_WRITE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    state
        .repository
        .create_upstream_delivery_version(&ctx, request, &key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn list_upstream_delivery_versions(
    ctx: AuthContext,
    State(state): State<DrugInspectionDocumentAppState>,
    Path(document_id): Path<Uuid>,
) -> Result<
    Json<Vec<wms_domain::UpstreamDeliveryDocumentVersion>>,
    DrugInspectionDocumentHandlerError,
> {
    ctx.require_permission(M_DI_DOCUMENT_READ_PERMISSION)?;
    state
        .repository
        .list_upstream_delivery_versions(ctx.owner_id, document_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, DrugInspectionDocumentHandlerError> {
    headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 200)
        .map(str::to_string)
        .ok_or(DrugInspectionDocumentHandlerError::IdempotencyRequired)
}

enum DrugInspectionDocumentHandlerError {
    Auth(AuthError),
    IdempotencyRequired,
    Repository(DrugInspectionDocumentRepositoryError),
}

impl From<AuthError> for DrugInspectionDocumentHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<DrugInspectionDocumentRepositoryError> for DrugInspectionDocumentHandlerError {
    fn from(value: DrugInspectionDocumentRepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl IntoResponse for DrugInspectionDocumentHandlerError {
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
            Self::Repository(DrugInspectionDocumentRepositoryError::Invalid(error)) => {
                validation_error(error)
            }
            Self::Repository(DrugInspectionDocumentRepositoryError::NotFound) => (
                StatusCode::NOT_FOUND,
                "M_DI_DOCUMENT_NOT_FOUND",
                "药检单、附件或上游随货同行单不存在",
            ),
            Self::Repository(DrugInspectionDocumentRepositoryError::Conflict(
                "reviewer_is_uploader",
            )) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_DI_REVIEWER_IS_UPLOADER",
                "上传人与审核人必须是不同用户",
            ),
            Self::Repository(DrugInspectionDocumentRepositoryError::Conflict(_)) => (
                StatusCode::CONFLICT,
                "M_DI_DOCUMENT_CONFLICT",
                "当前状态或关联关系不允许此操作",
            ),
            Self::Repository(DrugInspectionDocumentRepositoryError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "M_DI_IDEMPOTENCY_CONFLICT",
                "幂等键已用于不同请求",
            ),
            Self::Repository(DrugInspectionDocumentRepositoryError::Audit(_))
            | Self::Repository(DrugInspectionDocumentRepositoryError::Database(_))
            | Self::Repository(DrugInspectionDocumentRepositoryError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M_DI_DOCUMENT_PERSISTENCE_FAILED",
                "药检资料或审计持久化失败",
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
        DrugInspectionDocumentValidationError::FieldRequired(_) => {
            ("M_DI_FIELD_REQUIRED", "药检资料必填字段缺失")
        }
        DrugInspectionDocumentValidationError::FieldTooLong(_) => {
            ("M_DI_FIELD_TOO_LONG", "药检资料字段超长")
        }
        DrugInspectionDocumentValidationError::InvalidSource => {
            ("M_DI_SOURCE_INVALID", "药检单来源非法")
        }
        DrugInspectionDocumentValidationError::InvalidProcessingMode => {
            ("M_DI_PROCESSING_MODE_INVALID", "图像处理方式非法")
        }
        DrugInspectionDocumentValidationError::InvalidDecision => {
            ("M_DI_REVIEW_DECISION_INVALID", "审核结论非法")
        }
        DrugInspectionDocumentValidationError::ReviewCommentRequired => {
            ("M_DI_REVIEW_COMMENT_REQUIRED", "退回修改必须填写审核意见")
        }
        DrugInspectionDocumentValidationError::ModificationReasonRequired => (
            "M_DI_MODIFICATION_REASON_REQUIRED",
            "重新上传必须填写修改原因",
        ),
        DrugInspectionDocumentValidationError::EmptyAsnSelection => {
            ("M_DI_ASN_REQUIRED", "至少关联一个 ASN")
        }
        DrugInspectionDocumentValidationError::EmptyAttachmentSelection => {
            ("M_DI_ATTACHMENT_INVALID", "上游随货同行单文件组合非法")
        }
        DrugInspectionDocumentValidationError::InvalidStampGeometry => {
            ("M_DI_STAMP_GEOMETRY_INVALID", "图章位置或尺寸超出页面")
        }
        DrugInspectionDocumentValidationError::InvalidMissingBehavior => (
            "M_DI_REQUIREMENT_RULE_INVALID",
            "商品类别不能为空，缺失处理只能选择警告或阻塞",
        ),
    };
    (StatusCode::UNPROCESSABLE_ENTITY, code, message)
}
