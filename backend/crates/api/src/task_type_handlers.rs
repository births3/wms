use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, put},
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use wms_domain::{
    ErrorResponse, PageMeta, SetTaskTypeEnabledRequest, TaskType, TaskTypeListResponse,
    UpsertTaskTypeRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    task_type::{PgTaskTypeRepository, TaskTypeError},
};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const READ_PERMISSION: &str = "mte.task_type.read";
const WRITE_PERMISSION: &str = "mte.task_type.write";

#[derive(Clone, Debug)]
pub struct TaskTypeAppState {
    repository: PgTaskTypeRepository,
}

#[derive(Debug)]
pub enum TaskTypeHandlerError {
    Auth(AuthError),
    TaskType(TaskTypeError),
    MissingIdempotencyKey,
}

impl TaskTypeAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: PgTaskTypeRepository::new(pool),
        }
    }
}

impl From<AuthError> for TaskTypeHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<TaskTypeError> for TaskTypeHandlerError {
    fn from(value: TaskTypeError) -> Self {
        Self::TaskType(value)
    }
}

impl IntoResponse for TaskTypeHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "M_TE_TASK_TYPE_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::TaskType(TaskTypeError::Validation(_)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_TE_TASK_TYPE_INVALID",
                "任务类型配置非法",
            ),
            Self::TaskType(TaskTypeError::NotFound) => (
                StatusCode::NOT_FOUND,
                "M_TE_TASK_TYPE_NOT_FOUND",
                "任务类型不存在",
            ),
            Self::TaskType(TaskTypeError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "M_TE_TASK_TYPE_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            Self::TaskType(
                TaskTypeError::Audit(_) | TaskTypeError::Database(_) | TaskTypeError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "M_TE_TASK_TYPE_INTERNAL",
                "任务类型处理失败",
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

pub fn task_type_router(state: TaskTypeAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/task-engine/task-types",
            get(list_task_types_handler),
        )
        .route(
            "/api/v1/task-engine/task-types/:task_type_code",
            put(upsert_task_type_handler),
        )
        .route(
            "/api/v1/task-engine/task-types/:task_type_code/enabled",
            patch(set_task_type_enabled_handler),
        )
        .with_state(state)
}

async fn list_task_types_handler(
    ctx: AuthContext,
    State(state): State<TaskTypeAppState>,
) -> Result<Json<TaskTypeListResponse>, TaskTypeHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    let data = state.repository.list(&ctx).await?;
    Ok(Json(TaskTypeListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
            total: None,
        },
        data,
    }))
}

async fn upsert_task_type_handler(
    ctx: AuthContext,
    State(state): State<TaskTypeAppState>,
    Path(task_type_code): Path<String>,
    headers: HeaderMap,
    Json(request): Json<UpsertTaskTypeRequest>,
) -> Result<Json<TaskType>, TaskTypeHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let outcome = state
        .repository
        .upsert(&ctx, &task_type_code, request, Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(outcome.value))
}

async fn set_task_type_enabled_handler(
    ctx: AuthContext,
    State(state): State<TaskTypeAppState>,
    Path(task_type_code): Path<String>,
    headers: HeaderMap,
    Json(request): Json<SetTaskTypeEnabledRequest>,
) -> Result<Json<TaskType>, TaskTypeHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let outcome = state
        .repository
        .set_enabled(&ctx, &task_type_code, request, Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(outcome.value))
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, TaskTypeHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(TaskTypeHandlerError::MissingIdempotencyKey)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;

    use super::{idempotency_key_from_headers, TaskTypeHandlerError};

    #[test]
    fn idempotency_header_is_required() {
        assert!(matches!(
            idempotency_key_from_headers(&HeaderMap::new()),
            Err(TaskTypeHandlerError::MissingIdempotencyKey)
        ));
    }
}
