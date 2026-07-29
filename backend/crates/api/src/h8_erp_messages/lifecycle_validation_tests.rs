//! H8 生命周期输入边界测试。

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;
use uuid::Uuid;

use super::{
    audit::snapshot_audit_actions,
    handlers::h8_erp_message_router,
    state::H8ErpMessageAppState,
    tests::{sample_message, test_ctx},
};

fn lifecycle_body(
    message: &wms_domain::H8ErpMessage,
    stage: &str,
    result: &str,
) -> serde_json::Value {
    serde_json::json!({
        "stage": stage,
        "result": result,
        "direction": message.direction,
        "message_type": message.message_type,
        "schema_version": message.schema_version,
        "external_ref": message.external_ref,
        "idempotency_key": message.idempotency_key,
        "correlation_id": message.correlation_id,
        "channel": message.channel,
        "connector_id": message.connector_id,
        "connector_code": message.connector_code,
        "config_version": message.config_version,
        "message_id": message.id
    })
}

async fn post_lifecycle(
    state: H8ErpMessageAppState,
    owner: Uuid,
    body: serde_json::Value,
) -> StatusCode {
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integration/erp-messages/lifecycle")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    request.extensions_mut().insert(test_ctx(owner));
    h8_erp_message_router(state)
        .oneshot(request)
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn invalid_lifecycle_result_is_rejected_before_message_insert() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let idempotency_key = Uuid::new_v4().to_string();
    let body = serde_json::json!({
        "stage": "send",
        "result": "Bearer must-not-enter-audit",
        "direction": "outbound",
        "message_type": "putaway_complete",
        "schema_version": "1",
        "external_ref": "ERP-INVALID-RESULT-1",
        "idempotency_key": idempotency_key,
        "correlation_id": "corr-invalid-result-1",
        "channel": "rest"
    });
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integration/erp-messages/lifecycle")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    request.extensions_mut().insert(test_ctx(owner));
    let response = h8_erp_message_router(state.clone())
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(state
        .repository
        .find_by_idempotency(
            owner,
            "putaway_complete",
            "ERP-INVALID-RESULT-1",
            &idempotency_key,
        )
        .await
        .unwrap()
        .is_none());
    assert!(snapshot_audit_actions(&state).is_empty());
}

#[tokio::test]
async fn direction_and_message_type_must_form_a_catalogued_pair() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let idempotency_key = Uuid::new_v4().to_string();
    let body = serde_json::json!({
        "stage": "receive",
        "result": "ok",
        "direction": "outbound",
        "message_type": "asn",
        "schema_version": "1",
        "external_ref": "ERP-INVALID-DIRECTION-1",
        "idempotency_key": idempotency_key,
        "correlation_id": "corr-invalid-direction-1",
        "channel": "rest"
    });

    let status = post_lifecycle(state.clone(), owner, body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(state
        .repository
        .find_by_idempotency(owner, "asn", "ERP-INVALID-DIRECTION-1", &idempotency_key,)
        .await
        .unwrap()
        .is_none());
    assert!(snapshot_audit_actions(&state).is_empty());
}

#[tokio::test]
async fn existing_message_requires_its_frozen_connector_binding_in_request() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let message = sample_message(owner, "processing");
    state.repository.upsert_for_test(&message).await.unwrap();
    let mut body = lifecycle_body(&message, "receive", "ok");
    let object = body.as_object_mut().unwrap();
    object.remove("connector_id");
    object.remove("config_version");

    let status = post_lifecycle(state.clone(), owner, body).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(snapshot_audit_actions(&state).is_empty());
}

#[tokio::test]
async fn invalid_stage_state_combination_returns_conflict_without_audit() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let message = sample_message(owner, "processing");
    state.repository.upsert_for_test(&message).await.unwrap();

    let status = post_lifecycle(state.clone(), owner, lifecycle_body(&message, "send", "ok")).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        state
            .repository
            .get(owner, message.id)
            .await
            .unwrap()
            .sync_status,
        "processing"
    );
    assert!(snapshot_audit_actions(&state).is_empty());
}

#[tokio::test]
async fn terminal_same_request_replay_does_not_duplicate_lifecycle_audit() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let message = sample_message(owner, "processing");
    state.repository.upsert_for_test(&message).await.unwrap();
    let body = lifecycle_body(&message, "receipt", "ok");

    assert_eq!(
        post_lifecycle(state.clone(), owner, body.clone()).await,
        StatusCode::OK
    );
    assert_eq!(
        post_lifecycle(state.clone(), owner, body).await,
        StatusCode::OK
    );
    assert_eq!(
        state
            .repository
            .get(owner, message.id)
            .await
            .unwrap()
            .sync_status,
        "succeeded"
    );
    assert_eq!(
        snapshot_audit_actions(&state)
            .iter()
            .filter(|action| action.as_str() == "h8_exchange_receipt")
            .count(),
        1
    );
}

#[tokio::test]
async fn awaiting_receipt_send_replay_is_idempotent_without_duplicate_audit() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let mut message = sample_message(owner, "processing");
    message.direction = "outbound".into();
    message.message_type = "putaway_complete".into();
    state.repository.upsert_for_test(&message).await.unwrap();
    let body = lifecycle_body(&message, "send", "ok");

    assert_eq!(
        post_lifecycle(state.clone(), owner, body.clone()).await,
        StatusCode::OK
    );
    assert_eq!(
        post_lifecycle(state.clone(), owner, body).await,
        StatusCode::OK
    );
    assert_eq!(
        state
            .repository
            .get(owner, message.id)
            .await
            .unwrap()
            .sync_status,
        "awaiting_receipt"
    );
    assert_eq!(
        snapshot_audit_actions(&state)
            .iter()
            .filter(|action| action.as_str() == "h8_exchange_send")
            .count(),
        1
    );
}

#[tokio::test]
async fn processing_receive_records_a_new_attempt_without_changing_status() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let message = sample_message(owner, "processing");
    state.repository.upsert_for_test(&message).await.unwrap();

    assert_eq!(
        post_lifecycle(
            state.clone(),
            owner,
            lifecycle_body(&message, "receive", "ok"),
        )
        .await,
        StatusCode::OK
    );
    assert_eq!(
        state
            .repository
            .get(owner, message.id)
            .await
            .unwrap()
            .sync_status,
        "processing"
    );
    assert_eq!(snapshot_audit_actions(&state), vec!["h8_exchange_receive"]);
}
