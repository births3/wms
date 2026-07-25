use crate::{
    audit,
    auth::PortalAuth,
    models::{
        AddressSummary, OrderDetail, OrderLineDetail, OrderQuery, OrderSummary, ReportSummary,
    },
    PortalError, PortalState,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use sqlx::Row;
use uuid::Uuid;

pub async fn list_addresses(
    State(state): State<PortalState>,
    auth: PortalAuth,
) -> Result<Json<Vec<AddressSummary>>, PortalError> {
    let rows = sqlx::query(
        "SELECT a.id, a.address_code, a.address_name
         FROM portal_customer_addresses a
         WHERE a.customer_id = $1
           AND (
               $2 = 'customer_admin'
               OR EXISTS (
                   SELECT 1 FROM portal_user_addresses ua
                   WHERE ua.address_id = a.id AND ua.user_id = $3
               )
           )
         ORDER BY a.address_code",
    )
    .bind(auth.customer_id)
    .bind(&auth.role)
    .bind(auth.user_id)
    .fetch_all(&state.pool)
    .await?;
    let addresses = rows
        .into_iter()
        .map(|row| {
            Ok(AddressSummary {
                id: row.try_get("id")?,
                address_code: row.try_get("address_code")?,
                address_name: row.try_get("address_name")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok(Json(addresses))
}

pub async fn list_orders(
    State(state): State<PortalState>,
    auth: PortalAuth,
    Query(query): Query<OrderQuery>,
) -> Result<Json<Vec<OrderSummary>>, PortalError> {
    let keyword = query.keyword.map(|value| format!("%{}%", value.trim()));
    let rows = sqlx::query(
        "SELECT
             o.id, o.order_no, o.status, o.delivery_address_id, a.address_name,
             o.shipped_at, o.signed_at,
             COUNT(DISTINCT l.id) AS line_count,
             COUNT(DISTINCT r.id) FILTER (
                 WHERE r.customer_copy_status = 'available'
             ) AS available_report_count,
             COUNT(DISTINCT l.id) FILTER (
                 WHERE r.id IS NULL OR r.customer_copy_status <> 'available'
             ) AS pending_report_count
         FROM portal_orders o
         JOIN portal_customer_addresses a ON a.id = o.delivery_address_id
         JOIN portal_order_lines l ON l.order_id = o.id
         LEFT JOIN portal_report_versions r
           ON r.product_id = l.product_id
          AND r.batch_no = l.batch_no
          AND r.is_current
         WHERE o.customer_id = $1
           AND o.status IN ('shipped', 'signed')
           AND (
               $2 = 'customer_admin'
               OR EXISTS (
                   SELECT 1 FROM portal_user_addresses ua
                   WHERE ua.address_id = o.delivery_address_id AND ua.user_id = $3
               )
           )
           AND ($4::uuid IS NULL OR o.delivery_address_id = $4)
           AND ($5::text IS NULL OR o.status = $5)
           AND (
               $6::text IS NULL
               OR o.order_no ILIKE $6
               OR l.product_code ILIKE $6
               OR l.product_name ILIKE $6
               OR l.batch_no ILIKE $6
           )
         GROUP BY o.id, a.address_name
         ORDER BY o.shipped_at DESC, o.order_no",
    )
    .bind(auth.customer_id)
    .bind(&auth.role)
    .bind(auth.user_id)
    .bind(query.address_id)
    .bind(query.status)
    .bind(keyword)
    .fetch_all(&state.pool)
    .await?;
    let orders = rows
        .into_iter()
        .map(order_summary_from_row)
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    audit(
        &state.pool,
        Some(auth.user_id),
        Some(auth.customer_id),
        "query",
        "orders",
        "list",
        serde_json::json!({ "result_count": orders.len() }),
    )
    .await?;
    Ok(Json(orders))
}

pub async fn get_order(
    State(state): State<PortalState>,
    auth: PortalAuth,
    Path(order_id): Path<Uuid>,
) -> Result<Json<OrderDetail>, PortalError> {
    let row = sqlx::query(
        "SELECT o.id, o.order_no, o.status, o.delivery_address_id,
                o.address_snapshot, o.shipped_at, o.signed_at
         FROM portal_orders o
         WHERE o.id = $1
           AND o.customer_id = $2
           AND o.status IN ('shipped', 'signed')
           AND (
               $3 = 'customer_admin'
               OR EXISTS (
                   SELECT 1 FROM portal_user_addresses ua
                   WHERE ua.address_id = o.delivery_address_id AND ua.user_id = $4
               )
           )",
    )
    .bind(order_id)
    .bind(auth.customer_id)
    .bind(&auth.role)
    .bind(auth.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(PortalError::NotFound)?;
    let line_rows = sqlx::query(
        "SELECT id, product_id, product_code, product_name, batch_no,
                quantity::double precision AS quantity
         FROM portal_order_lines
         WHERE order_id = $1
         ORDER BY product_code, batch_no",
    )
    .bind(order_id)
    .fetch_all(&state.pool)
    .await?;
    let mut lines = Vec::with_capacity(line_rows.len());
    for line in line_rows {
        let product_id: Uuid = line.try_get("product_id")?;
        let batch_no: String = line.try_get("batch_no")?;
        let report_rows = sqlx::query(
            "SELECT id, report_id, version_number, report_no, status, is_current,
                    modification_reason, customer_copy_status,
                    customer_copy_file_name, customer_copy_size,
                    digitally_signed_original, confirmed_at
             FROM portal_report_versions
             WHERE product_id = $1
               AND batch_no = $2
               AND ($3 OR is_current)
             ORDER BY version_number DESC",
        )
        .bind(product_id)
        .bind(&batch_no)
        .bind(auth.can_view_report_history)
        .fetch_all(&state.pool)
        .await?;
        let reports = report_rows
            .into_iter()
            .map(report_summary_from_row)
            .collect::<Result<Vec<_>, sqlx::Error>>()?;
        lines.push(OrderLineDetail {
            id: line.try_get("id")?,
            product_id,
            product_code: line.try_get("product_code")?,
            product_name: line.try_get("product_name")?,
            batch_no,
            quantity: line.try_get("quantity")?,
            reports,
        });
    }
    audit(
        &state.pool,
        Some(auth.user_id),
        Some(auth.customer_id),
        "view",
        "order",
        &order_id.to_string(),
        serde_json::json!({}),
    )
    .await?;
    Ok(Json(OrderDetail {
        id: row.try_get("id")?,
        order_no: row.try_get("order_no")?,
        status: row.try_get("status")?,
        delivery_address_id: row.try_get("delivery_address_id")?,
        address_snapshot: row.try_get("address_snapshot")?,
        shipped_at: row.try_get("shipped_at")?,
        signed_at: row.try_get("signed_at")?,
        lines,
    }))
}

pub async fn authorize_report(
    state: &PortalState,
    auth: &PortalAuth,
    report_version_id: Uuid,
) -> Result<(String, String), PortalError> {
    let row = sqlx::query(
        "SELECT DISTINCT r.customer_copy_storage_key, r.customer_copy_file_name
         FROM portal_report_versions r
         JOIN portal_order_lines l
           ON l.product_id = r.product_id AND l.batch_no = r.batch_no
         JOIN portal_orders o ON o.id = l.order_id
         WHERE r.id = $1
           AND r.customer_copy_status = 'available'
           AND r.customer_copy_storage_key IS NOT NULL
           AND o.customer_id = $2
           AND o.status IN ('shipped', 'signed')
           AND ($3 OR r.is_current)
           AND (
               $4 = 'customer_admin'
               OR EXISTS (
                   SELECT 1 FROM portal_user_addresses ua
                   WHERE ua.address_id = o.delivery_address_id AND ua.user_id = $5
               )
           )
         LIMIT 1",
    )
    .bind(report_version_id)
    .bind(auth.customer_id)
    .bind(auth.can_view_report_history)
    .bind(&auth.role)
    .bind(auth.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(PortalError::NotFound)?;
    Ok((
        row.try_get("customer_copy_storage_key")?,
        row.try_get("customer_copy_file_name")?,
    ))
}

fn order_summary_from_row(row: sqlx::postgres::PgRow) -> Result<OrderSummary, sqlx::Error> {
    Ok(OrderSummary {
        id: row.try_get("id")?,
        order_no: row.try_get("order_no")?,
        status: row.try_get("status")?,
        delivery_address_id: row.try_get("delivery_address_id")?,
        address_name: row.try_get("address_name")?,
        shipped_at: row.try_get("shipped_at")?,
        signed_at: row.try_get("signed_at")?,
        line_count: row.try_get("line_count")?,
        available_report_count: row.try_get("available_report_count")?,
        pending_report_count: row.try_get("pending_report_count")?,
    })
}

fn report_summary_from_row(row: sqlx::postgres::PgRow) -> Result<ReportSummary, sqlx::Error> {
    Ok(ReportSummary {
        id: row.try_get("id")?,
        report_id: row.try_get("report_id")?,
        version_number: row.try_get("version_number")?,
        report_no: row.try_get("report_no")?,
        status: row.try_get("status")?,
        is_current: row.try_get("is_current")?,
        modification_reason: row.try_get("modification_reason")?,
        customer_copy_status: row.try_get("customer_copy_status")?,
        customer_copy_file_name: row.try_get("customer_copy_file_name")?,
        customer_copy_size: row.try_get("customer_copy_size")?,
        digitally_signed_original: row.try_get("digitally_signed_original")?,
        confirmed_at: row.try_get("confirmed_at")?,
    })
}
