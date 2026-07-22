//! H8 ERP 连接 HTTP handlers。

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wms_domain::{
    apply_update, can_activate, can_physically_delete, reject_route_overlap_with_actives,
    required_inbound_scopes, resolve_active_connector, CreateH8ErpConnectorRequest, H8ErpConnector,
    H8ErpConnectorError, H8ErpConnectorListResponse, H8ErpConnectorTestResult, PageMeta,
    UpdateH8ErpConnectorRequest,
};

use crate::auth::{AuthContext, AuthError};

use super::audit::{audit_snapshot, write_audit};
use super::error::{H8ErpConnectorHandlerError, H8ErpConnectorRepoError};
use super::idempotency::{
    ensure_inbound_api_key_scopes, idempotency_key, load_idempotent_response, request_hash,
    store_idempotent_response,
};
use super::probe::run_connection_probe;
use super::state::{H8ErpConnectorAppState, H8_CONFIG_READ, H8_CONFIG_WRITE};

pub fn h8_erp_connector_router(state: H8ErpConnectorAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/config/erp-connectors",
            get(list_connectors).post(create_connector),
        )
        .route(
            "/api/v1/config/erp-connectors/route-resolve",
            get(resolve_connector_route),
        )
        .route(
            "/api/v1/config/erp-connectors/:id",
            get(get_connector)
                .patch(update_connector)
                .delete(delete_connector),
        )
        .route(
            "/api/v1/config/erp-connectors/:id/test",
            post(test_connector),
        )
        .route(
            "/api/v1/config/erp-connectors/:id/activate",
            post(activate_connector),
        )
        .route(
            "/api/v1/config/erp-connectors/:id/disable",
            post(disable_connector),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct RouteResolveQuery {
    direction: String,
    message_type: String,
    warehouse_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct RouteResolveResponse {
    connector: H8ErpConnector,
    required_inbound_scopes: Vec<&'static str>,
}

async fn list_connectors(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
) -> Result<Json<H8ErpConnectorListResponse>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_READ)?;
    let data = state.repository.list(ctx.owner_id).await?;
    let len = data.len();
    Ok(Json(H8ErpConnectorListResponse {
        data,
        page: PageMeta {
            next_cursor: None,
            count: len as u32,
        },
    }))
}

/// AC8：运行时按货主+仓+方向+消息类型解析唯一 active 连接，并返回最小入站 scope。
async fn resolve_connector_route(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Query(q): Query<RouteResolveQuery>,
) -> Result<Json<RouteResolveResponse>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_READ)?;
    let warehouse_id = match (q.warehouse_id, ctx.warehouse_scope) {
        (Some(requested), Some(scope)) if requested != scope => {
            return Err(AuthError::PermissionDenied("warehouse scope".into()).into());
        }
        (None, Some(scope)) => Some(scope),
        (requested, _) => requested,
    };
    let actives = state.repository.list_active(ctx.owner_id).await?;
    let connector = resolve_active_connector(
        &actives,
        warehouse_id,
        q.direction.trim(),
        q.message_type.trim(),
    )
    .map_err(H8ErpConnectorRepoError::Domain)?
    .clone();
    let scopes = required_inbound_scopes(&connector.message_types);
    Ok(Json(RouteResolveResponse {
        connector,
        required_inbound_scopes: scopes,
    }))
}

async fn get_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<H8ErpConnector>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_READ)?;
    Ok(Json(state.repository.get(ctx.owner_id, id).await?))
}

