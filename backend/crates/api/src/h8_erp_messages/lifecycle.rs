//! Worker 交换阶段到 H8 消息状态的用例编排。

use axum::{extract::Path, extract::State, http::HeaderMap, Json};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;
use wms_domain::{
    normalize_exchange_lifecycle_result, should_enter_dead, H8ErpBusinessReceiptRequest,
    H8ErpMessage, H8ErrorClass, H8MessageError,
};

use crate::auth::{AuthContext, AuthError};

use super::{
    audit::{
        append_memory_audit_requests, dead_entry_audit_request, exchange_lifecycle_audit_request,
        write_exchange_lifecycle_audit,
    },
    error::{H8ErpMessageHandlerError, H8ErpMessageRepoError},
    scope::require_message_warehouse_scope,
    state::{H8ErpMessageAppState, H8_MSG_WRITE, H8_RECEIPT_WRITE},
};

#[derive(Debug, Deserialize)]
pub(super) struct LifecycleRequest {
    stage: String,
    result: String,
}

pub(super) async fn record_lifecycle(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<LifecycleRequest>,
) -> Result<Json<H8ErpMessage>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    let message = state.repository.get(ctx.owner_id, id).await?;
    require_message_warehouse_scope(&state, &ctx, &message).await?;
    let result = safe_lifecycle_result(body.stage.trim(), body.result.trim())?;
    Ok(Json(
        apply_lifecycle_status(
            &state,
            &ctx,
            message,
            body.stage.trim(),
            &result,
            None,
            Utc::now(),
        )
        .await?,
    ))
}

pub(super) async fn record_business_receipt(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<H8ErpBusinessReceiptRequest>,
) -> Result<Json<H8ErpMessage>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_RECEIPT_WRITE)?;
    let message = state.repository.get(ctx.owner_id, id).await?;
    let connector_id = message
        .connector_id
        .ok_or(H8ErpMessageHandlerError::BadRequest(
            "business receipt connector binding required",
        ))?;
    let config_version = message
        .config_version
        .ok_or(H8ErpMessageHandlerError::BadRequest(
            "business receipt connector binding required",
        ))?;
    let connector = state
        .connector_repository
        .get_version(ctx.owner_id, connector_id, config_version)
        .await
        .map_err(|error| {
            tracing::error!(
                target: "h8.erp_messages",
                ?error,
                %connector_id,
                config_version,
                "frozen connector version unavailable for receipt"
            );
            super::error::H8ErpMessageRepoError::Db("frozen connector version unavailable".into())
        })?;
    if connector.api_key_id != Some(ctx.user_id) {
        return Err(AuthError::PermissionDenied("frozen connector API key binding".into()).into());
    }
    if ctx
        .warehouse_scope
        .is_some_and(|scope| message.warehouse_id != Some(scope))
        || (!connector.warehouse_ids.is_empty()
            && message
                .warehouse_id
                .is_none_or(|warehouse_id| !connector.warehouse_ids.contains(&warehouse_id)))
    {
        return Err(
            AuthError::PermissionDenied("frozen connector warehouse binding".into()).into(),
        );
    }
    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(H8ErpMessageHandlerError::BadRequest(
            "Idempotency-Key required",
        ))?;
    let result = body.result.trim();
    if message.direction != "outbound"
        || !matches!(result, "ok" | "rejected")
        || body.schema_version.trim() != message.schema_version
        || body.correlation_id.trim() != message.correlation_id
        || idempotency_key != message.idempotency_key
    {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "business receipt binding mismatch",
        ));
    }
    let rejection = body
        .error_summary
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if result == "rejected" && rejection.is_none() {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "error_summary required",
        ));
    }
    if result == "ok" && rejection.is_some() {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "error_summary is only allowed for rejected receipts",
        ));
    }
    if (message.sync_status == "acked" && result == "ok")
        || (message.sync_status == "dead" && result == "rejected")
    {
        return Ok(Json(message));
    }
    let now = Utc::now();
    if result == "rejected" {
        let error_summary = rejection.ok_or(H8ErpMessageHandlerError::BadRequest(
            "error_summary required",
        ))?;
        let mut projected = message.clone();
        projected.sync_status = "dead".into();
        let requests = vec![
            exchange_lifecycle_audit_request(&ctx, &projected, "receipt", result, now)?,
            dead_entry_audit_request(&ctx, &projected, now),
        ];
        let rejected = state
            .repository
            .mark_dead(
                ctx.owner_id,
                message.id,
                error_summary,
                &ctx.user_id.to_string(),
                now,
                &requests,
            )
            .await?;
        append_memory_audit_requests(&state, &requests);
        return Ok(Json(rejected));
    }
    Ok(Json(
        apply_lifecycle_status(&state, &ctx, message, "receipt", "ok", None, now).await?,
    ))
}

pub(super) fn safe_lifecycle_result(
    stage: &str,
    result: &str,
) -> Result<String, H8ErpMessageHandlerError> {
    normalize_exchange_lifecycle_result(stage, result).ok_or(H8ErpMessageHandlerError::BadRequest(
        "invalid exchange lifecycle result",
    ))
}

enum LifecycleEffect<'a> {
    AuditOnly,
    Transition {
        target: &'static str,
        error_summary: Option<&'a str>,
    },
    IdempotentTerminal,
}

