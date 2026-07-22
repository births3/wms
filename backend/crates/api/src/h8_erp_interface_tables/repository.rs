use axum::async_trait;
use chrono::{DateTime, Utc};
use deadpool_tiberius_rustls::{tiberius_rustls::Query, Manager, Pool};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use uuid::Uuid;
use wms_domain::{
    enforce_interface_table_scope, interface_table_spec, redacted_payload_summary,
    sanitize_error_summary, H8ErpConnector, H8ErpInterfaceTableDetail, H8ErpInterfaceTableField,
    H8ErpInterfaceTableListResponse, H8ErpInterfaceTableQuery, H8ErpInterfaceTableRow,
};

use super::error::H8InterfaceTableRepoError;

#[async_trait]
pub(crate) trait H8InterfaceTableRepository: Send + Sync {
    async fn list(
        &self,
        connector: &H8ErpConnector,
        query: &H8ErpInterfaceTableQuery,
        actor_warehouse_scope: Option<Uuid>,
    ) -> Result<H8ErpInterfaceTableListResponse, H8InterfaceTableRepoError>;

    async fn detail(
        &self,
        connector: &H8ErpConnector,
        table_key: &str,
        row_id: &str,
        actor_warehouse_scope: Option<Uuid>,
    ) -> Result<H8ErpInterfaceTableDetail, H8InterfaceTableRepoError>;
}

#[derive(Default)]
pub(crate) struct MemoryH8InterfaceTableRepository {
    rows: Mutex<Vec<H8ErpInterfaceTableRow>>,
}

impl MemoryH8InterfaceTableRepository {
    pub(crate) fn with_rows(rows: Vec<H8ErpInterfaceTableRow>) -> Self {
        Self {
            rows: Mutex::new(rows),
        }
    }
}

#[async_trait]
impl H8InterfaceTableRepository for MemoryH8InterfaceTableRepository {
    async fn list(
        &self,
        connector: &H8ErpConnector,
        query: &H8ErpInterfaceTableQuery,
        actor_warehouse_scope: Option<Uuid>,
    ) -> Result<H8ErpInterfaceTableListResponse, H8InterfaceTableRepoError> {
        query
            .validate()
            .map_err(|err| H8InterfaceTableRepoError::Db(format!("invalid query: {err:?}")))?;
        let mut rows = self.rows.lock().expect("interface rows").clone();
        let sync_statuses = query.sync_statuses();
        rows.retain(|row| {
            row.connector_id == connector.id
                && row.table_key == query.table_key
                && row.owner_id == connector.owner_id
                && row.updated_at >= query.updated_from
                && row.updated_at <= query.updated_to
                && (sync_statuses.is_empty() || sync_statuses.contains(&row.sync_status.as_str()))
                && query
                    .warehouse_id
                    .is_none_or(|value| row.warehouse_id == Some(value))
                && query
                    .external_doc_no
                    .as_deref()
                    .is_none_or(|value| row.business_key.as_deref() == Some(value))
                && query
                    .source_outbox_id
                    .as_deref()
                    .is_none_or(|value| row.business_key.as_deref() == Some(value))
                && query
                    .event_type
                    .as_deref()
                    .is_none_or(|value| row.event_type.as_deref() == Some(value))
                && query
                    .idempotency_key
                    .as_deref()
                    .is_none_or(|value| row.idempotency_key.as_deref() == Some(value))
                && enforce_interface_table_scope(
                    row.owner_id,
                    row.warehouse_id,
                    connector.owner_id,
                    actor_warehouse_scope,
                    &connector.warehouse_ids,
                )
                .is_ok()
        });
        let total = rows.len() as u64;
        let start = (u64::from(query.page - 1) * u64::from(query.page_size)) as usize;
        let items = rows
            .into_iter()
            .skip(start)
            .take(query.page_size as usize)
            .collect();
        Ok(H8ErpInterfaceTableListResponse {
            items,
            total,
            page: query.page,
            page_size: query.page_size,
        })
    }

