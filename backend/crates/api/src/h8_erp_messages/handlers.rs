//! H8 ERP 消息 HTTP handlers。

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;
use wms_domain::{
    H8ErpMessageDetail, H8ErpMessageListResponse, H8ErpMessageStats, PageMeta,
    ReplayH8ErpMessageRequest,
};

use crate::auth::AuthContext;

use super::error::H8ErpMessageHandlerError;
use super::state::{H8ErpMessageAppState, H8_MSG_READ, H8_MSG_WRITE};

pub fn h8_erp_message_router(state: H8ErpMessageAppState) -> Router {
    Router::new()
        .route("/api/v1/integration/erp-messages", get(list_messages))
        .route("/api/v1/integration/erp-messages/stats", get(message_stats))
        .route("/api/v1/integration/erp-messages/:id", get(get_message))
        .route(
            "/api/v1/integration/erp-messages/:id/replay",
            post(replay_message),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    direction: Option<String>,
    message_type: Option<String>,
    status: Option<String>,
    created_from: Option<DateTime<Utc>>,
    created_to: Option<DateTime<Utc>>,
}

async fn list_messages(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<H8ErpMessageListResponse>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_READ)?;
    let data = state
        .repository
        .list(
            ctx.owner_id,
            q.direction.as_deref(),
            q.message_type.as_deref(),
            q.status.as_deref(),
            q.created_from,
            q.created_to,
        )
        .await?;
    let len = data.len();
    Ok(Json(H8ErpMessageListResponse {
        data,
        page: PageMeta {
            next_cursor: None,
            count: len as u32,
        },
    }))
}

async fn get_message(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<H8ErpMessageDetail>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_READ)?;
    let message = state.repository.get(ctx.owner_id, id).await?;
    let attempts = state.repository.list_attempts(ctx.owner_id, id).await?;
    Ok(Json(H8ErpMessageDetail { message, attempts }))
}

async fn message_stats(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
) -> Result<Json<H8ErpMessageStats>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_READ)?;
    Ok(Json(state.repository.stats(ctx.owner_id).await?))
}

async fn replay_message(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReplayH8ErpMessageRequest>,
) -> Result<Json<wms_domain::H8ErpMessage>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    if !body.confirmed {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "confirmed must be true",
        ));
    }
    if body.reason.trim().is_empty() {
        return Err(H8ErpMessageHandlerError::BadRequest("reason required"));
    }
    let actor = ctx.user_id.to_string();
    let message = state
        .repository
        .replay(ctx.owner_id, id, body.reason.trim(), &actor, Utc::now())
        .await?;
    Ok(Json(message))
}
