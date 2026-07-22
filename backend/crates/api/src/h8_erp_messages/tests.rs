//! H8 消息内存仓储、重放与 H2 审计 sink 测试。

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;
use wms_domain::{
    audit_summary_is_safe, message_audit_summary, H8ErpMessage, H8ErpMessageAttempt,
    H8ErpMessageListResponse, H8WorkerHeartbeatRequest, SetH8WorkerClaimControlRequest,
    H8_MESSAGE_DEAD_AUDIT_ACTION,
};

use crate::audit::AuditLog;
use crate::auth::AuthContext;

use super::audit::{
    snapshot_audit_actions, write_dead_entry_audit, write_exchange_lifecycle_audit,
    write_message_audit, write_owner_audit,
};
use super::repository::MemoryH8ErpMessageRepository;
use super::state::H8ErpMessageAppState;

pub(super) fn sample_message(owner: Uuid, status: &str) -> H8ErpMessage {
    let now = Utc::now();
    H8ErpMessage {
        id: Uuid::new_v4(),
        owner_id: owner,
        warehouse_id: None,
        connector_id: Some(Uuid::nil()),
        connector_code: Some("demo".into()),
        config_version: Some(1),
        direction: "inbound".into(),
        message_type: "asn".into(),
        schema_version: "1".into(),
        channel: "rest".into(),
        external_ref: "ERP-ASN-1".into(),
        wms_resource_id: None,
        idempotency_key: "idem-1".into(),
        correlation_id: "corr-1".into(),
        sync_status: status.into(),
        retry_count: 2,
        next_retry_at: None,
        last_error_summary: Some("mapping failed".into()),
        payload_digest: "digest".into(),
        claimed_by: None,
        lease_expires_at: None,
        created_at: now,
        updated_at: now,
        completed_at: None,
        acked_at: None,
    }
}

pub(super) fn test_ctx(owner: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::nil(),
        owner_id: owner,
        actor_name: "tester".into(),
        permissions: vec![
            "h8.erp_connector.read".into(),
            "h8.erp_connector.write".into(),
        ],
        jti: "jti-test".into(),
        warehouse_scope: None,
    }
}

#[tokio::test]
async fn list_filters_by_status_and_stats() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let dead = sample_message(owner, "dead");
    let ok = sample_message(owner, "succeeded");
    state.repository.upsert_for_test(&dead).await.unwrap();
    state.repository.upsert_for_test(&ok).await.unwrap();
    let listed = state
        .repository
        .list(
            owner,
            None,
            None,
            Some("dead"),
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            200,
        )
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].sync_status, "dead");
    let stats = state
        .repository
        .stats(owner, None, None, None)
        .await
        .unwrap();
    assert_eq!(stats.total, 2);
    assert_eq!(stats.dead, 1);
    assert_eq!(stats.succeeded, 1);
}

#[tokio::test]
async fn list_filters_by_warehouse_and_trace_keys() {
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
            Some(warehouse),
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
async fn list_cursor_is_stable_when_created_at_matches() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let created_at = Utc::now();
    for value in 1..=3 {
        let mut message = sample_message(owner, "pending");
        message.id = Uuid::from_u128(value);
        message.created_at = created_at;
        state.repository.upsert_for_test(&message).await.unwrap();
    }

    let mut request = Request::builder()
        .uri("/api/v1/integration/erp-messages?limit=2")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(test_ctx(owner));
    let response = super::handlers::h8_erp_message_router(state.clone())
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let first: H8ErpMessageListResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        first
            .data
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![Uuid::from_u128(3), Uuid::from_u128(2)]
    );
    let cursor = first.page.next_cursor.expect("next cursor");

    let mut request = Request::builder()
        .uri(format!(
            "/api/v1/integration/erp-messages?limit=2&cursor={cursor}"
        ))
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(test_ctx(owner));
    let response = super::handlers::h8_erp_message_router(state)
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let second: H8ErpMessageListResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(second.data.len(), 1);
    assert_eq!(second.data[0].id, Uuid::from_u128(1));
    assert!(second.page.next_cursor.is_none());
}

