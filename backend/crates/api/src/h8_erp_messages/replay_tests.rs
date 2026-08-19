//! H8 人工重放到 Worker 接管的定向测试。

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use tower::ServiceExt;
use uuid::Uuid;
use wms_domain::H8ErpMessageListResponse;

use super::{
    handlers::h8_erp_message_router,
    state::H8ErpMessageAppState,
    tests::{sample_message, test_ctx},
};

#[tokio::test]
async fn replay_marker_can_be_claimed_immediately_by_worker() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "failed");
    let id = msg.id;
    state.repository.upsert_for_test(&msg).await.unwrap();
    let now = Utc::now();
    state
        .repository
        .replay(owner, id, "manual fix", "admin", now)
        .await
        .unwrap();

    let claimed = state
        .repository
        .claim(owner, id, "worker-1", 60, now)
        .await
        .unwrap();

    assert_eq!(claimed.claimed_by.as_deref(), Some("worker-1"));
    assert_eq!(claimed.sync_status, "processing");
}

#[tokio::test]
async fn list_filters_manual_replays_by_connector() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    let mut selected = sample_message(owner, "processing");
    selected.connector_id = Some(connector_id);
    selected.claimed_by = Some("replay:admin".into());
    let mut normal_claim = sample_message(owner, "processing");
    normal_claim.connector_id = Some(connector_id);
    normal_claim.claimed_by = Some("worker-2".into());
    let mut other_connector = sample_message(owner, "processing");
    other_connector.connector_id = Some(Uuid::new_v4());
    other_connector.claimed_by = Some("replay:admin".into());
    for message in [&selected, &normal_claim, &other_connector] {
        state.repository.upsert_for_test(message).await.unwrap();
    }

    let mut request = Request::builder()
        .uri(format!(
            "/api/v1/integration/erp-messages?status=processing&connector_id={connector_id}&replay_requested=true&created_from=1970-01-01T00:00:00Z"
        ))
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(test_ctx(owner));
    let response = h8_erp_message_router(state).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let listed: H8ErpMessageListResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(listed.data.len(), 1);
    assert_eq!(listed.data[0].id, selected.id);
}
