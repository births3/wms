//! H8 消息 H2 审计（脱敏摘要）。

use chrono::Utc;
use wms_domain::{message_audit_summary, H8ErpMessage};

use crate::{
    audit::{append_event, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

use super::state::H8ErpMessageAppState;

pub(crate) async fn write_message_audit(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    action: &str,
    message: &H8ErpMessage,
    result: &str,
) {
    let Some(pool) = &state.audit_pool else {
        return;
    };
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
    let _ = append_event(pool, &req).await;
}
