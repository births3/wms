//! H2 审计写入。

use chrono::Utc;
use wms_domain::H8ErpConnector;

use crate::{
    audit::{AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

pub(crate) fn audit_request(
    ctx: &AuthContext,
    action: &str,
    connector: &H8ErpConnector,
    before: Option<serde_json::Value>,
) -> AuditWriteRequest {
    let after = audit_snapshot(connector);
    let mut req = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H8",
        "h8_erp_connector",
        connector.id.to_string(),
        Some(AuditDiff::compute(
            before.unwrap_or(serde_json::Value::Null),
            after,
        )),
    );
    req.occurred_at = Utc::now();
    req
}

pub(crate) fn audit_snapshot(c: &H8ErpConnector) -> serde_json::Value {
    // 脱敏：不记录 secret alias 明文内容以外的“是否配置”
    serde_json::json!({
        "id": c.id,
        "connector_code": c.connector_code,
        "connector_name": c.connector_name,
        "warehouse_ids": c.warehouse_ids,
        "directions": c.directions,
        "message_types": c.message_types,
        "channel_mode": c.channel_mode,
        "api_base_url": c.api_base_url,
        "interface_db_host": c.interface_db_host,
        "interface_db_port": c.interface_db_port,
        "interface_db_name": c.interface_db_name,
        "interface_db_username": c.interface_db_username,
        "api_key_id": c.api_key_id,
        "bearer_secret_alias_set": c.bearer_secret_alias.as_ref().is_some_and(|s| !s.is_empty()),
        "interface_db_password_alias_set": c.interface_db_password_alias.as_ref().is_some_and(|s| !s.is_empty()),
        "interface_probe_db_username": c.interface_probe_db_username,
        "interface_probe_db_password_alias_set": c.interface_probe_db_password_alias.as_ref().is_some_and(|s| !s.is_empty()),
        "interface_probe_config_version": c.interface_probe_config_version,
        "status": c.status,
        "config_version": c.config_version,
        "last_tested_succeeded": c.last_tested_succeeded,
        "last_tested_version": c.last_tested_version,
    })
}
