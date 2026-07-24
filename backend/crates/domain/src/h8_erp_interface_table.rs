//! US-H8-004：ERP 接口表只读探查的纯领域契约。

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub const H8_INTERFACE_TABLE_PAGE_SIZE_MAX: u32 = 100;
pub const H8_INTERFACE_TABLE_MAX_RANGE_DAYS: i64 = 31;
pub const H8_INTERFACE_TABLE_PAYLOAD_SUMMARY_MAX_BYTES: usize = 4096;
/// 原始报文或结构化业务字段进入 JSON 解析前允许的最大字节数。
pub const H8_INTERFACE_TABLE_PAYLOAD_PARSE_MAX_BYTES: usize = 1024 * 1024;

const INBOUND_STATUSES: &[&str] = &["pending", "processing", "success", "failed", "dead"];
const OUTBOUND_STATUSES: &[&str] = &[
    "pending",
    "processing",
    "success",
    "failed",
    "dead",
    "acked",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct H8InterfaceTableSpec {
    pub table_key: &'static str,
    pub has_warehouse_id: bool,
    pub has_wms_resource_id: bool,
    pub allowed_sync_statuses: &'static [&'static str],
}

const INTERFACE_TABLE_SPECS: &[H8InterfaceTableSpec] = &[
    H8InterfaceTableSpec {
        table_key: "if_in_asn",
        has_warehouse_id: true,
        has_wms_resource_id: true,
        allowed_sync_statuses: INBOUND_STATUSES,
    },
    H8InterfaceTableSpec {
        table_key: "if_in_outbound_order",
        has_warehouse_id: true,
        has_wms_resource_id: true,
        allowed_sync_statuses: INBOUND_STATUSES,
    },
    H8InterfaceTableSpec {
        table_key: "if_in_return_order",
        has_warehouse_id: true,
        has_wms_resource_id: true,
        allowed_sync_statuses: INBOUND_STATUSES,
    },
    H8InterfaceTableSpec {
        table_key: "if_in_product_master",
        has_warehouse_id: false,
        has_wms_resource_id: true,
        allowed_sync_statuses: INBOUND_STATUSES,
    },
    H8InterfaceTableSpec {
        table_key: "if_in_product_change",
        has_warehouse_id: false,
        has_wms_resource_id: true,
        allowed_sync_statuses: INBOUND_STATUSES,
    },
    H8InterfaceTableSpec {
        table_key: "if_out_message",
        has_warehouse_id: false,
        has_wms_resource_id: false,
        allowed_sync_statuses: OUTBOUND_STATUSES,
    },
];

pub fn interface_table_spec(table_key: &str) -> Option<&'static H8InterfaceTableSpec> {
    INTERFACE_TABLE_SPECS
        .iter()
        .find(|spec| spec.table_key == table_key)
}

pub fn interface_table_specs() -> &'static [H8InterfaceTableSpec] {
    INTERFACE_TABLE_SPECS
}

