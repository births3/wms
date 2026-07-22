//! H8 ERP 消息 HTTP handlers。

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;
use wms_domain::{
    validate_schema_version, ClaimH8ErpMessageRequest, H8ErpMessage, H8ErpMessageDetail,
    H8ErpMessageListResponse, H8ErpMessageStats, H8WorkerClaimDecision, H8WorkerHeartbeatRequest,
    H8WorkerRuntimeResponse, H8WorkerStatus, PageMeta, PurgeH8ErpMessagesRequest,
    PurgeH8ErpMessagesResponse, ReplayH8ErpMessageRequest, SetH8WorkerClaimControlRequest,
    UpdateH8PayloadRetentionPolicyRequest, UpsertH8ErpMessageLifecycleRequest,
};

use crate::auth::AuthContext;

use super::audit::{
    write_dead_entry_audit, write_exchange_lifecycle_audit, write_message_audit, write_owner_audit,
};
use super::error::H8ErpMessageHandlerError;
use super::lifecycle::{apply_inbound_lifecycle_status, record_lifecycle, safe_lifecycle_result};
use super::repository::H8ErpMessageCursor;
use super::state::{H8ErpMessageAppState, H8_MSG_READ, H8_MSG_WRITE};

const DEFAULT_LIST_LIMIT: u32 = 50;
const MAX_LIST_LIMIT: u32 = 200;

pub fn h8_erp_message_router(state: H8ErpMessageAppState) -> Router {
    Router::new()
        .route("/api/v1/integration/erp-messages", get(list_messages))
        .route("/api/v1/integration/erp-messages/stats", get(message_stats))
        .route(
            "/api/v1/integration/erp-messages/payload-retention",
            get(list_payload_retention_policies).post(update_payload_retention_policy),
        )
        .route(
            "/api/v1/integration/erp-messages/worker-runtime",
            get(worker_runtime),
        )
        .route(
            "/api/v1/integration/erp-messages/worker-runtime/heartbeat",
            post(record_worker_heartbeat),
        )
        .route(
            "/api/v1/integration/erp-messages/worker-runtime/control",
            post(set_worker_claim_control),
        )
        .route(
            "/api/v1/integration/erp-messages/worker-runtime/claim-decision",
            get(worker_claim_decision),
        )
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
            "/api/v1/integration/erp-messages/:id/payload",
            get(decrypt_message_payload),
        )
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
    connector_code: Option<String>,
    connector_id: Option<Uuid>,
    channel: Option<String>,
    replay_requested: Option<bool>,
    warehouse_id: Option<Uuid>,
    external_ref: Option<String>,
    idempotency_key: Option<String>,
    correlation_id: Option<String>,
    created_from: Option<DateTime<Utc>>,
    created_to: Option<DateTime<Utc>>,
    cursor: Option<String>,
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct MarkDeadRequest {
    error_summary: String,
}

#[derive(Debug, Deserialize)]
struct ClaimDecisionQuery {
    connector_id: Uuid,
    direction: String,
}

#[derive(Debug, Deserialize)]
struct StatsQuery {
    connector_code: Option<String>,
    channel: Option<String>,
    message_type: Option<String>,
}

async fn list_messages(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<H8ErpMessageListResponse>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_READ)?;
    let direction = q.direction.as_deref().map(str::trim);
    let message_type = q.message_type.as_deref().map(str::trim);
    let status = q.status.as_deref().map(str::trim);
    let connector_code = q.connector_code.as_deref().map(str::trim);
    let channel = q.channel.as_deref().map(str::trim);
    let external_ref = q.external_ref.as_deref().map(str::trim);
    let idempotency_key = q.idempotency_key.as_deref().map(str::trim);
    let correlation_id = q.correlation_id.as_deref().map(str::trim);
    if let Some(value) = direction {
        wms_domain::validate_direction(value)
            .map_err(super::error::H8ErpMessageRepoError::Domain)?;
    }
    if let Some(value) = message_type {
        wms_domain::validate_message_type_in_catalog(value)
            .map_err(super::error::H8ErpMessageRepoError::Domain)?;
    }
    if let Some(value) = status {
        wms_domain::validate_sync_status(value)
            .map_err(super::error::H8ErpMessageRepoError::Domain)?;
    }
    if connector_code.is_some_and(str::is_empty) {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "connector_code required",
        ));
    }
    if let Some(value) = channel {
        wms_domain::validate_channel(value).map_err(super::error::H8ErpMessageRepoError::Domain)?;
    }
    for (field, value) in [
        ("external_ref", external_ref),
        ("idempotency_key", idempotency_key),
        ("correlation_id", correlation_id),
    ] {
        if value.is_some_and(str::is_empty) {
            return Err(H8ErpMessageHandlerError::BadRequest(field));
        }
    }
    if q.created_from
        .zip(q.created_to)
        .is_some_and(|(from, to)| from > to)
    {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "created_from must not exceed created_to",
        ));
    }
    let limit = q.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    if !(1..=MAX_LIST_LIMIT).contains(&limit) {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "limit must be 1..=200",
        ));
    }
    let cursor = q.cursor.as_deref().map(parse_message_cursor).transpose()?;
    let window_from = q
        .created_from
        .or(cursor.map(|value| value.window_from))
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
    if cursor.is_some_and(|value| value.window_from != window_from) {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "cursor does not match created_from",
        ));
    }
    let mut data = state
        .repository
        .list(
            ctx.owner_id,
            direction,
            message_type,
            status,
            connector_code,
            q.connector_id,
            channel,
            q.replay_requested.unwrap_or(false),
            q.warehouse_id,
            external_ref,
            idempotency_key,
            correlation_id,
            Some(window_from),
            q.created_to,
            cursor,
            limit,
        )
        .await?;
    let next_cursor = if data.len() > limit as usize {
        data.pop();
        data.last()
            .map(|message| format_message_cursor(window_from, message))
    } else {
        None
    };
    let len = data.len();
    Ok(Json(H8ErpMessageListResponse {
        data,
        page: PageMeta {
            next_cursor,
            count: len as u32,
        },
    }))
}

