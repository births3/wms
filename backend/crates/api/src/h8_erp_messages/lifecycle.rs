//! Worker 交换阶段到 H8 消息状态的用例编排。

use axum::{extract::Path, extract::State, Json};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;
use wms_domain::{sanitize_error_summary, H8ErpMessage};

use crate::auth::AuthContext;

use super::{
    audit::write_exchange_lifecycle_audit,
    error::H8ErpMessageHandlerError,
    scope::require_message_warehouse_scope,
    state::{H8ErpMessageAppState, H8_MSG_WRITE},
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
    let result = safe_lifecycle_result(body.stage.trim(), body.result.trim());
    write_exchange_lifecycle_audit(&state, &ctx, &message, body.stage.trim(), &result).await?;
    Ok(Json(message))
}

pub(super) fn safe_lifecycle_result(stage: &str, result: &str) -> String {
    if stage == "final_failure" {
        sanitize_error_summary(result)
    } else {
        result.to_string()
    }
}

pub(super) async fn apply_inbound_lifecycle_status(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    message: H8ErpMessage,
    stage: &str,
    result: &str,
    now: DateTime<Utc>,
) -> Result<H8ErpMessage, H8ErpMessageHandlerError> {
    if message.direction != "inbound" {
        return Ok(message);
    }
    let target = match (stage, result, message.sync_status.as_str()) {
        ("receive", _, "pending" | "failed") => Some(("processing", None)),
        ("final_failure", result, "processing") => Some(("failed", Some(result))),
        ("receipt", "ok", "processing") => Some(("succeeded", None)),
        _ => None,
    };
    let Some((status, error_summary)) = target else {
        return Ok(message);
    };
    state
        .repository
        .transition_lifecycle_status(
            ctx.owner_id,
            message.id,
            status,
            error_summary,
            &format!("worker:{}", ctx.user_id),
            now,
        )
        .await
        .map_err(Into::into)
}
