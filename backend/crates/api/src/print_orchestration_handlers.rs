//! H9 print orchestration HTTP handlers.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    AggregationFieldCatalogResponse, AggregationRuleTestResult, AggregationRuleVersion,
    AggregationRuleVersionListResponse, CreateAggregationRuleDraftRequest, CreateCutoffPlanRequest,
    CreatePrintSuiteDraftRequest, CutoffPlan, CutoffPlanListResponse,
    DeliveryNoteCandidateListResponse, DeliveryNoteGroup, DeliveryNoteGroupListResponse,
    ErrorResponse, ManualDeliveryNoteCutoffRequest, PrintDocumentCategoryListResponse,
    PrintSuiteInstanceListResponse, PrintSuiteTestResult, PrintSuiteVersion,
    PrintSuiteVersionListResponse, PublishRouteBindingRequest, RouteBinding,
    RouteBindingListResponse, TestAggregationRuleRequest, TestPrintSuiteRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    document_numbering::DocumentNumberingError,
    print_orchestration::{PrintOrchestrationError, PrintOrchestrationService},
};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const READ_PERMISSION: &str = "h9.print_orchestration.read";
const WRITE_PERMISSION: &str = "h9.print_orchestration.write";

/// H9 print orchestration HTTP state.
#[derive(Clone, Debug)]
pub struct PrintOrchestrationAppState {
    service: PrintOrchestrationService,
}

#[derive(Debug)]
enum PrintOrchestrationHandlerError {
    Auth(AuthError),
    Orchestration(PrintOrchestrationError),
    MissingIdempotencyKey,
}

#[derive(Debug, Deserialize)]
struct WarehouseFilter {
    warehouse_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct SuiteInstanceFilter {
    group_id: Option<Uuid>,
}

impl PrintOrchestrationAppState {
    /// Builds the H9 print orchestration HTTP state.
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            service: PrintOrchestrationService::with_postgres(pool),
        }
    }
}

impl From<AuthError> for PrintOrchestrationHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<PrintOrchestrationError> for PrintOrchestrationHandlerError {
    fn from(value: PrintOrchestrationError) -> Self {
        Self::Orchestration(value)
    }
}

impl IntoResponse for PrintOrchestrationHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Auth(error) => return error.into_response(),
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "H9_PRINT_ORCHESTRATION_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::Orchestration(PrintOrchestrationError::InvalidRequest) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_PRINT_ORCHESTRATION_INVALID",
                "打印编排参数非法",
            ),
            Self::Orchestration(PrintOrchestrationError::EffectivePeriodOverlap) => (
                StatusCode::CONFLICT,
                "H9_PRINT_ORCHESTRATION_PERIOD_OVERLAP",
                "同级配置的有效期重叠",
            ),
            Self::Orchestration(PrintOrchestrationError::CutoffPlanNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_CUTOFF_PLAN_NOT_FOUND",
                "截单计划不存在",
            ),
            Self::Orchestration(PrintOrchestrationError::InvalidState) => (
                StatusCode::CONFLICT,
                "H9_CUTOFF_PLAN_STATE_INVALID",
                "截单计划状态不允许当前操作",
            ),
            Self::Orchestration(PrintOrchestrationError::RouteBindingNotFound) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_ROUTE_BINDING_NOT_FOUND",
                "送货地址没有有效线路绑定",
            ),
            Self::Orchestration(PrintOrchestrationError::OrderNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_DELIVERY_NOTE_ORDER_NOT_FOUND",
                "出库订单不存在或尚未冻结线路",
            ),
            Self::Orchestration(PrintOrchestrationError::OrderNotEligibleForCutoff) => (
                StatusCode::CONFLICT,
                "H9_DELIVERY_NOTE_ORDER_STATE_INVALID",
                "只有已确认的出库订单可以截单",
            ),
            Self::Orchestration(PrintOrchestrationError::AggregationBoundaryMismatch) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_DELIVERY_NOTE_BOUNDARY_MISMATCH",
                "订单不属于同一货主、仓库和送货地址",
            ),
            Self::Orchestration(PrintOrchestrationError::AggregationRuleMismatch) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_AGGREGATION_RULE_MISMATCH",
                "所选订单不属于同一个归集规则分组",
            ),
            Self::Orchestration(PrintOrchestrationError::AggregationRuleNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_AGGREGATION_RULE_NOT_FOUND",
                "归集规则版本不存在",
            ),
            Self::Orchestration(PrintOrchestrationError::AggregationRuleInvalidState) => (
                StatusCode::CONFLICT,
                "H9_AGGREGATION_RULE_STATE_INVALID",
                "归集规则版本状态不允许当前操作",
            ),
            Self::Orchestration(PrintOrchestrationError::PrintSuiteNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_PRINT_SUITE_NOT_FOUND",
                "打印组套版本不存在",
            ),
            Self::Orchestration(PrintOrchestrationError::PrintSuiteInvalidState) => (
                StatusCode::CONFLICT,
                "H9_PRINT_SUITE_STATE_INVALID",
                "打印组套版本状态不允许当前操作",
            ),
            Self::Orchestration(PrintOrchestrationError::PrintSuiteCategoryInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_PRINT_SUITE_CATEGORY_INVALID",
                "单据分类未在 M1 字典登记或来源模式不匹配",
            ),
            Self::Orchestration(PrintOrchestrationError::PrintSuiteBindingInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_PRINT_SUITE_BINDING_INVALID",
                "打印项绑定非法：rendered 需已发布模板版本，external_file 需稳定文件引用",
            ),
            Self::Orchestration(PrintOrchestrationError::DeliveryNoteGroupNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_DELIVERY_NOTE_GROUP_NOT_FOUND",
                "随货同行单归集组不存在",
            ),
            Self::Orchestration(PrintOrchestrationError::OrderAlreadyCutoff) => (
                StatusCode::CONFLICT,
                "H9_DELIVERY_NOTE_ORDER_ALREADY_CUTOFF",
                "订单已归入随货同行单",
            ),
            Self::Orchestration(PrintOrchestrationError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "H9_PRINT_ORCHESTRATION_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            Self::Orchestration(PrintOrchestrationError::DocumentNumbering(
                DocumentNumberingError::RuleNotFound,
            )) => (
                StatusCode::CONFLICT,
                "H9_DELIVERY_NOTE_NUMBERING_RULE_MISSING",
                "随货同行单编号规则未配置",
            ),
            Self::Orchestration(PrintOrchestrationError::DocumentNumbering(
                DocumentNumberingError::DocumentTypeInvalid,
            )) => (
                StatusCode::CONFLICT,
                "H9_DELIVERY_NOTE_CATEGORY_INVALID",
                "随货同行单分类未启用",
            ),
            Self::Orchestration(PrintOrchestrationError::DocumentNumbering(_))
            | Self::Orchestration(PrintOrchestrationError::Audit(_))
            | Self::Orchestration(PrintOrchestrationError::Database(_))
            | Self::Orchestration(PrintOrchestrationError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H9_DELIVERY_NOTE_CUTOFF_FAILED",
                "随货同行单截单失败",
            ),
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