fn parse_message_cursor(value: &str) -> Result<H8ErpMessageCursor, H8ErpMessageHandlerError> {
    let mut parts = value.splitn(3, ',');
    let window_from = parts
        .next()
        .ok_or(H8ErpMessageHandlerError::BadRequest("invalid cursor"))?;
    let created_at = parts
        .next()
        .ok_or(H8ErpMessageHandlerError::BadRequest("invalid cursor"))?;
    let id = parts
        .next()
        .ok_or(H8ErpMessageHandlerError::BadRequest("invalid cursor"))?;
    let window_from = DateTime::parse_from_rfc3339(window_from)
        .map_err(|_| H8ErpMessageHandlerError::BadRequest("invalid cursor"))?
        .with_timezone(&Utc);
    let created_at = DateTime::parse_from_rfc3339(created_at)
        .map_err(|_| H8ErpMessageHandlerError::BadRequest("invalid cursor"))?
        .with_timezone(&Utc);
    let id = id
        .parse::<Uuid>()
        .map_err(|_| H8ErpMessageHandlerError::BadRequest("invalid cursor"))?;
    Ok(H8ErpMessageCursor {
        window_from,
        created_at,
        id,
    })
}

fn format_message_cursor(window_from: DateTime<Utc>, message: &H8ErpMessage) -> String {
    format!(
        "{},{},{}",
        window_from.to_rfc3339_opts(SecondsFormat::Nanos, true),
        message
            .created_at
            .to_rfc3339_opts(SecondsFormat::Nanos, true),
        message.id
    )
}

async fn get_message(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<H8ErpMessageDetail>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_READ)?;
    let message = state.repository.get(ctx.owner_id, id).await?;
    let attempts = state.repository.list_attempts(ctx.owner_id, id).await?;
    let (payload_retained, payload_expires_at) = state
        .payload_repository
        .payload_status(ctx.owner_id, id, Utc::now())
        .await?;
    // US-H8-003 AC11：查询详情写 H2 审计引用
    write_message_audit(&state, &ctx, "h8_message_detail_query", &message, "viewed").await?;
    Ok(Json(H8ErpMessageDetail {
        message,
        attempts,
        payload_retained,
        payload_expires_at,
    }))
}

fn encryption_master_key() -> Option<String> {
    std::env::var("WMS_ENCRYPTION_MASTER_KEY").ok()
}

fn encryption_key_version() -> String {
    std::env::var("WMS_ENCRYPTION_KEY_VERSION")
        .unwrap_or_else(|_| "v1".into())
        .trim()
        .to_string()
}

fn encryption_master_keys() -> Result<HashMap<String, String>, H8ErpMessageHandlerError> {
    let mut keys = match std::env::var("WMS_ENCRYPTION_PREVIOUS_MASTER_KEYS") {
        Ok(value) => serde_json::from_str::<HashMap<String, String>>(&value).map_err(|_| {
            super::error::H8ErpMessageRepoError::Domain(
                wms_domain::H8MessageError::EncryptionKeyUnavailable,
            )
        })?,
        Err(_) => HashMap::new(),
    };
    if keys
        .iter()
        .any(|(version, key)| version.trim().is_empty() || version.len() > 64 || key.len() < 32)
    {
        return Err(super::error::H8ErpMessageRepoError::Domain(
            wms_domain::H8MessageError::EncryptionKeyUnavailable,
        )
        .into());
    }
    if let Some(current) = encryption_master_key() {
        keys.insert(encryption_key_version(), current);
    }
    Ok(keys)
}

