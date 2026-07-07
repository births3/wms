//! H9 print template HTTP handlers.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use wms_domain::{ErrorResponse, PageMeta};

use crate::{
    auth::{AuthContext, AuthError},
    print_template::{
        PgPrintTemplateRepository, PrintFieldDefinitionListResponse, PrintFieldLibraryListResponse,
        PrintRecord, PrintTemplateError, PrintTemplateListResponse, PrintTemplatePreviewRequest,
        PrintTemplatePreviewResponse, PrintTemplatePrintRequest, PrintTemplateVersion,
        PrintTemplateVersionListResponse, ResolvePrintTemplateRequest,
        ResolvePrintTemplateResponse, SavePrintTemplateRequest,
    },
};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const READ_PERMISSION: &str = "h9.print_template.read";
const WRITE_PERMISSION: &str = "h9.print_template.write";
const PUBLISH_PERMISSION: &str = "h9.print_template.publish";
const PRINT_PERMISSION: &str = "h9.print_template.print";

#[derive(Clone, Debug)]
pub struct PrintTemplateAppState {
    pool: PgPool,
    repository: PgPrintTemplateRepository,
}

#[derive(Debug)]
enum PrintTemplateHandlerError {
    Auth(AuthError),
    PrintTemplate(PrintTemplateError),
    MissingIdempotencyKey,
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

        let (status, code, message, details) = match self {
            PrintTemplateHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "H9_PRINT_TEMPLATE_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
                serde_json::json!({}),
            ),
            PrintTemplateHandlerError::PrintTemplate(PrintTemplateError::TemplateNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_TEMPLATE_NOT_FOUND",
                "打印模板不存在",
                serde_json::json!({}),
            ),
            PrintTemplateHandlerError::PrintTemplate(PrintTemplateError::TemplateDisabled) => (
                StatusCode::CONFLICT,
                "H9_TEMPLATE_DISABLED",
                "打印模板已停用",
                serde_json::json!({}),
            ),
            PrintTemplateHandlerError::PrintTemplate(
                PrintTemplateError::FieldLibraryNotPublished,
            ) => (
                StatusCode::CONFLICT,
                "H9_FIELD_LIBRARY_NOT_PUBLISHED",
                "字段库版本未发布",
                serde_json::json!({}),
            ),
            PrintTemplateHandlerError::PrintTemplate(PrintTemplateError::TemplateJsonInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_TEMPLATE_JSON_INVALID",
                "hiprint 模板 JSON 非法",
                serde_json::json!({}),
            ),
            PrintTemplateHandlerError::PrintTemplate(
                PrintTemplateError::TemplateFieldMismatch(fields),
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_TEMPLATE_FIELD_MISMATCH",
                "模板字段绑定不存在",
                serde_json::json!({ "fields": fields }),
            ),
            PrintTemplateHandlerError::PrintTemplate(PrintTemplateError::TemplateFieldMissing(
                fields,
            )) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_TEMPLATE_FIELD_MISSING",
                "打印数据缺少必填字段",
                serde_json::json!({ "fields": fields }),
            ),
            PrintTemplateHandlerError::PrintTemplate(PrintTemplateError::InvalidRequest(_)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_PRINT_TEMPLATE_INVALID",
                "打印模板请求非法",
                serde_json::json!({}),
            ),
            PrintTemplateHandlerError::PrintTemplate(PrintTemplateError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "H9_PRINT_TEMPLATE_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
                serde_json::json!({}),
            ),
            PrintTemplateHandlerError::PrintTemplate(
                PrintTemplateError::Audit(_)
                | PrintTemplateError::Database(_)
                | PrintTemplateError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H9_PRINT_TEMPLATE_FAILED",
                "打印模板处理失败",
                serde_json::json!({}),
            ),
            PrintTemplateHandlerError::Auth(_) => unreachable!("auth error returned above"),
        };

        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details,
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
        .route(
            "/api/v1/print-templates/field-libraries/:version_id/fields",
            get(list_print_field_definitions_handler),
        )
        .route(
            "/api/v1/print-templates/templates",
            get(list_print_templates_handler).post(save_print_template_handler),
        )
        .route(
            "/api/v1/print-templates/templates/:template_id/versions",
            get(list_print_template_versions_handler),
        )
        .route(
            "/api/v1/print-templates/resolve",
            post(resolve_print_template_handler),
        )
        .route(
            "/api/v1/print-templates/preview",
            post(preview_print_template_handler),
        )
        .route(
            "/api/v1/print-templates/print",
            post(record_print_template_handler),
        )
        .with_state(state)
}

