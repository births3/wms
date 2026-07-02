//! Runtime Axum handlers for M-CG document numbering read APIs.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use wms_domain::{DocumentNumberAllocationListResponse, ErrorResponse, PageMeta};

use crate::{
    auth::{AuthContext, AuthError},
    document_numbering_repository::{
        DocumentNumberAllocationQuery, DocumentNumberRule, DocumentNumberRuleListResponse,
        DocumentNumberingError, PgDocumentNumberingService, SetDocumentNumberRuleEnabledRequest,
        UpsertDocumentNumberRuleRequest, DEFAULT_DOCUMENT_NUMBER_ALLOCATION_LIMIT,
    },
};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const READ_PERMISSION: &str = "mcg.document_numbering.read";
const WRITE_PERMISSION: &str = "mcg.document_numbering.write";

#[derive(Clone, Debug)]
pub struct DocumentNumberingAppState {
    pool: PgPool,
    service: PgDocumentNumberingService,
}

#[derive(Debug, Deserialize)]
struct ListDocumentNumberRulesQuery {
    document_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListDocumentNumberAllocationsQuery {
    document_type: Option<String>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: Option<u32>,
}

#[derive(Debug)]
enum DocumentNumberingHandlerError {
    Auth(AuthError),
    DocumentNumbering(DocumentNumberingError),
    MissingIdempotencyKey,
}

impl DocumentNumberingAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            pool,
            service: PgDocumentNumberingService::new(),
        }
    }
}

impl From<AuthError> for DocumentNumberingHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<DocumentNumberingError> for DocumentNumberingHandlerError {
    fn from(value: DocumentNumberingError) -> Self {
        Self::DocumentNumbering(value)
    }
}

impl IntoResponse for DocumentNumberingHandlerError {
    fn into_response(self) -> Response {
        if let DocumentNumberingHandlerError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            DocumentNumberingHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "MCG_DOCUMENT_NUMBERING_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            DocumentNumberingHandlerError::DocumentNumbering(
                DocumentNumberingError::RuleNotFound,
            ) => (
                StatusCode::NOT_FOUND,
                "MCG_DOCUMENT_NUMBERING_RULE_NOT_FOUND",
                "单据号规则不存在",
            ),
            DocumentNumberingHandlerError::DocumentNumbering(
                DocumentNumberingError::IdempotencyConflict,
            ) => (
                StatusCode::CONFLICT,
                "MCG_DOCUMENT_NUMBERING_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            DocumentNumberingHandlerError::DocumentNumbering(
                DocumentNumberingError::DocumentTypeInvalid
                | DocumentNumberingError::InvalidRule
                | DocumentNumberingError::InvalidEffectiveWindow
                | DocumentNumberingError::TemplateInvalid
                | DocumentNumberingError::SequenceOverflow,
            ) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "MCG_DOCUMENT_NUMBERING_INVALID",
                "单据号规则或查询条件非法",
            ),
            DocumentNumberingHandlerError::DocumentNumbering(
                DocumentNumberingError::Audit(_)
                | DocumentNumberingError::Database(_)
                | DocumentNumberingError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "MCG_DOCUMENT_NUMBER_ALLOCATION_QUERY_FAILED",
                "单据号处理失败",
            ),
            DocumentNumberingHandlerError::Auth(_) => unreachable!("auth error returned above"),
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

pub fn document_numbering_router(state: DocumentNumberingAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/code-generator/document-number-rules",
            get(list_document_number_rules_handler),
        )
        .route(
            "/api/v1/code-generator/document-number-rules/:rule_code",
            put(upsert_document_number_rule_handler),
        )
        .route(
            "/api/v1/code-generator/document-number-rules/:rule_code/enabled",
            patch(set_document_number_rule_enabled_handler),
        )
        .route(
            "/api/v1/code-generator/document-number-allocations",
            get(list_document_number_allocations_handler),
        )
        .with_state(state)
}

async fn list_document_number_rules_handler(
    ctx: AuthContext,
    State(state): State<DocumentNumberingAppState>,
    Query(query): Query<ListDocumentNumberRulesQuery>,
) -> Result<Json<DocumentNumberRuleListResponse>, DocumentNumberingHandlerError> {
    require_any_permission(&ctx, &[READ_PERMISSION, WRITE_PERMISSION])?;
    let data = state
        .service
        .list_rules(&state.pool, &ctx, query.document_type.as_deref())
        .await?;

    Ok(Json(DocumentNumberRuleListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
        },
        data,
    }))
}

async fn upsert_document_number_rule_handler(
    ctx: AuthContext,
    State(state): State<DocumentNumberingAppState>,
    Path(rule_code): Path<String>,
    headers: HeaderMap,
    Json(req): Json<UpsertDocumentNumberRuleRequest>,
) -> Result<Json<DocumentNumberRule>, DocumentNumberingHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let outcome = state
        .service
        .upsert_rule(
            &state.pool,
            &ctx,
            &rule_code,
            req,
            Utc::now(),
            &idempotency_key,
        )
        .await?;
    Ok(Json(outcome.value))
}

async fn set_document_number_rule_enabled_handler(
    ctx: AuthContext,
    State(state): State<DocumentNumberingAppState>,
    Path(rule_code): Path<String>,
    headers: HeaderMap,
    Json(req): Json<SetDocumentNumberRuleEnabledRequest>,
) -> Result<Json<DocumentNumberRule>, DocumentNumberingHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let outcome = state
        .service
        .set_rule_enabled(
            &state.pool,
            &ctx,
            &rule_code,
            req,
            Utc::now(),
            &idempotency_key,
        )
        .await?;
    Ok(Json(outcome.value))
}

async fn list_document_number_allocations_handler(
    ctx: AuthContext,
    State(state): State<DocumentNumberingAppState>,
    Query(query): Query<ListDocumentNumberAllocationsQuery>,
) -> Result<Json<DocumentNumberAllocationListResponse>, DocumentNumberingHandlerError> {
    require_any_permission(&ctx, &[READ_PERMISSION, WRITE_PERMISSION])?;
    let data = state
        .service
        .list_allocations(
            &state.pool,
            &ctx,
            DocumentNumberAllocationQuery {
                document_type: query.document_type,
                from: query.from,
                to: query.to,
                limit: query
                    .limit
                    .unwrap_or(DEFAULT_DOCUMENT_NUMBER_ALLOCATION_LIMIT),
            },
        )
        .await?;

    Ok(Json(DocumentNumberAllocationListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
        },
        data,
    }))
}

fn idempotency_key_from_headers(
    headers: &HeaderMap,
) -> Result<String, DocumentNumberingHandlerError> {
    let Some(value) = headers.get(IDEMPOTENCY_KEY_HEADER) else {
        return Err(DocumentNumberingHandlerError::MissingIdempotencyKey);
    };
    let key = value
        .to_str()
        .map_err(|_| DocumentNumberingHandlerError::MissingIdempotencyKey)?
        .trim();
    if key.is_empty() {
        return Err(DocumentNumberingHandlerError::MissingIdempotencyKey);
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