    async fn detail(
        &self,
        connector: &H8ErpConnector,
        table_key: &str,
        row_id: &str,
        actor_warehouse_scope: Option<Uuid>,
    ) -> Result<H8ErpInterfaceTableDetail, H8InterfaceTableRepoError> {
        let row = self
            .rows
            .lock()
            .expect("interface rows")
            .iter()
            .find(|row| {
                row.connector_id == connector.id
                    && row.table_key == table_key
                    && row.row_id == row_id
                    && enforce_interface_table_scope(
                        row.owner_id,
                        row.warehouse_id,
                        connector.owner_id,
                        actor_warehouse_scope,
                        &connector.warehouse_ids,
                    )
                    .is_ok()
            })
            .cloned()
            .ok_or(H8InterfaceTableRepoError::NotFound)?;
        Ok(detail_from_row(row))
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct ProbePoolKey {
    connector_id: Uuid,
    probe_config_version: i64,
    transport_config_version: i64,
}

#[derive(Default)]
pub(crate) struct MssqlH8InterfaceTableRepository {
    pools: Mutex<HashMap<ProbePoolKey, Pool>>,
}

impl MssqlH8InterfaceTableRepository {
    fn pool_for(&self, connector: &H8ErpConnector) -> Result<Pool, H8InterfaceTableRepoError> {
        let (Some(username), Some(alias), Some(host), Some(database), Some(port)) = (
            connector.interface_probe_db_username.as_deref(),
            connector.interface_probe_db_password_alias.as_deref(),
            connector.interface_db_host.as_deref(),
            connector.interface_db_name.as_deref(),
            connector.interface_db_port,
        ) else {
            return Err(H8InterfaceTableRepoError::ProbeCredentialNotConfigured);
        };
        if username.trim().is_empty() || alias.trim().is_empty() {
            return Err(H8InterfaceTableRepoError::ProbeCredentialNotConfigured);
        }
        let password = crate::secrets::resolve_secret_alias(alias)
            .map_err(|_| H8InterfaceTableRepoError::SecretNotResolved)?;
        if password.is_empty() && std::env::var("WMS_SECRETS_REQUIRE_RESOLVE").is_ok() {
            return Err(H8InterfaceTableRepoError::SecretNotResolved);
        }
        let key = ProbePoolKey {
            connector_id: connector.id,
            probe_config_version: connector.interface_probe_config_version,
            transport_config_version: connector.config_version,
        };
        if let Some(pool) = self.pools.lock().expect("probe pool").get(&key).cloned() {
            return Ok(pool);
        }
        let pool = Manager::new()
            .host(host)
            .port(port as u16)
            .database(database)
            .basic_authentication(username, password)
            .trust_cert()
            .max_size(4)
            .wait_timeout(Duration::from_secs(2))
            .create_timeout(Duration::from_secs(5))
            .recycle_timeout(Duration::from_secs(5))
            .create_pool()
            .map_err(|err| H8InterfaceTableRepoError::Db(sanitize_db_error(err.to_string())))?;
        let mut pools = self.pools.lock().expect("probe pool");
        // 凭据版本变更后旧池不可再复用；及时移除旧版本，避免长期配置变更造成池缓存增长。
        pools.retain(|pool_key, _| {
            pool_key.connector_id != connector.id
                || (pool_key.probe_config_version == connector.interface_probe_config_version
                    && pool_key.transport_config_version == connector.config_version)
        });
        pools.insert(key, pool.clone());
        Ok(pool)
    }

    async fn query_rows(
        &self,
        connector: &H8ErpConnector,
        query: &H8ErpInterfaceTableQuery,
        actor_warehouse_scope: Option<Uuid>,
        row_id: Option<Uuid>,
    ) -> Result<(Vec<H8ErpInterfaceTableRow>, u64), H8InterfaceTableRepoError> {
        let spec = interface_table_spec(&query.table_key)
            .ok_or(H8InterfaceTableRepoError::ConnectorNotSupported)?;
        let projection = projection_for(spec.table_key);
        let detail_mode = row_id.is_some();
        let sync_statuses = query.sync_statuses();
        let mut filters = vec!["owner_id = @P1".to_string()];
        let mut next = 2u32;
        if !detail_mode {
            filters.push("updated_at >= @P2".to_string());
            filters.push("updated_at <= @P3".to_string());
            next = 4;
        }
        if !sync_statuses.is_empty() {
            let placeholders = (0..sync_statuses.len())
                .map(|offset| format!("@P{}", next + offset as u32))
                .collect::<Vec<_>>();
            filters.push(format!("sync_status IN ({})", placeholders.join(",")));
            next += sync_statuses.len() as u32;
        }
        if let Some(value) = query.warehouse_id {
            filters.push(format!("warehouse_id = @P{next}"));
            next += 1;
            let _ = value;
        }
        if let Some(value) = actor_warehouse_scope {
            if spec.has_warehouse_id {
                filters.push(format!("warehouse_id = @P{next}"));
                next += 1;
                let _ = value;
            }
        }
        if !connector.warehouse_ids.is_empty() && spec.has_warehouse_id {
            let placeholders: Vec<_> = connector
                .warehouse_ids
                .iter()
                .enumerate()
                .map(|(offset, _)| format!("@P{}", next + offset as u32))
                .collect();
            filters.push(format!("warehouse_id IN ({})", placeholders.join(",")));
            next += connector.warehouse_ids.len() as u32;
        }
        if let Some(value) = query.external_doc_no.as_deref() {
            filters.push(format!("external_doc_no = @P{next}"));
            next += 1;
            let _ = value;
        }
        if let Some(value) = query.source_outbox_id.as_deref() {
            filters.push(format!("source_outbox_id = @P{next}"));
            next += 1;
            let _ = value;
        }
        if let Some(value) = query.event_type.as_deref() {
            filters.push(format!("event_type = @P{next}"));
            next += 1;
            let _ = value;
        }
        if let Some(value) = query.external_ref.as_deref() {
            filters.push(format!("external_ref = @P{next}"));
            next += 1;
            let _ = value;
        }
        if let Some(value) = query.idempotency_key.as_deref() {
            filters.push(format!("idempotency_key = @P{next}"));
            next += 1;
            let _ = value;
        }
        if let Some(value) = query.wms_resource_id.as_deref() {
            filters.push(format!("wms_resource_id = @P{next}"));
            next += 1;
            let _ = value;
        }
        if let Some(row_id) = row_id {
            filters.push(format!("id = @P{next}"));
            next += 1;
            let _ = row_id;
        }
        let offset = u64::from(query.page - 1) * u64::from(query.page_size);
        let list_sql = format!(
            "SELECT {projection} FROM dbo.{table} WHERE {filters} ORDER BY updated_at DESC, id OFFSET @P{next} ROWS FETCH NEXT @P{} ROWS ONLY",
            next + 1,
            projection = projection,
            table = spec.table_key,
            filters = filters.join(" AND "),
            next = next,
        );
        let count_sql = format!(
            "SELECT COUNT_BIG(1) AS total_count FROM dbo.{table} WHERE {filters}",
            table = spec.table_key,
            filters = filters.join(" AND "),
        );
        let pool = self.pool_for(connector)?;
        let mut conn = pool
            .get()
            .await
            .map_err(|err| H8InterfaceTableRepoError::Db(sanitize_db_error(err.to_string())))?;
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            let bind_filters = |statement: &mut Query<'_>| {
                statement.bind(connector.owner_id);
                if !detail_mode {
                    statement.bind(query.updated_from);
                    statement.bind(query.updated_to);
                }
                for value in &sync_statuses {
                    statement.bind((*value).to_owned());
                }
                if let Some(value) = query.warehouse_id {
                    statement.bind(value);
                }
                if let Some(value) = actor_warehouse_scope {
                    if spec.has_warehouse_id {
                        statement.bind(value);
                    }
                }
                // The connector whitelist predicate is always emitted when the
                // whitelist is non-empty, so its parameters must be bound even
                // when the request also has an actor/query warehouse predicate.
                if !connector.warehouse_ids.is_empty() && spec.has_warehouse_id {
                    for value in &connector.warehouse_ids {
                        statement.bind(*value);
                    }
                }
                if let Some(value) = query.external_doc_no.as_deref() {
                    statement.bind(value.to_owned());
                }
                if let Some(value) = query.source_outbox_id.as_deref() {
                    statement.bind(value.to_owned());
                }
                if let Some(value) = query.event_type.as_deref() {
                    statement.bind(value.to_owned());
                }
                if let Some(value) = query.external_ref.as_deref() {
                    statement.bind(value.to_owned());
                }
                if let Some(value) = query.idempotency_key.as_deref() {
                    statement.bind(value.to_owned());
                }
                if let Some(value) = query.wms_resource_id.as_deref() {
                    statement.bind(value.to_owned());
                }
                if let Some(value) = row_id {
                    statement.bind(value);
                }
            };

            let mut count_statement = Query::new(count_sql);
            bind_filters(&mut count_statement);
            let count_rows = count_statement
                .query(&mut *conn)
                .await
                .map_err(|err| H8InterfaceTableRepoError::Db(sanitize_db_error(err.to_string())))?
                .into_first_result()
                .await
                .map_err(|err| H8InterfaceTableRepoError::Db(sanitize_db_error(err.to_string())))?;
            let total = count_rows
                .first()
                .and_then(|row| row.get::<i64, _>("total_count"))
                .unwrap_or_default() as u64;

            let mut statement = Query::new(list_sql);
            bind_filters(&mut statement);
            statement.bind(offset as i64);
            statement.bind(query.page_size as i32);
            let stream = statement
                .query(&mut *conn)
                .await
                .map_err(|err| H8InterfaceTableRepoError::Db(sanitize_db_error(err.to_string())))?;
            let rows = stream
                .into_first_result()
                .await
                .map_err(|err| H8InterfaceTableRepoError::Db(sanitize_db_error(err.to_string())))?;
            let mut mapped = Vec::with_capacity(rows.len());
            for row in rows {
                mapped.push(map_row(row, connector.id, spec.table_key)?);
            }
            Ok((mapped, total))
        })
        .await
        .map_err(|_| {
            tracing::warn!(
                target: "h8.interface_table",
                connector_id = %connector.id,
                table_key = %query.table_key,
                "interface table query timed out"
            );
            H8InterfaceTableRepoError::Db("interface probe timeout".into())
        })??;
        Ok(result)
    }
}

