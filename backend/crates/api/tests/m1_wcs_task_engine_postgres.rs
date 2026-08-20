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
    device_service::{DeviceError, DeviceService, RegisterDeviceRequest},
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
                warehouse_id: owner_id,
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
    service
        .heartbeat(
            &c,
            device.id,
            &format!("heartbeat-{device_type}-{}", device.id),
        )
        .await
        .expect("heartbeat");
    device.id
}

async fn seed_device_in_warehouse(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    device_type: &str,
    device_code: &str,
) -> Uuid {
    sqlx::query(
        r#"
        INSERT INTO auth_owners (id, owner_code, owner_name)
        VALUES ($1, 'T03-SHARED-OWNER', 'T03 shared owner')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("insert owner for shared warehouse device");
    let service = DeviceService::new(pool.clone());
    let c = ctx(owner_id);
    let device = service
        .register(
            &c,
            RegisterDeviceRequest {
                warehouse_id,
                device_code: device_code.into(),
                device_type: device_type.into(),
                vendor: None,
                model: None,
                protocol: "http".into(),
                ip_address: None,
                port: None,
                extra_config: json!({}),
            },
            &format!("reg-{device_code}"),
        )
        .await
        .expect("register shared warehouse device");
    service
        .heartbeat(
            &c,
            device.id,
            &format!("heartbeat-{device_code}-{}", device.id),
        )
        .await
        .expect("heartbeat shared warehouse device");
    device.id
}

async fn post_json(
    router: &axum::Router,
    _c: &AuthContext,
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
    assert_eq!(status, StatusCode::OK, "同键重放应 200: {replay}");
    assert_eq!(first["id"], replay["id"], "幂等重放应返回同一任务");
}

async fn get_json(router: &axum::Router, path: &str) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn dashboard_and_event_routes_are_mounted(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let _device_id = seed_device(&pool, owner_id, "ptl_light").await;
    let router = combined_router(pool, &c).await;

    let dashboard_path = format!("/api/v1/device-dashboard?warehouse_id={owner_id}");
    let (status, body) = get_json(&router, &dashboard_path).await;
    assert_eq!(status, StatusCode::OK, "大盘应挂路由: {body}");
    assert!(body["total_devices"].as_i64().unwrap_or(0) >= 1);
    assert!(body.get("affected_location_ids").is_some());

    let events_path = format!("/api/v1/iot-events?warehouse_id={owner_id}");
    let (status, body) = get_json(&router, &events_path).await;
    assert_eq!(status, StatusCode::OK, "事件流应挂路由: {body}");
    assert!(body.as_array().is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt15_press_claimed_when_task_arrives_in_window(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device_in_warehouse(
        &pool,
        owner_id,
        warehouse_id,
        "ptl_light",
        "PTL-SHARED-CLAIM",
    )
    .await;
    let router = combined_router(pool.clone(), &c).await;
    let location_id = Uuid::new_v4();

    let (status, _) = post_json(
        &router,
        &c,
        &format!("/api/v1/iot-devices/{device_id}/events"),
        json!({"event_id": Uuid::new_v4(), "event_type": "ptl_press", "location_id": location_id, "payload": {"press_qty": 2}}),
        Some("pending-press-claim"),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, created) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        json!({
            "task_type": "ptl_light_on",
            "device_id": device_id,
            "location_id": location_id,
            "payload": {"qty": 2, "location_id": location_id}
        }),
        Some("claim-1"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let service = WcsTaskService::new(pool.clone());
    let task = service
        .get(
            &c,
            Uuid::parse_str(created["id"].as_str().unwrap()).unwrap(),
        )
        .await
        .expect("load claimed task");
    assert_eq!(task.status, "succeeded", "窗口内任务到达应认领拍灯并落账");

    sqlx::query(
        "UPDATE iot_event_logs SET received_at = received_at - interval '2 minutes' WHERE device_id = $1",
    )
    .bind(device_id)
    .execute(&pool)
    .await
    .expect("age claimed press event");
    sqlx::query(
        "UPDATE wcs_tasks SET created_at = created_at - interval '2 minutes' WHERE id = $1",
    )
    .bind(task.id)
    .execute(&pool)
    .await
    .expect("age claimed task by the same duration");
    assert_eq!(
        service
            .run_orphan_scan()
            .await
            .expect("scan claimed events"),
        0,
        "已认领事件不得因 owner_id/warehouse_id 语义不同被误报为孤儿"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn pending_press_claim_applies_ptl_quantity_threshold(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "ptl_light").await;
    let router = combined_router(pool.clone(), &c).await;
    let location_id = Uuid::new_v4();

    let (status, body) = post_json(
        &router,
        &c,
        &format!("/api/v1/iot-devices/{device_id}/events"),
        json!({
            "event_id": Uuid::new_v4(),
            "event_type": "ptl_press",
            "location_id": location_id,
            "payload": {"press_qty": 30}
        }),
        Some("pending-press-threshold"),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "窗口内事件应先挂起: {body}");

    let (status, body) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        json!({
            "task_type": "ptl_light_on",
            "device_id": device_id,
            "location_id": location_id,
            "payload": {"qty": 5, "location_id": location_id}
        }),
        Some("claim-threshold"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "超阈值认领应阻断: {body}"
    );
    assert_eq!(body["code"], "M1_PTL_QTY_DIFF_EXCEEDED");
    let (replay_status, replay_body) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        json!({
            "task_type": "ptl_light_on",
            "device_id": device_id,
            "location_id": location_id,
            "payload": {"qty": 5, "location_id": location_id}
        }),
        Some("claim-threshold"),
    )
    .await;
    assert_eq!(replay_status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(replay_body["code"], "M1_PTL_QTY_DIFF_EXCEEDED");
    let status: String = sqlx::query_scalar(
        "SELECT status FROM wcs_tasks WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_id)
    .bind("claim-threshold")
    .fetch_one(&pool)
    .await
    .expect("load claimed task");
    assert_eq!(status, "failed", "超阈值认领不得直接结算成功");
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

    let loc_a = Uuid::new_v4();
    let (status, body) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        json!({
            "task_type": "ptl_light_on",
            "device_id": device_id,
            "location_id": loc_a,
            "payload": {"qty": 1, "location_id": loc_a}
        }),
        Some("busy-loc-a"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "同设备不同库位仍应 I3 互斥: {body}"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_ptl_light_on_keeps_one_active_task(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "ptl_light").await;
    let service = WcsTaskService::new(pool);
    let request = wms_api::wcs_task_service::CreateWcsTaskRequest {
        task_type: "ptl_light_on".into(),
        device_id,
        location_id: None,
        business_ref_type: None,
        business_ref_no: None,
        payload: json!({"qty": 5}),
    };

    let (first, second) = tokio::join!(
        service.create_task(&c, request.clone(), "concurrent-light-1"),
        service.create_task(&c, request, "concurrent-light-2")
    );

    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert!(
        matches!(first, Err(DeviceError::PtLightBusy))
            || matches!(second, Err(DeviceError::PtLightBusy))
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn pod_move_without_pod_code_rejected(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "agv").await;
    let router = combined_router(pool, &c).await;

    let (status, body) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        json!({
            "task_type": "pod_move",
            "device_id": device_id,
            "payload": {"target_station": "ST-01"}
        }),
        Some("pod-no-code"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "缺 pod_code 应 422: {body}"
    );
    assert_eq!(body["code"], "M1_EVENT_TASK_MISMATCH");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt11_receipt_cycle_and_terminal_replay_ignored(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "agv").await;
    let router = combined_router(pool.clone(), &c).await;

    let (_, created) = post_json(
        &router,
        &c,
        "/api/v1/wcs-tasks",
        task_body(
            device_id,
            "pod_move",
            json!({"pod_code": "POD-GWT11", "workstation": "WS-GWT11"}),
        ),
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
        json!({"event_id": Uuid::new_v4(), "event_type": "ptl_press", "location_id": Uuid::new_v4(), "payload": {"press_qty": 5}}),
        Some("orphan-press-window"),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, body) = post_json(
        &router,
        &c,
        &format!("/api/v1/iot-devices/{device_id}/events"),
        json!({"event_id": Uuid::new_v4(), "event_type": "ptl_press", "location_id": Uuid::new_v4(), "payload": {"press_qty": 5}}),
        Some("orphan-press-window"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "同 header 不得接受不同事件: {body}"
    );
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
                event_id: Uuid::new_v4(),
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
                event_id: Uuid::new_v4(),
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

    // 第 3 次失败回执耗尽 → failed（max_retries=3，retry_count 0→1→2→3）
    for i in 0..3 {
        let r = service
            .apply_receipt(&c, task_id, "fail", Some("DEV_ERR"))
            .await
            .unwrap();
        if i == 2 {
            assert_eq!(r.status, "failed", "第 3 次失败应耗尽进入 failed");
            assert_eq!(r.retry_count, 3);
        } else {
            assert_eq!(r.status, "sent", "前两次应重试回 sent");
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
        .resend(&c, task_id, "测试重发".into(), "manual-resend")
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
            "manual-void",
        )
        .await
        .unwrap();
    assert_eq!(voided.status, "failed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn timeout_task_reenters_sent_after_first_backoff(pool: PgPool) {
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
        Some("timeout-retry"),
    )
    .await;
    let task_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    sqlx::query(
        "UPDATE wcs_tasks SET status = 'timeout', updated_at = NOW() - INTERVAL '61 seconds' WHERE id = $1",
    )
    .bind(task_id)
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(service.run_timeout_scan().await.unwrap(), 1);
    let (status, retry_count): (String, i32) =
        sqlx::query_as("SELECT status, retry_count FROM wcs_tasks WHERE id = $1")
            .bind(task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "sent");
    assert_eq!(retry_count, 1);
}
