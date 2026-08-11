//! Runtime Axum handlers for H2 lifecycle operations.

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
    AuditArchivePartitionState as AuditArchivePartitionStateDto,
    AuditArchivePartitionStateListResponse, AuditArchiveRunRequest, AuditArchiveRunResponse,
    BusinessArchiveJob as BusinessArchiveJobDto,
    BusinessRetentionPolicy as BusinessRetentionPolicyDto, BusinessRetentionPolicyListResponse,
    ErrorResponse, EventDelivery as EventDeliveryDto, EventDeliveryListResponse,
    EventDeliveryNackRequest, PageMeta, PlanBusinessArchiveJobRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    h2_lifecycle::{
        acknowledge_event_delivery, list_audit_partition_states, pending_event_deliveries,
        plan_business_archive_job, record_delivery_failure, run_audit_archive_cycle,
        AuditArchiveRun, AuditPartitionState, BusinessArchiveJob, BusinessRetentionPolicy,
        EventDelivery, H2LifecycleError,
    },
};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const READ_PERMISSIONS: &[&str] = &["audit.read", "h2.lifecycle.read"];
const WRITE_PERMISSIONS: &[&str] = &["audit.write", "h2.lifecycle.write"];

#[derive(Clone, Debug)]
pub struct H2LifecycleAppState {
    pool: PgPool,
}

#[derive(Debug, Deserialize)]
struct PendingDeliveriesQuery {
    limit: Option<i64>,
}

#[derive(Debug)]
enum H2LifecycleHandlerError {
    Auth(AuthError),
    Lifecycle(H2LifecycleError),
    MissingIdempotencyKey,
}

impl H2LifecycleAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl From<AuthError> for H2LifecycleHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<H2LifecycleError> for H2LifecycleHandlerError {
    fn from(value: H2LifecycleError) -> Self {
        Self::Lifecycle(value)
    }
}