async fn list_payload_retention_policies(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
) -> Result<Json<Vec<wms_domain::H8PayloadRetentionPolicy>>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_READ)?;
    Ok(Json(
        state.payload_repository.list_policies(ctx.owner_id).await?,
    ))
}

async fn update_payload_retention_policy(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Json(body): Json<UpdateH8PayloadRetentionPolicyRequest>,
) -> Result<Json<wms_domain::H8PayloadRetentionPolicy>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    let key_version = encryption_key_version();
    if body.enabled
        && (key_version.is_empty()
            || key_version.len() > 64
            || encryption_master_key()
                .as_deref()
                .is_none_or(|value| value.len() < 32))
    {
        return Err(super::error::H8ErpMessageRepoError::Domain(
            wms_domain::H8MessageError::EncryptionKeyUnavailable,
        )
        .into());
    }
    let policy = state
        .payload_repository
        .update_policy(ctx.owner_id, &body, &ctx.user_id.to_string(), Utc::now())
        .await?;
    write_owner_audit(
        &state,
        &ctx,
        "h8_payload_retention_update",
        serde_json::json!({
            "connector_id": policy.connector_id,
            "enabled": policy.enabled,
            "retention_days": policy.retention_days,
            "payload": null,
        }),
    )
    .await?;
    Ok(Json(policy))
}

async fn decrypt_message_payload(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Path(id): Path<Uuid>,
) -> Result<(HeaderMap, Json<wms_domain::H8DecryptedPayload>), H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    let keys = encryption_master_keys()?;
    let payload = state
        .payload_repository
        .decrypt_payload(ctx.owner_id, id, &keys, Utc::now())
        .await?;
    write_owner_audit(
        &state,
        &ctx,
        "h8_payload_decrypt",
        serde_json::json!({
            "message_id": id,
            "expires_at": payload.expires_at,
            "payload": null,
        }),
    )
    .await?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    Ok((headers, Json(payload)))
}

async fn message_stats(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<H8ErpMessageStats>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_READ)?;
    let connector_code = query.connector_code.as_deref().map(str::trim);
    let channel = query.channel.as_deref().map(str::trim);
    let message_type = query.message_type.as_deref().map(str::trim);
    if connector_code.is_some_and(str::is_empty) {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "connector_code required",
        ));
    }
    if let Some(channel) = channel {
        wms_domain::validate_channel(channel)
            .map_err(|error| super::error::H8ErpMessageRepoError::Domain(error))?;
    }
    if let Some(message_type) = message_type {
        wms_domain::validate_message_type_in_catalog(message_type)
            .map_err(|error| super::error::H8ErpMessageRepoError::Domain(error))?;
    }
    Ok(Json(
        state
            .repository
            .stats(ctx.owner_id, connector_code, channel, message_type)
            .await?,
    ))
}

async fn worker_runtime(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
) -> Result<Json<H8WorkerRuntimeResponse>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_READ)?;
    Ok(Json(
        state
            .runtime_repository
            .list_runtime(ctx.owner_id, Utc::now())
            .await?,
    ))
}

async fn record_worker_heartbeat(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Json(body): Json<H8WorkerHeartbeatRequest>,
) -> Result<Json<H8WorkerStatus>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    Ok(Json(
        state
            .runtime_repository
            .record_heartbeat(ctx.owner_id, &body, Utc::now())
            .await?,
    ))
}

async fn set_worker_claim_control(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Json(body): Json<SetH8WorkerClaimControlRequest>,
) -> Result<Json<wms_domain::H8WorkerClaimControl>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    let control = state
        .runtime_repository
        .set_claim_control(ctx.owner_id, &body, &ctx.user_id.to_string(), Utc::now())
        .await?;
    let action = if control.paused {
        "h8_worker_claim_pause"
    } else {
        "h8_worker_claim_resume"
    };
    write_owner_audit(
        &state,
        &ctx,
        action,
        serde_json::json!({
            "action": action,
            "connector_id": control.connector_id,
            "direction": control.direction,
            "reason": control.reason,
            "paused_until": control.paused_until,
            "payload": null,
        }),
    )
    .await?;
    Ok(Json(control))
}

