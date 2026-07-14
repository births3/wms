use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    CreateWarehouseTaskRequest, ErrorResponse, PageMeta, TaskGroup, TaskGroupListResponse,
    TaskListQuery, TaskPriorityRule, TaskTransitionAction, TaskWorkerListResponse,
    TransitionWarehouseTaskRequest, UpsertTaskGroupRequest, UpsertTaskPriorityRuleRequest,
    WarehouseTask, WarehouseTaskListResponse,
};

use crate::{
    auth::{AuthContext, AuthError},
    task_engine::{PgTaskEngineRepository, TaskEngineError},
};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const READ_PERMISSION: &str = "mte.task.read";
const READ_ALL_PERMISSION: &str = "mte.task.read_all";
const WRITE_PERMISSION: &str = "mte.task.write";
const ASSIGN_PERMISSION: &str = "mte.task.assign";
const EXECUTE_PERMISSION: &str = "mte.task.execute";
const GROUP_WRITE_PERMISSION: &str = "mte.task_group.write";
const PRIORITY_RULE_WRITE_PERMISSION: &str = "mte.priority_rule.write";

#[derive(Clone, Debug)]
pub struct TaskEngineAppState {
    repository: PgTaskEngineRepository,
}

#[derive(Debug)]
pub enum TaskEngineHandlerError {
    Auth(AuthError),
    TaskEngine(TaskEngineError),
    MissingIdempotencyKey,
}

impl TaskEngineAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: PgTaskEngineRepository::new(pool),
        }
    }
}

impl From<AuthError> for TaskEngineHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<TaskEngineError> for TaskEngineHandlerError {
    fn from(value: TaskEngineError) -> Self {
        Self::TaskEngine(value)
    }
}

impl IntoResponse for TaskEngineHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "M_TE_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::TaskEngine(TaskEngineError::Validation(_)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_TASK_INVALID",
                "任务数据非法",
            ),
            Self::TaskEngine(TaskEngineError::PriorityRuleInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_RULE_INVALID",
                "优先级规则参数非法",
            ),
            Self::TaskEngine(TaskEngineError::ReleaseConditionNotMet) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_RELEASE_CONDITION_NOT_MET",
                "任务释放条件未满足",
            ),
            Self::TaskEngine(TaskEngineError::TaskTypeNotFound) => (
                StatusCode::NOT_FOUND,
                "M_TE_TASK_TYPE_NOT_FOUND",
                "任务类型不存在或未启用",
            ),
            Self::TaskEngine(TaskEngineError::TaskGroupNotFound) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_TASK_GROUP_NOT_FOUND",
                "任务组不存在、不适用或未启用",
            ),
            Self::TaskEngine(TaskEngineError::WarehouseNotFound) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_WAREHOUSE_NOT_FOUND",
                "仓库不存在或未启用",
            ),
            Self::TaskEngine(TaskEngineError::ZoneNotFound) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_ZONE_NOT_FOUND",
                "任务组库区不存在或不属于指定仓库",
            ),
            Self::TaskEngine(TaskEngineError::UserNotFound) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_USER_NOT_FOUND",
                "任务组成员不存在或未启用",
            ),
            Self::TaskEngine(TaskEngineError::WorkerNotQualified) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_WORKER_NOT_QUALIFIED",
                "人员不具备该任务组资格",
            ),
            Self::TaskEngine(TaskEngineError::WorkerQualificationExpired) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_CERT_EXPIRED",
                "人员任务资格已过期",
            ),
            Self::TaskEngine(TaskEngineError::WorkerAtCapacity) => (
                StatusCode::CONFLICT,
                "M_TE_WORKER_AT_CAPACITY",
                "人员同时在手任务已达上限",
            ),
            Self::TaskEngine(TaskEngineError::NoAvailableWorker) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_NO_AVAILABLE_WORKER",
                "任务组内没有可用人员",
            ),
            Self::TaskEngine(TaskEngineError::TaskNotFound) => {
                (StatusCode::NOT_FOUND, "M_TE_TASK_NOT_FOUND", "任务不存在")
            }
            Self::TaskEngine(TaskEngineError::NotAssignee) => (
                StatusCode::FORBIDDEN,
                "M_TE_NOT_ASSIGNEE",
                "仅任务当前执行人可执行此操作",
            ),
            Self::TaskEngine(TaskEngineError::InvalidTransition) => (
                StatusCode::CONFLICT,
                "M_TE_INVALID_TRANSITION",
                "当前任务状态不允许此操作",
            ),
            Self::TaskEngine(TaskEngineError::QuantityDifferenceRequiresException) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_QUANTITY_DIFFERENCE_REQUIRES_EXCEPTION",
                "实际数量与计划数量不一致时必须上报异常",
            ),
            Self::TaskEngine(TaskEngineError::SourceTaskConflict) => (
                StatusCode::CONFLICT,
                "M_TE_SOURCE_TASK_CONFLICT",
                "同一业务触发源已存在参数不同的任务",
            ),
            Self::TaskEngine(TaskEngineError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "M_TE_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            Self::TaskEngine(
                TaskEngineError::Audit(_)
                | TaskEngineError::Database(_)
                | TaskEngineError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M_TE_EXECUTION_INTERNAL",
                "任务引擎处理失败",
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

pub fn task_engine_router(state: TaskEngineAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/task-engine/task-groups",
            get(list_task_groups_handler),
        )
        .route(
            "/api/v1/task-engine/workers",
            get(list_task_workers_handler),
        )
        .route(
            "/api/v1/task-engine/task-groups/:task_group_code",
            put(upsert_task_group_handler),
        )
        .route(
            "/api/v1/task-engine/tasks",
            get(list_tasks_handler).post(create_task_handler),
        )
        .route(
            "/api/v1/task-engine/tasks/:task_id/transitions",
            post(transition_task_handler),
        )
        .route(
            "/api/v1/task-engine/priority-rule",
            get(get_priority_rule_handler).put(upsert_priority_rule_handler),
        )
        .with_state(state)
}

