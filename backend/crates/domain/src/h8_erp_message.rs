//! US-H8-002 / US-H8-003：ERP 消息目录、信封、错误分类、日志状态机与重放规则（纯 domain）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

// ── US-H8-002 受控消息目录 ──────────────────────────────────────────────

/// 首批入站：ASN、出库订单、退货申请、商品主数据、商品主数据变更。
pub const H8_INBOUND_MESSAGE_TYPES: [&str; 5] = [
    "asn",
    "outbound_order",
    "return_order",
    "product_master",
    "product_change",
];

/// 首批出站：入库完成、库存状态、报损报溢、档案补录、对账差异、发货确认、库存快照。
pub const H8_OUTBOUND_MESSAGE_TYPES: [&str; 7] = [
    "putaway_complete",
    "inventory_status",
    "stock_adjustment",
    "archive_revision",
    "reconciliation_diff",
    "shipment_confirm",
    "inventory_snapshot",
];

/// 连接配置与路由共用的完整受控目录（入站 ∪ 出站）。
pub const H8_CATALOG_MESSAGE_TYPES: [&str; 12] = [
    "asn",
    "outbound_order",
    "return_order",
    "product_master",
    "product_change",
    "putaway_complete",
    "inventory_status",
    "stock_adjustment",
    "archive_revision",
    "reconciliation_diff",
    "shipment_confirm",
    "inventory_snapshot",
];

pub const H8_MESSAGE_DIRECTIONS: [&str; 2] = ["inbound", "outbound"];
pub const H8_MESSAGE_CHANNELS: [&str; 2] = ["rest", "interface_table"];
pub const H8_SUPPORTED_SCHEMA_VERSIONS: [&str; 1] = ["1"];

/// 消息处理状态（US-H8-003 AC3）。
pub const H8_MESSAGE_STATUSES: [&str; 7] = [
    "pending",
    "processing",
    "succeeded",
    "awaiting_receipt",
    "failed",
    "dead",
    "acked",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum H8MessageError {
    UnknownMessageType,
    InvalidDirection,
    InvalidChannel,
    InvalidStatus,
    IllegalTransition,
    NotRetryable,
    LeaseConflict,
    ClaimPaused,
    ReplayNotAllowed,
    FieldRequired(&'static str),
    OwnerMismatch,
    WarehouseOutOfScope,
    MappingMissing,
    UnsupportedSchemaVersion,
    InvalidRetentionDays,
    EncryptionKeyUnavailable,
    PayloadUnavailable,
    PayloadExpired,
}

/// 入站消息类型所需最小 API Key scope（与连接配置 AC9 对齐并扩展出库/退货）。
pub fn inbound_scope_for_catalog_type(message_type: &str) -> Option<&'static str> {
    match message_type {
        "asn" => Some("inbound:push"),
        "product_master" | "product_change" => Some("master-data:write"),
        "outbound_order" => Some("outbound:push"),
        "return_order" => Some("return:push"),
        _ => None,
    }
}

pub fn is_inbound_message_type(message_type: &str) -> bool {
    H8_INBOUND_MESSAGE_TYPES.contains(&message_type)
}

pub fn is_outbound_message_type(message_type: &str) -> bool {
    H8_OUTBOUND_MESSAGE_TYPES.contains(&message_type)
}

pub fn is_catalog_message_type(message_type: &str) -> bool {
    H8_CATALOG_MESSAGE_TYPES.contains(&message_type)
}

pub fn validate_message_type_in_catalog(message_type: &str) -> Result<(), H8MessageError> {
    if is_catalog_message_type(message_type) {
        Ok(())
    } else {
        Err(H8MessageError::UnknownMessageType)
    }
}

pub fn validate_direction(direction: &str) -> Result<(), H8MessageError> {
    if H8_MESSAGE_DIRECTIONS.contains(&direction) {
        Ok(())
    } else {
        Err(H8MessageError::InvalidDirection)
    }
}

pub fn validate_channel(channel: &str) -> Result<(), H8MessageError> {
    if H8_MESSAGE_CHANNELS.contains(&channel) {
        Ok(())
    } else {
        Err(H8MessageError::InvalidChannel)
    }
}

pub fn validate_sync_status(status: &str) -> Result<(), H8MessageError> {
    if H8_MESSAGE_STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(H8MessageError::InvalidStatus)
    }
}

