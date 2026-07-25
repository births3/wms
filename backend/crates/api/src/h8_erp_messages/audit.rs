//! H8 消息 H2 审计（脱敏摘要）。
//!
//! 内存 sink 始终记录，便于单测证明真实调用路径；有 `audit_pool` 时同时 append 到 H2 表。

use chrono::Utc;
use wms_domain::{
    is_exchange_audit_stage, message_audit_summary, normalize_exchange_lifecycle_result,
    H8ErpMessage, H8_MESSAGE_DEAD_AUDIT_ACTION,
};

use crate::{
    audit::{append_event, AuditDiff, AuditError, AuditWriteRequest},
    auth::AuthContext,
    sync::lock_recover,
};

use super::error::H8ErpMessageHandlerError;
use super::state::H8ErpMessageAppState;

pub(crate) async fn write_message_audit(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    action: &str,
    message: &H8ErpMessage,
    result: &str,
) -> Result<(), AuditError> {
    let req = message_audit_request(ctx, action, message, result, Utc::now());
    persist_audit(state.audit_pool.as_ref(), &req).await?;
    append_memory_audit_requests(state, std::slice::from_ref(&req));
    Ok(())
}

pub(crate) fn message_audit_request(
    ctx: &AuthContext,
    action: &str,
    message: &H8ErpMessage,
    result: &str,
    occurred_at: chrono::DateTime<Utc>,
) -> AuditWriteRequest {
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
    req.occurred_at = occurred_at;
    req
}

pub(crate) fn append_memory_audit_requests(
    state: &H8ErpMessageAppState,
    requests: &[AuditWriteRequest],
) {
    let mut log = lock_recover(&state.audit_log);
    for request in requests {
        log.append_event(request.clone());
    }
}

pub(crate) async fn write_owner_audit(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    action: &str,
    after: serde_json::Value,
) -> Result<(), AuditError> {
    let mut req = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H8",
        "h8_erp_message",
        ctx.owner_id.to_string(),
        Some(AuditDiff::compute(serde_json::Value::Null, after)),
    );
    req.occurred_at = Utc::now();
    persist_audit(state.audit_pool.as_ref(), &req).await?;
    {
        let mut log = lock_recover(&state.audit_log);
        log.append_event(req);
    }
    Ok(())
}

async fn persist_audit(
    pool: Option<&sqlx::PgPool>,
    request: &AuditWriteRequest,
) -> Result<(), AuditError> {
    if let Some(pool) = pool {
        append_event(pool, request).await?;
    }
    Ok(())
}

/// US-H8-003 AC6：进入 dead 时写 H2。
#[cfg(test)]
pub(crate) async fn write_dead_entry_audit(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    message: &H8ErpMessage,
) -> Result<(), AuditError> {
    write_message_audit(state, ctx, H8_MESSAGE_DEAD_AUDIT_ACTION, message, "dead").await
}

pub(crate) fn dead_entry_audit_request(
    ctx: &AuthContext,
    message: &H8ErpMessage,
    occurred_at: chrono::DateTime<Utc>,
) -> AuditWriteRequest {
    message_audit_request(
        ctx,
        H8_MESSAGE_DEAD_AUDIT_ACTION,
        message,
        "dead",
        occurred_at,
    )
}

pub(crate) fn exchange_lifecycle_audit_request(
    ctx: &AuthContext,
    message: &H8ErpMessage,
    stage: &str,
    result: &str,
    occurred_at: chrono::DateTime<Utc>,
) -> Result<AuditWriteRequest, H8ErpMessageHandlerError> {
    if !is_exchange_audit_stage(stage) {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "invalid exchange audit stage",
        ));
    }
    let result = normalize_exchange_lifecycle_result(stage, result).ok_or(
        H8ErpMessageHandlerError::BadRequest("invalid exchange lifecycle result"),
    )?;
    Ok(message_audit_request(
        ctx,
        &format!("h8_exchange_{stage}"),
        message,
        &result,
        occurred_at,
    ))
}

/// US-H8-002 AC11：交换生命周期阶段审计。
pub(crate) async fn write_exchange_lifecycle_audit(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    message: &H8ErpMessage,
    stage: &str,
    result: &str,
) -> Result<(), H8ErpMessageHandlerError> {
    let request = exchange_lifecycle_audit_request(ctx, message, stage, result, Utc::now())?;
    persist_audit(state.audit_pool.as_ref(), &request).await?;
    append_memory_audit_requests(state, std::slice::from_ref(&request));
    Ok(())
}

#[cfg(test)]
pub(crate) fn snapshot_audit_actions(state: &H8ErpMessageAppState) -> Vec<String> {
    let log = state.audit_log.lock().expect("audit log");
    log.events()
        .iter()
        .map(|event| event.action.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    use super::*;
    use crate::auth::AuthContext;

    #[tokio::test]
    async fn persistent_audit_failure_is_not_swallowed() {
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(50))
            .connect_lazy("postgres://wms:wms@127.0.0.1:1/wms")
            .expect("lazy pool");
        let context = AuthContext {
            user_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            actor_name: "audit-test".into(),
            permissions: vec![],
            jti: "audit-test".into(),
            warehouse_scope: None,
        };
        let request = AuditWriteRequest::from_auth_context(
            &context,
            "h8_payload_decrypt",
            "H8",
            "h8_erp_message",
            "payload-test",
            None,
        );
        assert!(persist_audit(Some(&pool), &request).await.is_err());
    }
}
