use axum::{body::to_bytes, body::Body, http::Request, Extension};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{auth::AuthContext, state_machine::state_machine_router};

#[sqlx::test(migrations = "../../migrations")]
async fn h6_exposes_the_m4_short_pick_state_contract(_pool: PgPool) {
    let app = state_machine_router().layer(Extension(AuthContext {
        user_id: Uuid::new_v4(),
        owner_id: Uuid::new_v4(),
        actor_name: "h6-wave4-contract".to_string(),
        permissions: vec!["h6.state_machine.read".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/state-machines/outbound_order")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("state machine route should respond");
    assert_eq!(response.status(), 200);
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable"),
    )
    .expect("state machine response should be JSON");
    let state_codes = payload["states"]
        .as_array()
        .expect("states should be an array")
        .iter()
        .map(|state| state["code"].as_str().expect("state code should be text"))
        .collect::<Vec<_>>();
    for state in ["picked", "picked_short", "reviewed_short"] {
        assert!(state_codes.contains(&state), "missing M4 state {state}");
    }

    let response = state_machine_router()
        .layer(Extension(AuthContext {
            user_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            actor_name: "h6-wave4-contract".to_string(),
            permissions: vec!["h6.state_machine.read".to_string()],
            jti: Uuid::new_v4().to_string(),
            warehouse_scope: None,
        }))
        .oneshot(
            Request::builder()
                .uri("/api/v1/state-machines/outbound_order/transition-validation?from_state=in_wave&to_state=picked_short&event_code=short_pick_recorded")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("transition route should respond");
    assert_eq!(response.status(), 200);
    let payload: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable"),
    )
    .expect("transition response should be JSON");
    assert_eq!(payload["allowed"], true);
}
