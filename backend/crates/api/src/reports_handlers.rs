//! M6 报表运行时 handler：按 owner_id 隔离的 PostgreSQL 查询。

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use wms_domain::{
    ErrorResponse, GspLedgerReport, GspLedgerRow, PageMeta, ReportQueryRequest,
    ReportQueryResponse, ReportRow,
};

use crate::auth::{AuthContext, AuthError};

const READ_PERMISSION: &str = "m6.report.read";

#[derive(Clone, Debug)]
pub struct ReportsAppState {
    pool: Arc<PgPool>,
}

impl ReportsAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }
}

#[derive(Debug)]
pub enum ReportsHandlerError {
    Auth(AuthError),
    Database(String),
    UnsupportedReportCode(String),
}

impl From<AuthError> for ReportsHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl IntoResponse for ReportsHandlerError {
    fn into_response(self) -> Response {
        if let ReportsHandlerError::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            ReportsHandlerError::UnsupportedReportCode(code) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "REPORT_UNSUPPORTED_CODE",
                format!("unsupported report_code: {code}"),
            ),
            ReportsHandlerError::Database(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "REPORT_DATABASE_ERROR",
                message,
            ),
            ReportsHandlerError::Auth(_) => unreachable!(),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message,
                severity: "error".to_string(),
                details: json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

pub fn reports_router(state: ReportsAppState) -> Router {
    Router::new()
        .route("/api/v1/reports/query", post(query_report_handler))
        .route(
            "/api/v1/reports/gsp/inbound-ledger",
            post(gsp_inbound_ledger_handler),
        )
        .route(
            "/api/v1/reports/gsp/outbound-ledger",
            post(gsp_outbound_ledger_handler),
        )
        .route(
            "/api/v1/reports/gsp/inventory-ledger",
            post(gsp_inventory_ledger_handler),
        )
        .with_state(state)
}

/// 运行时挂载入口：从连接池构造报表 Router，避免 bin 侧多占有效行。
pub fn mount_reports(pool: PgPool) -> Router {
    reports_router(ReportsAppState::with_postgres(pool))
}

async fn query_report_handler(
    ctx: AuthContext,
    State(state): State<ReportsAppState>,
    Json(req): Json<ReportQueryRequest>,
) -> Result<Json<ReportQueryResponse>, ReportsHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    let limit = req.limit.unwrap_or(50).min(200) as i64;
    let (metric, count) = match req.report_code.as_str() {
        "m6_inbound_summary" => (
            "receiving_orders",
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)::BIGINT
                  FROM receiving_orders
                 WHERE owner_id = $1
                "#,
            )
            .bind(ctx.owner_id)
            .fetch_one(state.pool.as_ref())
            .await,
        ),
        "m6_outbound_summary" => (
            "outbound_orders",
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)::BIGINT
                  FROM outbound_orders
                 WHERE owner_id = $1
                "#,
            )
            .bind(ctx.owner_id)
            .fetch_one(state.pool.as_ref())
            .await,
        ),
        _ => return Err(ReportsHandlerError::UnsupportedReportCode(req.report_code)),
    };
    let count = count.map_err(|error| ReportsHandlerError::Database(error.to_string()))?;

    let rows = vec![ReportRow {
        values: json!({
            "metric": metric,
            "count": count,
            "owner_id": ctx.owner_id,
            "filters": req.filters,
        }),
    }];
    Ok(Json(ReportQueryResponse {
        report_code: req.report_code,
        generated_at: Utc::now(),
        page: PageMeta {
            next_cursor: None,
            count: rows.len().min(limit as usize) as u32,
        },
        rows,
    }))
}

async fn gsp_inbound_ledger_handler(
    ctx: AuthContext,
    State(state): State<ReportsAppState>,
    Json(req): Json<ReportQueryRequest>,
) -> Result<Json<GspLedgerReport>, ReportsHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    let limit = req.limit.unwrap_or(50).min(200) as i64;
    let rows = sqlx::query_as::<_, InboundLedgerSqlRow>(
        r#"
        SELECT r.occurred_at,
               l.product_code,
               l.batch_no,
               r.actual_qty AS quantity_delta,
               o.document_type,
               o.receipt_no AS document_no
          FROM receiving_order_receipts r
          JOIN receiving_orders o
            ON o.id = r.receiving_order_id AND o.owner_id = r.owner_id
          LEFT JOIN LATERAL (
                SELECT product_code, batch_no
                  FROM receiving_order_lines
                 WHERE receiving_order_id = o.id AND owner_id = o.owner_id
                 ORDER BY line_no
                 LIMIT 1
          ) l ON TRUE
         WHERE r.owner_id = $1
         ORDER BY r.occurred_at DESC
         LIMIT $2
        "#,
    )
    .bind(ctx.owner_id)
    .bind(limit)
    .fetch_all(state.pool.as_ref())
    .await
    .map_err(|error| ReportsHandlerError::Database(error.to_string()))?;

    Ok(Json(map_gsp_ledger("inbound", rows, &req)))
}

