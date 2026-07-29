//! US-H8-002：ERP↔WMS 语义转换与处理管线（纯 domain，无 IO）。
//!
//! 三级边界：
//! - 业务模块只认 WMS canonical 命令/事件
//! - H8 负责路由、幂等、M-PM 规整与 canonical 转换
//! - 通道适配器只处理协议 / ERP DTO，不得把 DTO 泄漏到 M1–M4 domain

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::h8_erp_message::{
    classify_h8_error, is_inbound_message_type, is_outbound_message_type,
    message_idempotency_identity, validate_message_type_in_catalog, warehouse_in_scope,
    H8ErrorClass, H8MessageConfigBinding, H8MessageEnvelope, H8MessageError,
};

/// 入站管线有序步骤（AC3）。
pub const H8_INBOUND_PIPELINE_STEPS: [&str; 8] = [
    "auth_scope",
    "owner_warehouse_scope",
    "route_unique",
    "idempotency",
    "schema_validate",
    "mpm_normalize",
    "business_api",
    "ack_success",
];

/// M-PM 字段映射规则（H8 侧只消费映射结果，不实现规则存储）。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct H8FieldMappingRule {
    pub external_field: String,
    pub canonical_field: String,
    /// 可选：identity | uppercase | trim
    pub transform: String,
}

/// 入站 canonical 命令：业务模块可安全消费，不含 ERP DTO。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8CanonicalInboundCommand {
    pub owner_id: Uuid,
    pub warehouse_id: Option<Uuid>,
    pub message_type: String,
    pub external_ref: String,
    pub idempotency_key: String,
    pub correlation_id: String,
    pub connector_id: Uuid,
    pub config_version: i64,
    pub channel: String,
    /// 仅 canonical 字段
    #[schema(value_type = Object)]
    pub fields: Map<String, Value>,
    pub occurred_at: DateTime<Utc>,
}

/// 出站 canonical 事件：业务模块写入 outbox 的载荷形态。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub struct H8CanonicalOutboundEvent {
    pub owner_id: Uuid,
    pub warehouse_id: Option<Uuid>,
    pub message_type: String,
    pub external_ref: String,
    pub idempotency_key: String,
    pub correlation_id: String,
    pub wms_resource_id: String,
    #[schema(value_type = Object)]
    pub fields: Map<String, Value>,
    pub occurred_at: DateTime<Utc>,
}

/// ERP DTO 仅允许停留在适配器边界；转换失败不得把原始 external 写入业务字段。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct H8MpmNormalizeResult {
    pub canonical_fields: Map<String, Value>,
    pub unresolved_external_fields: Vec<String>,
}

pub fn apply_mpm_normalize(
    raw_external: &Map<String, Value>,
    rules: &[H8FieldMappingRule],
) -> Result<H8MpmNormalizeResult, H8MessageError> {
    let mut canonical = Map::new();
    let mut unresolved = Vec::new();
    for (ext_key, value) in raw_external {
        let rule = rules.iter().find(|r| r.external_field == *ext_key);
        match rule {
            Some(r) => {
                canonical.insert(
                    r.canonical_field.clone(),
                    apply_field_transform(value, &r.transform),
                );
            }
            None => unresolved.push(ext_key.clone()),
        }
    }
    unresolved.sort();
    if !unresolved.is_empty() {
        return Err(H8MessageError::MappingMissing);
    }
    Ok(H8MpmNormalizeResult {
        canonical_fields: canonical,
        unresolved_external_fields: unresolved,
    })
}

fn apply_field_transform(value: &Value, transform: &str) -> Value {
    match (transform, value.as_str()) {
        ("uppercase", Some(s)) => Value::String(s.to_ascii_uppercase()),
        ("trim", Some(s)) => Value::String(s.trim().to_string()),
        _ => value.clone(),
    }
}

