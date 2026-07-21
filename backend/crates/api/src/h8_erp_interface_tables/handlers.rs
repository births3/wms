use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use std::time::Instant;
use uuid::Uuid;
use wms_domain::{
    interface_table_spec, H8ErpInterfaceTableConnectorOption, H8ErpInterfaceTableDetail,
    H8ErpInterfaceTableListResponse, H8ErpInterfaceTableQuery, H8InterfaceTableQueryError,
};

use crate::auth::AuthContext;

use super::{
    audit::write_query_audit,
    error::{H8InterfaceTableHandlerError, H8InterfaceTableRepoError},
    state::{H8ErpInterfaceTableAppState, H8_INTERFACE_TABLE_READ},
};

pub fn h8_erp_interface_table_router(state: H8ErpInterfaceTableAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/h8/erp-interface-tables/connectors",
            get(list_connectors),
        )
        .route("/api/v1/h8/erp-interface-tables/rows", get(list_rows))
        .route(
            "/api/v1/h8/erp-interface-tables/rows/:row_id",
            get(detail_row),
        )
        .with_state(state)
}

async fn list_connectors(
    ctx: AuthContext,
    State(state): State<H8ErpInterfaceTableAppState>,
) -> Result<Json<Vec<H8ErpInterfaceTableConnectorOption>>, H8InterfaceTableHandlerError> {
    ctx.require_permission(H8_INTERFACE_TABLE_READ)?;
    let connectors = state.connectors.list(ctx.owner_id).await.map_err(|_| {
        H8InterfaceTableHandlerError::Repo(H8InterfaceTableRepoError::Db(
            "connector lookup failed".into(),
        ))
    })?;
    Ok(Json(
        connectors
            .into_iter()
            .filter(|connector| {
                connector.channel_mode == "interface_table"
                    || connector.channel_mode == "rest_primary_table_fallback"
            })
            .map(|connector| H8ErpInterfaceTableConnectorOption {
                id: connector.id,
                connector_code: connector.connector_code,
                connector_name: connector.connector_name,
                channel_mode: connector.channel_mode,
                status: connector.status,
                warehouse_ids: connector.warehouse_ids,
                probe_credentials_configured: probe_credentials_configured(
                    connector.interface_probe_db_username.as_deref(),
                    connector.interface_probe_db_password_alias.as_deref(),
                ),
            })
            .collect(),
    ))
}

fn probe_credentials_configured(username: Option<&str>, password_alias: Option<&str>) -> bool {
    username.is_some_and(|value| !value.trim().is_empty())
        && password_alias.is_some_and(|value| !value.trim().is_empty())
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    connector_id: Option<Uuid>,
    table_key: Option<String>,
    sync_status: Option<String>,
    time_from: Option<DateTime<Utc>>,
    time_to: Option<DateTime<Utc>>,
    warehouse_id: Option<Uuid>,
    external_doc_no: Option<String>,
    source_outbox_id: Option<String>,
    event_type: Option<String>,
    external_ref: Option<String>,
    wms_resource_id: Option<String>,
    idempotency_key: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct DetailQuery {
    connector_id: Option<Uuid>,
    table_key: Option<String>,
}

async fn list_rows(
    ctx: AuthContext,
    State(state): State<H8ErpInterfaceTableAppState>,
    Query(raw): Query<ListQuery>,
) -> Result<Json<H8ErpInterfaceTableListResponse>, H8InterfaceTableHandlerError> {
    ctx.require_permission(H8_INTERFACE_TABLE_READ)?;
    let connector_id = raw.connector_id.ok_or_else(|| {
        H8InterfaceTableHandlerError::BadRequest("connector_id is required".into())
    })?;
    let table_key = raw
        .table_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| H8InterfaceTableHandlerError::BadRequest("table_key is required".into()))?
        .to_string();
    let connector = state
        .connectors
        .get(ctx.owner_id, connector_id)
        .await
        .map_err(|_| H8InterfaceTableHandlerError::Repo(H8InterfaceTableRepoError::NotFound))?;
    ensure_connector_supported(&connector)?;
    ensure_probe_credentials_configured(&connector)?;
    ensure_scope_query(&ctx, &connector, &table_key, raw.warehouse_id)?;
    let started = Instant::now();
    let now = Utc::now();
    let query = H8ErpInterfaceTableQuery {
        connector_id,
        table_key,
        updated_from: raw.time_from.unwrap_or(now - Duration::days(7)),
        updated_to: raw.time_to.unwrap_or(now),
        sync_status: raw.sync_status,
        warehouse_id: raw.warehouse_id,
        external_doc_no: raw.external_doc_no,
        source_outbox_id: raw.source_outbox_id,
        event_type: raw.event_type,
        external_ref: raw.external_ref,
        wms_resource_id: raw.wms_resource_id,
        idempotency_key: raw.idempotency_key,
        page: raw.page.unwrap_or(1),
        page_size: raw.page_size.unwrap_or(50),
    };
    validate_query(&query)?;
    let table_spec = interface_table_spec(&query.table_key).ok_or_else(|| {
        H8InterfaceTableHandlerError::BadRequest("table_key is not allowlisted".into())
    })?;
    let response = match state
        .repository
        .list(&connector, &query, ctx.warehouse_scope)
        .await
    {
        Ok(response) => response,
        Err(error) => {
            write_query_audit(
                &state,
                &ctx,
                "h8_interface_table_list_query",
                connector.id,
                &query.table_key,
                filter_summary(&query, table_spec.has_warehouse_id),
                0,
            )
            .await?;
            return Err(error.into());
        }
    };
    write_query_audit(
        &state,
        &ctx,
        "h8_interface_table_list_query",
        connector.id,
        &query.table_key,
        filter_summary(&query, table_spec.has_warehouse_id),
        response.total,
    )
    .await?;
    tracing::info!(
        target: "h8.interface_table",
        action = "list",
        connector_id = %connector.id,
        table_key = %query.table_key,
        result_count = response.total,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "interface table query completed"
    );
    Ok(Json(response))
}

