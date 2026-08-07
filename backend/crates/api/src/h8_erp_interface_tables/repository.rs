use axum::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use deadpool_tiberius_rustls::{
    tiberius_rustls::{EncryptionLevel, Query},
    Manager, Pool,
};
use std::{collections::HashMap, net::IpAddr, sync::Mutex, time::Duration};
use uuid::Uuid;
use wms_domain::{
    enforce_interface_table_scope, interface_table_spec, redacted_payload_summary,
    sanitize_error_summary, H8ErpConnector, H8ErpInterfaceTableDetail, H8ErpInterfaceTableField,
    H8ErpInterfaceTableListResponse, H8ErpInterfaceTableQuery, H8ErpInterfaceTableRow,
    H8_INTERFACE_TABLE_PAYLOAD_PARSE_MAX_BYTES, H8_INTERFACE_TABLE_PAYLOAD_SUMMARY_MAX_BYTES,
};

use super::error::H8InterfaceTableRepoError;
use crate::sync::lock_recover;

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
        let mut rows = lock_recover(&self.rows).clone();
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
        let row = lock_recover(&self.rows)
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

#[derive(Clone, Copy)]
struct V19TableContract {
    table_key: &'static str,
    primary_key: &'static str,
    primary_key_is_text: bool,
    business_key: &'static str,
    has_handelflag: bool,
}

const V19_TABLE_CONTRACTS: [V19TableContract; 16] = [
    V19TableContract {
        table_key: "x_wmsinter_GoodsInfo",
        primary_key: "seqid",
        primary_key_is_text: false,
        business_key: "GoodsCode",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_CustomerInfo",
        primary_key: "seqid",
        primary_key_is_text: false,
        business_key: "ClientCode",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_SupplierInfo",
        primary_key: "seqid",
        primary_key_is_text: false,
        business_key: "SupplierCode",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_InboundOrder",
        primary_key: "OrderID",
        primary_key_is_text: false,
        business_key: "ERPBillCode",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_InboundOrderItems",
        primary_key: "ItemID",
        primary_key_is_text: false,
        business_key: "ERPBillCode",
        has_handelflag: false,
    },
    V19TableContract {
        table_key: "x_wmsinter_OutboundOrder",
        primary_key: "OrderID",
        primary_key_is_text: false,
        business_key: "ERPBillCode",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_OutboundOrderItems",
        primary_key: "ItemID",
        primary_key_is_text: false,
        business_key: "ERPBillCode",
        has_handelflag: false,
    },
    V19TableContract {
        table_key: "x_wmsinter_OrderFeedback",
        primary_key: "FeedbackID",
        primary_key_is_text: false,
        business_key: "ERPBillCode",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_OrderCommand",
        primary_key: "CommandID",
        primary_key_is_text: true,
        business_key: "ERPBillCode",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_InboundFeedback",
        primary_key: "FeedbackID",
        primary_key_is_text: false,
        business_key: "ERPBillCode",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_OutboundFeedback",
        primary_key: "FeedbackID",
        primary_key_is_text: false,
        business_key: "ERPBillCode",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_WmsEvent",
        primary_key: "EventID",
        primary_key_is_text: false,
        business_key: "IdempotencyKey",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_InventoryPushHeader",
        primary_key: "PushID",
        primary_key_is_text: false,
        business_key: "SnapshotID",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_InventoryPushItems",
        primary_key: "ItemID",
        primary_key_is_text: false,
        business_key: "SnapshotID",
        has_handelflag: false,
    },
    V19TableContract {
        table_key: "x_wmsinter_InventoryReceiveHeader",
        primary_key: "ReceiveID",
        primary_key_is_text: false,
        business_key: "SnapshotID",
        has_handelflag: true,
    },
    V19TableContract {
        table_key: "x_wmsinter_InventoryReceiveItems",
        primary_key: "ItemID",
        primary_key_is_text: false,
        business_key: "SnapshotID",
        has_handelflag: false,
    },
];

