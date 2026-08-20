//! T02：设备注册/启停/心跳/绑定/离线告警（GWT 1-8）。

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Extension,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    device_handlers::{device_router, DeviceAppState},
    device_service::{DeviceService, UnbindRequest},
};

fn manage_ctx(owner_id: Uuid) -> AuthContext {
    auth(owner_id, "m1.device.manage")
}

fn bind_ctx(owner_id: Uuid) -> AuthContext {
    auth(owner_id, "m1.device-bind.manage")
}

fn auth(owner_id: Uuid, permission: &str) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "device-op".into(),
        permissions: vec![permission.into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn router(pool: PgPool, ctx: &AuthContext) -> axum::Router {
    device_router(DeviceAppState::with_postgres(pool)).layer(Extension(ctx.clone()))
}

async fn post_json(
    router: &axum::Router,
    ctx: &AuthContext,
    path: &str,
    body: Value,
    idempotency_key: Option<&str>,
) -> (StatusCode, Value) {
    request_json(router, "POST", ctx, path, body, idempotency_key).await
}

async fn patch_json(
    router: &axum::Router,
    ctx: &AuthContext,
    path: &str,
    body: Value,
    idempotency_key: Option<&str>,
) -> (StatusCode, Value) {
    request_json(router, "PATCH", ctx, path, body, idempotency_key).await
}

async fn request_json(
    router: &axum::Router,
    method: &str,
    _ctx: &AuthContext,
    path: &str,
    body: Value,
    idempotency_key: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(key) = idempotency_key {
        builder = builder.header("idempotency-key", key);
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt1_register_device_returns_201_offline(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let ctx = manage_ctx(owner_id);
    let router = router(pool.clone(), &ctx).await;

    let (status, body) = post_json(
        &router,
        &ctx,
        "/api/v1/iot-devices",
        json!({
            "warehouse_id": warehouse_id,
            "device_code": "PTL-01",
            "device_type": "ptl_light",
            "vendor": "厂商A",
            "protocol": "http"
        }),
        Some("reg-1"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["device_code"], "PTL-01");
    assert_eq!(body["device_type"], "ptl_light");
    assert_eq!(body["online_status"], "offline");
    assert_eq!(body["enabled"], true);
    assert_eq!(body["warehouse_id"], warehouse_id.to_string());
    assert_eq!(body["version"], 1);

    let audits: i64 = sqlx::query_scalar(
        r#"SELECT count(*) FROM audit_event WHERE action = 'register_device' AND resource_id = $1"#,
    )
    .bind(body["id"].as_str().unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audits, 1, "注册应同事务写 H2 审计");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt2_duplicate_code_conflicts(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = manage_ctx(owner_id);
    let router = router(pool.clone(), &ctx).await;

    let (status, _) = post_json(
        &router,
        &ctx,
        "/api/v1/iot-devices",
        json!({"warehouse_id": owner_id, "device_code": "PTL-01", "device_type": "ptl_light", "protocol": "http"}),
        Some("reg-1"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = post_json(
        &router,
        &ctx,
        "/api/v1/iot-devices",
        json!({"warehouse_id": owner_id, "device_code": "PTL-01", "device_type": "ptl_light", "protocol": "http"}),
        Some("reg-2"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "M1_DEVICE_DUPLICATE_CODE");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt3_invalid_device_type_rejected(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = manage_ctx(owner_id);
    let router = router(pool.clone(), &ctx).await;

    let (status, body) = post_json(
        &router,
        &ctx,
        "/api/v1/iot-devices",
        json!({"warehouse_id": owner_id, "device_code": "ROB-01", "device_type": "robot", "protocol": "http"}),
        Some("reg-1"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M1_DEVICE_TYPE_INVALID");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt9_idempotent_register_replays(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = manage_ctx(owner_id);
    let router = router(pool.clone(), &ctx).await;

    let (status, first) = post_json(
        &router,
        &ctx,
        "/api/v1/iot-devices",
        json!({"warehouse_id": owner_id, "device_code": "PTL-01", "device_type": "ptl_light", "protocol": "http"}),
        Some("reg-1"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, replay) = post_json(
        &router,
        &ctx,
        "/api/v1/iot-devices",
        json!({"warehouse_id": owner_id, "device_code": "PTL-01", "device_type": "ptl_light", "protocol": "http"}),
        Some("reg-1"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(first["id"], replay["id"], "幂等重放应返回同一设备");
}

async fn register_online_ptl(pool: &PgPool, owner_id: Uuid) -> (Uuid, Uuid) {
    let service = DeviceService::new(pool.clone());
    let ctx = manage_ctx(owner_id);
    let device = service
        .register(
            &ctx,
            wms_api::device_service::RegisterDeviceRequest {
                warehouse_id: owner_id,
                device_code: "PTL-01".into(),
                device_type: "ptl_light".into(),
                vendor: None,
                model: None,
                protocol: "http".into(),
                ip_address: None,
                port: None,
                extra_config: json!({}),
            },
            "reg-bind",
        )
        .await
        .expect("register device");
    service
        .heartbeat(&ctx, device.id, "heartbeat-register-online")
        .await
        .expect("heartbeat");
    (device.id, owner_id)
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt4_bind_conflict_and_ok(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (device_id, _) = register_online_ptl(&pool, owner_id).await;
    let ctx = bind_ctx(owner_id);
    let router = router(pool.clone(), &ctx).await;
    let location_id = Uuid::new_v4();

    let (status, body) = post_json(
        &router,
        &ctx,
        "/api/v1/location-device-bindings",
        json!({
            "location_id": location_id,
            "device_id": device_id,
            "binding_role": "ptl_light",
            "point_address": "A-01"
        }),
        Some("bind-1"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "首次绑定应成功: {body}");

    let (status, body) = post_json(
        &router,
        &ctx,
        "/api/v1/location-device-bindings",
        json!({
            "location_id": location_id,
            "device_id": device_id,
            "binding_role": "ptl_light",
            "point_address": "A-02"
        }),
        Some("bind-2"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "同库位同角色应 409: {body}");
    assert_eq!(body["code"], "M1_BIND_CONFLICT");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt5_bind_role_device_mismatch(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let service = DeviceService::new(pool.clone());
    let ctx = manage_ctx(owner_id);
    let device = service
        .register(
            &ctx,
            wms_api::device_service::RegisterDeviceRequest {
                warehouse_id: owner_id,
                device_code: "DWS-01".into(),
                device_type: "dws".into(),
                vendor: None,
                model: None,
                protocol: "http".into(),
                ip_address: None,
                port: None,
                extra_config: json!({}),
            },
            "reg-1",
        )
        .await
        .expect("register");
    service
        .heartbeat(&ctx, device.id, "heartbeat-role-mismatch")
        .await
        .expect("heartbeat");

    let router = router(pool.clone(), &bind_ctx(owner_id)).await;
    let (status, body) = post_json(
        &router,
        &bind_ctx(owner_id),
        "/api/v1/location-device-bindings",
        json!({
            "location_id": Uuid::new_v4(),
            "device_id": device.id,
            "binding_role": "ptl_light"
        }),
        Some("bind-1"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M1_BIND_DEVICE_MISMATCH");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt6_bind_offline_device_blocked(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let service = DeviceService::new(pool.clone());
    let ctx = manage_ctx(owner_id);
    let device = service
        .register(
            &ctx,
            wms_api::device_service::RegisterDeviceRequest {
                warehouse_id: owner_id,
                device_code: "PTL-02".into(),
                device_type: "ptl_light".into(),
                vendor: None,
                model: None,
                protocol: "http".into(),
                ip_address: None,
                port: None,
                extra_config: json!({}),
            },
            "reg-1",
        )
        .await
        .expect("register");

    let router = router(pool.clone(), &bind_ctx(owner_id)).await;
    let (status, body) = post_json(
        &router,
        &bind_ctx(owner_id),
        "/api/v1/location-device-bindings",
        json!({
            "location_id": Uuid::new_v4(),
            "device_id": device.id,
            "binding_role": "ptl_light"
        }),
        Some("bind-1"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M1_DEVICE_OFFLINE");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt7_heartbeat_marks_online_then_scan_offline(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (device_id, _) = register_online_ptl(&pool, owner_id).await;
    let service = DeviceService::new(pool.clone());

    // 心跳后应 online
    let row =
        sqlx::query_as::<_, (String,)>(r#"SELECT online_status FROM iot_devices WHERE id = $1"#)
            .bind(device_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row.0, "online");

    // 人为把心跳时间拨旧，扫描应置 offline 并写 H4 告警
    sqlx::query(
        r#"UPDATE iot_devices SET last_heartbeat_at = now() - interval '5 minutes' WHERE id = $1"#,
    )
    .bind(device_id)
    .execute(&pool)
    .await
    .unwrap();

    let count = service.run_heartbeat_scan().await.expect("scan");
    assert_eq!(count, 1, "应恰好扫描到一台离线设备");

    let row =
        sqlx::query_as::<_, (String,)>(r#"SELECT online_status FROM iot_devices WHERE id = $1"#)
            .bind(device_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row.0, "offline");

    let events: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*) FROM event_bus_event
         WHERE event_type = 'business.device_offline'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events, 1, "应写一条 device_offline 事件");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt8_disable_device_blocks_and_unbind_works(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (device_id, _) = register_online_ptl(&pool, owner_id).await;
    let ctx = manage_ctx(owner_id);
    let router = router(pool.clone(), &ctx).await;
    let location_id = Uuid::new_v4();

    // 绑定
    let (status, body) = post_json(
        &router,
        &bind_ctx(owner_id),
        "/api/v1/location-device-bindings",
        json!({
            "location_id": location_id,
            "device_id": device_id,
            "binding_role": "ptl_light"
        }),
        Some("bind-1"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let binding_id = body["id"].as_str().unwrap().to_string();

    // 停用设备
    let (status, _body) = patch_json(
        &router,
        &ctx,
        &format!("/api/v1/iot-devices/{device_id}"),
        json!({"enabled": false, "expected_version": 1}),
        Some("device-disable"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 停用后绑定新点位被拒
    let (status, body) = post_json(
        &router,
        &bind_ctx(owner_id),
        "/api/v1/location-device-bindings",
        json!({
            "location_id": Uuid::new_v4(),
            "device_id": device_id,
            "binding_role": "ptl_light"
        }),
        Some("bind-2"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "M1_DEVICE_DISABLED");

    // 解绑（软解绑置 valid_to）
    let (status, _) = post_json(
        &router,
        &bind_ctx(owner_id),
        &format!("/api/v1/location-device-bindings/{binding_id}/unbind"),
        json!(UnbindRequest {
            reason: "换绑测试".into()
        }),
        Some("unbind-1"),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let valid_to: i64 = sqlx::query_scalar(
        r#"SELECT count(*) FROM location_device_bindings WHERE id = $1 AND valid_to IS NOT NULL"#,
    )
    .bind(Uuid::parse_str(&binding_id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(valid_to, 1, "解绑应置 valid_to");
}

#[sqlx::test(migrations = "../../migrations")]
async fn device_update_rejects_stale_version(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let (device_id, _) = register_online_ptl(&pool, owner_id).await;
    let ctx = manage_ctx(owner_id);
    let router = router(pool, &ctx).await;

    let (status, updated) = patch_json(
        &router,
        &ctx,
        &format!("/api/v1/iot-devices/{device_id}"),
        json!({"vendor": "厂商B", "expected_version": 1}),
        Some("device-update-1"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["version"], 2);

    let (status, body) = patch_json(
        &router,
        &ctx,
        &format!("/api/v1/iot-devices/{device_id}"),
        json!({"vendor": "过期写入", "expected_version": 1}),
        Some("device-update-stale"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "M1_DEVICE_VERSION_CONFLICT");
}
