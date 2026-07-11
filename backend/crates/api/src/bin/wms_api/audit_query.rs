//! Audit query routes extracted from wms_api bin for page-size control.
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::audit::{
    list_events, AuditError, AuditEventPage, AuditEventQuery, AuditEventQueryCursor,
    AuditEventRecord, DEFAULT_AUDIT_EVENT_QUERY_LIMIT, MAX_AUDIT_EVENT_QUERY_LIMIT,
};
use wms_api::auth::AuthContext;
use wms_domain::{AuditActor, AuditEvent, AuditEventListResponse, ErrorResponse};

#[derive(Clone)]
pub(crate) struct AuditQueryState {
    pub(crate) pool: PgPool,
}

pub(crate) fn audit_query_router(state: AuditQueryState) -> Router {
    Router::new()
        .route("/api/v1/audit/events", get(list_audit_events_handler))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct AuditEventQueryParams {
    resource_type: Option<String>,
    actor_id: Option<Uuid>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: Option<u32>,
    cursor: Option<String>,
}

#[derive(Debug)]
enum AuditQueryError {
    InvalidCursor,
    PermissionDenied,
    Query,
}

impl From<AuditError> for AuditQueryError {
    fn from(_value: AuditError) -> Self {
        Self::Query
    }
}

impl IntoResponse for AuditQueryError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AuditQueryError::InvalidCursor => (
                StatusCode::BAD_REQUEST,
                "H2_AUDIT_QUERY_CURSOR_INVALID",
                "审计查询游标格式无效",
            ),
            AuditQueryError::PermissionDenied => {
                (StatusCode::FORBIDDEN, "AUTH_003", "缺少审计查询权限")
            }
            AuditQueryError::Query => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H2_AUDIT_QUERY_FAILED",
                "审计查询失败",
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

async fn list_audit_events_handler(
    ctx: AuthContext,
    State(state): State<AuditQueryState>,
    Query(params): Query<AuditEventQueryParams>,
) -> Result<Json<AuditEventListResponse>, AuditQueryError> {
    ctx.require_permission("audit.read")
        .map_err(|_| AuditQueryError::PermissionDenied)?;
    let query = AuditEventQuery {
        owner_id: ctx.owner_id,
        resource_type: params.resource_type,
        actor_id: params.actor_id,
        from: params.from,
        to: params.to,
        cursor: params
            .cursor
            .as_deref()
            .map(parse_audit_cursor)
            .transpose()?,
        limit: params
            .limit
            .unwrap_or(DEFAULT_AUDIT_EVENT_QUERY_LIMIT)
            .clamp(1, MAX_AUDIT_EVENT_QUERY_LIMIT),
    };
    let page = list_events(&state.pool, &query).await?;
    Ok(Json(audit_event_response(page)?))
}

fn audit_event_response(page: AuditEventPage) -> Result<AuditEventListResponse, AuditQueryError> {
    Ok(AuditEventListResponse {
        data: page.events.into_iter().map(audit_event_dto).collect(),
        next_cursor: page.next_cursor.map(format_audit_cursor),
    })
}

fn audit_event_dto(record: AuditEventRecord) -> AuditEvent {
    let trace_id = record
        .request_id
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    AuditEvent {
        id: record.id,
        owner_id: record.owner_id,
        resource_type: record.resource_type,
        resource_id: record.resource_id,
        action: record.action,
        trace_id,
        occurred_at: record.occurred_at,
        actor: AuditActor {
            actor_id: record.actor_id,
            actor_name: record.actor_name,
            owner_id: record.owner_id,
            jti: record.jti,
        },
        diff: record
            .diff
            .map(|value| serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!({})))
            .unwrap_or_else(|| serde_json::json!({})),
    }
}

fn parse_audit_cursor(value: &str) -> Result<AuditEventQueryCursor, AuditQueryError> {
    let (micros, id) = value
        .split_once(':')
        .ok_or(AuditQueryError::InvalidCursor)?;
    let timestamp_micros = micros
        .parse::<i64>()
        .map_err(|_| AuditQueryError::InvalidCursor)?;
    let id = id
        .parse::<i64>()
        .map_err(|_| AuditQueryError::InvalidCursor)?;
    let occurred_at = Utc
        .timestamp_micros(timestamp_micros)
        .single()
        .ok_or(AuditQueryError::InvalidCursor)?;
    Ok(AuditEventQueryCursor { occurred_at, id })
}

fn format_audit_cursor(cursor: AuditEventQueryCursor) -> String {
    format!("{}:{}", cursor.occurred_at.timestamp_micros(), cursor.id)
}