async fn detail_row(
    ctx: AuthContext,
    State(state): State<H8ErpInterfaceTableAppState>,
    Path(row_id): Path<String>,
    Query(raw): Query<DetailQuery>,
) -> Result<Json<H8ErpInterfaceTableDetail>, H8InterfaceTableHandlerError> {
    ctx.require_permission(H8_INTERFACE_TABLE_READ)?;
    let connector_id = raw.connector_id.ok_or_else(|| {
        H8InterfaceTableHandlerError::BadRequest("connector_id is required".into())
    })?;
    let table_key = raw
        .table_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| H8InterfaceTableHandlerError::BadRequest("table_key is required".into()))?;
    let connector = state
        .connectors
        .get(ctx.owner_id, connector_id)
        .await
        .map_err(|_| H8InterfaceTableHandlerError::Repo(H8InterfaceTableRepoError::NotFound))?;
    ensure_connector_supported(&connector)?;
    ensure_probe_credentials_configured(&connector)?;
    ensure_scope_query(&ctx, &connector, table_key, None)?;
    let started = Instant::now();
    let table_spec = interface_table_spec(table_key).ok_or_else(|| {
        H8InterfaceTableHandlerError::BadRequest("table_key is not allowlisted".into())
    })?;
    if Uuid::parse_str(&row_id).is_err() {
        write_query_audit(
            &state,
            &ctx,
            "h8_interface_table_detail_query",
            connector.id,
            table_key,
            serde_json::json!({
                "row_id": row_id,
                "warehouse_column": if table_spec.has_warehouse_id { "warehouse_id" } else { "无仓列" },
                "hit": false,
            }),
            0,
        )
        .await?;
        return Err(H8InterfaceTableHandlerError::Repo(
            H8InterfaceTableRepoError::NotFound,
        ));
    }
    let detail = match state
        .repository
        .detail(&connector, table_key, &row_id, ctx.warehouse_scope)
        .await
    {
        Ok(detail) => detail,
        Err(error) => {
            write_query_audit(
                &state,
                &ctx,
                "h8_interface_table_detail_query",
                connector.id,
                table_key,
                serde_json::json!({
                    "row_id": row_id,
                    "warehouse_column": if table_spec.has_warehouse_id { "warehouse_id" } else { "无仓列" },
                    "hit": false,
                }),
                0,
            )
            .await?;
            return Err(error.into());
        }
    };
    write_query_audit(
        &state,
        &ctx,
        "h8_interface_table_detail_query",
        connector.id,
        table_key,
        serde_json::json!({
            "row_id": row_id,
            "warehouse_column": if table_spec.has_warehouse_id { "warehouse_id" } else { "无仓列" },
            "hit": true,
        }),
        1,
    )
    .await?;
    tracing::info!(
        target: "h8.interface_table",
        action = "detail",
        connector_id = %connector.id,
        table_key = %table_key,
        result_count = 1u64,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "interface table detail query completed"
    );
    Ok(Json(detail))
}

fn ensure_connector_supported(
    connector: &wms_domain::H8ErpConnector,
) -> Result<(), H8InterfaceTableHandlerError> {
    if connector.channel_mode != "interface_table"
        && connector.channel_mode != "rest_primary_table_fallback"
    {
        return Err(H8InterfaceTableHandlerError::Repo(
            H8InterfaceTableRepoError::ConnectorNotSupported,
        ));
    }
    Ok(())
}