/// 入站：信封 + 路由绑定 + M-PM 规整 → canonical 命令。
pub fn build_inbound_canonical(
    envelope: &H8MessageEnvelope,
    binding: &H8MessageConfigBinding,
    raw_external: &Map<String, Value>,
    rules: &[H8FieldMappingRule],
    connector_whitelist: &[Uuid],
    principal_warehouses: Option<&[Uuid]>,
) -> Result<H8CanonicalInboundCommand, H8MessageError> {
    envelope.validate()?;
    if !is_inbound_message_type(&envelope.message_type) {
        return Err(H8MessageError::UnknownMessageType);
    }
    if binding.message_type != envelope.message_type {
        return Err(H8MessageError::UnknownMessageType);
    }
    warehouse_in_scope(
        envelope.warehouse_id,
        connector_whitelist,
        principal_warehouses,
    )?;
    let normalized = apply_mpm_normalize(raw_external, rules)?;
    Ok(H8CanonicalInboundCommand {
        owner_id: envelope.owner_id,
        warehouse_id: envelope.warehouse_id,
        message_type: envelope.message_type.clone(),
        external_ref: envelope.external_ref.clone(),
        idempotency_key: envelope.idempotency_key.clone(),
        correlation_id: envelope.correlation_id.clone(),
        connector_id: binding.connector_id,
        config_version: binding.config_version,
        channel: binding.channel.clone(),
        fields: normalized.canonical_fields,
        occurred_at: envelope.occurred_at,
    })
}

/// 出站：canonical 事件 → ERP 侧字段（仅按规则反向映射，适配器再组 DTO）。
pub fn build_outbound_external_fields(
    event: &H8CanonicalOutboundEvent,
    rules: &[H8FieldMappingRule],
) -> Result<Map<String, Value>, H8MessageError> {
    if !is_outbound_message_type(&event.message_type) {
        return Err(H8MessageError::UnknownMessageType);
    }
    validate_message_type_in_catalog(&event.message_type)?;
    let mut external = Map::new();
    for (canon_key, value) in &event.fields {
        let rule = rules.iter().find(|r| r.canonical_field == *canon_key);
        match rule {
            Some(r) => {
                external.insert(r.external_field.clone(), value.clone());
            }
            None => return Err(H8MessageError::MappingMissing),
        }
    }
    Ok(external)
}

/// 处理开始后配置版本冻结：后续连接编辑不得改变本消息绑定。
pub fn config_binding_is_frozen(
    message_binding: &H8MessageConfigBinding,
    current_connector_version: i64,
) -> bool {
    // 消息侧始终以 message_binding.config_version 为准；即使连接已升版本也不切换
    let _ = current_connector_version;
    message_binding.config_version >= 1
}

/// 幂等身份（入站/出站共用）。
pub fn exchange_idempotency_key(
    owner_id: Uuid,
    message_type: &str,
    external_ref: &str,
    idempotency_key: &str,
) -> String {
    message_idempotency_identity(owner_id, message_type, external_ref, idempotency_key)
}

/// 档案补录：H8 只投递 / 转换 product_change，不得直接改 ASN 状态（AC12）。
pub fn archive_revision_h8_may_mutate_asn() -> bool {
    false
}

/// 至少一次投递：通道切换不得更换业务幂等键。
pub fn preserve_idempotency_on_channel_switch(
    original_key: &str,
    new_channel: &str,
) -> Result<String, H8MessageError> {
    if original_key.trim().is_empty() {
        return Err(H8MessageError::FieldRequired("idempotency_key"));
    }
    let _ = new_channel;
    Ok(original_key.to_string())
}

/// 错误是否应进入自动重试队列。
pub fn should_auto_retry(error_kind: &str) -> bool {
    matches!(classify_h8_error(error_kind), H8ErrorClass::Retryable)
}

/// 脱敏错误摘要：去掉 token / password 片段。
pub fn sanitize_error_summary(raw: &str) -> String {
    let mut out = raw.to_string();
    for key in ["password", "token", "api_key", "bearer"] {
        redact_named_secret(&mut out, key);
    }
    redact_bearer_scheme(&mut out);
    if out.chars().count() > 256 {
        out = out.chars().take(255).collect();
        out.push('…');
    }
    out
}

