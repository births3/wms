//! H9 print template HTTP handlers.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use sqlx::PgPool;
use wms_domain::{ErrorResponse, PageMeta};

use crate::{
    auth::{AuthContext, AuthError},
    print_template::{
        PgPrintTemplateRepository, PrintFieldLibraryListResponse, PrintTemplateError,
    },
};

const READ_PERMISSION: &str = "h9.print_template.read";
const WRITE_PERMISSION: &str = "h9.print_template.write";
const PUBLISH_PERMISSION: &str = "h9.print_template.publish";

#[derive(Clone, Debug)]
pub struct PrintTemplateAppState {
    pool: PgPool,
    repository: PgPrintTemplateRepository,
}

#[derive(Debug)]
enum PrintTemplateHandlerError {
    Auth(AuthError),
    PrintTemplate(PrintTemplateError),
}

impl PrintTemplateAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            pool,
            repository: PgPrintTemplateRepository::new(),
        }
    }
}

impl From<AuthError> for PrintTemplateHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<PrintTemplateError> for PrintTemplateHandlerError {
    fn from(value: PrintTemplateError) -> Self {
        Self::PrintTemplate(value)
    }
}

impl IntoResponse for PrintTemplateHandlerError {
    fn into_response(self) -> Response {
        if let PrintTemplateHandlerError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            PrintTemplateHandlerError::PrintTemplate(PrintTemplateError::InvalidRequest(_)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_PRINT_TEMPLATE_INVALID",
                "打印模板请求非法",
            ),
            PrintTemplateHandlerError::PrintTemplate(PrintTemplateError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "H9_PRINT_TEMPLATE_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            PrintTemplateHandlerError::PrintTemplate(
                PrintTemplateError::Audit(_)
                | PrintTemplateError::Database(_)
                | PrintTemplateError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H9_PRINT_TEMPLATE_FAILED",
                "打印模板处理失败",
            ),
            PrintTemplateHandlerError::Auth(_) => unreachable!("auth error returned above"),
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

pub fn print_template_router(state: PrintTemplateAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/print-templates/field-libraries",
            get(list_print_field_libraries_handler),
        )
        .with_state(state)
}

async fn list_print_field_libraries_handler(
    ctx: AuthContext,
    State(state): State<PrintTemplateAppState>,
) -> Result<Json<PrintFieldLibraryListResponse>, PrintTemplateHandlerError> {
    require_any_permission(
        &ctx,
        &[READ_PERMISSION, WRITE_PERMISSION, PUBLISH_PERMISSION],
    )?;
    let data = state.repository.list_field_libraries(&state.pool).await?;
    Ok(Json(PrintFieldLibraryListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
        },
        data,
    }))
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