async fn list_print_field_definitions_handler(
    ctx: AuthContext,
    State(state): State<PrintTemplateAppState>,
    Path(version_id): Path<uuid::Uuid>,
) -> Result<Json<PrintFieldDefinitionListResponse>, PrintTemplateHandlerError> {
    require_any_permission(
        &ctx,
        &[READ_PERMISSION, WRITE_PERMISSION, PUBLISH_PERMISSION],
    )?;
    let data = state
        .repository
        .list_field_version_fields(&state.pool, version_id)
        .await?;
    Ok(Json(PrintFieldDefinitionListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
        },
        data,
    }))
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

async fn list_print_templates_handler(
    ctx: AuthContext,
    State(state): State<PrintTemplateAppState>,
) -> Result<Json<PrintTemplateListResponse>, PrintTemplateHandlerError> {
    require_any_permission(
        &ctx,
        &[
            READ_PERMISSION,
            WRITE_PERMISSION,
            PUBLISH_PERMISSION,
            PRINT_PERMISSION,
        ],
    )?;
    let data = state.repository.list_templates(&state.pool, &ctx).await?;
    Ok(Json(PrintTemplateListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
        },
        data,
    }))
}

async fn save_print_template_handler(
    ctx: AuthContext,
    State(state): State<PrintTemplateAppState>,
    headers: HeaderMap,
    Json(req): Json<SavePrintTemplateRequest>,
) -> Result<Json<PrintTemplateVersion>, PrintTemplateHandlerError> {
    if req.publish {
        require_any_permission(&ctx, &[PUBLISH_PERMISSION])?;
    } else {
        require_any_permission(&ctx, &[WRITE_PERMISSION, PUBLISH_PERMISSION])?;
    }
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .repository
        .save_template(&state.pool, &ctx, req, Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn list_print_template_versions_handler(
    ctx: AuthContext,
    State(state): State<PrintTemplateAppState>,
    Path(template_id): Path<uuid::Uuid>,
) -> Result<Json<PrintTemplateVersionListResponse>, PrintTemplateHandlerError> {
    require_any_permission(
        &ctx,
        &[READ_PERMISSION, WRITE_PERMISSION, PUBLISH_PERMISSION],
    )?;
    let data = state
        .repository
        .list_template_versions(&state.pool, &ctx, template_id)
        .await?;
    Ok(Json(PrintTemplateVersionListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
        },
        data,
    }))
}

async fn resolve_print_template_handler(
    ctx: AuthContext,
    State(state): State<PrintTemplateAppState>,
    Json(req): Json<ResolvePrintTemplateRequest>,
) -> Result<Json<ResolvePrintTemplateResponse>, PrintTemplateHandlerError> {
    require_any_permission(&ctx, &[READ_PERMISSION, PRINT_PERMISSION])?;
    Ok(Json(
        state
            .repository
            .resolve_template(&state.pool, &ctx, req)
            .await?,
    ))
}

async fn preview_print_template_handler(
    ctx: AuthContext,
    State(state): State<PrintTemplateAppState>,
    Json(req): Json<PrintTemplatePreviewRequest>,
) -> Result<Json<PrintTemplatePreviewResponse>, PrintTemplateHandlerError> {
    require_any_permission(&ctx, &[READ_PERMISSION, PRINT_PERMISSION])?;
    Ok(Json(
        state
            .repository
            .preview_template(&state.pool, &ctx, req)
            .await?,
    ))
}

async fn record_print_template_handler(
    ctx: AuthContext,
    State(state): State<PrintTemplateAppState>,
    headers: HeaderMap,
    Json(req): Json<PrintTemplatePrintRequest>,
) -> Result<Json<PrintRecord>, PrintTemplateHandlerError> {
    require_any_permission(&ctx, &[PRINT_PERMISSION])?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .repository
        .record_print(&state.pool, &ctx, req, Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(result.value))
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, PrintTemplateHandlerError> {
    let Some(value) = headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .or_else(|| headers.get("Idempotency-Key"))
    else {
        return Err(PrintTemplateHandlerError::MissingIdempotencyKey);
    };
    let value = value
        .to_str()
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(PrintTemplateHandlerError::MissingIdempotencyKey)?;
    Ok(value.to_string())
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