#[async_trait]
impl H8InterfaceTableRepository for MssqlH8InterfaceTableRepository {
    async fn list(
        &self,
        connector: &H8ErpConnector,
        query: &H8ErpInterfaceTableQuery,
        actor_warehouse_scope: Option<Uuid>,
    ) -> Result<H8ErpInterfaceTableListResponse, H8InterfaceTableRepoError> {
        let (rows, total) = self
            .query_rows(connector, query, actor_warehouse_scope, None)
            .await?;
        Ok(H8ErpInterfaceTableListResponse {
            items: rows,
            total,
            page: query.page,
            page_size: query.page_size,
        })
    }

    async fn detail(
        &self,
        connector: &H8ErpConnector,
        table_key: &str,
        row_id: &str,
        actor_warehouse_scope: Option<Uuid>,
    ) -> Result<H8ErpInterfaceTableDetail, H8InterfaceTableRepoError> {
        let row_id = Uuid::parse_str(row_id).map_err(|_| H8InterfaceTableRepoError::NotFound)?;
        let now = Utc::now();
        let query = H8ErpInterfaceTableQuery {
            connector_id: connector.id,
            table_key: table_key.into(),
            updated_from: now - chrono::Duration::days(31),
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
            page_size: 1,
        };
        let (rows, _) = self
            .query_rows(connector, &query, actor_warehouse_scope, Some(row_id))
            .await?;
        let row = rows
            .into_iter()
            .next()
            .ok_or(H8InterfaceTableRepoError::NotFound)?;
        Ok(detail_from_row(row))
    }
}