fn v19_table_contract(table_key: &str) -> Option<&'static V19TableContract> {
    V19_TABLE_CONTRACTS
        .iter()
        .find(|contract| contract.table_key == table_key)
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
        let password = crate::secrets::resolve_secret_alias_for_probe(Some(alias))
            .map_err(|_| H8InterfaceTableRepoError::SecretNotResolved)?;
        let key = ProbePoolKey {
            connector_id: connector.id,
            probe_config_version: connector.interface_probe_config_version,
            transport_config_version: connector.config_version,
        };
        if let Some(pool) = lock_recover(&self.pools).get(&key).cloned() {
            return Ok(pool);
        }
        let mut manager = Manager::new()
            .host(host)
            .port(port as u16)
            .database(database)
            .basic_authentication(username, password);
        if is_loopback_probe_host(host) {
            manager = manager.encryption(EncryptionLevel::NotSupported);
        } else {
            manager = manager.trust_cert();
        }
        let pool = manager
            .max_size(4)
            .wait_timeout(Duration::from_secs(2))
            .create_timeout(Duration::from_secs(5))
            .recycle_timeout(Duration::from_secs(5))
            .create_pool()
            .map_err(|err| H8InterfaceTableRepoError::Db(sanitize_db_error(err.to_string())))?;
        let mut pools = lock_recover(&self.pools);
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
        _actor_warehouse_scope: Option<Uuid>,
        row_id: Option<&str>,
    ) -> Result<(Vec<H8ErpInterfaceTableRow>, u64), H8InterfaceTableRepoError> {
        let spec = interface_table_spec(&query.table_key)
            .ok_or(H8InterfaceTableRepoError::ConnectorNotSupported)?;
        let contract = v19_table_contract(spec.table_key)
            .ok_or(H8InterfaceTableRepoError::ConnectorNotSupported)?;
        if row_id
            .is_some_and(|value| !contract.primary_key_is_text && value.parse::<i32>().is_err())
        {
            return Err(H8InterfaceTableRepoError::NotFound);
        }
        let detail_mode = row_id.is_some();
        let projection = projection_for(spec.table_key, detail_mode)
            .ok_or(H8InterfaceTableRepoError::ConnectorNotSupported)?;
        let sync_statuses = query.sync_statuses();
        let handelflags = sync_statuses
            .iter()
            .map(|status| {
                handelflag_for_status(status).ok_or_else(|| {
                    H8InterfaceTableRepoError::Db("invalid v1.9 handelflag filter".into())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let owner_code = std::env::var("H8_OWNER_CODE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                H8InterfaceTableRepoError::Db("interface owner code is not configured".into())
            })?;
        let mut filters = vec!["OwnerCode = @P1".to_string()];
        let mut next = 2u32;
        if !detail_mode {
            filters.push("inserttime >= @P2".to_string());
            filters.push("inserttime <= @P3".to_string());
            next = 4;
        }
        if !handelflags.is_empty() {
            let placeholders = (0..handelflags.len())
                .map(|offset| format!("@P{}", next + offset as u32))
                .collect::<Vec<_>>();
            filters.push(format!("handelflag IN ({})", placeholders.join(",")));
            next += handelflags.len() as u32;
        }
        if let Some(value) = query.external_doc_no.as_deref() {
            filters.push(format!("{} = @P{next}", contract.business_key));
            next += 1;
            let _ = value;
        }
        if let Some(value) = query.event_type.as_deref() {
            filters.push(format!("EventType = @P{next}"));
            next += 1;
            let _ = value;
        }
        if let Some(value) = query.idempotency_key.as_deref() {
            filters.push(format!("IdempotencyKey = @P{next}"));
            next += 1;
            let _ = value;
        }
        if let Some(row_id) = row_id {
            filters.push(format!("{} = @P{next}", contract.primary_key));
            next += 1;
            let _ = row_id;
        }
        let offset = u64::from(query.page - 1) * u64::from(query.page_size);
        let end = offset + u64::from(query.page_size);
        let list_sql = format!(
            "SELECT * FROM (SELECT {projection}, ROW_NUMBER() OVER (ORDER BY inserttime DESC, {primary_key}) AS __row_num FROM dbo.{table} WHERE {filters}) page WHERE __row_num > @P{next} AND __row_num <= @P{} ORDER BY __row_num",
            next + 1,
            projection = projection,
            table = spec.table_key,
            primary_key = contract.primary_key,
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
                statement.bind(owner_code.clone());
                if !detail_mode {
                    statement.bind(query.updated_from.naive_utc());
                    statement.bind(query.updated_to.naive_utc());
                }
                for value in &handelflags {
                    statement.bind(*value);
                }
                if let Some(value) = query.external_doc_no.as_deref() {
                    statement.bind(value.to_owned());
                }
                if let Some(value) = query.event_type.as_deref() {
                    statement.bind(value.to_owned());
                }
                if let Some(value) = query.idempotency_key.as_deref() {
                    statement.bind(value.to_owned());
                }
                if let Some(value) = row_id {
                    if contract.primary_key_is_text {
                        statement.bind(value.to_owned());
                    } else if let Ok(value) = value.parse::<i32>() {
                        statement.bind(value);
                    }
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
            statement.bind(end as i64);
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
                mapped.push(map_row(
                    row,
                    connector.id,
                    connector.owner_id,
                    spec.table_key,
                    detail_mode,
                )?);
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

fn is_loopback_probe_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
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

fn projection_for(table_key: &str, detail_mode: bool) -> Option<String> {
    let contract = v19_table_contract(table_key)?;
    let processing = if contract.has_handelflag {
        "handelflag, retry_count, COALESCE(NULLIF(CONVERT(nvarchar(50), error_code), N''), handelmsg) AS last_error, COALESCE(processtime, inserttime) AS updated_at"
    } else {
        "CAST(NULL AS int) AS handelflag, 0 AS retry_count, CAST(NULL AS nvarchar(200)) AS last_error, inserttime AS updated_at"
    };
    let event_type = if table_key == "x_wmsinter_WmsEvent" {
        "EventType AS event_type"
    } else {
        "CAST(NULL AS nvarchar(50)) AS event_type"
    };
    let payload = if table_key == "x_wmsinter_WmsEvent" {
        "PayloadJson AS payload_json"
    } else {
        "N'{}' AS payload_json"
    };
    let product = match (table_key, detail_mode) {
        ("x_wmsinter_GoodsInfo", false) => {
            ", GoodsCode AS product_code, GoodsName AS product_name, Spec AS spec"
        }
        ("x_wmsinter_GoodsInfo", true) => {
            ", GoodsCode AS product_code, GoodsName AS product_name, Spec AS spec, License AS approval_no, ProduceCorp AS manufacturer, SpecialCategory AS special_drug_category, StoreMemo AS storage_condition, PackagingJson AS packaging_json, SchemaVersion AS schema_version"
        }
        _ => "",
    };
    Some(format!(
        "CONVERT(nvarchar(64), {primary_key}) AS row_id, OwnerCode AS owner_code, CONVERT(nvarchar(128), {business_key}) AS business_key, {event_type}, {processing}, IdempotencyKey AS idempotency_key, {payload}, inserttime AS created_at{product}",
        primary_key = contract.primary_key,
        business_key = contract.business_key,
    ))
}

fn map_row(
    row: deadpool_tiberius_rustls::tiberius_rustls::Row,
    connector_id: Uuid,
    owner_id: Uuid,
    table_key: &str,
    detail_mode: bool,
) -> Result<H8ErpInterfaceTableRow, H8InterfaceTableRepoError> {
    let row_id = row
        .get::<&str, _>("row_id")
        .ok_or_else(|| H8InterfaceTableRepoError::Db("interface row id missing".into()))?;
    let created_at = utc_datetime(&row, "created_at")
        .ok_or_else(|| H8InterfaceTableRepoError::Db("interface row created_at missing".into()))?;
    let updated_at = utc_datetime(&row, "updated_at")
        .ok_or_else(|| H8InterfaceTableRepoError::Db("interface row updated_at missing".into()))?;
    let payload = row.get::<&str, _>("payload_json").unwrap_or("{}");
    Ok(H8ErpInterfaceTableRow {
        row_id: row_id.to_string(),
        connector_id,
        table_key: table_key.into(),
        owner_id,
        warehouse_id: None,
        business_key: row.get::<&str, _>("business_key").map(ToOwned::to_owned),
        business_fields: product_master_business_fields(&row, table_key, detail_mode),
        event_type: row.get::<&str, _>("event_type").map(ToOwned::to_owned),
        external_ref: None,
        wms_resource_id: None,
        sync_status: handelflag_status(row.get::<i32, _>("handelflag")).into(),
        retry_count: row.get::<i32, _>("retry_count").unwrap_or_default(),
        last_error: row.get::<&str, _>("last_error").map(sanitize_error_summary),
        idempotency_key: row.get::<&str, _>("idempotency_key").map(ToOwned::to_owned),
        created_at,
        updated_at,
        payload_summary: redacted_payload_summary(payload),
    })
}

fn product_master_business_fields(
    row: &deadpool_tiberius_rustls::tiberius_rustls::Row,
    table_key: &str,
    detail_mode: bool,
) -> Vec<H8ErpInterfaceTableField> {
    let field = |key: &str| H8ErpInterfaceTableField {
        key: key.into(),
        value: row.get::<&str, _>(key).map(ToOwned::to_owned),
    };
    let mut fields = vec![field("owner_code")];
    if table_key != "x_wmsinter_GoodsInfo" {
        return fields;
    }
    fields.extend([field("product_code"), field("product_name"), field("spec")]);
    if detail_mode {
        fields.extend(
            [
                "approval_no",
                "manufacturer",
                "special_drug_category",
                "storage_condition",
                "schema_version",
            ]
            .into_iter()
            .map(field),
        );
        fields.push(H8ErpInterfaceTableField {
            key: "packaging_levels".into(),
            value: safe_packaging_levels(row.get::<&str, _>("packaging_json")),
        });
    }
    fields
}

fn utc_datetime(
    row: &deadpool_tiberius_rustls::tiberius_rustls::Row,
    column: &str,
) -> Option<DateTime<Utc>> {
    row.get::<NaiveDateTime, _>(column)
        .map(|value| value.and_utc())
        .or_else(|| row.get::<DateTime<Utc>, _>(column))
}

fn handelflag_for_status(status: &str) -> Option<i32> {
    match status {
        "pending" => Some(0),
        "awaiting_receipt" => Some(1),
        "processing" => Some(2),
        "failed" => Some(3),
        "dead" => Some(4),
        "acked" => Some(5),
        _ => None,
    }
}

fn handelflag_status(handelflag: Option<i32>) -> &'static str {
    match handelflag {
        Some(0) => "pending",
        Some(1) => "awaiting_receipt",
        Some(2) => "processing",
        Some(3) => "failed",
        Some(4) => "dead",
        Some(5) => "acked",
        Some(_) => "unknown",
        None => "readonly",
    }
}

fn safe_packaging_levels(raw: Option<&str>) -> Option<String> {
    let raw = raw?;
    if raw.len() > H8_INTERFACE_TABLE_PAYLOAD_PARSE_MAX_BYTES {
        return Some("[包装数据过大，已省略]".into());
    }
    let Ok(serde_json::Value::Array(levels)) = serde_json::from_str(raw) else {
        return Some("[包装数据格式无效]".into());
    };
    if levels.is_empty() {
        return Some("[包装数据格式无效]".into());
    }
    let mut safe = Vec::with_capacity(levels.len());
    for (index, level) in levels.into_iter().enumerate() {
        let serde_json::Value::Object(level) = level else {
            return Some("[包装数据格式无效]".into());
        };
        let (Some(unit), Some(ratio_to_base), Some(is_base), Some(is_default)) = (
            level.get("unit").and_then(serde_json::Value::as_str),
            level
                .get("ratio_to_base")
                .and_then(serde_json::Value::as_i64),
            level.get("is_base").and_then(serde_json::Value::as_bool),
            level.get("is_default").and_then(serde_json::Value::as_bool),
        ) else {
            return Some("[包装数据格式无效]".into());
        };
        let sort_order = level
            .get("sort_order")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(index as i64 + 1);
        if unit.trim().is_empty() || ratio_to_base <= 0 {
            return Some("[包装数据格式无效]".into());
        }
        safe.push(serde_json::json!({
            "unit": unit,
            "ratio_to_base": ratio_to_base,
            "is_base": is_base,
            "is_default": is_default,
            "sort_order": sort_order,
        }));
    }
    let Ok(serialized) = serde_json::to_string(&safe) else {
        return Some("[包装数据格式无效]".into());
    };
    if serialized.len() > H8_INTERFACE_TABLE_PAYLOAD_SUMMARY_MAX_BYTES {
        return Some("[包装数据过大，已省略]".into());
    }
    Some(serialized)
}

fn detail_from_row(row: H8ErpInterfaceTableRow) -> H8ErpInterfaceTableDetail {
    let mut fields = [
        ("id", Some(row.row_id.clone())),
        ("owner_id", Some(row.owner_id.to_string())),
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
    .collect::<Vec<_>>();
    fields.extend(row.business_fields.clone());
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
#[path = "repository_tests.rs"]
mod tests;