async fn worker_claim_decision(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Query(query): Query<ClaimDecisionQuery>,
) -> Result<Json<H8WorkerClaimDecision>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    Ok(Json(
        state
            .runtime_repository
            .claim_decision(
                ctx.owner_id,
                query.connector_id,
                query.direction.trim(),
                Utc::now(),
            )
            .await?,
    ))
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
    write_message_audit(&state, &ctx, "h8_message_replay", &message, "accepted").await?;
    Ok(Json(message))
}

async fn claim_message(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ClaimH8ErpMessageRequest>,
) -> Result<Json<H8ErpMessage>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    let existing = state.repository.get(ctx.owner_id, id).await?;
    if let Some(connector_id) = existing.connector_id {
        let decision = state
            .runtime_repository
            .claim_decision(ctx.owner_id, connector_id, &existing.direction, Utc::now())
            .await?;
        if !decision.allowed {
            return Err(super::error::H8ErpMessageRepoError::Domain(
                wms_domain::H8MessageError::ClaimPaused,
            )
            .into());
        }
    }
    let lease = body.lease_seconds.unwrap_or(300);
    let message = state
        .repository
        .claim(ctx.owner_id, id, body.worker_id.trim(), lease, Utc::now())
        .await?;
    write_message_audit(&state, &ctx, "h8_message_claim", &message, "claimed").await?;
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
    write_dead_entry_audit(&state, &ctx, &message).await?;
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
    write_message_audit(&state, &ctx, "h8_message_archive", &message, "archived").await?;
    Ok(Json(message))
}

async fn record_lifecycle_upsert(
    ctx: AuthContext,
    State(state): State<H8ErpMessageAppState>,
    Json(body): Json<UpsertH8ErpMessageLifecycleRequest>,
) -> Result<Json<H8ErpMessage>, H8ErpMessageHandlerError> {
    ctx.require_permission(H8_MSG_WRITE)?;
    if !wms_domain::is_exchange_audit_stage(body.stage.trim()) {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "invalid exchange audit stage",
        ));
    }
    wms_domain::validate_direction(body.direction.trim())
        .map_err(super::error::H8ErpMessageRepoError::Domain)?;
    wms_domain::validate_message_type_in_catalog(body.message_type.trim())
        .map_err(super::error::H8ErpMessageRepoError::Domain)?;
    wms_domain::validate_channel(body.channel.trim())
        .map_err(super::error::H8ErpMessageRepoError::Domain)?;
    for (field, value) in [
        ("result", body.result.as_str()),
        ("external_ref", body.external_ref.as_str()),
        ("idempotency_key", body.idempotency_key.as_str()),
        ("correlation_id", body.correlation_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(super::error::H8ErpMessageRepoError::Domain(
                wms_domain::H8MessageError::FieldRequired(field),
            )
            .into());
        }
    }
    if !matches!(body.stage.trim(), "receive" | "final_failure") {
        validate_schema_version(body.schema_version.trim())
            .map_err(super::error::H8ErpMessageRepoError::Domain)?;
    }
    let now = Utc::now();
    let mut message = if let Some(id) = body.message_id {
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
    let connector_changed = matches!(
        (message.connector_id, body.connector_id),
        (Some(bound), Some(requested)) if bound != requested
    );
    let version_changed = matches!(
        (message.config_version, body.config_version),
        (Some(bound), Some(requested)) if bound != requested
    );
    if connector_changed
        || version_changed
        || message.direction != body.direction.trim()
        || message.message_type != body.message_type.trim()
        || message.schema_version != body.schema_version.trim()
        || message.channel != body.channel.trim()
    {
        return Err(H8ErpMessageHandlerError::BadRequest(
            "message config binding must not change",
        ));
    }
    let lifecycle_result = safe_lifecycle_result(body.stage.trim(), body.result.trim());
    write_exchange_lifecycle_audit(&state, &ctx, &message, body.stage.trim(), &lifecycle_result)
        .await?;
    message = apply_inbound_lifecycle_status(
        &state,
        &ctx,
        message,
        body.stage.trim(),
        &lifecycle_result,
        now,
    )
    .await?;
    if body.stage.trim() == "receive" {
        if let (Some(connector_id), Some(payload)) = (message.connector_id, body.payload.as_ref()) {
            let serialized = serde_json::to_string(payload)
                .map_err(|_| H8ErpMessageHandlerError::BadRequest("payload must be json"))?;
            let master_key = encryption_master_key();
            state
                .payload_repository
                .capture_payload(
                    ctx.owner_id,
                    message.id,
                    connector_id,
                    &serialized,
                    master_key.as_deref(),
                    &encryption_key_version(),
                    now,
                )
                .await?;
        }
    }
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
    .await?;
    Ok(Json(PurgeH8ErpMessagesResponse {
        deleted,
        retention_days,
    }))
}