fn projection_for(table_key: &str) -> &'static str {
    match table_key {
        "if_out_message" => "id, owner_id, CAST(NULL AS uniqueidentifier) AS warehouse_id, source_outbox_id AS business_key, event_type, external_ref, CAST(NULL AS nvarchar(64)) AS wms_resource_id, sync_status, retry_count, last_error, idempotency_key, payload_json, created_at, updated_at",
        "if_in_product_master" | "if_in_product_change" => "id, owner_id, CAST(NULL AS uniqueidentifier) AS warehouse_id, external_doc_no AS business_key, CAST(NULL AS nvarchar(64)) AS event_type, CAST(NULL AS nvarchar(128)) AS external_ref, wms_resource_id, sync_status, retry_count, last_error, idempotency_key, payload_json, created_at, updated_at",
        "if_in_outbound_order" => "id, owner_id, warehouse_id, external_doc_no AS business_key, CAST(NULL AS nvarchar(64)) AS event_type, CAST(NULL AS nvarchar(128)) AS external_ref, wms_resource_id, sync_status, retry_count, last_error, idempotency_key, payload_json, created_at, updated_at",
        _ => "id, owner_id, warehouse_id, external_doc_no AS business_key, CAST(NULL AS nvarchar(64)) AS event_type, external_ref, wms_resource_id, sync_status, retry_count, last_error, idempotency_key, payload_json, created_at, updated_at",
    }
}