/// Builds the H9 print orchestration routes.
pub fn print_orchestration_router(state: PrintOrchestrationAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/print-orchestration/delivery-note-candidates",
            get(list_delivery_note_candidates_handler),
        )
        .route(
            "/api/v1/print-orchestration/delivery-note-groups",
            get(list_delivery_note_groups_handler),
        )
        .route(
            "/api/v1/print-orchestration/delivery-note-groups/manual-cutoff",
            post(manual_delivery_note_cutoff_handler),
        )
        .route(
            "/api/v1/print-orchestration/route-bindings",
            get(list_route_bindings_handler).post(publish_route_binding_handler),
        )
        .route(
            "/api/v1/print-orchestration/cutoff-plans",
            get(list_cutoff_plans_handler).post(create_cutoff_plan_handler),
        )
        .route(
            "/api/v1/print-orchestration/cutoff-plans/:plan_id/publish",
            post(publish_cutoff_plan_handler),
        )
        .route(
            "/api/v1/print-orchestration/aggregation-fields",
            get(list_aggregation_fields_handler),
        )
        .route(
            "/api/v1/print-orchestration/aggregation-rules/versions",
            get(list_aggregation_rules_handler).post(create_aggregation_rule_draft_handler),
        )
        .route(
            "/api/v1/print-orchestration/aggregation-rules/versions/:version_id/test",
            post(test_aggregation_rule_handler),
        )
        .route(
            "/api/v1/print-orchestration/aggregation-rules/versions/:version_id/publish",
            post(publish_aggregation_rule_handler),
        )
        .route(
            "/api/v1/print-orchestration/aggregation-rules/versions/:version_id/disable",
            post(disable_aggregation_rule_handler),
        )
        .route(
            "/api/v1/print-orchestration/print-document-categories",
            get(list_print_document_categories_handler),
        )
        .route(
            "/api/v1/print-orchestration/print-suites/versions",
            get(list_print_suites_handler).post(create_print_suite_draft_handler),
        )
        .route(
            "/api/v1/print-orchestration/print-suites/versions/:version_id/test",
            post(test_print_suite_handler),
        )
        .route(
            "/api/v1/print-orchestration/print-suites/versions/:version_id/publish",
            post(publish_print_suite_handler),
        )
        .route(
            "/api/v1/print-orchestration/print-suites/versions/:version_id/disable",
            post(disable_print_suite_handler),
        )
        .route(
            "/api/v1/print-orchestration/suite-instances",
            get(list_print_suite_instances_handler),
        )
        .with_state(state)
}

