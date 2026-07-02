//! Runtime Axum handlers for M-CG document numbering read APIs.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use wms_domain::{DocumentNumberAllocationListResponse, ErrorResponse, PageMeta};

use crate::{
    auth::AuthContext,
    document_numbering_repository::{
        DocumentNumberAllocationQuery, DocumentNumberingError, PgDocumentNumberingService,
        DEFAULT_DOCUMENT_NUMBER_ALLOCATION_LIMIT,
    },
};

#[derive(Clone, Debug)]
pub struct DocumentNumberingAppState {
    pool: PgPool,
    service: PgDocumentNumberingService,
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
    Query,
}

impl DocumentNumberingAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            pool,
            service: PgDocumentNumberingService::new(),
        }
    }
}

impl From<DocumentNumberingError> for DocumentNumberingHandlerError {
    fn from(_value: DocumentNumberingError) -> Self {
        Self::Query
    }
}

impl IntoResponse for DocumentNumberingHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            DocumentNumberingHandlerError::Query => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "MCG_DOCUMENT_NUMBER_ALLOCATION_QUERY_FAILED",
                "单据号生成记录查询失败",
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

pub fn document_numbering_router(state: DocumentNumberingAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/code-generator/document-number-allocations",
            get(list_document_number_allocations_handler),
        )
        .with_state(state)
}

async fn list_document_number_allocations_handler(
    ctx: AuthContext,
    State(state): State<DocumentNumberingAppState>,
    Query(query): Query<ListDocumentNumberAllocationsQuery>,
) -> Result<Json<DocumentNumberAllocationListResponse>, DocumentNumberingHandlerError> {
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
