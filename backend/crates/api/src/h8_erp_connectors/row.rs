//! H8 ERP 连接行映射。

use chrono::Utc;
use uuid::Uuid;
use wms_domain::H8ErpConnector;

#[derive(sqlx::FromRow)]
pub(crate) struct H8ErpConnectorRow {
    id: Uuid,
    owner_id: Uuid,
    connector_code: String,
    connector_name: String,
    warehouse_ids: Vec<Uuid>,
    directions: Vec<String>,
    message_types: Vec<String>,
    channel_mode: String,
    api_base_url: Option<String>,
    interface_db_host: Option<String>,
    interface_db_port: Option<i32>,
    interface_db_name: Option<String>,
    interface_db_username: Option<String>,
    api_key_id: Option<Uuid>,
    bearer_secret_alias: Option<String>,
    interface_db_password_alias: Option<String>,
    interface_probe_db_username: Option<String>,
    interface_probe_db_password_alias: Option<String>,
    interface_probe_config_version: i64,
    status: String,
    config_version: i64,
    first_activated_at: Option<chrono::DateTime<Utc>>,
    last_tested_version: Option<i64>,
    last_tested_at: Option<chrono::DateTime<Utc>>,
    last_tested_succeeded: Option<bool>,
    last_tested_error_summary: Option<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl From<H8ErpConnectorRow> for H8ErpConnector {
    fn from(r: H8ErpConnectorRow) -> Self {
        Self {
            id: r.id,
            owner_id: r.owner_id,
            connector_code: r.connector_code,
            connector_name: r.connector_name,
            warehouse_ids: r.warehouse_ids,
            directions: r.directions,
            message_types: r.message_types,
            channel_mode: r.channel_mode,
            api_base_url: r.api_base_url,
            interface_db_host: r.interface_db_host,
            interface_db_port: r.interface_db_port,
            interface_db_name: r.interface_db_name,
            interface_db_username: r.interface_db_username,
            api_key_id: r.api_key_id,
            bearer_secret_alias: r.bearer_secret_alias,
            interface_db_password_alias: r.interface_db_password_alias,
            interface_probe_db_username: r.interface_probe_db_username,
            interface_probe_db_password_alias_set: r
                .interface_probe_db_password_alias
                .as_deref()
                .is_some_and(|alias| !alias.trim().is_empty()),
            interface_probe_db_password_alias: r.interface_probe_db_password_alias,
            interface_probe_config_version: r.interface_probe_config_version,
            status: r.status,
            config_version: r.config_version,
            first_activated_at: r.first_activated_at,
            last_tested_version: r.last_tested_version,
            last_tested_at: r.last_tested_at,
            last_tested_succeeded: r.last_tested_succeeded,
            last_tested_error_summary: r.last_tested_error_summary,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