async fn gsp_outbound_ledger_handler(
    ctx: AuthContext,
    State(state): State<ReportsAppState>,
    Json(req): Json<ReportQueryRequest>,
) -> Result<Json<GspLedgerReport>, ReportsHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    let limit = req.limit.unwrap_or(50).min(200) as i64;
    let rows = sqlx::query_as::<_, OutboundLedgerSqlRow>(
        r#"
        SELECT s.shipped_at AS occurred_at,
               l.product_code,
               l.batch_no,
               -1 * COALESCE(l.planned_qty, 0) AS quantity_delta,
               o.status AS document_type,
               o.wms_order_no AS document_no
          FROM outbound_shipments s
          JOIN outbound_orders o
            ON o.id = s.outbound_order_id AND o.owner_id = s.owner_id
          LEFT JOIN LATERAL (
                SELECT product_code, batch_no, planned_qty
                  FROM outbound_order_lines
                 WHERE outbound_order_id = o.id AND owner_id = o.owner_id
                 ORDER BY line_no
                 LIMIT 1
          ) l ON TRUE
         WHERE s.owner_id = $1
         ORDER BY s.shipped_at DESC NULLS LAST
         LIMIT $2
        "#,
    )
    .bind(ctx.owner_id)
    .bind(limit)
    .fetch_all(state.pool.as_ref())
    .await
    .map_err(|error| ReportsHandlerError::Database(error.to_string()))?;

    Ok(Json(map_gsp_ledger(
        "outbound",
        rows.into_iter()
            .map(|row| InboundLedgerSqlRow {
                occurred_at: row.occurred_at,
                product_code: row.product_code,
                batch_no: row.batch_no,
                quantity_delta: row.quantity_delta,
                document_type: row.document_type,
                document_no: row.document_no,
            })
            .collect(),
        &req,
    )))
}

async fn gsp_inventory_ledger_handler(
    ctx: AuthContext,
    State(state): State<ReportsAppState>,
    Json(req): Json<ReportQueryRequest>,
) -> Result<Json<GspLedgerReport>, ReportsHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    let limit = req.limit.unwrap_or(50).min(200) as i64;
    let rows = sqlx::query_as::<_, InboundLedgerSqlRow>(
        r#"
        SELECT m.occurred_at,
               b.product_code,
               b.batch_no,
               m.qty_delta AS quantity_delta,
               m.movement_type AS document_type,
               m.id::text AS document_no
          FROM inventory_movements m
          JOIN inventory_batches b
            ON b.id = m.batch_id AND b.owner_id = m.owner_id
         WHERE m.owner_id = $1
         ORDER BY m.occurred_at DESC
         LIMIT $2
        "#,
    )
    .bind(ctx.owner_id)
    .bind(limit)
    .fetch_all(state.pool.as_ref())
    .await
    .map_err(|error| ReportsHandlerError::Database(error.to_string()))?;

    Ok(Json(map_gsp_ledger("inventory", rows, &req)))
}

#[derive(Debug, sqlx::FromRow)]
struct InboundLedgerSqlRow {
    occurred_at: Option<chrono::DateTime<Utc>>,
    product_code: Option<String>,
    batch_no: Option<String>,
    quantity_delta: Option<wms_domain::Quantity>,
    document_type: Option<String>,
    document_no: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct OutboundLedgerSqlRow {
    occurred_at: Option<chrono::DateTime<Utc>>,
    product_code: Option<String>,
    batch_no: Option<String>,
    quantity_delta: Option<wms_domain::Quantity>,
    document_type: Option<String>,
    document_no: Option<String>,
}

fn map_gsp_ledger(
    ledger_type: &str,
    rows: Vec<InboundLedgerSqlRow>,
    req: &ReportQueryRequest,
) -> GspLedgerReport {
    let mapped: Vec<GspLedgerRow> = rows
        .into_iter()
        .map(|row| GspLedgerRow {
            ledger_type: ledger_type.to_string(),
            occurred_at: row.occurred_at,
            product_code: row.product_code,
            batch_no: row.batch_no,
            quantity_delta: row.quantity_delta,
            document_type: row.document_type,
            document_no: row.document_no,
            approval_source: None,
            approval_id: None,
            operator_id: None,
            operator_name: None,
            values: json!({ "filters": req.filters }),
        })
        .collect();
    GspLedgerReport {
        ledger_type: ledger_type.to_string(),
        generated_at: Utc::now(),
        page: PageMeta {
            next_cursor: None,
            count: mapped.len() as u32,
        },
        rows: mapped,
    }
}