fn lifecycle_effect<'a>(
    message: &H8ErpMessage,
    stage: &str,
    result: &'a str,
    failure_class: H8ErrorClass,
) -> Result<LifecycleEffect<'a>, H8ErpMessageHandlerError> {
    let effect = match (
        message.direction.as_str(),
        message.sync_status.as_str(),
        stage,
        result,
    ) {
        ("inbound" | "outbound", "pending" | "failed", "receive", "ok" | "received") => {
            LifecycleEffect::Transition {
                target: "processing",
                error_summary: None,
            }
        }
        ("inbound" | "outbound", "processing", "receive", "ok" | "received")
        | ("inbound" | "outbound", "processing", "convert", "ok")
        | ("inbound", "processing", "business_api", "started" | "ok")
        | ("outbound", "processing", "send", "started") => LifecycleEffect::AuditOnly,
        ("inbound", "processing", "receipt", "ok") => LifecycleEffect::Transition {
            target: "succeeded",
            error_summary: None,
        },
        ("outbound", "processing", "send", "ok") => LifecycleEffect::Transition {
            target: "awaiting_receipt",
            error_summary: None,
        },
        ("outbound", "awaiting_receipt", "receipt", "ok") => LifecycleEffect::Transition {
            target: "acked",
            error_summary: None,
        },
        ("inbound" | "outbound", "processing", "final_failure", _) => LifecycleEffect::Transition {
            target: if should_enter_dead(failure_class, message.retry_count + 1, 5) {
                "dead"
            } else {
                "failed"
            },
            error_summary: Some(result),
        },
        ("outbound", "awaiting_receipt", "final_failure", _) => LifecycleEffect::Transition {
            target: if should_enter_dead(H8ErrorClass::Retryable, message.retry_count + 1, 5) {
                "dead"
            } else {
                "processing"
            },
            error_summary: Some(result),
        },
        ("inbound", "succeeded", "receipt", "ok")
        | ("outbound", "acked", "receipt", "ok")
        | ("outbound", "awaiting_receipt", "send", "ok")
        | ("outbound", "dead", "final_failure", _) => LifecycleEffect::IdempotentTerminal,
        _ => return Err(H8ErpMessageRepoError::Domain(H8MessageError::IllegalTransition).into()),
    };
    Ok(effect)
}

pub(crate) async fn apply_lifecycle_status(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    message: H8ErpMessage,
    stage: &str,
    result: &str,
    wms_resource_id: Option<&str>,
    now: DateTime<Utc>,
) -> Result<H8ErpMessage, H8ErpMessageHandlerError> {
    apply_lifecycle_status_with_failure_class(
        state,
        ctx,
        message,
        stage,
        result,
        wms_resource_id,
        H8ErrorClass::Retryable,
        now,
    )
    .await
}

pub(crate) async fn apply_lifecycle_failure(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    message: H8ErpMessage,
    result: &str,
    failure_class: H8ErrorClass,
    now: DateTime<Utc>,
) -> Result<H8ErpMessage, H8ErpMessageHandlerError> {
    apply_lifecycle_status_with_failure_class(
        state,
        ctx,
        message,
        "final_failure",
        result,
        None,
        failure_class,
        now,
    )
    .await
}

async fn apply_lifecycle_status_with_failure_class(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    message: H8ErpMessage,
    stage: &str,
    result: &str,
    wms_resource_id: Option<&str>,
    failure_class: H8ErrorClass,
    now: DateTime<Utc>,
) -> Result<H8ErpMessage, H8ErpMessageHandlerError> {
    let (target, error_summary) = match lifecycle_effect(&message, stage, result, failure_class)? {
        LifecycleEffect::IdempotentTerminal => return Ok(message),
        LifecycleEffect::AuditOnly => {
            write_exchange_lifecycle_audit(state, ctx, &message, stage, result).await?;
            return Ok(message);
        }
        LifecycleEffect::Transition {
            target,
            error_summary,
        } => (target, error_summary),
    };
    let mut projected = message.clone();
    projected.sync_status = target.into();
    let mut requests = vec![exchange_lifecycle_audit_request(
        ctx, &projected, stage, result, now,
    )?];
    if target == "dead" {
        requests.push(dead_entry_audit_request(ctx, &projected, now));
    }
    let updated = state
        .repository
        .transition_lifecycle_status(
            ctx.owner_id,
            message.id,
            target,
            error_summary,
            wms_resource_id,
            &format!("worker:{}", ctx.user_id),
            now,
            &requests,
        )
        .await
        .map_err(H8ErpMessageHandlerError::from)?;
    append_memory_audit_requests(state, &requests);
    Ok(updated)
}

pub(super) async fn mark_dead_with_audit(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    message: H8ErpMessage,
    error_summary: &str,
    now: DateTime<Utc>,
) -> Result<H8ErpMessage, H8ErpMessageHandlerError> {
    let mut projected = message.clone();
    projected.sync_status = "dead".into();
    let requests = vec![dead_entry_audit_request(ctx, &projected, now)];
    let updated = state
        .repository
        .mark_dead(
            ctx.owner_id,
            message.id,
            error_summary,
            &ctx.user_id.to_string(),
            now,
            &requests,
        )
        .await?;
    append_memory_audit_requests(state, &requests);
    Ok(updated)
}
