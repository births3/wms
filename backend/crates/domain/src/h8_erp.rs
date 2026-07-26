//! US-H8-001：ERP 连接配置领域规则（纯 domain，无 IO）。
// @governance: skip-page-size H8 connector domain keeps transport/probe version invariants together; split after H8 migration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

pub const H8_CHANNEL_MODES: [&str; 3] = ["rest", "interface_table", "rest_primary_table_fallback"];
pub const H8_DIRECTIONS: [&str; 2] = ["inbound", "outbound"];
/// 连接路由消息类型：与 US-H8-002 受控目录一致（入站 ∪ 出站）。
pub const H8_MESSAGE_TYPES: [&str; 12] = crate::h8_erp_message::H8_CATALOG_MESSAGE_TYPES;
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

/// AC9：连接消息类型所需的最小入站 scope 集合（去重、保序）。
pub fn required_inbound_scopes(message_types: &[String]) -> Vec<&'static str> {
    let mut scopes = Vec::new();
    for message_type in message_types {
        if let Some(scope) = inbound_scope_for_message_type(message_type) {
            if !scopes.contains(&scope) {
                scopes.push(scope);
            }
        }
    }
    scopes
}

/// AC9：API Key 已授权 scope 必须覆盖连接消息类型所需最小集。
pub fn api_key_scopes_cover_messages(
    message_types: &[String],
    granted_scopes: &[String],
) -> Result<(), H8ErpConnectorError> {
    for need in required_inbound_scopes(message_types) {
        if !granted_scopes.iter().any(|granted| granted == need) {
            return Err(H8ErpConnectorError::InsufficientApiKeyScope);
        }
    }
    Ok(())
}

/// AC12：在途消息状态（与 `h8_erp_in_flight_messages.status` 对齐）。
pub const H8_INFLIGHT_RUNNING: &str = "running";
pub const H8_INFLIGHT_PAUSED: &str = "paused";

/// AC7：接口表通道最小对象清单（探测时校验声明，不写业务单据）。
pub const H8_INTERFACE_TABLE_REQUIRED_OBJECTS: [&str; 2] = ["if_out_message", "if_in_message"];

/// AC5/7：启用或测试时相对已 active 连接的路由重叠检查。
pub fn reject_route_overlap_with_actives(
    candidate: &H8ErpConnector,
    actives: &[H8ErpConnector],
) -> Result<(), H8ErpConnectorError> {
    for other in actives {
        if routes_overlap(candidate, other) {
            return Err(H8ErpConnectorError::RouteOverlap);
        }
    }
    Ok(())
}

/// AC8 运行时：连接是否匹配仓库+方向+消息类型（空仓库白名单=全仓）。
pub fn connector_matches_route(
    connector: &H8ErpConnector,
    warehouse_id: Option<Uuid>,
    direction: &str,
    message_type: &str,
) -> bool {
    if connector.status != "active" {
        return false;
    }
    if !connector.directions.iter().any(|d| d == direction) {
        return false;
    }
    if !connector.message_types.iter().any(|m| m == message_type) {
        return false;
    }
    match warehouse_id {
        None => true,
        Some(wid) => connector.warehouse_ids.is_empty() || connector.warehouse_ids.contains(&wid),
    }
}

/// AC8：在 active 集合中解析唯一路由；0 条 NotFound，>1 条 RouteOverlap。
pub fn resolve_active_connector<'a>(
    actives: &'a [H8ErpConnector],
    warehouse_id: Option<Uuid>,
    direction: &str,
    message_type: &str,
) -> Result<&'a H8ErpConnector, H8ErpConnectorError> {
    let matched: Vec<_> = actives
        .iter()
        .filter(|c| connector_matches_route(c, warehouse_id, direction, message_type))
        .collect();
    match matched.as_slice() {
        [] => Err(H8ErpConnectorError::NotFound),
        [one] => Ok(*one),
        _ => Err(H8ErpConnectorError::RouteOverlap),
    }
}

/// AC12：停用后在途应变为 paused；再启用后 running 才可续传。
pub fn inflight_status_after_disable(current: &str) -> Option<&'static str> {
    if current == H8_INFLIGHT_RUNNING {
        Some(H8_INFLIGHT_PAUSED)
    } else {
        None
    }
}