fn redact_named_secret(out: &mut String, key: &str) {
    let mut offset = 0;
    while offset < out.len() {
        let Some(found) = out[offset..].to_ascii_lowercase().find(key) else {
            break;
        };
        let key_end = offset + found + key.len();
        let bytes = out.as_bytes();
        let mut separator = key_end;
        while separator < bytes.len()
            && (bytes[separator].is_ascii_whitespace() || matches!(bytes[separator], b'"' | b'\''))
        {
            separator += 1;
        }
        if separator >= bytes.len() || !matches!(bytes[separator], b':' | b'=') {
            offset = key_end;
            continue;
        }
        let mut value_start = separator + 1;
        while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
            value_start += 1;
        }
        let quote = if value_start < bytes.len() && matches!(bytes[value_start], b'"' | b'\'') {
            let quote = bytes[value_start] as char;
            value_start += 1;
            Some(quote)
        } else {
            None
        };
        if value_start >= out.len() {
            break;
        }
        let value_end = out[value_start..]
            .char_indices()
            .find(|(_, ch)| {
                quote.map_or_else(
                    || ch.is_whitespace() || matches!(ch, ',' | ';' | '}' | ']' | '"' | '\''),
                    |quote| *ch == quote,
                )
            })
            .map(|(index, _)| value_start + index)
            .unwrap_or(out.len());
        out.replace_range(value_start..value_end, "***");
        offset = value_start + 3;
    }
}

fn redact_bearer_scheme(out: &mut String) {
    let mut offset = 0;
    while offset < out.len() {
        let Some(found) = out[offset..].to_ascii_lowercase().find("bearer") else {
            break;
        };
        let bearer_end = offset + found + "bearer".len();
        let bytes = out.as_bytes();
        if bearer_end >= bytes.len() || !bytes[bearer_end].is_ascii_whitespace() {
            offset = bearer_end;
            continue;
        }
        let mut value_start = bearer_end;
        while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
            value_start += 1;
        }
        let quote = if value_start < bytes.len() && matches!(bytes[value_start], b'"' | b'\'') {
            let quote = bytes[value_start] as char;
            value_start += 1;
            Some(quote)
        } else {
            None
        };
        let value_end = out[value_start..]
            .char_indices()
            .find(|(_, ch)| {
                quote.map_or_else(
                    || ch.is_whitespace() || matches!(ch, ',' | ';' | '}' | ']' | '"' | '\''),
                    |quote| *ch == quote,
                )
            })
            .map(|(index, _)| value_start + index)
            .unwrap_or(out.len());
        out.replace_range(value_start..value_end, "***");
        offset = value_start + 3;
    }
}

/// 入站处理成功前不得返回成功回执（AC3）：仅 succeeded 可 ack。
pub fn may_return_success_ack(sync_status: &str) -> bool {
    sync_status == "succeeded" || sync_status == "acked"
}

/// H2 审计用脱敏摘要（AC11）：仅标识与结果，不含明文凭据或完整报文。
pub fn message_audit_summary(
    action: &str,
    message_id: Uuid,
    owner_id: Uuid,
    message_type: &str,
    external_ref: &str,
    idempotency_key: &str,
    correlation_id: &str,
    sync_status: &str,
    connector_id: Option<Uuid>,
    config_version: Option<i64>,
    result: &str,
) -> serde_json::Value {
    serde_json::json!({
        "action": action,
        "message_id": message_id,
        "owner_id": owner_id,
        "message_type": message_type,
        "external_ref": external_ref,
        "idempotency_key": idempotency_key,
        "correlation_id": correlation_id,
        "sync_status": sync_status,
        "connector_id": connector_id,
        "config_version": config_version,
        "result": result,
        // 显式禁止字段（审计消费者不得期望这些键）
        "payload": null,
        "secret": null,
        "token": null,
    })
}

/// US-H8-002 AC11：交换生命周期审计动作（receive→…→final_failure）。
pub const H8_EXCHANGE_AUDIT_STAGES: [&str; 6] = [
    "receive",
    "convert",
    "business_api",
    "send",
    "receipt",
    "final_failure",
];

pub fn is_exchange_audit_stage(stage: &str) -> bool {
    H8_EXCHANGE_AUDIT_STAGES.contains(&stage)
}