pub fn validate_schema_version(version: &str) -> Result<(), H8MessageError> {
    if H8_SUPPORTED_SCHEMA_VERSIONS.contains(&version) {
        Ok(())
    } else {
        Err(H8MessageError::UnsupportedSchemaVersion)
    }
}

/// 业务幂等判定键：货主 + 消息类型 + 外部业务标识 + Idempotency-Key。
pub fn message_idempotency_identity(
    owner_id: Uuid,
    message_type: &str,
    external_ref: &str,
    idempotency_key: &str,
) -> String {
    format!("{owner_id}|{message_type}|{external_ref}|{idempotency_key}")
}

/// 消息开始处理后绑定的连接/配置版本不可静默切换。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8MessageConfigBinding {
    pub connector_id: Uuid,
    pub config_version: i64,
    pub channel: String,
    pub message_type: String,
}

/// 最小消息信封（US-H8-002）。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8MessageEnvelope {
    pub owner_id: Uuid,
    pub warehouse_id: Option<Uuid>,
    pub direction: String,
    pub message_type: String,
    pub schema_version: String,
    pub external_ref: String,
    pub idempotency_key: String,
    pub connector_id: Option<Uuid>,
    pub config_version: Option<i64>,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    pub payload_digest: String,
    pub wms_resource_id: Option<String>,
}

impl H8MessageEnvelope {
    pub fn validate(&self) -> Result<(), H8MessageError> {
        validate_message_direction(&self.direction, &self.message_type)?;
        validate_schema_version(&self.schema_version)?;
        if self.external_ref.trim().is_empty() {
            return Err(H8MessageError::FieldRequired("external_ref"));
        }
        if self.idempotency_key.trim().is_empty() {
            return Err(H8MessageError::FieldRequired("idempotency_key"));
        }
        if self.correlation_id.trim().is_empty() {
            return Err(H8MessageError::FieldRequired("correlation_id"));
        }
        if self.payload_digest.trim().is_empty() {
            return Err(H8MessageError::FieldRequired("payload_digest"));
        }
        Ok(())
    }

    pub fn with_config_binding(mut self, binding: H8MessageConfigBinding) -> Self {
        self.connector_id = Some(binding.connector_id);
        self.config_version = Some(binding.config_version);
        self.message_type = binding.message_type;
        self
    }
}

pub fn validate_message_direction(
    direction: &str,
    message_type: &str,
) -> Result<(), H8MessageError> {
    validate_direction(direction)?;
    validate_message_type_in_catalog(message_type)?;
    if (direction == "inbound" && !is_inbound_message_type(message_type))
        || (direction == "outbound" && !is_outbound_message_type(message_type))
    {
        return Err(H8MessageError::UnknownMessageType);
    }
    Ok(())
}

/// 错误是否可自动重试（US-H8-002 AC9 / US-H8-003 AC5）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum H8ErrorClass {
    Retryable,
    NonRetryable,
}

pub fn classify_h8_error(kind: &str) -> H8ErrorClass {
    match kind {
        "network"
        | "timeout"
        | "rate_limit"
        | "temporary_unavailable"
        | "circuit_open"
        | "lease_expired" => H8ErrorClass::Retryable,
        "auth"
        | "permission"
        | "schema"
        | "mapping"
        | "business_validation"
        | "route_ambiguous"
        | "owner_mismatch" => H8ErrorClass::NonRetryable,
        _ => H8ErrorClass::NonRetryable,
    }
}

/// 仓库必须落在连接白名单与主体授权范围交集内。
pub fn warehouse_in_scope(
    warehouse_id: Option<Uuid>,
    connector_whitelist: &[Uuid],
    principal_warehouses: Option<&[Uuid]>,
) -> Result<(), H8MessageError> {
    let Some(wid) = warehouse_id else {
        // 无仓消息允许：白名单为空时
        return if connector_whitelist.is_empty() {
            Ok(())
        } else {
            Err(H8MessageError::WarehouseOutOfScope)
        };
    };
    if !connector_whitelist.is_empty() && !connector_whitelist.contains(&wid) {
        return Err(H8MessageError::WarehouseOutOfScope);
    }
    if let Some(allowed) = principal_warehouses {
        if !allowed.is_empty() && !allowed.contains(&wid) {
            return Err(H8MessageError::WarehouseOutOfScope);
        }
    }
    Ok(())
}

