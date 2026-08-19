use axum::http::HeaderMap;
use axum::{extract::Path, extract::Query, extract::State, Json};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::ReviewOutboundOrderRequest;

use super::{
    get_outbound_order_handler, get_outbound_review_handler, list_outbound_orders_handler,
    review_outbound_order_handler, ListOutboundOrdersQuery, Wave4AppState, Wave4HandlerError,
};
use crate::auth::{AuthContext, AuthError};

fn ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "wave4-review-handler-test".to_string(),
        permissions: permissions
            .iter()
            .map(|permission| permission.to_string())
            .collect(),
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[tokio::test]
async fn outbound_read_handlers_require_m4_read_or_write_before_postgres() {
    let owner_id = Uuid::new_v4();
    let pool = PgPool::connect_lazy("postgres://localhost/wms")
        .expect("lazy pool should not connect during handler read auth test");
    let state = Wave4AppState::with_postgres(pool);

    let list_denied = list_outbound_orders_handler(
        ctx(owner_id, &[]),
        State(state.clone()),
        Query(ListOutboundOrdersQuery::default()),
    )
    .await
    .expect_err("outbound list should require m4.read or m4.write before repository access");
    assert!(matches!(
        list_denied,
        Wave4HandlerError::Auth(AuthError::PermissionDenied(permission))
            if permission == "m4.read|m4.write"
    ));

    let detail_denied = get_outbound_order_handler(
        ctx(owner_id, &[]),
        State(state.clone()),
        Path(Uuid::new_v4()),
    )
    .await
    .expect_err("outbound detail should require m4.read or m4.write before repository access");
    assert!(matches!(
        detail_denied,
        Wave4HandlerError::Auth(AuthError::PermissionDenied(permission))
            if permission == "m4.read|m4.write"
    ));

    let review_denied =
        get_outbound_review_handler(ctx(owner_id, &[]), State(state), Path(Uuid::new_v4()))
            .await
            .expect_err("review query should require m4.read or m4.write");
    assert!(matches!(
        review_denied,
        Wave4HandlerError::Auth(AuthError::PermissionDenied(permission))
            if permission == "m4.read|m4.write"
    ));
}

#[tokio::test]
async fn outbound_review_submit_requires_write_permission_and_idempotency() {
    let owner_id = Uuid::new_v4();
    let state = Wave4AppState::with_postgres(
        PgPool::connect_lazy("postgres://localhost/wms")
            .expect("lazy pool should support review auth test"),
    );
    let request = ReviewOutboundOrderRequest {
        reviewer_id: Uuid::new_v4(),
        review_mode: "pda_loose".to_string(),
        second_reviewer_id: None,
        lines: vec![],
    };

    let denied = review_outbound_order_handler(
        ctx(owner_id, &[]),
        State(state.clone()),
        Path(Uuid::new_v4()),
        HeaderMap::new(),
        Json(request.clone()),
    )
    .await
    .expect_err("review submit should require m4.write");
    assert!(matches!(
        denied,
        Wave4HandlerError::Auth(AuthError::PermissionDenied(permission))
            if permission == "m4.write"
    ));

    let missing_key = review_outbound_order_handler(
        ctx(owner_id, &["m4.write"]),
        State(state),
        Path(Uuid::new_v4()),
        HeaderMap::new(),
        Json(request),
    )
    .await
    .expect_err("review submit should require Idempotency-Key");
    assert_eq!(missing_key, Wave4HandlerError::InvalidIdempotencyKey);
}