impl IntoResponse for H2LifecycleHandlerError {
    fn into_response(self) -> Response {
        if let H2LifecycleHandlerError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            H2LifecycleHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "H2_LIFECYCLE_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            H2LifecycleHandlerError::Lifecycle(H2LifecycleError::NotFound) => (
                StatusCode::NOT_FOUND,
                "H2_LIFECYCLE_NOT_FOUND",
                "H2 生命周期对象不存在",
            ),
            H2LifecycleHandlerError::Lifecycle(H2LifecycleError::InvalidInput(_)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H2_LIFECYCLE_INVALID",
                "H2 生命周期请求参数非法",
            ),
            H2LifecycleHandlerError::Lifecycle(
                H2LifecycleError::Audit(_) | H2LifecycleError::Database(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H2_LIFECYCLE_FAILED",
                "H2 生命周期处理失败",
            ),
            H2LifecycleHandlerError::Auth(_) => unreachable!("auth error returned above"),
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

pub fn h2_lifecycle_router(state: H2LifecycleAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/audit/archive/partitions",
            get(list_audit_archive_partitions_handler),
        )
        .route(
            "/api/v1/audit/archive/runs",
            post(run_audit_archive_handler),
        )
        .route(
            "/api/v1/event-bus/deliveries/pending",
            get(list_pending_event_deliveries_handler),
        )
        .route(
            "/api/v1/event-bus/deliveries/:delivery_id/ack",
            post(ack_event_delivery_handler),
        )
        .route(
            "/api/v1/event-bus/deliveries/:delivery_id/nack",
            post(nack_event_delivery_handler),
        )
        .route(
            "/api/v1/business-retention/policies",
            get(list_business_retention_policies_handler),
        )
        .route(
            "/api/v1/business-retention/jobs",
            post(plan_business_archive_job_handler),
        )
        .with_state(state)
}

async fn list_audit_archive_partitions_handler(
    ctx: AuthContext,
    State(state): State<H2LifecycleAppState>,
) -> Result<Json<AuditArchivePartitionStateListResponse>, H2LifecycleHandlerError> {
    require_any_permission(&ctx, READ_PERMISSIONS)?;
    let data = list_audit_partition_states(&state.pool).await?;
    Ok(Json(AuditArchivePartitionStateListResponse {
        page: page_meta(data.len()),
        data: data.into_iter().map(audit_partition_dto).collect(),
    }))
}

async fn run_audit_archive_handler(
    ctx: AuthContext,
    State(state): State<H2LifecycleAppState>,
    headers: HeaderMap,
    Json(req): Json<AuditArchiveRunRequest>,
) -> Result<Json<AuditArchiveRunResponse>, H2LifecycleHandlerError> {
    require_any_permission(&ctx, WRITE_PERMISSIONS)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let run = run_audit_archive_cycle(
        &state.pool,
        ctx.owner_id,
        req.reference_date.unwrap_or_else(|| now.date_naive()),
        now,
        &idempotency_key,
    )
    .await?;
    Ok(Json(audit_archive_run_dto(run)))
}

async fn list_pending_event_deliveries_handler(
    ctx: AuthContext,
    State(state): State<H2LifecycleAppState>,
    Query(query): Query<PendingDeliveriesQuery>,
) -> Result<Json<EventDeliveryListResponse>, H2LifecycleHandlerError> {
    require_any_permission(&ctx, READ_PERMISSIONS)?;
    let data =
        pending_event_deliveries(&state.pool, ctx.owner_id, query.limit.unwrap_or(100)).await?;
    Ok(Json(EventDeliveryListResponse {
        page: page_meta(data.len()),
        data: data.into_iter().map(event_delivery_dto).collect(),
    }))
}

async fn ack_event_delivery_handler(
    ctx: AuthContext,
    State(state): State<H2LifecycleAppState>,
    Path(delivery_id): Path<Uuid>,
) -> Result<Json<EventDeliveryDto>, H2LifecycleHandlerError> {
    require_any_permission(&ctx, WRITE_PERMISSIONS)?;
    let delivery =
        acknowledge_event_delivery(&state.pool, ctx.owner_id, delivery_id, Utc::now()).await?;
    Ok(Json(event_delivery_dto(delivery)))
}

async fn nack_event_delivery_handler(
    ctx: AuthContext,
    State(state): State<H2LifecycleAppState>,
    Path(delivery_id): Path<Uuid>,
    Json(req): Json<EventDeliveryNackRequest>,
) -> Result<Json<EventDeliveryDto>, H2LifecycleHandlerError> {
    require_any_permission(&ctx, WRITE_PERMISSIONS)?;
    let delivery = record_delivery_failure(
        &state.pool,
        ctx.owner_id,
        delivery_id,
        &req.error,
        Utc::now(),
    )
    .await?;
    Ok(Json(event_delivery_dto(delivery)))
}

async fn list_business_retention_policies_handler(
    ctx: AuthContext,
    State(state): State<H2LifecycleAppState>,
) -> Result<Json<BusinessRetentionPolicyListResponse>, H2LifecycleHandlerError> {
    require_any_permission(&ctx, READ_PERMISSIONS)?;
    let data =
        crate::h2_lifecycle::list_business_retention_policies(&state.pool, ctx.owner_id).await?;
    Ok(Json(BusinessRetentionPolicyListResponse {
        page: page_meta(data.len()),
        data: data
            .into_iter()
            .map(business_retention_policy_dto)
            .collect(),
    }))
}

async fn plan_business_archive_job_handler(
    ctx: AuthContext,
    State(state): State<H2LifecycleAppState>,
    headers: HeaderMap,
    Json(req): Json<PlanBusinessArchiveJobRequest>,
) -> Result<Json<BusinessArchiveJobDto>, H2LifecycleHandlerError> {
    require_any_permission(&ctx, WRITE_PERMISSIONS)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let now = Utc::now();
    let job = plan_business_archive_job(
        &state.pool,
        ctx.owner_id,
        &req.policy_code,
        &req.table_name,
        req.reference_date.unwrap_or_else(|| now.date_naive()),
        now,
        &idempotency_key,
    )
    .await?;
    Ok(Json(business_archive_job_dto(job)))
}

fn page_meta(len: usize) -> PageMeta {
    PageMeta {
        next_cursor: None,
        count: len as u32,
        total: None,
    }
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, H2LifecycleHandlerError> {
    let Some(value) = headers.get(IDEMPOTENCY_KEY_HEADER) else {
        return Err(H2LifecycleHandlerError::MissingIdempotencyKey);
    };
    let key = value
        .to_str()
        .map_err(|_| H2LifecycleHandlerError::MissingIdempotencyKey)?
        .trim();
    if key.is_empty() {
        return Err(H2LifecycleHandlerError::MissingIdempotencyKey);
    }
    Ok(key.to_string())
}

fn require_any_permission(ctx: &AuthContext, permissions: &[&str]) -> Result<(), AuthError> {
    if permissions
        .iter()
        .any(|permission| ctx.has_permission(permission))
    {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied(permissions.join("|")))
    }
}

fn audit_partition_dto(state: AuditPartitionState) -> AuditArchivePartitionStateDto {
    AuditArchivePartitionStateDto {
        partition_name: state.partition_name,
        partition_start: state.partition_start,
        partition_end: state.partition_end,
        storage_tier: state.storage_tier.as_str().to_string(),
        target_tier: state.target_tier.as_str().to_string(),
    }
}

fn audit_archive_run_dto(run: AuditArchiveRun) -> AuditArchiveRunResponse {
    AuditArchiveRunResponse {
        id: run.id,
        owner_id: run.owner_id,
        reference_date: run.reference_date,
        partitions_seen: run.partitions_seen,
        partitions_archived: run.partitions_archived,
        created_at: run.created_at,
    }
}

fn event_delivery_dto(delivery: EventDelivery) -> EventDeliveryDto {
    EventDeliveryDto {
        id: delivery.id,
        event_id: delivery.event_id,
        status: delivery.status.as_str().to_string(),
        attempt_count: delivery.attempt_count,
    }
}

fn business_retention_policy_dto(policy: BusinessRetentionPolicy) -> BusinessRetentionPolicyDto {
    BusinessRetentionPolicyDto {
        id: policy.id,
        owner_id: policy.owner_id,
        policy_code: policy.policy_code,
        retention_years: policy.retention_years,
        online_retention_months: policy.online_retention_months,
        permanent: policy.permanent,
        special_drug: policy.special_drug,
    }
}

fn business_archive_job_dto(job: BusinessArchiveJob) -> BusinessArchiveJobDto {
    BusinessArchiveJobDto {
        id: job.id,
        owner_id: job.owner_id,
        policy_code: job.policy_code,
        table_name: job.table_name,
        target_layer: job.target_layer,
        status: job.status,
        cutoff_date: job.cutoff_date,
        delete_allowed: job.delete_allowed,
    }
}