// ── US-H8-003 状态机 / 租约 / 重放 ──────────────────────────────────────

/// 合法状态迁移。
pub fn can_transition_message_status(from: &str, to: &str) -> Result<(), H8MessageError> {
    validate_sync_status(from)?;
    validate_sync_status(to)?;
    let ok = matches!(
        (from, to),
        ("pending", "processing")
            | ("processing", "succeeded")
            | ("processing", "awaiting_receipt")
            | ("processing", "failed")
            | ("processing", "dead")
            | ("awaiting_receipt", "acked")
            | ("awaiting_receipt", "processing")
            | ("awaiting_receipt", "dead")
            | ("failed", "processing")
            | ("failed", "dead")
            | ("dead", "processing") // 人工重放
    );
    if ok {
        Ok(())
    } else {
        Err(H8MessageError::IllegalTransition)
    }
}

/// Worker 认领：常规消息要求租约失效；人工重放标记允许 Worker 立即接管。
pub fn can_claim_message(
    status: &str,
    claimed_by: Option<&str>,
    lease_expires_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Result<(), H8MessageError> {
    match status {
        "pending" | "failed" => {
            if lease_expires_at.is_some_and(|exp| exp > now) {
                return Err(H8MessageError::LeaseConflict);
            }
            Ok(())
        }
        "processing" => {
            if claimed_by.is_some_and(|actor| actor.starts_with("replay:")) {
                return Ok(());
            }
            if lease_expires_at.is_some_and(|exp| exp > now) {
                Err(H8MessageError::LeaseConflict)
            } else {
                // 租约超时可恢复
                Ok(())
            }
        }
        "succeeded" | "awaiting_receipt" | "acked" | "dead" => Err(H8MessageError::LeaseConflict),
        _ => Err(H8MessageError::InvalidStatus),
    }
}

/// 仅 failed/dead 允许人工重放（AC7）。
pub fn can_replay_message(status: &str) -> Result<(), H8MessageError> {
    if status == "failed" || status == "dead" {
        Ok(())
    } else {
        Err(H8MessageError::ReplayNotAllowed)
    }
}

/// 重试耗尽或不可重试 → dead。
pub fn should_enter_dead(error_class: H8ErrorClass, retry_count: i32, max_retries: i32) -> bool {
    matches!(error_class, H8ErrorClass::NonRetryable) || retry_count >= max_retries
}

/// ADR-0018 L2 标准重试：1/2/4/8/16 秒 ±20%，后续尝试保持 16 秒基线上限。
pub fn standard_retry_delay_millis(attempt_number: i32, idempotency_key: &str) -> i64 {
    const BASE_MILLIS: [i64; 5] = [1_000, 2_000, 4_000, 8_000, 16_000];
    let first = idempotency_key.chars().next().unwrap_or_default() as u64;
    let last = idempotency_key.chars().last().unwrap_or_default() as u64;
    let seed = (first * 31 + last * 17 + idempotency_key.chars().count() as u64) % 4_001;
    let jitter_permyriad = 8_000 + seed as i64;
    BASE_MILLIS[(attempt_number.clamp(1, 5) - 1) as usize] * jitter_permyriad / 10_000
}

/// 未配置保留策略时禁止自动删除（AC10）。
pub fn may_auto_purge(retention_days: Option<i32>) -> bool {
    retention_days.is_some_and(|d| d > 0)
}

/// Worker 心跳由服务端保存绝对失效时间，避免页面各自猜测健康阈值。
pub fn derive_worker_health(
    heartbeat_expires_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> &'static str {
    if heartbeat_expires_at > now {
        "healthy"
    } else {
        "stale"
    }
}

/// 暂停到期后自动允许认领；未设置到期时间则保持暂停直到人工恢复。
pub fn is_worker_claim_paused(
    paused: bool,
    paused_until: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    paused && paused_until.is_none_or(|until| until > now)
}

/// 完整报文保留默认 7 天，允许范围 1..=30 天。
pub fn resolve_payload_retention_days(days: Option<i32>) -> Result<i32, H8MessageError> {
    let days = days.unwrap_or(7);
    if (1..=30).contains(&days) {
        Ok(days)
    } else {
        Err(H8MessageError::InvalidRetentionDays)
    }
}

/// 消息主记录 API 视图（脱敏，不含完整报文）。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8ErpMessage {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub warehouse_id: Option<Uuid>,
    pub connector_id: Option<Uuid>,
    pub connector_code: Option<String>,
    pub config_version: Option<i64>,
    pub direction: String,
    pub message_type: String,
    pub schema_version: String,
    pub channel: String,
    pub external_ref: String,
    pub wms_resource_id: Option<String>,
    pub idempotency_key: String,
    pub correlation_id: String,
    pub sync_status: String,
    pub retry_count: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub last_error_summary: Option<String>,
    pub payload_digest: String,
    pub claimed_by: Option<String>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub acked_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8ErpMessageAttempt {
    pub id: Uuid,
    pub message_id: Uuid,
    pub attempt_no: i32,
    pub channel: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub result: String,
    pub error_summary: Option<String>,
    pub actor: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpMessageListResponse {
    pub data: Vec<H8ErpMessage>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpMessageDetail {
    pub message: H8ErpMessage,
    pub attempts: Vec<H8ErpMessageAttempt>,
    pub payload_retained: bool,
    pub payload_expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReplayH8ErpMessageRequest {
    pub reason: String,
    /// 客户端确认二次确认（true 才执行）。
    pub confirmed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpMessageStats {
    pub owner_id: Uuid,
    pub total: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub dead: i64,
    pub processing: i64,
    pub pending: i64,
    pub retry_total: i64,
    /// 处理时延 P95（毫秒），来自尝试 finished-started；无样本时为 0。
    pub p95_latency_ms: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ClaimH8ErpMessageRequest {
    pub worker_id: String,
    /// 租约秒数，默认 300。
    pub lease_seconds: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PurgeH8ErpMessagesRequest {
    /// 必须为 true；且货主已配置 retention_days 才允许清理终态消息。
    pub confirmed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PurgeH8ErpMessagesResponse {
    pub deleted: i64,
    pub retention_days: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8WorkerHeartbeatRequest {
    pub worker_id: String,
    pub worker_version: String,
    pub connector_id: Uuid,
    pub directions: Vec<String>,
    pub current_claims: i32,
    pub heartbeat_ttl_seconds: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8WorkerStatus {
    pub worker_id: String,
    pub worker_version: String,
    pub connector_id: Uuid,
    pub directions: Vec<String>,
    pub current_claims: i32,
    pub created_at: DateTime<Utc>,
    pub last_heartbeat_at: DateTime<Utc>,
    pub heartbeat_expires_at: DateTime<Utc>,
    pub health: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8WorkerClaimControl {
    pub connector_id: Uuid,
    pub direction: String,
    pub paused: bool,
    pub reason: String,
    pub paused_until: Option<DateTime<Utc>>,
    pub updated_by: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct SetH8WorkerClaimControlRequest {
    pub connector_id: Uuid,
    pub direction: String,
    pub paused: bool,
    pub reason: String,
    pub paused_until: Option<DateTime<Utc>>,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8WorkerRuntimeResponse {
    pub workers: Vec<H8WorkerStatus>,
    pub controls: Vec<H8WorkerClaimControl>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8WorkerClaimDecision {
    pub allowed: bool,
    pub reason: Option<String>,
    pub paused_until: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8PayloadRetentionPolicy {
    pub connector_id: Uuid,
    pub enabled: bool,
    pub retention_days: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct UpdateH8PayloadRetentionPolicyRequest {
    pub connector_id: Uuid,
    pub enabled: bool,
    pub retention_days: Option<i32>,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8DecryptedPayload {
    pub message_id: Uuid,
    pub payload: String,
    pub expires_at: DateTime<Utc>,
}

/// Worker 入站/出站交换阶段上报；仅 receive 可携带完整报文用于摘要/受控加密。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertH8ErpMessageLifecycleRequest {
    pub stage: String,
    pub result: String,
    pub direction: String,
    pub message_type: String,
    pub schema_version: String,
    pub external_ref: String,
    pub idempotency_key: String,
    pub correlation_id: String,
    pub channel: String,
    pub connector_id: Option<Uuid>,
    pub connector_code: Option<String>,
    pub config_version: Option<i64>,
    pub message_id: Option<Uuid>,
    pub warehouse_id: Option<Uuid>,
    pub wms_resource_id: Option<String>,
    pub payload: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ErpBusinessReceiptRequest {
    pub result: String,
    pub error_summary: Option<String>,
    pub schema_version: String,
    pub correlation_id: String,
}

/// 根据尝试样本估算 P95（最近似：排序后 95 分位）。
pub fn estimate_p95_latency_ms(samples_ms: &[i64]) -> i64 {
    if samples_ms.is_empty() {
        return 0;
    }
    let mut sorted = samples_ms.to_vec();
    sorted.sort_unstable();
    let idx = ((sorted.len() as f64) * 0.95).ceil() as usize;
    let i = idx.saturating_sub(1).min(sorted.len() - 1);
    sorted[i]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_covers_story_inbound_and_outbound() {
        assert_eq!(H8_INBOUND_MESSAGE_TYPES.len(), 5);
        assert_eq!(H8_OUTBOUND_MESSAGE_TYPES.len(), 7);
        assert_eq!(H8_CATALOG_MESSAGE_TYPES.len(), 12);
        assert!(is_inbound_message_type("asn"));
        assert!(is_outbound_message_type("shipment_confirm"));
        assert!(!is_inbound_message_type("shipment_confirm"));
        assert!(validate_message_type_in_catalog("free_text").is_err());
    }

    #[test]
    fn envelope_rejects_direction_type_mismatch() {
        let mut env = sample_envelope("inbound", "asn");
        env.validate().unwrap();
        env.message_type = "shipment_confirm".into();
        assert_eq!(env.validate(), Err(H8MessageError::UnknownMessageType));
    }

    #[test]
    fn envelope_rejects_unsupported_schema_version() {
        let mut env = sample_envelope("inbound", "asn");
        env.schema_version = "2".into();
        assert_eq!(
            env.validate(),
            Err(H8MessageError::UnsupportedSchemaVersion)
        );
    }

    #[test]
    fn error_classification_retryable_vs_not() {
        assert_eq!(classify_h8_error("timeout"), H8ErrorClass::Retryable);
        assert_eq!(classify_h8_error("mapping"), H8ErrorClass::NonRetryable);
        assert_eq!(classify_h8_error("auth"), H8ErrorClass::NonRetryable);
    }

    #[test]
    fn standard_retry_delay_uses_adr_0018_schedule_and_stable_jitter() {
        let key = "idem-1";
        let delays = (1..=6)
            .map(|attempt| standard_retry_delay_millis(attempt, key))
            .collect::<Vec<_>>();
        assert_eq!(delays, vec![809, 1_618, 3_237, 6_474, 12_948, 12_948]);
        for (delay, base) in delays.into_iter().zip([1, 2, 4, 8, 16, 16]) {
            assert!((base * 800..=base * 1_200).contains(&delay));
        }
    }

    #[test]
    fn config_binding_records_connector_version() {
        let env = sample_envelope("outbound", "putaway_complete").with_config_binding(
            H8MessageConfigBinding {
                connector_id: Uuid::nil(),
                config_version: 3,
                channel: "rest".into(),
                message_type: "putaway_complete".into(),
            },
        );
        assert_eq!(env.config_version, Some(3));
        assert_eq!(env.connector_id, Some(Uuid::nil()));
    }

    #[test]
    fn warehouse_scope_intersection() {
        let w1 = Uuid::new_v4();
        let w2 = Uuid::new_v4();
        warehouse_in_scope(Some(w1), &[w1], Some(&[w1, w2])).unwrap();
        assert!(warehouse_in_scope(Some(w2), &[w1], Some(&[w1, w2])).is_err());
        assert!(warehouse_in_scope(Some(w1), &[w1], Some(&[w2])).is_err());
    }

    #[test]
    fn status_machine_and_replay_rules() {
        can_transition_message_status("pending", "processing").unwrap();
        can_transition_message_status("processing", "awaiting_receipt").unwrap();
        can_transition_message_status("awaiting_receipt", "acked").unwrap();
        can_transition_message_status("awaiting_receipt", "processing").unwrap();
        can_transition_message_status("awaiting_receipt", "dead").unwrap();
        can_transition_message_status("processing", "failed").unwrap();
        can_transition_message_status("failed", "dead").unwrap();
        can_transition_message_status("dead", "processing").unwrap();
        assert!(can_transition_message_status("succeeded", "acked").is_err());
        assert!(can_transition_message_status("succeeded", "pending").is_err());
        assert!(can_transition_message_status("acked", "processing").is_err());
        can_replay_message("failed").unwrap();
        can_replay_message("dead").unwrap();
        assert!(can_replay_message("succeeded").is_err());
    }

    #[test]
    fn worker_health_and_pause_follow_server_timestamps() {
        let now = Utc::now();
        assert_eq!(
            derive_worker_health(now + chrono::Duration::seconds(1), now),
            "healthy"
        );
        assert_eq!(derive_worker_health(now, now), "stale");
        assert!(is_worker_claim_paused(
            true,
            Some(now + chrono::Duration::seconds(1)),
            now
        ));
        assert!(!is_worker_claim_paused(true, Some(now), now));
        assert!(is_worker_claim_paused(true, None, now));
        assert!(!is_worker_claim_paused(false, None, now));
    }

    #[test]
    fn payload_retention_defaults_to_seven_and_caps_at_thirty_days() {
        assert_eq!(resolve_payload_retention_days(None).unwrap(), 7);
        assert_eq!(resolve_payload_retention_days(Some(30)).unwrap(), 30);
        assert!(resolve_payload_retention_days(Some(0)).is_err());
        assert!(resolve_payload_retention_days(Some(31)).is_err());
    }

    #[test]
    fn claim_respects_lease_and_terminal() {
        let now = Utc::now();
        can_claim_message("pending", None, None, now).unwrap();
        assert!(can_claim_message(
            "pending",
            None,
            Some(now + chrono::Duration::seconds(30)),
            now
        )
        .is_err());
        can_claim_message(
            "processing",
            None,
            Some(now - chrono::Duration::seconds(1)),
            now,
        )
        .unwrap();
        can_claim_message(
            "processing",
            Some("replay:admin"),
            Some(now + chrono::Duration::minutes(5)),
            now,
        )
        .unwrap();
        assert!(can_claim_message("succeeded", None, None, now).is_err());
        assert!(can_claim_message("acked", None, None, now).is_err());
    }

    #[test]
    fn dead_entry_and_purge_policy() {
        assert!(should_enter_dead(H8ErrorClass::NonRetryable, 0, 5));
        assert!(should_enter_dead(H8ErrorClass::Retryable, 5, 5));
        assert!(!should_enter_dead(H8ErrorClass::Retryable, 2, 5));
        assert!(!may_auto_purge(None));
        assert!(!may_auto_purge(Some(0)));
        assert!(may_auto_purge(Some(30)));
    }

    #[test]
    fn p95_estimation() {
        assert_eq!(estimate_p95_latency_ms(&[]), 0);
        assert_eq!(estimate_p95_latency_ms(&[10, 20, 30, 40, 100]), 100);
    }

    #[test]
    fn inbound_scopes_for_catalog() {
        assert_eq!(inbound_scope_for_catalog_type("asn"), Some("inbound:push"));
        assert_eq!(
            inbound_scope_for_catalog_type("outbound_order"),
            Some("outbound:push")
        );
        assert_eq!(
            inbound_scope_for_catalog_type("return_order"),
            Some("return:push")
        );
        assert_eq!(inbound_scope_for_catalog_type("putaway_complete"), None);
    }

    fn sample_envelope(direction: &str, message_type: &str) -> H8MessageEnvelope {
        H8MessageEnvelope {
            owner_id: Uuid::nil(),
            warehouse_id: None,
            direction: direction.into(),
            message_type: message_type.into(),
            schema_version: "1".into(),
            external_ref: "ERP-1".into(),
            idempotency_key: "idem-1".into(),
            connector_id: None,
            config_version: None,
            correlation_id: "corr-1".into(),
            occurred_at: Utc::now(),
            payload_digest: "abc".into(),
            wms_resource_id: None,
        }
    }
}
