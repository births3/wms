use axum::{
    body::{to_bytes, Body},
    http::{header::CONTENT_TYPE, Request, StatusCode},
    response::Response,
};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    dock_appointment_handlers::{dock_appointment_router, DockAppointmentAppState},
};

fn context(permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id: Uuid::new_v4(),
        actor_name: "dock-api-test".to_string(),
        permissions: permissions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        jti: Uuid::new_v4().to_string(),
    }
}

fn lazy_pool() -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(50))
        .connect_lazy("postgres://wms:wms@127.0.0.1:1/wms")
        .expect("test PostgreSQL URL should be accepted lazily")
}

fn request(ctx: AuthContext, idempotency_key: Option<&str>) -> Request<Body> {
    let now = Utc::now();
    let payload = json!({
        "dock_id": Uuid::new_v4(),
        "warehouse_id": Uuid::new_v4(),
        "appointment_no": "APT-API-001",
        "document_type": "purchase_inbound",
        "document_no": "ASN-API-001",
        "window_start_at": (now + Duration::hours(1)).to_rfc3339(),
        "window_end_at": (now + Duration::hours(2)).to_rfc3339(),
        "vehicle_plate_no": "沪A12345",
        "vehicle_type": "厢式货车",
        "driver_name": "测试司机",
        "driver_phone": "13800000000"
    });
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/v1/dock-appointments")
        .header(CONTENT_TYPE, "application/json");
    if let Some(idempotency_key) = idempotency_key {
        builder = builder.header("Idempotency-Key", idempotency_key);
    }
    let mut request = builder
        .body(Body::from(payload.to_string()))
        .expect("test request should build");
    request.extensions_mut().insert(ctx);
    request
}

async fn json_body(response: Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&body).expect("response should be JSON")
}

#[tokio::test]
async fn missing_idempotency_key_is_rejected_by_api_layer() {
    let app = dock_appointment_router(DockAppointmentAppState::with_postgres(lazy_pool()));
    let response = app
        .oneshot(request(context(&["dock.manage"]), None))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(response).await["code"],
        "H_DOCK_IDEMPOTENCY_REQUIRED"
    );
}

#[tokio::test]
async fn missing_dock_permission_is_rejected_by_api_layer() {
    let app = dock_appointment_router(DockAppointmentAppState::with_postgres(lazy_pool()));
    let response = app
        .oneshot(request(context(&["dock.read"]), None))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(json_body(response).await["code"], "AUTH-005");
}

#[tokio::test]
async fn valid_request_reaches_repository_boundary() {
    let app = dock_appointment_router(DockAppointmentAppState::with_postgres(lazy_pool()));
    let response = app
        .oneshot(request(
            context(&["dock.manage"]),
            Some("dock-api-test-001"),
        ))
        .await
        .expect("router should respond");

    // 这里是 API 边界测试，不伪造数据库成功证据；持久化失败码证明请求已越过鉴权和幂等校验进入 repository。
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        json_body(response).await["code"],
        "H_DOCK_PERSISTENCE_FAILED"
    );
}

#[tokio::test]
async fn arrival_endpoint_requires_idempotency_key() {
    let app = dock_appointment_router(DockAppointmentAppState::with_postgres(lazy_pool()));
    let payload = json!({
        "appointment_no": "APT-ARRIVAL-API-001",
        "vehicle_plate_no": "沪A12345",
        "driver_name": "测试司机",
        "vehicle_type": "厢式货车"
    });
    let mut request = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/dock-appointments/{}/arrive",
            Uuid::new_v4()
        ))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .expect("arrival request should build");
    request.extensions_mut().insert(context(&["dock.manage"]));

    let response = app.oneshot(request).await.expect("router should respond");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(response).await["code"],
        "H_DOCK_IDEMPOTENCY_REQUIRED"
    );
}
