//! Audit query routes extracted from wms_api bin for page-size control.
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::audit::{
    export_events, list_events, AuditError, AuditEventPage, AuditEventQuery, AuditEventQueryCursor,
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
        .route(
            "/api/v1/audit/events/export",
            get(export_audit_events_handler),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct AuditEventQueryParams {
    resource_type: Option<String>,
    action: Option<String>,
    resource_id: Option<String>,
    product_code: Option<String>,
    batch_no: Option<String>,
    actor_id: Option<Uuid>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: Option<u32>,
    cursor: Option<String>,
}

#[derive(Debug)]
enum AuditQueryError {
    ExportTooLarge,
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
            AuditQueryError::ExportTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "H2_AUDIT_EXPORT_TOO_LARGE",
                "审计导出结果超过 100000 条上限",
            ),
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
    let cursor = params
        .cursor
        .as_deref()
        .map(parse_audit_cursor)
        .transpose()?;
    let query = build_audit_event_query(
        &ctx,
        &params,
        cursor,
        params
            .limit
            .unwrap_or(DEFAULT_AUDIT_EVENT_QUERY_LIMIT)
            .clamp(1, MAX_AUDIT_EVENT_QUERY_LIMIT),
    );
    let page = list_events(&state.pool, &query).await?;
    Ok(Json(audit_event_response(page)?))
}

async fn export_audit_events_handler(
    ctx: AuthContext,
    State(state): State<AuditQueryState>,
    Query(params): Query<AuditEventQueryParams>,
) -> Result<Response, AuditQueryError> {
    ctx.require_permission("audit.read")
        .map_err(|_| AuditQueryError::PermissionDenied)?;
    let query = build_audit_event_query(&ctx, &params, None, MAX_AUDIT_EVENT_QUERY_LIMIT);
    let events = export_events(&state.pool, &query)
        .await
        .map_err(|error| match error {
            AuditError::ExportTooLarge => AuditQueryError::ExportTooLarge,
            other => AuditQueryError::from(other),
        })?;
    let mut csv = String::from(
        "id,occurred_at,actor_id,actor_name,owner_id,action,module,resource_type,resource_id,diff,request_id,ip,user_agent,prev_hash,self_hash\n",
    );
    for event in events {
        write_audit_event_csv_row(&mut csv, &event);
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"audit-events.csv\""),
    );
    Ok((headers, format!("\u{feff}{csv}")).into_response())
}

fn build_audit_event_query(
    ctx: &AuthContext,
    params: &AuditEventQueryParams,
    cursor: Option<AuditEventQueryCursor>,
    limit: u32,
) -> AuditEventQuery {
    AuditEventQuery {
        owner_id: ctx.owner_id,
        resource_type: params.resource_type.clone(),
        action: params.action.clone(),
        resource_id: params.resource_id.clone(),
        product_code: params.product_code.clone(),
        batch_no: params.batch_no.clone(),
        actor_id: params.actor_id,
        from: params.from,
        to: params.to,
        cursor,
        limit,
    }
}

fn write_audit_event_csv_row(output: &mut String, event: &AuditEventRecord) {
    use std::fmt::Write as _;

    let diff = event
        .diff
        .as_ref()
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_default();
    let row = [
        event.id.to_string(),
        event.occurred_at.to_rfc3339(),
        event.actor_id.to_string(),
        event.actor_name.clone(),
        event.owner_id.to_string(),
        event.action.clone(),
        event.module.clone(),
        event.resource_type.clone(),
        event.resource_id.clone(),
        diff,
        event
            .request_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        event.ip.clone().unwrap_or_default(),
        event.user_agent.clone().unwrap_or_default(),
        event.prev_hash.clone().unwrap_or_default(),
        event.self_hash.clone(),
    ];
    let line = row
        .iter()
        .map(|value| csv_escape(value))
        .collect::<Vec<_>>()
        .join(",");
    let _ = writeln!(output, "{line}");
}

fn csv_escape(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
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
        ip: record.ip,
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

#[cfg(test)]
mod tests {
    use super::csv_escape;

    #[test]
    fn csv_escape_quotes_delimiters_and_line_breaks() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b\"c\nd"), "\"a,b\"\"c\nd\"");
    }
}
