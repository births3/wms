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
    validate_schema_version, ClaimH8ErpMessageRequest, H8ErpMessage, H8ErpMessageDetail,
    H8ErpMessageListResponse, H8ErpMessageStats, PageMeta, PurgeH8ErpMessagesRequest,
    PurgeH8ErpMessagesResponse, ReplayH8ErpMessageRequest,
};

use crate::auth::AuthContext;

use super::audit::{
    write_dead_entry_audit, write_exchange_lifecycle_audit, write_message_audit, write_owner_audit,
};
use super::error::H8ErpMessageHandlerError;
use super::state::{H8ErpMessageAppState, H8_MSG_READ, H8_MSG_WRITE};

pub fn h8_erp_message_router(state: H8ErpMessageAppState) -> Router {
    Router::new()
        .route("/api/v1/integration/erp-messages", get(list_messages))
        .route("/api/v1/integration/erp-messages/stats", get(message_stats))
        .route(
            "/api/v1/integration/erp-messages/purge",
            post(purge_messages),
        )
        // Worker 真实交换路径：无 id 时按幂等键 upsert 并写 lifecycle 审计
        .route(
            "/api/v1/integration/erp-messages/lifecycle",
            post(record_lifecycle_upsert),
        )
        .route("/api/v1/integration/erp-messages/:id", get(get_message))
        .route(
            "/api/v1/integration/erp-messages/:id/replay",
            post(replay_message),
        )
        .route(
            "/api/v1/integration/erp-messages/:id/claim",
            post(claim_message),
        )
        .route(
            "/api/v1/integration/erp-messages/:id/dead",
            post(mark_dead_message),
        )
        .route(
            "/api/v1/integration/erp-messages/:id/archive",
            post(archive_message),
        )
        .route(
            "/api/v1/integration/erp-messages/:id/lifecycle",
            post(record_lifecycle),
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

#[derive(Debug, Deserialize)]
struct MarkDeadRequest {
    error_summary: String,
}

#[derive(Debug, Deserialize)]
struct LifecycleRequest {
    stage: String,
    result: String,
}

/// Worker 入站/出站共用：按幂等键定位或创建消息并写交换阶段审计。
#[derive(Debug, Deserialize)]
struct LifecycleUpsertRequest {
    stage: String,
    result: String,
    direction: String,
    message_type: String,
    schema_version: String,
    external_ref: String,
    idempotency_key: String,
    correlation_id: String,
    channel: String,
    connector_id: Option<Uuid>,
    connector_code: Option<String>,
    config_version: Option<i64>,
    message_id: Option<Uuid>,
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
    // US-H8-003 AC11：查询详情写 H2 审计引用
    write_message_audit(&state, &ctx, "h8_message_detail_query", &message, "viewed").await;
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
) -> Result<Json<H8ErpMessage>, H8ErpMessageHandlerError> {
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
    write_message_audit(&state, &ctx, "h8_message_replay", &message, "accepted").await;
    Ok(Json(message))
}

async fn claim_message(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ClaimH8ErpMessageRequest>,
) -> Result<Json<H8ErpMessage>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    let lease = body.lease_seconds.unwrap_or(300);
    let message = state
        .repository
        .claim(ctx.owner_id, id, body.worker_id.trim(), lease, Utc::now())
        .await?;
    write_message_audit(&state, &ctx, "h8_message_claim", &message, "claimed").await;
    Ok(Json(message))
}

async fn mark_dead_message(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<MarkDeadRequest>,
) -> Result<Json<H8ErpMessage>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    if body.error_summary.trim().is_empty() {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "error_summary required",
        ));
    }
    let actor = ctx.user_id.to_string();
    let message = state
        .repository
        .mark_dead(
            ctx.owner_id,
            id,
            body.error_summary.trim(),
            &actor,
            Utc::now(),
        )
        .await?;
    // US-H8-003 AC6：进入 dead 必须写 H2
    write_dead_entry_audit(&state, &ctx, &message).await;
    Ok(Json(message))
}

async fn archive_message(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<H8ErpMessage>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    let actor = ctx.user_id.to_string();
    let message = state
        .repository
        .mark_archived(ctx.owner_id, id, &actor, Utc::now())
        .await?;
    write_message_audit(&state, &ctx, "h8_message_archive", &message, "archived").await;
    Ok(Json(message))
}

async fn record_lifecycle(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<LifecycleRequest>,
) -> Result<Json<H8ErpMessage>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    let message = state.repository.get(ctx.owner_id, id).await?;
    write_exchange_lifecycle_audit(
        &state,
        &ctx,
        &message,
        body.stage.trim(),
        body.result.trim(),
    )
    .await?;
    Ok(Json(message))
}

async fn record_lifecycle_upsert(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Json(body): Json<LifecycleUpsertRequest>,
) -> Result<Json<H8ErpMessage>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    validate_schema_version(body.schema_version.trim())
        .map_err(super::error::H8ErpMessageRepoError::Domain)?;
    let now = Utc::now();
    let message = if let Some(id) = body.message_id {
        state.repository.get(ctx.owner_id, id).await?
    } else {
        let existing = state
            .repository
            .find_by_idempotency(
                ctx.owner_id,
                body.message_type.trim(),
                body.external_ref.trim(),
                body.idempotency_key.trim(),
            )
            .await?;
        if let Some(m) = existing {
            m
        } else {
            let m = H8ErpMessage {
                id: Uuid::new_v4(),
                owner_id: ctx.owner_id,
                warehouse_id: None,
                connector_id: body.connector_id,
                connector_code: body.connector_code.clone(),
                config_version: body.config_version,
                direction: body.direction.trim().to_string(),
                message_type: body.message_type.trim().to_string(),
                schema_version: body.schema_version.trim().to_string(),
                channel: body.channel.trim().to_string(),
                external_ref: body.external_ref.trim().to_string(),
                wms_resource_id: None,
                idempotency_key: body.idempotency_key.trim().to_string(),
                correlation_id: body.correlation_id.trim().to_string(),
                sync_status: "processing".into(),
                retry_count: 0,
                next_retry_at: None,
                last_error_summary: None,
                payload_digest: "worker-lifecycle".into(),
                claimed_by: Some(format!("worker:{}", ctx.user_id)),
                lease_expires_at: Some(now + chrono::Duration::minutes(10)),
                created_at: now,
                updated_at: now,
                completed_at: None,
                acked_at: None,
            };
            state.repository.upsert_for_test(&m).await?;
            m
        }
    };
    write_exchange_lifecycle_audit(
        &state,
        &ctx,
        &message,
        body.stage.trim(),
        body.result.trim(),
    )
    .await?;
    Ok(Json(message))
}

async fn purge_messages(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Json(body): Json<PurgeH8ErpMessagesRequest>,
) -> Result<Json<PurgeH8ErpMessagesResponse>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    if !body.confirmed {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "confirmed must be true",
        ));
    }
    let (deleted, retention_days) = state
        .repository
        .purge_terminal(ctx.owner_id, None, Utc::now())
        .await?;
    write_owner_audit(
        &state,
        &ctx,
        "h8_message_purge",
        serde_json::json!({
            "action": "h8_message_purge",
            "deleted": deleted,
            "retention_days": retention_days,
            "payload": null,
        }),
    )
    .await;
    Ok(Json(PurgeH8ErpMessagesResponse {
        deleted,
        retention_days,
    }))
}