fn ensure_probe_credentials_configured(
    connector: &wms_domain::H8ErpConnector,
) -> Result<(), H8InterfaceTableHandlerError> {
    if !probe_credentials_configured(
        connector.interface_probe_db_username.as_deref(),
        connector.interface_probe_db_password_alias.as_deref(),
    ) {
        return Err(H8InterfaceTableHandlerError::Repo(
            H8InterfaceTableRepoError::ProbeCredentialNotConfigured,
        ));
    }
    Ok(())
}

fn ensure_scope_query(
    ctx: &AuthContext,
    connector: &wms_domain::H8ErpConnector,
    table_key: &str,
    warehouse_id: Option<Uuid>,
) -> Result<(), H8InterfaceTableHandlerError> {
    let spec = interface_table_spec(table_key).ok_or_else(|| {
        H8InterfaceTableHandlerError::BadRequest("table_key is not allowlisted".into())
    })?;
    if !spec.has_warehouse_id && ctx.warehouse_scope.is_some() {
        return Err(H8InterfaceTableHandlerError::Repo(
            H8InterfaceTableRepoError::Forbidden,
        ));
    }
    if let Some(warehouse_id) = warehouse_id {
        if ctx
            .warehouse_scope
            .is_some_and(|scope| scope != warehouse_id)
            || (!connector.warehouse_ids.is_empty()
                && !connector.warehouse_ids.contains(&warehouse_id))
        {
            return Err(H8InterfaceTableHandlerError::Repo(
                H8InterfaceTableRepoError::Forbidden,
            ));
        }
    }
    Ok(())
}

fn validate_query(query: &H8ErpInterfaceTableQuery) -> Result<(), H8InterfaceTableHandlerError> {
    query.validate().map_err(|err| {
        let message = match err {
            H8InterfaceTableQueryError::TableNotAllowed => "table_key is not allowlisted",
            H8InterfaceTableQueryError::InvalidTimeRange => "time_from must be <= time_to",
            H8InterfaceTableQueryError::TimeRangeTooLarge => "time range must be <= 31 days",
            H8InterfaceTableQueryError::InvalidPage => "page must be >= 1 and page_size 1..=100",
            H8InterfaceTableQueryError::InvalidSyncStatus => "sync_status is invalid for table",
            H8InterfaceTableQueryError::FilterNotSupported(field) => {
                return H8InterfaceTableHandlerError::BadRequest(format!(
                    "filter is not supported for table: {field}"
                ))
            }
        };
        H8InterfaceTableHandlerError::BadRequest(message.into())
    })
}

fn filter_summary(query: &H8ErpInterfaceTableQuery, has_warehouse_id: bool) -> serde_json::Value {
    serde_json::json!({
        "connector_id": query.connector_id,
        "table_key": query.table_key,
        "updated_from": query.updated_from,
        "updated_to": query.updated_to,
        "sync_status": query.sync_status,
        "warehouse_id": query.warehouse_id,
        "external_doc_no": query.external_doc_no,
        "source_outbox_id": query.source_outbox_id,
        "event_type": query.event_type,
        "external_ref": query.external_ref,
        "wms_resource_id": query.wms_resource_id,
        "idempotency_key": query.idempotency_key,
        "page": query.page,
        "page_size": query.page_size,
        "warehouse_column": if has_warehouse_id { "warehouse_id" } else { "无仓列" },
    })
}

#[cfg(test)]
mod tests {
    use super::{filter_summary, probe_credentials_configured};
    use chrono::Utc;
    use uuid::Uuid;
    use wms_domain::H8ErpInterfaceTableQuery;

    #[test]
    fn audit_summary_marks_owner_wide_tables_without_warehouse_column() {
        let now = Utc::now();
        let query = H8ErpInterfaceTableQuery {
            connector_id: Uuid::new_v4(),
            table_key: "if_in_product_master".into(),
            updated_from: now,
            updated_to: now,
            sync_status: None,
            warehouse_id: None,
            external_doc_no: None,
            source_outbox_id: None,
            event_type: None,
            external_ref: None,
            wms_resource_id: None,
            idempotency_key: None,
            page: 1,
            page_size: 50,
        };
        let summary = filter_summary(&query, false);
        assert_eq!(summary["warehouse_column"], "无仓列");
    }

    #[test]
    fn connector_selector_only_marks_a_complete_probe_credential_pair() {
        assert!(!probe_credentials_configured(Some("probe"), None));
        assert!(!probe_credentials_configured(
            Some("  "),
            Some("vault://h8/probe")
        ));
        assert!(probe_credentials_configured(
            Some("probe"),
            Some("vault://h8/probe")
        ));
    }
}
