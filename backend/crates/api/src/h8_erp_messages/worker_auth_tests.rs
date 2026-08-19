use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;
use uuid::Uuid;

use crate::auth::AuthContext;

use super::{handlers::h8_erp_message_router, state::H8ErpMessageAppState, tests::sample_message};

#[tokio::test]
async fn worker_key_can_heartbeat_but_cannot_run_message_admin_actions() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::new_v4();
    let message = sample_message(owner, "failed");
    let message_id = message.id;
    state.repository.upsert_for_test(&message).await.unwrap();
    let worker = AuthContext {
        user_id: Uuid::nil(),
        owner_id: owner,
        actor_name: "h8-worker".into(),
        permissions: vec!["h8.erp_connector.read".into(), "h8.erp_worker.write".into()],
        jti: "api-key:h8-worker".into(),
        warehouse_scope: None,
    };
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integration/erp-messages/worker-runtime/heartbeat")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "worker_id": "worker-1", "worker_version": "test",
                "connector_id": Uuid::nil(), "directions": ["inbound"],
                "current_claims": 0, "heartbeat_ttl_seconds": 30
            })
            .to_string(),
        ))
        .unwrap();
    request.extensions_mut().insert(worker.clone());
    let response = h8_erp_message_router(state.clone())
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let mut request = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/integration/erp-messages/{message_id}/replay"
        ))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"reason":"retry","confirmed":true}"#))
        .unwrap();
    request.extensions_mut().insert(worker);
    let response = h8_erp_message_router(state).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
