//! H8 消息仓库范围授权。

use axum::{extract::Query, extract::State, Json};
use serde::Deserialize;
use uuid::Uuid;
use wms_domain::{H8ErpMessage, H8ErpMessageStats};

use crate::{
    auth::{AuthContext, AuthError},
    warehouse_scope::load_user_warehouse_scopes,
};

use super::{
    error::H8ErpMessageHandlerError,
    state::{H8ErpMessageAppState, H8_MSG_READ, H8_MSG_WRITE},
};

#[derive(Debug, Deserialize)]
pub(super) struct StatsQuery {
    connector_code: Option<String>,
    channel: Option<String>,
    message_type: Option<String>,
}

pub(super) async fn authorized_warehouse_ids(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    requested: Option<Uuid>,
) -> Result<Option<Vec<Uuid>>, H8ErpMessageHandlerError> {
    if let Some(scope) = ctx.warehouse_scope {
        if requested.is_some_and(|warehouse_id| warehouse_id != scope) {
            return Err(AuthError::PermissionDenied("warehouse scope".into()).into());
        }
        return Ok(Some(vec![scope]));
    }
    let pool = state
        .audit_pool
        .as_ref()
        .ok_or_else(|| AuthError::PermissionDenied("warehouse scope".into()));
    let scopes = match pool {
        Ok(pool) => load_user_warehouse_scopes(pool, ctx)
            .await
            .map_err(|error| super::error::H8ErpMessageRepoError::Db(error.to_string()))?,
        Err(_) if ctx.has_permission(H8_MSG_WRITE) => {
            return Ok(requested.map(|warehouse_id| vec![warehouse_id]));
        }
        Err(error) => return Err(error.into()),
    };
    if scopes.is_empty() && ctx.has_permission(H8_MSG_WRITE) {
        return Ok(requested.map(|warehouse_id| vec![warehouse_id]));
    }
    if scopes.is_empty() || requested.is_some_and(|warehouse_id| !scopes.contains(&warehouse_id)) {
        return Err(AuthError::PermissionDenied("warehouse scope".into()).into());
    }
    Ok(Some(
        requested.map_or(scopes, |warehouse_id| vec![warehouse_id]),
    ))
}

pub(super) async fn require_message_warehouse_scope(
    state: &H8ErpMessageAppState,
    ctx: &AuthContext,
    message: &H8ErpMessage,
) -> Result<(), H8ErpMessageHandlerError> {
    let scopes = authorized_warehouse_ids(state, ctx, None).await?;
    if scopes.as_ref().is_none_or(|values| {
        message
            .warehouse_id
            .is_some_and(|warehouse_id| values.contains(&warehouse_id))
    }) {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied("warehouse scope".into()).into())
    }
}

pub(super) async fn message_stats(
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
            .map_err(super::error::H8ErpMessageRepoError::Domain)?;
    }
    if let Some(message_type) = message_type {
        wms_domain::validate_message_type_in_catalog(message_type)
            .map_err(super::error::H8ErpMessageRepoError::Domain)?;
    }
    let warehouse_ids = authorized_warehouse_ids(&state, &ctx, None).await?;
    Ok(Json(
        state
            .repository
            .stats(
                ctx.owner_id,
                connector_code,
                channel,
                message_type,
                warehouse_ids.as_deref(),
            )
            .await?,
    ))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{Method, Request, StatusCode},
    };
    use tower::ServiceExt;
    use uuid::Uuid;
    use wms_domain::{H8ErpMessage, H8ErpMessageListResponse};

    use super::super::{
        handlers::h8_erp_message_router,
        state::H8ErpMessageAppState,
        tests::{sample_message, test_ctx},
    };

    #[tokio::test]
    async fn repository_list_filters_by_warehouse_and_trace_keys() {
        let state = H8ErpMessageAppState::with_memory();
        let owner = Uuid::new_v4();
        let warehouse = Uuid::new_v4();
        let mut selected = sample_message(owner, "failed");
        selected.warehouse_id = Some(warehouse);
        selected.external_ref = "ERP-SELECTED".into();
        selected.idempotency_key = "idem-selected".into();
        selected.correlation_id = "corr-selected".into();
        let other = sample_message(owner, "failed");
        state.repository.upsert_for_test(&selected).await.unwrap();
        state.repository.upsert_for_test(&other).await.unwrap();

        let listed = state
            .repository
            .list(
                owner,
                None,
                None,
                None,
                None,
                None,
                None,
                false,
                Some(&[warehouse]),
                Some("ERP-SELECTED"),
                Some("idem-selected"),
                Some("corr-selected"),
                Some(selected.created_at - chrono::Duration::seconds(1)),
                Some(selected.created_at + chrono::Duration::seconds(1)),
                None,
                200,
            )
            .await
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, selected.id);
    }

    #[tokio::test]
    async fn api_key_scope_limits_reads_actions_and_new_lifecycle_messages() {
        let state = H8ErpMessageAppState::with_memory();
        let owner = Uuid::new_v4();
        let allowed_warehouse = Uuid::new_v4();
        let denied_warehouse = Uuid::new_v4();
        let mut allowed = sample_message(owner, "failed");
        allowed.warehouse_id = Some(allowed_warehouse);
        let mut denied = sample_message(owner, "failed");
        denied.warehouse_id = Some(denied_warehouse);
        state.repository.upsert_for_test(&allowed).await.unwrap();
        state.repository.upsert_for_test(&denied).await.unwrap();
        let mut ctx = test_ctx(owner);
        ctx.warehouse_scope = Some(allowed_warehouse);

        let mut request = Request::builder()
            .uri("/api/v1/integration/erp-messages")
            .body(Body::empty())
            .unwrap();
        request.extensions_mut().insert(ctx.clone());
        let response = h8_erp_message_router(state.clone())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let listed: H8ErpMessageListResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(listed.data.len(), 1);
        assert_eq!(listed.data[0].id, allowed.id);

        for uri in [
            format!("/api/v1/integration/erp-messages?warehouse_id={denied_warehouse}"),
            format!("/api/v1/integration/erp-messages/{}", denied.id),
        ] {
            let mut request = Request::builder().uri(uri).body(Body::empty()).unwrap();
            request.extensions_mut().insert(ctx.clone());
            let response = h8_erp_message_router(state.clone())
                .oneshot(request)
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }

        let mut request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/api/v1/integration/erp-messages/{}/replay",
                denied.id
            ))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"reason":"retry","confirmed":true}"#))
            .unwrap();
        request.extensions_mut().insert(ctx.clone());
        let response = h8_erp_message_router(state.clone())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/integration/erp-messages/purge")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"confirmed":true}"#))
            .unwrap();
        request.extensions_mut().insert(ctx.clone());
        let response = h8_erp_message_router(state.clone())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let lifecycle = serde_json::json!({
            "stage": "receive",
            "result": "ok",
            "direction": "inbound",
            "message_type": "asn",
            "schema_version": "1",
            "external_ref": "ERP-SCOPED-LIFECYCLE",
            "idempotency_key": "idem-scoped-lifecycle",
            "correlation_id": "corr-scoped-lifecycle",
            "channel": "interface_table"
        });
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/integration/erp-messages/lifecycle")
            .header("content-type", "application/json")
            .body(Body::from(lifecycle.to_string()))
            .unwrap();
        request.extensions_mut().insert(ctx);
        let response = h8_erp_message_router(state).oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created: H8ErpMessage = serde_json::from_slice(&body).unwrap();
        assert_eq!(created.warehouse_id, Some(allowed_warehouse));
    }
}