#[tokio::test]
async fn stats_filter_by_connector_channel_and_message_type() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let mut selected = sample_message(owner, "succeeded");
    selected.connector_code = Some("SELF-ERP".into());
    selected.channel = "interface_table".into();
    selected.message_type = "asn".into();
    let mut other = sample_message(owner, "dead");
    other.connector_code = Some("OTHER-ERP".into());
    other.channel = "rest".into();
    other.message_type = "outbound_order".into();
    state.repository.upsert_for_test(&selected).await.unwrap();
    state.repository.upsert_for_test(&other).await.unwrap();

    let stats = state
        .repository
        .stats(
            owner,
            Some("SELF-ERP"),
            Some("interface_table"),
            Some("asn"),
        )
        .await
        .unwrap();
    assert_eq!(stats.total, 1);
    assert_eq!(stats.succeeded, 1);
    assert_eq!(stats.dead, 0);
}

#[tokio::test]
async fn list_rejects_invalid_query_values_at_http_boundary() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let mut request = Request::builder()
        .uri("/api/v1/integration/erp-messages?direction=sideways")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(test_ctx(owner));
    let response = super::handlers::h8_erp_message_router(state)
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_rejects_invalid_cursor_and_limit() {
    for uri in [
        "/api/v1/integration/erp-messages?cursor=broken",
        "/api/v1/integration/erp-messages?limit=0",
        "/api/v1/integration/erp-messages?limit=201",
    ] {
        let state = H8ErpMessageAppState::with_memory();
        let owner = Uuid::new_v4();
        let mut request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        request.extensions_mut().insert(test_ctx(owner));
        let response = super::handlers::h8_erp_message_router(state)
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{uri}");
    }
}

#[tokio::test]
async fn preflight_schema_failure_still_records_receive_and_final_failure() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    for stage in ["receive", "final_failure"] {
        let body = serde_json::json!({
            "stage": stage,
            "result": "preflight_rejected",
            "direction": "inbound",
            "message_type": "asn",
            "schema_version": "999",
            "external_ref": "ERP-BAD-SCHEMA-1",
            "idempotency_key": "idem-bad-schema-1",
            "correlation_id": "corr-bad-schema-1",
            "channel": "interface_table"
        });
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/integration/erp-messages/lifecycle")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        request.extensions_mut().insert(test_ctx(owner));
        let response = super::handlers::h8_erp_message_router(state.clone())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let actions = snapshot_audit_actions(&state);
    assert!(actions.iter().any(|action| action == "h8_exchange_receive"));
    assert!(actions
        .iter()
        .any(|action| action == "h8_exchange_final_failure"));
}

#[tokio::test]
async fn invalid_lifecycle_stage_is_rejected_before_message_insert() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let body = serde_json::json!({
        "stage": "free_text",
        "result": "ok",
        "direction": "inbound",
        "message_type": "asn",
        "schema_version": "1",
        "external_ref": "ERP-INVALID-STAGE-1",
        "idempotency_key": Uuid::new_v4().to_string(),
        "correlation_id": "corr-invalid-stage-1",
        "channel": "interface_table"
    });
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integration/erp-messages/lifecycle")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    request.extensions_mut().insert(test_ctx(owner));
    let response = super::handlers::h8_erp_message_router(state.clone())
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(state
        .repository
        .find_by_idempotency(owner, "asn", "ERP-INVALID-STAGE-1", "idem-invalid-stage-1",)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn lifecycle_rejects_changes_to_existing_message_binding() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let message = sample_message(owner, "processing");
    state.repository.upsert_for_test(&message).await.unwrap();
    let base = serde_json::json!({
        "stage": "receive",
        "result": "retry",
        "direction": message.direction,
        "message_type": message.message_type,
        "schema_version": message.schema_version,
        "external_ref": message.external_ref,
        "idempotency_key": message.idempotency_key,
        "correlation_id": message.correlation_id,
        "channel": message.channel,
        "connector_id": message.connector_id,
        "connector_code": message.connector_code,
        "config_version": message.config_version
    });

    for (field, changed) in [
        ("connector_id", serde_json::json!(Uuid::new_v4())),
        ("config_version", serde_json::json!(2)),
        ("channel", serde_json::json!("interface_table")),
        ("direction", serde_json::json!("outbound")),
        ("schema_version", serde_json::json!("999")),
    ] {
        let mut body = base.clone();
        body[field] = changed;
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/integration/erp-messages/lifecycle")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        request.extensions_mut().insert(test_ctx(owner));
        let response = super::handlers::h8_erp_message_router(state.clone())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{field}");
    }
    assert!(snapshot_audit_actions(&state).is_empty());
}