/// 004 页面连接选择器的最小安全投影，不包含任何传输/探查 secret。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpInterfaceTableConnectorOption {
    pub id: Uuid,
    pub connector_code: String,
    pub connector_name: String,
    pub channel_mode: String,
    pub status: String,
    pub warehouse_ids: Vec<Uuid>,
    /// 仅表示独立探查账号的用户名与密码 alias 是否成对配置，不暴露 alias 内容。
    pub probe_credentials_configured: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpInterfaceTableQuery {
    pub connector_id: Uuid,
    pub table_key: String,
    pub updated_from: DateTime<Utc>,
    pub updated_to: DateTime<Utc>,
    pub sync_status: Option<String>,
    pub warehouse_id: Option<Uuid>,
    pub external_doc_no: Option<String>,
    pub source_outbox_id: Option<String>,
    pub event_type: Option<String>,
    pub external_ref: Option<String>,
    pub wms_resource_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpInterfaceTableListResponse {
    pub items: Vec<H8ErpInterfaceTableRow>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpInterfaceTableRow {
    pub row_id: String,
    pub connector_id: Uuid,
    pub table_key: String,
    pub owner_id: Uuid,
    pub warehouse_id: Option<Uuid>,
    pub business_key: Option<String>,
    /// 按接口表白名单返回的业务摘要字段；禁止放入原始 payload。
    pub business_fields: Vec<H8ErpInterfaceTableField>,
    pub event_type: Option<String>,
    pub external_ref: Option<String>,
    pub wms_resource_id: Option<String>,
    pub sync_status: String,
    pub retry_count: i32,
    pub last_error: Option<String>,
    pub idempotency_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub payload_summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpInterfaceTableDetail {
    pub row: H8ErpInterfaceTableRow,
    /// 仅返回白名单字段，不返回原始 payload_json。
    pub fields: Vec<H8ErpInterfaceTableField>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpInterfaceTableField {
    pub key: String,
    pub value: Option<String>,
}

impl H8ErpInterfaceTableQuery {
    pub fn sync_statuses(&self) -> Vec<&str> {
        self.sync_status
            .as_deref()
            .map(|value| value.split(',').map(str::trim).collect())
            .unwrap_or_default()
    }

    pub fn validate(&self) -> Result<(), H8InterfaceTableQueryError> {
        let spec = interface_table_spec(self.table_key.trim())
            .ok_or(H8InterfaceTableQueryError::TableNotAllowed)?;
        if self.updated_from > self.updated_to {
            return Err(H8InterfaceTableQueryError::InvalidTimeRange);
        }
        if self.updated_to - self.updated_from > Duration::days(H8_INTERFACE_TABLE_MAX_RANGE_DAYS) {
            return Err(H8InterfaceTableQueryError::TimeRangeTooLarge);
        }
        if self.page == 0
            || self.page_size == 0
            || self.page_size > H8_INTERFACE_TABLE_PAGE_SIZE_MAX
        {
            return Err(H8InterfaceTableQueryError::InvalidPage);
        }
        let sync_statuses = self.sync_statuses();
        if sync_statuses.len() > spec.allowed_sync_statuses.len()
            || sync_statuses
                .iter()
                .any(|status| !spec.allowed_sync_statuses.contains(status))
        {
            return Err(H8InterfaceTableQueryError::InvalidSyncStatus);
        }
        if self.warehouse_id.is_some() && !spec.has_warehouse_id {
            return Err(H8InterfaceTableQueryError::FilterNotSupported(
                "warehouse_id",
            ));
        }
        if self.external_doc_no.is_some() && self.table_key == "if_out_message" {
            return Err(H8InterfaceTableQueryError::FilterNotSupported(
                "external_doc_no",
            ));
        }
        if self.external_ref.is_some()
            && !matches!(self.table_key.as_str(), "if_in_asn" | "if_in_return_order")
        {
            return Err(H8InterfaceTableQueryError::FilterNotSupported(
                "external_ref",
            ));
        }
        if (self.source_outbox_id.is_some() || self.event_type.is_some())
            && self.table_key != "if_out_message"
        {
            return Err(H8InterfaceTableQueryError::FilterNotSupported(
                "source_outbox_id/event_type",
            ));
        }
        if self.wms_resource_id.is_some() && !spec.has_wms_resource_id {
            return Err(H8InterfaceTableQueryError::FilterNotSupported(
                "wms_resource_id",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum H8InterfaceTableQueryError {
    TableNotAllowed,
    InvalidTimeRange,
    TimeRangeTooLarge,
    InvalidPage,
    InvalidSyncStatus,
    FilterNotSupported(&'static str),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum H8InterfaceTableScopeError {
    OwnerMismatch,
    WarehouseOutOfScope,
    OwnerWideRequired,
}

pub fn enforce_interface_table_scope(
    row_owner_id: Uuid,
    row_warehouse_id: Option<Uuid>,
    actor_owner_id: Uuid,
    actor_warehouse_scope: Option<Uuid>,
    connector_warehouse_ids: &[Uuid],
) -> Result<(), H8InterfaceTableScopeError> {
    if row_owner_id != actor_owner_id {
        return Err(H8InterfaceTableScopeError::OwnerMismatch);
    }
    let Some(warehouse_id) = row_warehouse_id else {
        return if actor_warehouse_scope.is_none() {
            Ok(())
        } else {
            Err(H8InterfaceTableScopeError::OwnerWideRequired)
        };
    };
    if let Some(actor_warehouse_id) = actor_warehouse_scope {
        if actor_warehouse_id != warehouse_id {
            return Err(H8InterfaceTableScopeError::WarehouseOutOfScope);
        }
    }
    if !connector_warehouse_ids.is_empty() && !connector_warehouse_ids.contains(&warehouse_id) {
        return Err(H8InterfaceTableScopeError::WarehouseOutOfScope);
    }
    Ok(())
}

pub fn redacted_payload_summary(raw: &str) -> String {
    if raw.len() > H8_INTERFACE_TABLE_PAYLOAD_PARSE_MAX_BYTES {
        return "[报文已省略：内容过大]".into();
    }
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return "[报文已省略：格式无效]".into();
    };
    redact_value(&mut value);
    let serialized = serde_json::to_string(&value).unwrap_or_else(|_| "[报文已省略]".into());
    truncate_utf8(&serialized, H8_INTERFACE_TABLE_PAYLOAD_SUMMARY_MAX_BYTES)
}

fn redact_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if is_sensitive_key(key) {
                    *child = serde_json::Value::String("[REDACTED]".into());
                } else {
                    redact_value(child);
                }
            }
        }
        serde_json::Value::Array(values) => values.iter_mut().for_each(redact_value),
        _ => {}
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "password",
        "secret",
        "token",
        "credential",
        "authorization",
        "api_key",
        "apikey",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes.saturating_sub("…".len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &value[..end])
}