pub fn inflight_status_after_activate(current: &str) -> Option<&'static str> {
    if current == H8_INFLIGHT_PAUSED {
        Some(H8_INFLIGHT_RUNNING)
    } else {
        None
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
    /// Worker 传输账号；只用于出入站，不得用于 H8-004 探查。
    pub api_key_id: Option<Uuid>,
    pub bearer_secret_alias: Option<String>,
    pub interface_db_password_alias: Option<String>,
    /// H8-004 只读探查账号，与 Worker 账号和版本独立。
    pub interface_probe_db_username: Option<String>,
    /// 仅在服务端内部保存 alias；对外序列化为“是否已设置”，绝不回显 alias。
    #[serde(default, skip_serializing, skip_deserializing)]
    pub interface_probe_db_password_alias: Option<String>,
    /// 连接配置响应只返回是否已设置，不返回 alias 内容。
    pub interface_probe_db_password_alias_set: bool,
    pub interface_probe_config_version: i64,
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

/// US-H8-002 AC6：消息处理时绑定的不可变连接运行配置。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8ErpConnectorRuntimeConfig {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub connector_code: String,
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
    pub config_version: i64,
}

impl From<&H8ErpConnector> for H8ErpConnectorRuntimeConfig {
    fn from(connector: &H8ErpConnector) -> Self {
        Self {
            id: connector.id,
            owner_id: connector.owner_id,
            connector_code: connector.connector_code.clone(),
            warehouse_ids: connector.warehouse_ids.clone(),
            directions: connector.directions.clone(),
            message_types: connector.message_types.clone(),
            channel_mode: connector.channel_mode.clone(),
            api_base_url: connector.api_base_url.clone(),
            interface_db_host: connector.interface_db_host.clone(),
            interface_db_port: connector.interface_db_port,
            interface_db_name: connector.interface_db_name.clone(),
            interface_db_username: connector.interface_db_username.clone(),
            api_key_id: connector.api_key_id,
            bearer_secret_alias: connector.bearer_secret_alias.clone(),
            interface_db_password_alias: connector.interface_db_password_alias.clone(),
            config_version: connector.config_version,
        }
    }
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
    pub interface_probe_db_username: Option<String>,
    pub interface_probe_db_password_alias: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateH8ErpConnectorRequest {
    pub expected_config_version: i64,
    pub expected_probe_config_version: Option<i64>,
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
    pub interface_probe_db_username: Option<String>,
    pub interface_probe_db_password_alias: Option<String>,
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
    InsufficientApiKeyScope,
    IdempotencyConflict,
    ProbeVersionConflict,
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
        validate_probe_fields(
            self.interface_probe_db_username.as_deref(),
            self.interface_probe_db_password_alias.as_deref(),
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
        if crate::h8_erp_message::validate_message_type_in_catalog(t).is_err() {
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
    let probe_changed = req.interface_probe_db_username.is_some()
        || req.interface_probe_db_password_alias.is_some();
    if probe_changed
        && req.expected_probe_config_version != Some(current.interface_probe_config_version)
    {
        return Err(H8ErpConnectorError::ProbeVersionConflict);
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
    if req.interface_probe_db_username.is_some() {
        next.interface_probe_db_username = req
            .interface_probe_db_username
            .clone()
            .filter(|s| !s.trim().is_empty());
    }
    if req.interface_probe_db_password_alias.is_some() {
        next.interface_probe_db_password_alias = req
            .interface_probe_db_password_alias
            .clone()
            .filter(|s| !s.trim().is_empty());
        next.interface_probe_db_password_alias_set = next
            .interface_probe_db_password_alias
            .as_deref()
            .is_some_and(|alias| !alias.trim().is_empty());
    }

    validate_probe_fields(
        next.interface_probe_db_username.as_deref(),
        next.interface_probe_db_password_alias.as_deref(),
    )?;

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
    if probe_changed {
        next.interface_probe_config_version += 1;
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
    reject_route_overlap_with_actives(connector, actives)
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

fn validate_probe_fields(
    username: Option<&str>,
    password_alias: Option<&str>,
) -> Result<(), H8ErpConnectorError> {
    if username.is_none() && password_alias.is_none() {
        return Ok(());
    }
    require_nonempty(username, "interface_probe_db_username")?;
    validate_secret_alias(password_alias, "interface_probe_db_password_alias")
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
            interface_probe_db_username: None,
            interface_probe_db_password_alias: None,
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
            interface_probe_db_username: None,
            interface_probe_db_password_alias: None,
            interface_probe_db_password_alias_set: false,
            interface_probe_config_version: 1,
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
            expected_probe_config_version: None,
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
            interface_probe_db_username: None,
            interface_probe_db_password_alias: None,
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

    #[test]
    fn required_inbound_scopes_dedup_and_cover_check() {
        let types = vec!["asn".into(), "asn".into(), "product_master".into()];
        assert_eq!(
            required_inbound_scopes(&types),
            vec!["inbound:push", "master-data:write"]
        );
        assert_eq!(
            api_key_scopes_cover_messages(
                &types,
                &["inbound:push".into(), "master-data:write".into()]
            ),
            Ok(())
        );
        assert_eq!(
            api_key_scopes_cover_messages(&types, &["inbound:push".into()]),
            Err(H8ErpConnectorError::InsufficientApiKeyScope)
        );
    }

    #[test]
    fn version_conflict_on_stale_edit() {
        let c = sample_connector(Uuid::new_v4(), vec![], "testing");
        let req = UpdateH8ErpConnectorRequest {
            expected_config_version: 99,
            expected_probe_config_version: None,
            connector_name: Some("x".into()),
            warehouse_ids: None,
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
            interface_probe_db_username: None,
            interface_probe_db_password_alias: None,
        };
        assert_eq!(
            apply_update(&c, &req, Utc::now()),
            Err(H8ErpConnectorError::VersionConflict)
        );
    }

    #[test]
    fn probe_edit_requires_independent_expected_version() {
        let c = sample_connector(Uuid::new_v4(), vec![], "testing");
        let req = UpdateH8ErpConnectorRequest {
            expected_config_version: c.config_version,
            expected_probe_config_version: None,
            connector_name: None,
            warehouse_ids: None,
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
            interface_probe_db_username: Some("probe".into()),
            interface_probe_db_password_alias: None,
        };
        assert_eq!(
            apply_update(&c, &req, Utc::now()),
            Err(H8ErpConnectorError::ProbeVersionConflict)
        );
    }

    #[test]
    fn probe_edit_increments_only_probe_version() {
        let c = sample_connector(Uuid::new_v4(), vec![], "active");
        let req = UpdateH8ErpConnectorRequest {
            expected_config_version: c.config_version,
            expected_probe_config_version: Some(c.interface_probe_config_version),
            connector_name: None,
            warehouse_ids: None,
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
            interface_probe_db_username: Some("probe".into()),
            interface_probe_db_password_alias: Some("vault://h8/probe".into()),
        };
        let next = apply_update(&c, &req, Utc::now()).expect("ok");
        assert_eq!(next.config_version, c.config_version);
        assert_eq!(
            next.interface_probe_config_version,
            c.interface_probe_config_version + 1
        );
        assert_eq!(next.status, "active");
        assert_eq!(next.last_tested_succeeded, c.last_tested_succeeded);
    }

    #[test]
    fn connector_response_only_exposes_probe_alias_configured_flag() {
        let mut connector = sample_connector(Uuid::new_v4(), vec![], "testing");
        connector.interface_probe_db_password_alias = Some("vault://secret/probe".into());
        connector.interface_probe_db_password_alias_set = true;
        let value = serde_json::to_value(&connector).expect("serialize connector");
        assert!(value.get("interface_probe_db_password_alias").is_none());
        assert_eq!(value["interface_probe_db_password_alias_set"], true);
    }

    #[test]
    fn resolve_active_route_unique_and_ambiguous() {
        let mut a = sample_connector(Uuid::new_v4(), vec![], "active");
        a.directions = vec!["inbound".into()];
        a.message_types = vec!["asn".into()];
        let mut b = sample_connector(Uuid::new_v4(), vec![], "active");
        b.directions = vec!["inbound".into()];
        b.message_types = vec!["asn".into()];
        assert!(resolve_active_connector(std::slice::from_ref(&a), None, "inbound", "asn").is_ok());
        assert_eq!(
            resolve_active_connector(&[a.clone(), b], None, "inbound", "asn"),
            Err(H8ErpConnectorError::RouteOverlap)
        );
        assert_eq!(
            resolve_active_connector(&[], None, "inbound", "asn"),
            Err(H8ErpConnectorError::NotFound)
        );
    }

    #[test]
    fn inflight_disable_activate_transitions() {
        assert_eq!(
            inflight_status_after_disable(H8_INFLIGHT_RUNNING),
            Some(H8_INFLIGHT_PAUSED)
        );
        assert_eq!(
            inflight_status_after_activate(H8_INFLIGHT_PAUSED),
            Some(H8_INFLIGHT_RUNNING)
        );
        assert_eq!(inflight_status_after_disable(H8_INFLIGHT_PAUSED), None);
    }

    #[test]
    fn interface_table_required_objects_declared() {
        assert!(H8_INTERFACE_TABLE_REQUIRED_OBJECTS.contains(&"if_out_message"));
        assert!(H8_INTERFACE_TABLE_REQUIRED_OBJECTS.contains(&"if_in_message"));
    }
}