#[tokio::test]
async fn replay_failed_adds_attempt_and_sets_processing() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "failed");
    let id = msg.id;
    state.repository.upsert_for_test(&msg).await.unwrap();
    let replayed = state
        .repository
        .replay(owner, id, "manual fix", "admin", Utc::now())
        .await
        .unwrap();
    assert_eq!(replayed.sync_status, "processing");
    let attempts = state.repository.list_attempts(owner, id).await.unwrap();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].result, "replayed");
}

#[tokio::test]
async fn replay_succeeded_rejected() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "succeeded");
    let id = msg.id;
    state.repository.upsert_for_test(&msg).await.unwrap();
    let err = state
        .repository
        .replay(owner, id, "nope", "admin", Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        super::error::H8ErpMessageRepoError::Domain(wms_domain::H8MessageError::ReplayNotAllowed)
    ));
}

#[tokio::test]
async fn claim_pending_sets_lease_and_attempt() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "pending");
    let id = msg.id;
    state.repository.upsert_for_test(&msg).await.unwrap();
    let claimed = state
        .repository
        .claim(owner, id, "worker-1", 60, Utc::now())
        .await
        .unwrap();
    assert_eq!(claimed.sync_status, "processing");
    assert_eq!(claimed.claimed_by.as_deref(), Some("worker-1"));
    assert!(claimed.lease_expires_at.is_some());
    let attempts = state.repository.list_attempts(owner, id).await.unwrap();
    assert_eq!(attempts[0].result, "claimed");
}

#[tokio::test]
async fn claim_under_active_lease_conflicts() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let mut msg = sample_message(owner, "processing");
    msg.lease_expires_at = Some(Utc::now() + chrono::Duration::minutes(5));
    let id = msg.id;
    state.repository.upsert_for_test(&msg).await.unwrap();
    let err = state
        .repository
        .claim(owner, id, "worker-2", 60, Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        super::error::H8ErpMessageRepoError::Domain(wms_domain::H8MessageError::LeaseConflict)
    ));
}

#[tokio::test]
async fn worker_heartbeat_and_pause_control_gate_claims() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    let now = Utc::now();
    let status = state
        .runtime_repository
        .record_heartbeat(
            owner,
            &H8WorkerHeartbeatRequest {
                worker_id: "worker-1".into(),
                worker_version: "1.0.0".into(),
                connector_id,
                directions: vec!["inbound".into()],
                current_claims: 2,
                heartbeat_ttl_seconds: 30,
            },
            now,
        )
        .await
        .unwrap();
    assert_eq!(status.health, "healthy");
    assert_eq!(status.current_claims, 2);
    assert_eq!(status.created_at, now);

    let refreshed = state
        .runtime_repository
        .record_heartbeat(
            owner,
            &H8WorkerHeartbeatRequest {
                worker_id: "worker-1".into(),
                worker_version: "1.0.1".into(),
                connector_id,
                directions: vec!["inbound".into()],
                current_claims: 0,
                heartbeat_ttl_seconds: 30,
            },
            now + chrono::Duration::seconds(1),
        )
        .await
        .unwrap();
    assert_eq!(refreshed.created_at, now);
    assert_eq!(
        refreshed.last_heartbeat_at,
        now + chrono::Duration::seconds(1)
    );

    let control = state
        .runtime_repository
        .set_claim_control(
            owner,
            &SetH8WorkerClaimControlRequest {
                connector_id,
                direction: "inbound".into(),
                paused: true,
                reason: "ERP 维护".into(),
                paused_until: Some(now + chrono::Duration::minutes(5)),
                confirmed: true,
            },
            "admin",
            now,
        )
        .await
        .unwrap();
    assert!(control.paused);
    let decision = state
        .runtime_repository
        .claim_decision(owner, connector_id, "inbound", now)
        .await
        .unwrap();
    assert!(!decision.allowed);
    assert_eq!(decision.reason.as_deref(), Some("ERP 维护"));

    let expired = state
        .runtime_repository
        .claim_decision(
            owner,
            connector_id,
            "inbound",
            now + chrono::Duration::minutes(6),
        )
        .await
        .unwrap();
    assert!(expired.allowed);
}