async fn list_delivery_note_candidates_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Query(query): Query<WarehouseFilter>,
) -> Result<Json<DeliveryNoteCandidateListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state
            .service
            .list_delivery_note_candidates(&ctx, query.warehouse_id)
            .await?,
    ))
}

async fn list_delivery_note_groups_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Query(query): Query<WarehouseFilter>,
) -> Result<Json<DeliveryNoteGroupListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state
            .service
            .list_delivery_note_groups(&ctx, query.warehouse_id)
            .await?,
    ))
}

async fn manual_delivery_note_cutoff_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    headers: HeaderMap,
    Json(request): Json<ManualDeliveryNoteCutoffRequest>,
) -> Result<Json<DeliveryNoteGroup>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .manual_cutoff(&ctx, request, Utc::now(), idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn publish_route_binding_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    headers: HeaderMap,
    Json(request): Json<PublishRouteBindingRequest>,
) -> Result<Json<RouteBinding>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .publish_route_binding(&ctx, request, Utc::now(), idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn list_route_bindings_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Query(query): Query<WarehouseFilter>,
) -> Result<Json<RouteBindingListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state
            .service
            .list_route_bindings(&ctx, query.warehouse_id)
            .await?,
    ))
}

async fn create_cutoff_plan_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateCutoffPlanRequest>,
) -> Result<Json<CutoffPlan>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .create_cutoff_plan(&ctx, request, Utc::now(), idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn list_cutoff_plans_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Query(query): Query<WarehouseFilter>,
) -> Result<Json<CutoffPlanListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state
            .service
            .list_cutoff_plans(&ctx, query.warehouse_id)
            .await?,
    ))
}

async fn publish_cutoff_plan_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(plan_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<CutoffPlan>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .publish_cutoff_plan(&ctx, plan_id, Utc::now(), idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn list_aggregation_fields_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
) -> Result<Json<AggregationFieldCatalogResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(state.service.list_aggregation_fields(&ctx).await?))
}

async fn list_aggregation_rules_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
) -> Result<Json<AggregationRuleVersionListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(state.service.list_aggregation_rules(&ctx).await?))
}

async fn create_aggregation_rule_draft_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateAggregationRuleDraftRequest>,
) -> Result<Json<AggregationRuleVersion>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .create_aggregation_rule_draft(
            &ctx,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn test_aggregation_rule_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<TestAggregationRuleRequest>,
) -> Result<Json<AggregationRuleTestResult>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .test_aggregation_rule(
            &ctx,
            version_id,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn publish_aggregation_rule_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<AggregationRuleVersion>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .publish_aggregation_rule(
            &ctx,
            version_id,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn disable_aggregation_rule_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<AggregationRuleVersion>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .disable_aggregation_rule(
            &ctx,
            version_id,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn list_print_document_categories_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
) -> Result<Json<PrintDocumentCategoryListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state.service.list_print_document_categories(&ctx).await?,
    ))
}

async fn list_print_suites_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
) -> Result<Json<PrintSuiteVersionListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(state.service.list_print_suites(&ctx).await?))
}

async fn create_print_suite_draft_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    headers: HeaderMap,
    Json(request): Json<CreatePrintSuiteDraftRequest>,
) -> Result<Json<PrintSuiteVersion>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .create_print_suite_draft(
            &ctx,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn test_print_suite_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<TestPrintSuiteRequest>,
) -> Result<Json<PrintSuiteTestResult>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .test_print_suite(
            &ctx,
            version_id,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn publish_print_suite_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PrintSuiteVersion>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .publish_print_suite(
            &ctx,
            version_id,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn disable_print_suite_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PrintSuiteVersion>, PrintOrchestrationHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .disable_print_suite(
            &ctx,
            version_id,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn list_print_suite_instances_handler(
    ctx: AuthContext,
    State(state): State<PrintOrchestrationAppState>,
    Query(query): Query<SuiteInstanceFilter>,
) -> Result<Json<PrintSuiteInstanceListResponse>, PrintOrchestrationHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state
            .service
            .list_print_suite_instances(&ctx, query.group_id)
            .await?,
    ))
}

fn idempotency_key_from_headers(
    headers: &HeaderMap,
) -> Result<&str, PrintOrchestrationHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(PrintOrchestrationHandlerError::MissingIdempotencyKey)
}