async fn create_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateH8ErpConnectorRequest>,
) -> Result<(StatusCode, Json<H8ErpConnector>), H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let idem = idempotency_key(&headers)?;
    let hash = request_hash(&req)?;
    if let Some((status, body)) =
        load_idempotent_response(&state, ctx.owner_id, &idem, &hash).await?
    {
        let connector: H8ErpConnector = serde_json::from_value(body).map_err(|e| {
            H8ErpConnectorHandlerError::Repo(H8ErpConnectorRepoError::Db(e.to_string()))
        })?;
        return Ok((status, Json(connector)));
    }
    req.validate().map_err(H8ErpConnectorRepoError::Domain)?;
    let now = Utc::now();
    let probe_alias_set = req
        .interface_probe_db_password_alias
        .as_deref()
        .is_some_and(|alias| !alias.trim().is_empty());
    let connector = H8ErpConnector {
        id: Uuid::new_v4(),
        owner_id: ctx.owner_id,
        connector_code: req.connector_code.trim().to_string(),
        connector_name: req.connector_name.trim().to_string(),
        warehouse_ids: req.warehouse_ids,
        directions: req.directions,
        message_types: req.message_types,
        channel_mode: req.channel_mode,
        api_base_url: req.api_base_url,
        interface_db_host: req.interface_db_host,
        interface_db_port: req.interface_db_port,
        interface_db_name: req.interface_db_name,
        interface_db_username: req.interface_db_username,
        api_key_id: req.api_key_id,
        bearer_secret_alias: req.bearer_secret_alias,
        interface_db_password_alias: req.interface_db_password_alias,
        interface_probe_db_username: req.interface_probe_db_username,
        interface_probe_db_password_alias: req.interface_probe_db_password_alias,
        interface_probe_db_password_alias_set: probe_alias_set,
        interface_probe_config_version: 1,
        status: "testing".into(),
        config_version: 1,
        first_activated_at: None,
        last_tested_version: None,
        last_tested_at: None,
        last_tested_succeeded: None,
        last_tested_error_summary: None,
        created_at: now,
        updated_at: now,
    };
    ensure_inbound_api_key_scopes(&state, ctx.owner_id, &connector).await?;
    let saved = state.repository.insert(&connector).await?;
    write_audit(&state, &ctx, "h8_connector_create", &saved, None).await;
    store_idempotent_response(
        &state,
        ctx.owner_id,
        &idem,
        &hash,
        "POST",
        "/api/v1/config/erp-connectors",
        StatusCode::CREATED,
        &saved,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(saved)))
}

async fn update_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpdateH8ErpConnectorRequest>,
) -> Result<Json<H8ErpConnector>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let idem = idempotency_key(&headers)?;
    let hash = request_hash(&(&id, &req))?;
    if let Some((_status, body)) =
        load_idempotent_response(&state, ctx.owner_id, &idem, &hash).await?
    {
        let connector: H8ErpConnector = serde_json::from_value(body).map_err(|e| {
            H8ErpConnectorHandlerError::Repo(H8ErpConnectorRepoError::Db(e.to_string()))
        })?;
        return Ok(Json(connector));
    }
    let current = state.repository.get(ctx.owner_id, id).await?;
    let observed = current.config_version;
    let observed_probe = current.interface_probe_config_version;
    let before = audit_snapshot(&current);
    let next = apply_update(&current, &req, Utc::now()).map_err(H8ErpConnectorRepoError::Domain)?;
    ensure_inbound_api_key_scopes(&state, ctx.owner_id, &next).await?;
    let saved = state
        .repository
        .save(&next, observed, observed_probe)
        .await?;
    write_audit(&state, &ctx, "h8_connector_update", &saved, Some(before)).await;
    store_idempotent_response(
        &state,
        ctx.owner_id,
        &idem,
        &hash,
        "PATCH",
        &format!("/api/v1/config/erp-connectors/{id}"),
        StatusCode::OK,
        &saved,
    )
    .await?;
    Ok(Json(saved))
}

async fn test_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<H8ErpConnectorTestResult>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let idem = idempotency_key(&headers)?;
    let hash = request_hash(&("test", id))?;
    if let Some((_status, body)) =
        load_idempotent_response(&state, ctx.owner_id, &idem, &hash).await?
    {
        let result: H8ErpConnectorTestResult = serde_json::from_value(body).map_err(|e| {
            H8ErpConnectorHandlerError::Repo(H8ErpConnectorRepoError::Db(e.to_string()))
        })?;
        return Ok(Json(result));
    }
    let mut connector = state.repository.get(ctx.owner_id, id).await?;
    let observed = connector.config_version;
    let observed_probe = connector.interface_probe_config_version;
    // AC7：字段/alias、路由重叠、REST/接口表连通性；不写业务单据
    let (mut ok, mut err) = run_connection_probe(&connector).await;
    if ok {
        if let Err(scope_err) =
            ensure_inbound_api_key_scopes(&state, ctx.owner_id, &connector).await
        {
            ok = false;
            err = Some(format!("{scope_err:?}"));
        }
    }
    if ok {
        let actives = state.repository.list_active(ctx.owner_id).await?;
        if let Err(e) = reject_route_overlap_with_actives(&connector, &actives) {
            ok = false;
            err = Some(format!("{e:?}"));
        }
    }
    let now = Utc::now();
    let before = audit_snapshot(&connector);
    connector.last_tested_version = Some(connector.config_version);
    connector.last_tested_at = Some(now);
    connector.last_tested_succeeded = Some(ok);
    connector.last_tested_error_summary = err.clone();
    connector.updated_at = now;
    let saved = state
        .repository
        .save(&connector, observed, observed_probe)
        .await?;
    write_audit(&state, &ctx, "h8_connector_test", &saved, Some(before)).await;
    let result = H8ErpConnectorTestResult {
        succeeded: ok,
        error_summary: err,
        tested_version: connector.config_version,
        tested_at: now,
    };
    store_idempotent_response(
        &state,
        ctx.owner_id,
        &idem,
        &hash,
        "POST",
        &format!("/api/v1/config/erp-connectors/{id}/test"),
        StatusCode::OK,
        &result,
    )
    .await?;
    Ok(Json(result))
}