#[tokio::test]
async fn pause_handler_requires_write_permission_and_writes_audit() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    let body = serde_json::json!({
        "connector_id": connector_id,
        "direction": "inbound",
        "paused": true,
        "reason": "ERP 维护",
        "paused_until": null,
        "confirmed": true
    });
    let mut denied = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integration/erp-messages/worker-runtime/control")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let mut readonly = test_ctx(owner);
    readonly.permissions = vec!["h8.erp_connector.read".into()];
    denied.extensions_mut().insert(readonly);
    let denied = super::handlers::h8_erp_message_router(state.clone())
        .oneshot(denied)
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let mut allowed = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integration/erp-messages/worker-runtime/control")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    allowed.extensions_mut().insert(test_ctx(owner));
    let response = super::handlers::h8_erp_message_router(state.clone())
        .oneshot(allowed)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(snapshot_audit_actions(&state)
        .iter()
        .any(|action| action == "h8_worker_claim_pause"));
}

#[tokio::test]
async fn purge_requires_retention_policy() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let err = state
        .repository
        .purge_terminal(owner, None, Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        super::error::H8ErpMessageRepoError::Domain(wms_domain::H8MessageError::FieldRequired(
            "retention_days"
        ))
    ));
}

#[tokio::test]
async fn purge_terminal_only_when_retention_set() {
    let memory = Arc::new(MemoryH8ErpMessageRepository::default());
    memory.set_retention_for_test(Uuid::nil(), 7);
    let state = H8ErpMessageAppState {
        repository: memory.clone(),
        runtime_repository: Arc::new(
            super::runtime_repository::MemoryH8WorkerRuntimeRepository::default(),
        ),
        payload_repository: Arc::new(
            super::payload_repository::MemoryH8PayloadRepository::default(),
        ),
        audit_pool: None,
        audit_log: Arc::new(Mutex::new(AuditLog::default())),
    };
    let owner = Uuid::nil();
    let mut old = sample_message(owner, "succeeded");
    old.updated_at = Utc::now() - chrono::Duration::days(30);
    let keep = sample_message(owner, "failed");
    state.repository.upsert_for_test(&old).await.unwrap();
    state.repository.upsert_for_test(&keep).await.unwrap();
    let (deleted, days) = state
        .repository
        .purge_terminal(owner, Some(7), Utc::now())
        .await
        .unwrap();
    assert_eq!(days, 7);
    assert_eq!(deleted, 1);
    assert!(state.repository.get(owner, keep.id).await.is_ok());
}

#[tokio::test]
async fn write_message_audit_records_to_sink_on_replay_path() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "failed");
    state.repository.upsert_for_test(&msg).await.unwrap();
    let ctx = test_ctx(owner);
    let replayed = state
        .repository
        .replay(owner, msg.id, "fix", "admin", Utc::now())
        .await
        .unwrap();
    // 真实 shipped 审计入口（与 handlers 相同）
    write_message_audit(&state, &ctx, "h8_message_replay", &replayed, "accepted")
        .await
        .unwrap();
    let actions = snapshot_audit_actions(&state);
    assert!(
        actions.iter().any(|a| a == "h8_message_replay"),
        "expected h8_message_replay in {actions:?}"
    );
    let log = state.audit_log.lock().expect("log");
    let event = log
        .events()
        .iter()
        .find(|e| e.action == "h8_message_replay")
        .expect("event");
    assert_eq!(event.module, "H8");
    assert_eq!(event.resource_type, "h8_erp_message");
    assert_eq!(event.resource_id, replayed.id.to_string());
    let diff = event.diff.as_ref().expect("diff");
    let after = &diff.after;
    assert!(audit_summary_is_safe(after));
    assert!(after.get("payload").is_some_and(|v| v.is_null()));
}

