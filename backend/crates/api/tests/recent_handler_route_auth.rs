use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use wms_api::{
    device_handlers::{device_router, DeviceAppState},
    lpn_container_handlers::{lpn_container_router, LpnContainerAppState},
    replenishment_handlers::{replenishment_router, ReplenishmentAppState},
    wcs_task_handlers::{wcs_task_router, WcsTaskAppState},
};

async fn assert_auth_required(router: &Router, method: Method, path: &str) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .header("idempotency-key", "route-auth-check")
                .body(Body::from("{}"))
                .expect("build route auth request"),
        )
        .await
        .expect("route should respond");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
}

#[tokio::test]
async fn recent_routes_require_authentication() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://wms:wms@127.0.0.1/wms")
        .expect("lazy PostgreSQL pool");
    let id = "00000000-0000-0000-0000-000000000001";

    let device = device_router(DeviceAppState::with_postgres(pool.clone()));
    assert_auth_required(
        &device,
        Method::POST,
        &format!("/api/v1/iot-devices/{id}/heartbeat"),
    )
    .await;

    let lpn = lpn_container_router(LpnContainerAppState::with_postgres(pool.clone()));
    for (method, path) in [
        (Method::GET, "/api/v1/master-data/lpn-containers"),
        (
            Method::POST,
            "/api/v1/master-data/lpn-containers/batch-create",
        ),
        (
            Method::GET,
            "/api/v1/master-data/lpn-containers/00000000-0000-0000-0000-000000000001",
        ),
        (
            Method::GET,
            "/api/v1/master-data/lpn-container-type-policies",
        ),
    ] {
        assert_auth_required(&lpn, method, path).await;
    }

    let replenishment = replenishment_router(ReplenishmentAppState::with_postgres(pool.clone()));
    for (method, path) in [
        (Method::GET, "/api/v1/replenishment/location-groups"),
        (
            Method::GET,
            "/api/v1/replenishment/strategies/00000000-0000-0000-0000-000000000001/preview",
        ),
        (
            Method::GET,
            "/api/v1/replenishment/location-groups/00000000-0000-0000-0000-000000000001",
        ),
        (
            Method::POST,
            "/api/v1/replenishment/location-groups/00000000-0000-0000-0000-000000000001/disable",
        ),
        (
            Method::GET,
            "/api/v1/replenishment/tasks/00000000-0000-0000-0000-000000000001",
        ),
        (
            Method::POST,
            "/api/v1/replenishment/tasks/00000000-0000-0000-0000-000000000001/pick",
        ),
        (
            Method::POST,
            "/api/v1/replenishment/tasks/00000000-0000-0000-0000-000000000001/confirm",
        ),
        (
            Method::POST,
            "/api/v1/replenishment/tasks/00000000-0000-0000-0000-000000000001/cancel",
        ),
        (
            Method::POST,
            "/api/v1/replenishment/tasks/00000000-0000-0000-0000-000000000001/reassign",
        ),
        (
            Method::POST,
            "/api/v1/replenishment/tasks/00000000-0000-0000-0000-000000000001/return",
        ),
    ] {
        assert_auth_required(&replenishment, method, path).await;
    }

    let wcs = wcs_task_router(WcsTaskAppState::with_postgres(pool));
    for (method, path) in [
        (
            Method::GET,
            "/api/v1/wcs-tasks/00000000-0000-0000-0000-000000000001",
        ),
        (
            Method::POST,
            "/api/v1/wcs-tasks/00000000-0000-0000-0000-000000000001/resend",
        ),
        (
            Method::POST,
            "/api/v1/wcs-tasks/00000000-0000-0000-0000-000000000001/void",
        ),
        (
            Method::POST,
            "/api/v1/wcs-tasks/00000000-0000-0000-0000-000000000001/confirm-skip",
        ),
    ] {
        assert_auth_required(&wcs, method, path).await;
    }
}