async fn activate_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<H8ErpConnector>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let idem = idempotency_key(&headers)?;
    let hash = request_hash(&("activate", id))?;
    if let Some((_status, body)) =
        load_idempotent_response(&state, ctx.owner_id, &idem, &hash).await?
    {
        let connector: H8ErpConnector = serde_json::from_value(body).map_err(|e| {
            H8ErpConnectorHandlerError::Repo(H8ErpConnectorRepoError::Db(e.to_string()))
        })?;
        return Ok(Json(connector));
    }
    let mut connector = state.repository.get(ctx.owner_id, id).await?;
    let observed = connector.config_version;
    let observed_probe = connector.interface_probe_config_version;
    let actives = state.repository.list_active(ctx.owner_id).await?;
    can_activate(&connector, &actives).map_err(H8ErpConnectorRepoError::Domain)?;
    ensure_inbound_api_key_scopes(&state, ctx.owner_id, &connector).await?;
    let before = audit_snapshot(&connector);
    let now = Utc::now();
    if connector.first_activated_at.is_none() {
        connector.first_activated_at = Some(now);
    }
    connector.status = "active".into();
    connector.updated_at = now;
    let saved = state
        .repository
        .save(&connector, observed, observed_probe)
        .await?;
    // AC12：原连接重新启用后，paused 在途从原阶段续传（恢复 running）
    let resumed = state
        .repository
        .resume_inflight(ctx.owner_id, connector.id)
        .await
        .unwrap_or(0);
    write_audit(&state, &ctx, "h8_connector_activate", &saved, Some(before)).await;
    if resumed > 0 {
        write_audit(&state, &ctx, "h8_connector_inflight_resume", &saved, None).await;
    }
    store_idempotent_response(
        &state,
        ctx.owner_id,
        &idem,
        &hash,
        "POST",
        &format!("/api/v1/config/erp-connectors/{id}/activate"),
        StatusCode::OK,
        &saved,
    )
    .await?;
    Ok(Json(saved))
}

async fn disable_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<H8ErpConnector>, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let idem = idempotency_key(&headers)?;
    let hash = request_hash(&("disable", id))?;
    if let Some((_status, body)) =
        load_idempotent_response(&state, ctx.owner_id, &idem, &hash).await?
    {
        let connector: H8ErpConnector = serde_json::from_value(body).map_err(|e| {
            H8ErpConnectorHandlerError::Repo(H8ErpConnectorRepoError::Db(e.to_string()))
        })?;
        return Ok(Json(connector));
    }
    let mut connector = state.repository.get(ctx.owner_id, id).await?;
    let observed = connector.config_version;
    let observed_probe = connector.interface_probe_config_version;
    if connector.status != "active" {
        return Err(H8ErpConnectorHandlerError::Repo(
            H8ErpConnectorRepoError::Domain(H8ErpConnectorError::IllegalTransition),
        ));
    }
    let before = audit_snapshot(&connector);
    state
        .repository
        .pause_inflight(ctx.owner_id, connector.id)
        .await?;
    connector.status = "disabled".into();
    connector.updated_at = Utc::now();
    let saved = state
        .repository
        .save(&connector, observed, observed_probe)
        .await?;
    write_audit(&state, &ctx, "h8_connector_disable", &saved, Some(before)).await;
    store_idempotent_response(
        &state,
        ctx.owner_id,
        &idem,
        &hash,
        "POST",
        &format!("/api/v1/config/erp-connectors/{id}/disable"),
        StatusCode::OK,
        &saved,
    )
    .await?;
    Ok(Json(saved))
}

async fn delete_connector(
    ctx: AuthContext,
    State(state): State<H8ErpConnectorAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<StatusCode, H8ErpConnectorHandlerError> {
    ctx.require_permission(H8_CONFIG_WRITE)?;
    let _idem = idempotency_key(&headers)?;
    let connector = state.repository.get(ctx.owner_id, id).await?;
    let refs = state
        .repository
        .has_inflight_refs(ctx.owner_id, connector.id)
        .await?;
    if !can_physically_delete(&connector, refs) {
        return Err(H8ErpConnectorHandlerError::Repo(
            H8ErpConnectorRepoError::Domain(H8ErpConnectorError::DeleteNotAllowed),
        ));
    }
    let before = audit_snapshot(&connector);
    state.repository.delete(ctx.owner_id, id).await?;
    write_audit(
        &state,
        &ctx,
        "h8_connector_delete",
        &connector,
        Some(before),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