/// 交换生命周期结果必须符合阶段语义；所有结果统一脱敏并限制为 256 字符。
pub fn normalize_exchange_lifecycle_result(stage: &str, result: &str) -> Option<String> {
    let result = sanitize_error_summary(result.trim());
    if result.is_empty() {
        return None;
    }
    let valid = match stage {
        "receive" => matches!(result.as_str(), "ok" | "received"),
        "convert" => result == "ok",
        "business_api" | "send" => matches!(result.as_str(), "started" | "ok"),
        "receipt" => matches!(result.as_str(), "ok" | "rejected"),
        "final_failure" => true,
        _ => false,
    };
    valid.then_some(result)
}

/// US-H8-003 AC6：进入 dead 时必须产生的审计 action 名。
pub const H8_MESSAGE_DEAD_AUDIT_ACTION: &str = "h8_message_dead";

/// 审计摘要不得携带敏感载荷键。
pub fn audit_summary_is_safe(summary: &serde_json::Value) -> bool {
    let forbidden = [
        "password",
        "bearer",
        "api_key",
        "raw_payload",
        "token_value",
    ];
    let text = summary.to_string().to_ascii_lowercase();
    !forbidden.iter().any(|f| text.contains(f))
        && summary.get("payload").is_some_and(|v| v.is_null())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rules() -> Vec<H8FieldMappingRule> {
        vec![
            H8FieldMappingRule {
                external_field: "ITEM_CODE".into(),
                canonical_field: "product_code".into(),
                transform: "trim".into(),
            },
            H8FieldMappingRule {
                external_field: "QTY".into(),
                canonical_field: "qty".into(),
                transform: "identity".into(),
            },
        ]
    }

    fn envelope() -> H8MessageEnvelope {
        H8MessageEnvelope {
            owner_id: Uuid::nil(),
            warehouse_id: Some(Uuid::nil()),
            direction: "inbound".into(),
            message_type: "asn".into(),
            schema_version: "1".into(),
            external_ref: "ERP-1".into(),
            idempotency_key: "idem-1".into(),
            connector_id: None,
            config_version: None,
            correlation_id: "c1".into(),
            occurred_at: Utc::now(),
            payload_digest: "d".into(),
            wms_resource_id: None,
        }
    }

    #[test]
    fn mpm_rejects_unmapped_external_fields() {
        let mut raw = Map::new();
        raw.insert("ITEM_CODE".into(), json!(" P1 "));
        raw.insert("UNKNOWN".into(), json!("x"));
        let err = apply_mpm_normalize(&raw, &rules()).unwrap_err();
        assert_eq!(err, H8MessageError::MappingMissing);
    }

    #[test]
    fn mpm_builds_canonical_only() {
        let mut raw = Map::new();
        raw.insert("ITEM_CODE".into(), json!(" P1 "));
        raw.insert("QTY".into(), json!(2));
        let ok = apply_mpm_normalize(&raw, &rules()).unwrap();
        assert_eq!(ok.canonical_fields.get("product_code"), Some(&json!("P1")));
        assert!(!ok.canonical_fields.contains_key("ITEM_CODE"));
    }

    #[test]
    fn inbound_canonical_pipeline() {
        let binding = H8MessageConfigBinding {
            connector_id: Uuid::nil(),
            config_version: 2,
            channel: "rest".into(),
            message_type: "asn".into(),
        };
        let mut raw = Map::new();
        raw.insert("ITEM_CODE".into(), json!("A"));
        raw.insert("QTY".into(), json!(1));
        let cmd = build_inbound_canonical(
            &envelope(),
            &binding,
            &raw,
            &rules(),
            &[Uuid::nil()],
            Some(&[Uuid::nil()]),
        )
        .unwrap();
        assert_eq!(cmd.config_version, 2);
        assert_eq!(cmd.fields.get("product_code"), Some(&json!("A")));
    }

    #[test]
    fn outbound_external_requires_full_mapping() {
        let mut fields = Map::new();
        fields.insert("product_code".into(), json!("A"));
        let event = H8CanonicalOutboundEvent {
            owner_id: Uuid::nil(),
            warehouse_id: None,
            message_type: "putaway_complete".into(),
            external_ref: "e1".into(),
            idempotency_key: "i1".into(),
            correlation_id: "c".into(),
            wms_resource_id: "asn-1".into(),
            fields,
            occurred_at: Utc::now(),
        };
        let ext = build_outbound_external_fields(&event, &rules()).unwrap();
        assert_eq!(ext.get("ITEM_CODE"), Some(&json!("A")));
    }

    #[test]
    fn channel_switch_keeps_idempotency_key() {
        let k = preserve_idempotency_on_channel_switch("idem-x", "interface_table").unwrap();
        assert_eq!(k, "idem-x");
    }

    #[test]
    fn archive_revision_boundary() {
        assert!(!archive_revision_h8_may_mutate_asn());
    }

    #[test]
    fn sanitize_and_retry_class() {
        assert!(should_auto_retry("timeout"));
        assert!(!should_auto_retry("mapping"));
        let s = sanitize_error_summary(
            "auth failed Bearer supersecrettoken password=anotherlongsecret more",
        );
        assert!(s.contains("***"));
        assert!(!s.contains("supersecrettoken"));
        assert!(!s.contains("rettoken"));
        assert!(!s.contains("anotherlongsecret"));
    }

    #[test]
    fn sanitize_masks_colon_spacing_and_json_secret_values() {
        let sanitized = sanitize_error_summary(
            r#"token: secret-a password : "secret-b" {"api_key":"secret-c","bearer": "secret-d"} Authorization: Bearer secret-e"#,
        );
        for secret in ["secret-a", "secret-b", "secret-c", "secret-d", "secret-e"] {
            assert!(!sanitized.contains(secret), "leaked {secret}: {sanitized}");
        }
        assert!(sanitized.matches("***").count() >= 5);
        assert!(sanitized.chars().count() <= 256);
    }

    #[test]
    fn success_ack_only_after_succeeded() {
        assert!(!may_return_success_ack("processing"));
        assert!(may_return_success_ack("succeeded"));
    }

    #[test]
    fn pipeline_steps_cover_story_order() {
        assert_eq!(H8_INBOUND_PIPELINE_STEPS[0], "auth_scope");
        assert_eq!(H8_INBOUND_PIPELINE_STEPS[5], "mpm_normalize");
        assert_eq!(H8_INBOUND_PIPELINE_STEPS[7], "ack_success");
    }

    #[test]
    fn exchange_audit_stages_cover_story_ac11() {
        assert!(is_exchange_audit_stage("receive"));
        assert!(is_exchange_audit_stage("convert"));
        assert!(is_exchange_audit_stage("business_api"));
        assert!(is_exchange_audit_stage("send"));
        assert!(is_exchange_audit_stage("receipt"));
        assert!(is_exchange_audit_stage("final_failure"));
        assert!(!is_exchange_audit_stage("free_text"));
        assert_eq!(H8_MESSAGE_DEAD_AUDIT_ACTION, "h8_message_dead");
    }

    #[test]
    fn exchange_lifecycle_results_are_controlled_sanitized_and_limited() {
        assert_eq!(
            normalize_exchange_lifecycle_result("receive", " received "),
            Some("received".into())
        );
        assert_eq!(
            normalize_exchange_lifecycle_result("send", "started"),
            Some("started".into())
        );
        assert!(normalize_exchange_lifecycle_result("send", "retry").is_none());
        assert!(normalize_exchange_lifecycle_result("free_text", "ok").is_none());

        let normalized = normalize_exchange_lifecycle_result(
            "final_failure",
            &format!("Bearer super-secret {}", "错".repeat(300)),
        )
        .expect("final failure summary");
        assert!(!normalized.contains("super-secret"));
        assert!(normalized.contains("***"));
        assert_eq!(normalized.chars().count(), 256);
        assert!(normalized.ends_with('…'));
    }

    #[test]
    fn audit_summary_has_ids_without_payload_secrets() {
        let summary = message_audit_summary(
            "replay",
            Uuid::nil(),
            Uuid::nil(),
            "asn",
            "ERP-1",
            "idem-1",
            "corr-1",
            "processing",
            Some(Uuid::nil()),
            Some(2),
            "ok",
        );
        assert_eq!(summary["message_type"], "asn");
        assert_eq!(summary["result"], "ok");
        assert!(summary["payload"].is_null());
        assert!(audit_summary_is_safe(&summary));
        // 不得把疑似 token 塞进摘要字段名
        let bad = serde_json::json!({"password": "x", "payload": {"a": 1}});
        assert!(!audit_summary_is_safe(&bad));
    }
}
