//! H8 消息 H2 审计（脱敏摘要）。
//!
//! 内存 sink 始终记录，便于单测证明真实调用路径；有 `audit_pool` 时同时 append 到 H2 表。

use chrono::Utc;
use wms_domain::{
    is_exchange_audit_stage, message_audit_summary, H8ErpMessage, H8_MESSAGE_DEAD_AUDIT_ACTION,
};

use crate::{
    audit::{append_event, AuditDiff, AuditEventRecord, AuditWriteRequest},
    auth::AuthContext,
};

use super::error::H8ErpMessageHandlerError;
use super::state::H8ErpMessageAppState;

pub(crate) async fn write_message_audit(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    action: &str,
    message: &H8ErpMessage,
    result: &str,
) {
    let after = message_audit_summary(
        action,
        message.id,
        message.owner_id,
        &message.message_type,
        &message.external_ref,
        &message.idempotency_key,
        &message.correlation_id,
        &message.sync_status,
        message.connector_id,
        message.config_version,
        result,
    );
    let mut req = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H8",
        "h8_erp_message",
        message.id.to_string(),
        Some(AuditDiff::compute(serde_json::Value::Null, after)),
    );
    req.occurred_at = Utc::now();
    // 始终写入可观测 sink（软件路径验收）
    {
        let mut log = state.audit_log.lock().expect("audit log");
        log.append_event(req.clone());
    }
    if let Some(pool) = &state.audit_pool {
        let _ = append_event(pool, &req).await;
    }
}

pub(crate) async fn write_owner_audit(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    action: &str,
    after: serde_json::Value,
) {
    let mut req = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H8",
        "h8_erp_message",
        ctx.owner_id.to_string(),
        Some(AuditDiff::compute(serde_json::Value::Null, after)),
    );
    req.occurred_at = Utc::now();
    {
        let mut log = state.audit_log.lock().expect("audit log");
        log.append_event(req.clone());
    }
    if let Some(pool) = &state.audit_pool {
        let _ = append_event(pool, &req).await;
    }
}

/// US-H8-003 AC6：进入 dead 时写 H2。
pub(crate) async fn write_dead_entry_audit(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    message: &H8ErpMessage,
) {
    write_message_audit(state, ctx, H8_MESSAGE_DEAD_AUDIT_ACTION, message, "dead").await;
}

/// US-H8-002 AC11：交换生命周期阶段审计。
pub(crate) async fn write_exchange_lifecycle_audit(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    message: &H8ErpMessage,
    stage: &str,
    result: &str,
) -> Result<(), H8ErpMessageHandlerError> {
    if !is_exchange_audit_stage(stage) {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "invalid exchange audit stage",
        ));
    }
    let action = format!("h8_exchange_{stage}");
    write_message_audit(state, ctx, &action, message, result).await;
    Ok(())
}

pub(crate) fn snapshot_audit_actions(state: &H8ErpMessageAppState) -> Vec<String> {
    let log = state.audit_log.lock().expect("audit log");
    log.events()
        .iter()
        .map(|e: &AuditEventRecord| e.action.clone())
        .collect()
}