async fn get_priority_rule_handler(
    ctx: AuthContext,
    State(state): State<TaskEngineAppState>,
) -> Result<Json<TaskPriorityRule>, TaskEngineHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(state.repository.get_priority_rule(&ctx).await?))
}

async fn upsert_priority_rule_handler(
    ctx: AuthContext,
    State(state): State<TaskEngineAppState>,
    headers: HeaderMap,
    Json(request): Json<UpsertTaskPriorityRuleRequest>,
) -> Result<Json<TaskPriorityRule>, TaskEngineHandlerError> {
    ctx.require_permission(PRIORITY_RULE_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let outcome = state
        .repository
        .upsert_priority_rule(&ctx, request, Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(outcome.value))
}

async fn list_task_workers_handler(
    ctx: AuthContext,
    State(state): State<TaskEngineAppState>,
) -> Result<Json<TaskWorkerListResponse>, TaskEngineHandlerError> {
    ctx.require_permission(GROUP_WRITE_PERMISSION)?;
    let data = state.repository.list_worker_candidates(&ctx).await?;
    Ok(Json(TaskWorkerListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
        },
        data,
    }))
}

async fn list_task_groups_handler(
    ctx: AuthContext,
    State(state): State<TaskEngineAppState>,
) -> Result<Json<TaskGroupListResponse>, TaskEngineHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    let data = state.repository.list_task_groups(&ctx).await?;
    Ok(Json(TaskGroupListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
        },
        data,
    }))
}

async fn upsert_task_group_handler(
    ctx: AuthContext,
    State(state): State<TaskEngineAppState>,
    Path(task_group_code): Path<String>,
    headers: HeaderMap,
    Json(request): Json<UpsertTaskGroupRequest>,
) -> Result<Json<TaskGroup>, TaskEngineHandlerError> {
    ctx.require_permission(GROUP_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let outcome = state
        .repository
        .upsert_task_group(
            &ctx,
            &task_group_code,
            request,
            Utc::now(),
            &idempotency_key,
        )
        .await?;
    Ok(Json(outcome.value))
}

async fn list_tasks_handler(
    ctx: AuthContext,
    State(state): State<TaskEngineAppState>,
    Query(query): Query<TaskListQuery>,
) -> Result<Json<WarehouseTaskListResponse>, TaskEngineHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    if !query.mine_only {
        ctx.require_permission(READ_ALL_PERMISSION)?;
    }
    let data = state.repository.list_tasks(&ctx, query).await?;
    Ok(Json(WarehouseTaskListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
        },
        data,
    }))
}

async fn create_task_handler(
    ctx: AuthContext,
    State(state): State<TaskEngineAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateWarehouseTaskRequest>,
) -> Result<(StatusCode, Json<WarehouseTask>), TaskEngineHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let outcome = state
        .repository
        .create_task(&ctx, request, Utc::now(), &idempotency_key)
        .await?;
    Ok((StatusCode::CREATED, Json(outcome.value)))
}

async fn transition_task_handler(
    ctx: AuthContext,
    State(state): State<TaskEngineAppState>,
    Path(task_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<TransitionWarehouseTaskRequest>,
) -> Result<Json<WarehouseTask>, TaskEngineHandlerError> {
    let permission = match request.action {
        TaskTransitionAction::Start
        | TaskTransitionAction::Complete
        | TaskTransitionAction::ReportException => EXECUTE_PERMISSION,
        TaskTransitionAction::Assign
        | TaskTransitionAction::Release
        | TaskTransitionAction::Dispatch
        | TaskTransitionAction::Reassign
        | TaskTransitionAction::Recall
        | TaskTransitionAction::ResolveComplete
        | TaskTransitionAction::Cancel
        | TaskTransitionAction::Expedite => ASSIGN_PERMISSION,
    };
    ctx.require_permission(permission)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let outcome = state
        .repository
        .transition_task(&ctx, task_id, request, Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(outcome.value))
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, TaskEngineHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(TaskEngineHandlerError::MissingIdempotencyKey)
}
