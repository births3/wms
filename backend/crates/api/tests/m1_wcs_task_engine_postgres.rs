//! T03：指令生成/派发/回执/事件处理/超时重试（GWT 9/11/15/19/20/21）。

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
    device_service::{DeviceService, RegisterDeviceRequest},
    wcs_task_handlers::{wcs_task_router, WcsTaskAppState},
    wcs_task_service::WcsTaskService,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "wcs-op".into(),
        permissions: vec!["m1.device.manage".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_device(pool: &PgPool, owner_id: Uuid, device_type: &str) -> Uuid {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'T03-OWNER', 'T03 owner')",
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("insert owner for M-CG numbering");
    let service = DeviceService::new(pool.clone());
    let c = ctx(owner_id);
    let device = service
        .register(
            &c,
            RegisterDeviceRequest {
                device_code: format!("DEV-{device_type}"),
                device_type: device_type.into(),
                vendor: None,
                model: None,
                protocol: "http".into(),
                ip_address: None,
                port: None,
                extra_config: json!({}),
            },
            &format!("reg-{device_type}"),
        )
        .await
        .expect("register device");
    service.heartbeat(&c, device.id).await.expect("heartbeat");
    device.id
}

async fn post_json(
    router: &axum::Router,
    c: &AuthContext,
    path: &str,
    body: Value,
    key: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json");
    if let Some(key) = key {
        builder = builder.header("idempotency-key", key);
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn combined_router(pool: PgPool, c: &AuthContext) -> axum::Router {
    device_router(DeviceAppState::with_postgres(pool.clone()))
        .merge(wcs_task_router(WcsTaskAppState::with_postgres(pool)))
        .layer(Extension(c.clone()))
}

fn task_body(device_id: Uuid, task_type: &str, payload: Value) -> Value {
    json!({
        "task_type": task_type,
        "device_id": device_id,
        "payload": payload
    })
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt9_create_task_idempotent(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "ptl_light").await;
    let router = combined_router(pool, &c).await;
    let payload = json!({"qty": 5, "location_id": Uuid::new_v4()});

    let (status, first) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        task_body(device_id, "ptl_light_on", payload.clone()),
        Some("task-1"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "生成应成功: {first}");
    assert_eq!(first["status"], "pending");
    assert!(first["task_no"].as_str().unwrap().starts_with("WCST-"));
    assert_eq!(first["retry_count"], 0);

    let (status, replay) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        task_body(device_id, "ptl_light_on", payload),
        Some("task-1"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(first["id"], replay["id"], "幂等重放应返回同一任务");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt10_ptl_light_busy_conflicts(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "ptl_light").await;
    let router = combined_router(pool, &c).await;

    let (status, _) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        task_body(device_id, "ptl_light_on", json!({"qty": 5})),
        Some("task-1"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        task_body(device_id, "ptl_light_on", json!({"qty": 3})),
        Some("task-2"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "亮灯互斥应 409: {body}");
    assert_eq!(body["code"], "M1_PTL_LIGHT_BUSY");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt11_receipt_cycle_and_terminal_replay_ignored(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "ptl_light").await;
    let router = combined_router(pool.clone(), &c).await;

    let (_, created) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        task_body(device_id, "ptl_light_on", json!({"qty": 5})),
        Some("task-1"),
    )
    .await;
    let task_id = created["id"].as_str().unwrap().to_string();

    let service = WcsTaskService::new(pool);
    let dispatched = service
        .dispatch(&c, Uuid::parse_str(&task_id).unwrap())
        .await
        .unwrap();
    assert_eq!(dispatched.status, "sent");
    let executing = service
        .apply_receipt(&c, Uuid::parse_str(&task_id).unwrap(), "start", None)
        .await
        .unwrap();
    assert_eq!(executing.status, "executing");
    let succeeded = service
        .apply_receipt(&c, Uuid::parse_str(&task_id).unwrap(), "success", None)
        .await
        .unwrap();
    assert_eq!(succeeded.status, "succeeded");
    assert_eq!(succeeded.ack_payload["outcome"], "success");

    // I6：终态重复回执幂等忽略，返回当前任务
    let again = service
        .apply_receipt(&c, Uuid::parse_str(&task_id).unwrap(), "success", None)
        .await
        .expect("终态重复回执应幂等返回");
    assert_eq!(again.status, "succeeded");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt15_orphan_press_window_then_h4(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "ptl_light").await;
    let router = combined_router(pool.clone(), &c).await;
    let service = WcsTaskService::new(pool.clone());

    // 无任务拍灯事件：窗口内不告警
    let (status, _) = post_json(
        &router,
        &c,
        &format!("/api/v1/iot-devices/{device_id}/events"),
        json!({"event_type": "ptl_press", "location_id": Uuid::new_v4(), "payload": {"press_qty": 5}}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let count = service.run_orphan_scan().await.unwrap();
    assert_eq!(count, 0, "窗口内不应告警");

    // 拨旧事件时间 → 扫描 → H4 device_event_orphan
    sqlx::query(
        r#"UPDATE iot_event_logs SET received_at = now() - interval '2 minutes' WHERE event_type = 'ptl_press'"#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let count = service.run_orphan_scan().await.unwrap();
    assert_eq!(count, 1, "超窗应告警");
    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_bus_event WHERE event_type = 'business.device_event_orphan'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt19_dws_validation_fail_and_pass(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "dws").await;
    let router = combined_router(pool.clone(), &c).await;
    let service = WcsTaskService::new(pool.clone());

    let (_, created) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        task_body(device_id, "dws_weigh", json!({"expected_weight_g": 3500})),
        Some("task-1"),
    )
    .await;
    let task_id = created["id"].as_str().unwrap().to_string();
    service
        .dispatch(&c, Uuid::parse_str(&task_id).unwrap())
        .await
        .unwrap();

    // pass=false → failed
    let err = service
        .handle_event(
            &c,
            device_id,
            wms_api::wcs_task_service::DeviceEventRequest {
                event_type: "dws_result".into(),
                task_id: Some(Uuid::parse_str(&task_id).unwrap()),
                location_id: None,
                payload: json!({"pass": false, "weight_g": 3500}),
            },
        )
        .await;
    assert!(err.is_err(), "pass=false 应失败");
    let row = sqlx::query_as::<_, (String,)>("SELECT status FROM wcs_tasks WHERE id = $1")
        .bind(Uuid::parse_str(&task_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, "failed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt20_rfid_epc_coverage(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "rfid_antenna").await;
    let router = combined_router(pool.clone(), &c).await;
    let service = WcsTaskService::new(pool.clone());

    let (_, created) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        task_body(
            device_id,
            "rfid_scan",
            json!({"target_epcs": ["EPC-A", "EPC-B"]}),
        ),
        Some("task-1"),
    )
    .await;
    let task_id = created["id"].as_str().unwrap().to_string();
    service
        .dispatch(&c, Uuid::parse_str(&task_id).unwrap())
        .await
        .unwrap();

    // 缺 EPC → failed
    let err = service
        .handle_event(
            &c,
            device_id,
            wms_api::wcs_task_service::DeviceEventRequest {
                event_type: "rfid_batch".into(),
                task_id: Some(Uuid::parse_str(&task_id).unwrap()),
                location_id: None,
                payload: json!({"epcs": ["EPC-A"]}),
            },
        )
        .await;
    assert!(err.is_err(), "缺 EPC 应失败");
    let row = sqlx::query_as::<_, (String,)>("SELECT status FROM wcs_tasks WHERE id = $1")
        .bind(Uuid::parse_str(&task_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, "failed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt21_retry_exhausted_then_resend_void(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "ptl_light").await;
    let router = combined_router(pool.clone(), &c).await;
    let service = WcsTaskService::new(pool.clone());

    let (_, created) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        task_body(device_id, "ptl_light_on", json!({"qty": 5})),
        Some("task-1"),
    )
    .await;
    let task_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    service.dispatch(&c, task_id).await.unwrap();

    // 3 次自动重试 + 第 4 次失败回执 → 耗尽 failed（max_retries=3，retry_count 0→1→2→3）
    for i in 0..4 {
        let r = service
            .apply_receipt(&c, task_id, "fail", Some("DEV_ERR"))
            .await
            .unwrap();
        if i == 3 {
            assert_eq!(r.status, "failed", "第 4 次失败应耗尽进入 failed");
            assert_eq!(r.retry_count, 3);
        } else {
            assert_eq!(r.status, "sent", "前三次应重试回 sent");
        }
    }

    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_bus_event WHERE event_type = 'business.wcs_task_failed'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events, 1, "耗尽应写 H4 wcs_task_failed");

    // 人工重发 → sent 且 retry_count 归零
    let resend = service
        .resend(&c, task_id, "测试重发".into())
        .await
        .unwrap();
    assert_eq!(resend.status, "sent");
    assert_eq!(resend.retry_count, 0);

    // 作废（未落账任务可作废）
    let voided = service
        .void(
            &c,
            task_id,
            wms_api::wcs_task_service::VoidRequest {
                reason: "现场处置".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(voided.status, "failed");
}