fn map_row(
    row: deadpool_tiberius_rustls::tiberius_rustls::Row,
    connector_id: Uuid,
    table_key: &str,
) -> Result<H8ErpInterfaceTableRow, H8InterfaceTableRepoError> {
    let id = row
        .get::<Uuid, _>("id")
        .ok_or_else(|| H8InterfaceTableRepoError::Db("interface row id missing".into()))?;
    let owner_id = row
        .get::<Uuid, _>("owner_id")
        .ok_or_else(|| H8InterfaceTableRepoError::Db("interface row owner missing".into()))?;
    let created_at = row
        .get::<DateTime<Utc>, _>("created_at")
        .ok_or_else(|| H8InterfaceTableRepoError::Db("interface row created_at missing".into()))?;
    let updated_at = row
        .get::<DateTime<Utc>, _>("updated_at")
        .ok_or_else(|| H8InterfaceTableRepoError::Db("interface row updated_at missing".into()))?;
    let payload = row.get::<&str, _>("payload_json").unwrap_or("");
    Ok(H8ErpInterfaceTableRow {
        row_id: id.to_string(),
        connector_id,
        table_key: table_key.into(),
        owner_id,
        warehouse_id: row.get("warehouse_id"),
        business_key: row.get::<&str, _>("business_key").map(ToOwned::to_owned),
        event_type: row.get::<&str, _>("event_type").map(ToOwned::to_owned),
        external_ref: row.get::<&str, _>("external_ref").map(ToOwned::to_owned),
        wms_resource_id: row.get::<&str, _>("wms_resource_id").map(ToOwned::to_owned),
        sync_status: row
            .get::<&str, _>("sync_status")
            .unwrap_or("unknown")
            .to_string(),
        retry_count: row.get::<i32, _>("retry_count").unwrap_or_default(),
        last_error: row.get::<&str, _>("last_error").map(sanitize_error_summary),
        idempotency_key: row.get::<&str, _>("idempotency_key").map(ToOwned::to_owned),
        created_at,
        updated_at,
        payload_summary: redacted_payload_summary(payload),
    })
}

fn detail_from_row(row: H8ErpInterfaceTableRow) -> H8ErpInterfaceTableDetail {
    let fields = [
        ("id", Some(row.row_id.clone())),
        ("business_key", row.business_key.clone()),
        ("event_type", row.event_type.clone()),
        ("external_ref", row.external_ref.clone()),
        (
            "warehouse_id",
            row.warehouse_id.map(|value| value.to_string()),
        ),
        ("wms_resource_id", row.wms_resource_id.clone()),
        ("sync_status", Some(row.sync_status.clone())),
        ("retry_count", Some(row.retry_count.to_string())),
        ("last_error", row.last_error.clone()),
        ("idempotency_key", row.idempotency_key.clone()),
        ("created_at", Some(row.created_at.to_rfc3339())),
        ("updated_at", Some(row.updated_at.to_rfc3339())),
        ("payload_summary", Some(row.payload_summary.clone())),
    ]
    .into_iter()
    .map(|(key, value)| H8ErpInterfaceTableField {
        key: key.into(),
        value,
    })
    .collect();
    H8ErpInterfaceTableDetail { row, fields }
}

fn sanitize_db_error(message: String) -> String {
    let lower = message.to_ascii_lowercase();
    if lower.contains("login") || lower.contains("password") || lower.contains("credential") {
        "interface probe connection failed".into()
    } else {
        message.chars().take(300).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::projection_for;

    #[test]
    fn projections_only_reference_columns_present_in_each_interface_table() {
        let outbound = projection_for("if_in_outbound_order");
        assert!(outbound.contains("external_doc_no AS business_key"));
        assert!(outbound.contains("CAST(NULL AS nvarchar(128)) AS external_ref"));
        assert!(!outbound.contains(", external_ref, wms_resource_id"));

        let out_message = projection_for("if_out_message");
        assert!(out_message.contains("source_outbox_id AS business_key"));
        assert!(out_message.contains("CAST(NULL AS uniqueidentifier) AS warehouse_id"));
    }
}