#[tokio::test]
async fn mark_dead_writes_h2_dead_audit_action() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "failed");
    state.repository.upsert_for_test(&msg).await.unwrap();
    let ctx = test_ctx(owner);
    let dead = state
        .repository
        .mark_dead(owner, msg.id, "auth: invalid", "worker", Utc::now())
        .await
        .unwrap();
    assert_eq!(dead.sync_status, "dead");
    write_dead_entry_audit(&state, &ctx, &dead).await.unwrap();
    let actions = snapshot_audit_actions(&state);
    assert!(actions.iter().any(|a| a == H8_MESSAGE_DEAD_AUDIT_ACTION));
}

#[tokio::test]
async fn detail_query_and_purge_and_archive_write_audit() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "succeeded");
    state.repository.upsert_for_test(&msg).await.unwrap();
    let ctx = test_ctx(owner);
    write_message_audit(&state, &ctx, "h8_message_detail_query", &msg, "viewed")
        .await
        .unwrap();
    let archived = state
        .repository
        .mark_archived(owner, msg.id, "admin", Utc::now())
        .await
        .unwrap();
    write_message_audit(&state, &ctx, "h8_message_archive", &archived, "archived")
        .await
        .unwrap();
    write_owner_audit(
        &state,
        &ctx,
        "h8_message_purge",
        serde_json::json!({"deleted": 0, "payload": null}),
    )
    .await
    .unwrap();
    let actions = snapshot_audit_actions(&state);
    assert!(actions.iter().any(|a| a == "h8_message_detail_query"));
    assert!(actions.iter().any(|a| a == "h8_message_archive"));
    assert!(actions.iter().any(|a| a == "h8_message_purge"));
}

#[tokio::test]
async fn exchange_lifecycle_stages_write_audit_actions() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "processing");
    state.repository.upsert_for_test(&msg).await.unwrap();
    let ctx = test_ctx(owner);
    for stage in [
        "receive",
        "convert",
        "business_api",
        "send",
        "receipt",
        "final_failure",
    ] {
        write_exchange_lifecycle_audit(&state, &ctx, &msg, stage, "ok")
            .await
            .unwrap();
    }
    let actions = snapshot_audit_actions(&state);
    for stage in [
        "receive",
        "convert",
        "business_api",
        "send",
        "receipt",
        "final_failure",
    ] {
        let expected = format!("h8_exchange_{stage}");
        assert!(
            actions.iter().any(|a| a == &expected),
            "missing {expected} in {actions:?}"
        );
    }
}

#[test]
fn message_audit_summary_is_safe() {
    let msg = sample_message(Uuid::nil(), "failed");
    let summary = message_audit_summary(
        "h8_message_replay",
        msg.id,
        msg.owner_id,
        &msg.message_type,
        &msg.external_ref,
        &msg.idempotency_key,
        &msg.correlation_id,
        &msg.sync_status,
        msg.connector_id,
        msg.config_version,
        "accepted",
    );
    assert!(audit_summary_is_safe(&summary));
}

#[tokio::test]
async fn stats_include_p95_from_attempts() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "succeeded");
    let id = msg.id;
    state.repository.upsert_for_test(&msg).await.unwrap();
    let started = Utc::now() - chrono::Duration::milliseconds(200);
    let finished = Utc::now();
    state
        .repository
        .append_attempt_for_test(&H8ErpMessageAttempt {
            id: Uuid::new_v4(),
            message_id: id,
            attempt_no: 1,
            channel: "rest".into(),
            started_at: started,
            finished_at: Some(finished),
            result: "succeeded".into(),
            error_summary: None,
            actor: "worker".into(),
        })
        .await
        .unwrap();
    let stats = state
        .repository
        .stats(owner, None, None, None)
        .await
        .unwrap();
    assert!(stats.p95_latency_ms >= 100);
}
