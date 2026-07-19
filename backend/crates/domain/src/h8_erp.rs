//! US-H8-001：ERP 连接配置领域规则（纯 domain，无 IO）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

pub const H8_CHANNEL_MODES: [&str; 3] = ["rest", "interface_table", "rest_primary_table_fallback"];
pub const H8_DIRECTIONS: [&str; 2] = ["inbound", "outbound"];
pub const H8_MESSAGE_TYPES: [&str; 6] = [
    "asn",
    "outbound_order",
    "product_master",
    "return_order",
    "product_change",
    "archive_revision",
];
pub const H8_CONNECTOR_STATUSES: [&str; 3] = ["testing", "active", "disabled"];

/// 入站 API Key scope（按消息类型最小授权）。
pub fn inbound_scope_for_message_type(message_type: &str) -> Option<&'static str> {
    match message_type {
        "asn" => Some("inbound:push"),
        "product_master" | "product_change" => Some("master-data:write"),
        "outbound_order" => Some("outbound:push"),
        "return_order" => Some("return:push"),
        _ => None,
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8ErpConnector {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub connector_code: String,
    pub connector_name: String,
    pub warehouse_ids: Vec<Uuid>,
    pub directions: Vec<String>,
    pub message_types: Vec<String>,
    pub channel_mode: String,
    pub api_base_url: Option<String>,
    pub interface_db_host: Option<String>,
    pub interface_db_port: Option<i32>,
    pub interface_db_name: Option<String>,
    pub interface_db_username: Option<String>,
    pub api_key_id: Option<Uuid>,
    pub bearer_secret_alias: Option<String>,
    pub interface_db_password_alias: Option<String>,
    pub status: String,
    pub config_version: i64,
    pub first_activated_at: Option<DateTime<Utc>>,
    pub last_tested_version: Option<i64>,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub last_tested_succeeded: Option<bool>,
    pub last_tested_error_summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpConnectorListResponse {
    pub data: Vec<H8ErpConnector>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateH8ErpConnectorRequest {
    pub connector_code: String,
    pub connector_name: String,
    pub warehouse_ids: Vec<Uuid>,
    pub directions: Vec<String>,
    pub message_types: Vec<String>,
    pub channel_mode: String,
    pub api_base_url: Option<String>,
    pub interface_db_host: Option<String>,
    pub interface_db_port: Option<i32>,
    pub interface_db_name: Option<String>,
    pub interface_db_username: Option<String>,
    pub api_key_id: Option<Uuid>,
    pub bearer_secret_alias: Option<String>,
    pub interface_db_password_alias: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateH8ErpConnectorRequest {
    pub expected_config_version: i64,
    pub connector_name: Option<String>,
    pub warehouse_ids: Option<Vec<Uuid>>,
    pub directions: Option<Vec<String>>,
    pub message_types: Option<Vec<String>>,
    pub channel_mode: Option<String>,
    pub api_base_url: Option<String>,
    pub interface_db_host: Option<String>,
    pub interface_db_port: Option<i32>,
    pub interface_db_name: Option<String>,
    pub interface_db_username: Option<String>,
    pub api_key_id: Option<Uuid>,
    pub bearer_secret_alias: Option<String>,
    pub interface_db_password_alias: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpConnectorTestResult {
    pub succeeded: bool,
    pub error_summary: Option<String>,
    pub tested_version: i64,
    pub tested_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum H8ErpConnectorError {
    FieldRequired(&'static str),
    FieldTooLong(&'static str),
    InvalidChannelMode,
    InvalidDirection,
    InvalidMessageType,
    InvalidStatus,
    InvalidApiUrl,
    InvalidSecretAlias,
    ConditionalFieldRequired(&'static str),
    RouteOverlap,
    VersionConflict,
    TestRequired,
    NotFound,
    DeleteNotAllowed,
    IllegalTransition,
}

/// 两连接在「货主+仓库+方向+消息类型」上是否路由重叠（仅用于 active 启用前检查）。
pub fn routes_overlap(a: &H8ErpConnector, b: &H8ErpConnector) -> bool {
    if a.owner_id != b.owner_id || a.id == b.id {
        return false;
    }
    if !sets_intersect(&a.directions, &b.directions) {
        return false;
    }
    if !sets_intersect(&a.message_types, &b.message_types) {
        return false;
    }
    warehouses_overlap(&a.warehouse_ids, &b.warehouse_ids)
}

fn warehouses_overlap(a: &[Uuid], b: &[Uuid]) -> bool {
    // 空 = 全部仓库
    if a.is_empty() || b.is_empty() {
        return true;
    }
    a.iter().any(|w| b.contains(w))
}

fn sets_intersect(a: &[String], b: &[String]) -> bool {
    a.iter().any(|x| b.iter().any(|y| y == x))
}

impl CreateH8ErpConnectorRequest {
    pub fn validate(&self) -> Result<(), H8ErpConnectorError> {
        validate_text(&self.connector_code, "connector_code", 64)?;
        validate_text(&self.connector_name, "connector_name", 128)?;
        validate_directions(&self.directions)?;
        validate_message_types(&self.message_types)?;
        validate_channel_mode(&self.channel_mode)?;
        validate_channel_fields(
            &self.channel_mode,
            self.api_base_url.as_deref(),
            self.interface_db_host.as_deref(),
            self.interface_db_port,
            self.interface_db_name.as_deref(),
            self.interface_db_username.as_deref(),
            self.api_key_id,
            self.bearer_secret_alias.as_deref(),
            self.interface_db_password_alias.as_deref(),
            &self.directions,
        )?;
        Ok(())
    }
}

pub fn validate_channel_mode(mode: &str) -> Result<(), H8ErpConnectorError> {
    if H8_CHANNEL_MODES.contains(&mode) {
        Ok(())
    } else {
        Err(H8ErpConnectorError::InvalidChannelMode)
    }
}

pub fn validate_directions(dirs: &[String]) -> Result<(), H8ErpConnectorError> {
    if dirs.is_empty() {
        return Err(H8ErpConnectorError::FieldRequired("directions"));
    }
    for d in dirs {
        if !H8_DIRECTIONS.contains(&d.as_str()) {
            return Err(H8ErpConnectorError::InvalidDirection);
        }
    }
    Ok(())
}

pub fn validate_message_types(types: &[String]) -> Result<(), H8ErpConnectorError> {
    if types.is_empty() {
        return Err(H8ErpConnectorError::FieldRequired("message_types"));
    }
    for t in types {
        if !H8_MESSAGE_TYPES.contains(&t.as_str()) {
            return Err(H8ErpConnectorError::InvalidMessageType);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn validate_channel_fields(
    channel_mode: &str,
    api_base_url: Option<&str>,
    interface_db_host: Option<&str>,
    interface_db_port: Option<i32>,
    interface_db_name: Option<&str>,
    interface_db_username: Option<&str>,
    api_key_id: Option<Uuid>,
    bearer_secret_alias: Option<&str>,
    interface_db_password_alias: Option<&str>,
    directions: &[String],
) -> Result<(), H8ErpConnectorError> {
    let needs_rest = channel_mode == "rest" || channel_mode == "rest_primary_table_fallback";
    let needs_table =
        channel_mode == "interface_table" || channel_mode == "rest_primary_table_fallback";

    if needs_rest {
        let url = api_base_url
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or(H8ErpConnectorError::ConditionalFieldRequired(
                "api_base_url",
            ))?;
        validate_api_url(url)?;
        if directions.iter().any(|d| d == "inbound") && api_key_id.is_none() {
            return Err(H8ErpConnectorError::ConditionalFieldRequired("api_key_id"));
        }
        if directions.iter().any(|d| d == "outbound") {
            validate_secret_alias(bearer_secret_alias, "bearer_secret_alias")?;
        }
    }
    if needs_table {
        require_nonempty(interface_db_host, "interface_db_host")?;
        if interface_db_port.is_none_or(|p| p <= 0 || p > 65535) {
            return Err(H8ErpConnectorError::ConditionalFieldRequired(
                "interface_db_port",
            ));
        }
        require_nonempty(interface_db_name, "interface_db_name")?;
        require_nonempty(interface_db_username, "interface_db_username")?;
        validate_secret_alias(interface_db_password_alias, "interface_db_password_alias")?;
    }
    Ok(())
}

/// 配置关键编辑是否应使测试结果失效并（active 时）回 testing。
pub fn is_runtime_affecting_update(req: &UpdateH8ErpConnectorRequest) -> bool {
    req.warehouse_ids.is_some()
        || req.directions.is_some()
        || req.message_types.is_some()
        || req.channel_mode.is_some()
        || req.api_base_url.is_some()
        || req.interface_db_host.is_some()
        || req.interface_db_port.is_some()
        || req.interface_db_name.is_some()
        || req.interface_db_username.is_some()
        || req.api_key_id.is_some()
        || req.bearer_secret_alias.is_some()
        || req.interface_db_password_alias.is_some()
}

pub fn apply_update(
    current: &H8ErpConnector,
    req: &UpdateH8ErpConnectorRequest,
    now: DateTime<Utc>,
) -> Result<H8ErpConnector, H8ErpConnectorError> {
    if req.expected_config_version != current.config_version {
        return Err(H8ErpConnectorError::VersionConflict);
    }
    let mut next = current.clone();
    if let Some(name) = &req.connector_name {
        validate_text(name, "connector_name", 128)?;
        next.connector_name = name.trim().to_string();
    }
    if let Some(w) = &req.warehouse_ids {
        next.warehouse_ids = w.clone();
    }
    if let Some(d) = &req.directions {
        validate_directions(d)?;
        next.directions = d.clone();
    }
    if let Some(m) = &req.message_types {
        validate_message_types(m)?;
        next.message_types = m.clone();
    }
    if let Some(mode) = &req.channel_mode {
        validate_channel_mode(mode)?;
        next.channel_mode = mode.clone();
    }
    if req.api_base_url.is_some() {
        next.api_base_url = req.api_base_url.clone().filter(|s| !s.trim().is_empty());
    }
    if req.interface_db_host.is_some() {
        next.interface_db_host = req
            .interface_db_host
            .clone()
            .filter(|s| !s.trim().is_empty());
    }
    if req.interface_db_port.is_some() {
        next.interface_db_port = req.interface_db_port;
    }
    if req.interface_db_name.is_some() {
        next.interface_db_name = req
            .interface_db_name
            .clone()
            .filter(|s| !s.trim().is_empty());
    }
    if req.interface_db_username.is_some() {
        next.interface_db_username = req
            .interface_db_username
            .clone()
            .filter(|s| !s.trim().is_empty());
    }
    if req.api_key_id.is_some() {
        next.api_key_id = req.api_key_id;
    }
    if req.bearer_secret_alias.is_some() {
        next.bearer_secret_alias = req
            .bearer_secret_alias
            .clone()
            .filter(|s| !s.trim().is_empty());
    }
    if req.interface_db_password_alias.is_some() {
        next.interface_db_password_alias = req
            .interface_db_password_alias
            .clone()
            .filter(|s| !s.trim().is_empty());
    }

    validate_channel_fields(
        &next.channel_mode,
        next.api_base_url.as_deref(),
        next.interface_db_host.as_deref(),
        next.interface_db_port,
        next.interface_db_name.as_deref(),
        next.interface_db_username.as_deref(),
        next.api_key_id,
        next.bearer_secret_alias.as_deref(),
        next.interface_db_password_alias.as_deref(),
        &next.directions,
    )?;

    if is_runtime_affecting_update(req) {
        next.config_version += 1;
        next.last_tested_version = None;
        next.last_tested_at = None;
        next.last_tested_succeeded = None;
        next.last_tested_error_summary = None;
        if next.status == "active" {
            next.status = "testing".to_string();
        }
    }
    next.updated_at = now;
    Ok(next)
}

pub fn can_activate(
    connector: &H8ErpConnector,
    actives: &[H8ErpConnector],
) -> Result<(), H8ErpConnectorError> {
    if connector.status != "testing" && connector.status != "disabled" {
        return Err(H8ErpConnectorError::IllegalTransition);
    }
    let test_ok = connector.last_tested_succeeded == Some(true)
        && connector.last_tested_version == Some(connector.config_version);
    if !test_ok {
        return Err(H8ErpConnectorError::TestRequired);
    }
    for other in actives {
        if routes_overlap(connector, other) {
            return Err(H8ErpConnectorError::RouteOverlap);
        }
    }
    Ok(())
}

pub fn can_physically_delete(connector: &H8ErpConnector, has_business_refs: bool) -> bool {
    connector.first_activated_at.is_none() && !has_business_refs
}

/// 本地联调 transport=both 不等于生产 channel_mode 双写。
pub fn production_allows_simultaneous_dual_write(channel_mode: &str) -> bool {
    // 故事：rest_primary_table_fallback 是主备，不是同时双投递
    let _ = channel_mode;
    false
}

/// 配置 channel_mode → worker 出站 transport（与 Python channel_failover 对齐）。
pub fn outbound_transport_for_channel_mode(channel_mode: &str) -> &'static str {
    match channel_mode {
        "rest" => "http",
        "interface_table" => "table",
        "rest_primary_table_fallback" => "failover",
        _ => "table",
    }
}

fn validate_text(
    value: &str,
    field: &'static str,
    max_chars: usize,
) -> Result<(), H8ErpConnectorError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(H8ErpConnectorError::FieldRequired(field));
    }
    if trimmed.chars().count() > max_chars {
        return Err(H8ErpConnectorError::FieldTooLong(field));
    }
    Ok(())
}

fn require_nonempty(value: Option<&str>, field: &'static str) -> Result<(), H8ErpConnectorError> {
    if value.map(str::trim).is_none_or(|s| s.is_empty()) {
        Err(H8ErpConnectorError::ConditionalFieldRequired(field))
    } else {
        Ok(())
    }
}

fn validate_secret_alias(
    value: Option<&str>,
    field: &'static str,
) -> Result<(), H8ErpConnectorError> {
    let Some(raw) = value.map(str::trim).filter(|s| !s.is_empty()) else {
        return Err(H8ErpConnectorError::ConditionalFieldRequired(field));
    };
    // 只接受 alias 形态，拒绝疑似明文
    if raw.len() < 3 || raw.contains(' ') || raw.contains('\n') {
        return Err(H8ErpConnectorError::InvalidSecretAlias);
    }
    if raw.starts_with("sk-") || raw.len() > 256 {
        return Err(H8ErpConnectorError::InvalidSecretAlias);
    }
    Ok(())
}

fn validate_api_url(url: &str) -> Result<(), H8ErpConnectorError> {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("https://")
        || lower.starts_with("http://127.0.0.1")
        || lower.starts_with("http://localhost"))
    {
        return Err(H8ErpConnectorError::InvalidApiUrl);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_create() -> CreateH8ErpConnectorRequest {
        CreateH8ErpConnectorRequest {
            connector_code: "erp-a".into(),
            connector_name: "ERP A".into(),
            warehouse_ids: vec![],
            directions: vec!["inbound".into()],
            message_types: vec!["asn".into()],
            channel_mode: "rest".into(),
            api_base_url: Some("https://erp.example.com".into()),
            interface_db_host: None,
            interface_db_port: None,
            interface_db_name: None,
            interface_db_username: None,
            api_key_id: Some(Uuid::new_v4()),
            bearer_secret_alias: None,
            interface_db_password_alias: None,
        }
    }

    fn sample_connector(id: Uuid, warehouses: Vec<Uuid>, status: &str) -> H8ErpConnector {
        let now = Utc::now();
        H8ErpConnector {
            id,
            owner_id: Uuid::nil(),
            connector_code: format!("c-{id}"),
            connector_name: "n".into(),
            warehouse_ids: warehouses,
            directions: vec!["inbound".into()],
            message_types: vec!["asn".into()],
            channel_mode: "rest".into(),
            api_base_url: Some("https://erp.example.com".into()),
            interface_db_host: None,
            interface_db_port: None,
            interface_db_name: None,
            interface_db_username: None,
            api_key_id: Some(Uuid::new_v4()),
            bearer_secret_alias: None,
            interface_db_password_alias: None,
            status: status.into(),
            config_version: 1,
            first_activated_at: None,
            last_tested_version: Some(1),
            last_tested_at: Some(now),
            last_tested_succeeded: Some(true),
            last_tested_error_summary: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn create_rest_inbound_requires_api_key() {
        let mut req = base_create();
        req.api_key_id = None;
        assert_eq!(
            req.validate(),
            Err(H8ErpConnectorError::ConditionalFieldRequired("api_key_id"))
        );
    }

    #[test]
    fn create_interface_table_requires_db_fields() {
        let mut req = base_create();
        req.channel_mode = "interface_table".into();
        req.api_base_url = None;
        req.api_key_id = None;
        assert!(matches!(
            req.validate(),
            Err(H8ErpConnectorError::ConditionalFieldRequired(_))
        ));
        req.interface_db_host = Some("mssql".into());
        req.interface_db_port = Some(1433);
        req.interface_db_name = Some("wms_erp_if".into());
        req.interface_db_username = Some("erp_if".into());
        req.interface_db_password_alias = Some("vault://h8/db-pass".into());
        assert_eq!(req.validate(), Ok(()));
    }

    #[test]
    fn empty_warehouse_overlaps_any_explicit() {
        let a = sample_connector(Uuid::new_v4(), vec![], "active");
        let w = Uuid::new_v4();
        let b = sample_connector(Uuid::new_v4(), vec![w], "active");
        assert!(routes_overlap(&a, &b));
    }

    #[test]
    fn disjoint_warehouses_do_not_overlap() {
        let a = sample_connector(Uuid::new_v4(), vec![Uuid::new_v4()], "active");
        let b = sample_connector(Uuid::new_v4(), vec![Uuid::new_v4()], "active");
        assert!(!routes_overlap(&a, &b));
    }

    #[test]
    fn activate_requires_current_version_test() {
        let mut c = sample_connector(Uuid::new_v4(), vec![], "testing");
        c.last_tested_version = Some(0);
        assert_eq!(
            can_activate(&c, &[]),
            Err(H8ErpConnectorError::TestRequired)
        );
        c.last_tested_version = Some(1);
        assert_eq!(can_activate(&c, &[]), Ok(()));
    }

    #[test]
    fn activate_rejects_route_overlap() {
        let a = sample_connector(Uuid::new_v4(), vec![], "active");
        let b = sample_connector(Uuid::new_v4(), vec![], "testing");
        assert_eq!(
            can_activate(&b, std::slice::from_ref(&a)),
            Err(H8ErpConnectorError::RouteOverlap)
        );
    }

    #[test]
    fn active_runtime_edit_returns_to_testing() {
        let mut c = sample_connector(Uuid::new_v4(), vec![], "active");
        c.config_version = 2;
        c.last_tested_version = Some(2);
        let req = UpdateH8ErpConnectorRequest {
            expected_config_version: 2,
            connector_name: None,
            warehouse_ids: Some(vec![Uuid::new_v4()]),
            directions: None,
            message_types: None,
            channel_mode: None,
            api_base_url: None,
            interface_db_host: None,
            interface_db_port: None,
            interface_db_name: None,
            interface_db_username: None,
            api_key_id: None,
            bearer_secret_alias: None,
            interface_db_password_alias: None,
        };
        let next = apply_update(&c, &req, Utc::now()).expect("ok");
        assert_eq!(next.status, "testing");
        assert_eq!(next.config_version, 3);
        assert!(next.last_tested_succeeded.is_none());
    }

    #[test]
    fn delete_only_never_activated_without_refs() {
        let c = sample_connector(Uuid::new_v4(), vec![], "testing");
        assert!(can_physically_delete(&c, false));
        assert!(!can_physically_delete(&c, true));
        let mut activated = c;
        activated.first_activated_at = Some(Utc::now());
        assert!(!can_physically_delete(&activated, false));
    }

    #[test]
    fn production_never_simultaneous_dual_write() {
        assert!(!production_allows_simultaneous_dual_write(
            "rest_primary_table_fallback"
        ));
        assert!(!production_allows_simultaneous_dual_write("both"));
    }

    #[test]
    fn channel_mode_maps_to_outbound_transport() {
        assert_eq!(outbound_transport_for_channel_mode("rest"), "http");
        assert_eq!(
            outbound_transport_for_channel_mode("interface_table"),
            "table"
        );
        assert_eq!(
            outbound_transport_for_channel_mode("rest_primary_table_fallback"),
            "failover"
        );
    }

    #[test]
    fn inbound_scopes_are_minimal() {
        assert_eq!(inbound_scope_for_message_type("asn"), Some("inbound:push"));
        assert_eq!(
            inbound_scope_for_message_type("outbound_order"),
            Some("outbound:push")
        );
    }
}
