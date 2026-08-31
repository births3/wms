//! T03：受控模拟网关派发与回执 API。

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
    device_service::{DeviceService, RegisterDeviceRequest},
    wcs_task_handlers::{wcs_task_router, WcsTaskAppState},
    wcs_task_service::{CreateWcsTaskRequest, WcsTaskService},
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "simulator-operator".into(),
        permissions: vec!["m1.device.manage".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_device(pool: &PgPool, owner_id: Uuid) -> Uuid {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'SIM-OWNER', 'Simulator owner')",
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("insert simulator owner");
    let service = DeviceService::new(pool.clone());
    let context = ctx(owner_id);
    let device = service
        .register(
            &context,
            RegisterDeviceRequest {
                warehouse_id: owner_id,
                device_code: "SIM-AGV-01".into(),
                device_type: "agv".into(),
                vendor: None,
                model: None,
                protocol: "http".into(),
                ip_address: None,
                port: None,
                extra_config: json!({}),
            },
            "register-simulator-device",
        )
        .await
        .expect("register simulator device");
    service
        .heartbeat(&context, device.id, "heartbeat-simulator-device")
        .await
        .expect("heartbeat simulator device");
    device.id
}

async fn post_json(
    router: &axum::Router,
    path: &str,
    key: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .header("idempotency-key", key)
                .body(Body::from(body.to_string()))
                .expect("simulator POST body"),
        )
        .await
        .expect("simulator POST should respond");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("simulator POST body should read");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn get_json(router: &axum::Router, path: &str) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::empty())
                .expect("simulator GET body"),
        )
        .await
        .expect("simulator GET should respond");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("simulator GET body should read");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn register_device_in_warehouse(
    pool: &PgPool,
    context: &AuthContext,
    warehouse_id: Uuid,
    device_code: &str,
) -> Uuid {
    let service = DeviceService::new(pool.clone());
    let device = service
        .register(
            context,
            RegisterDeviceRequest {
                warehouse_id,
                device_code: device_code.into(),
                device_type: "agv".into(),
                vendor: None,
                model: None,
                protocol: "http".into(),
                ip_address: None,
                port: None,
                extra_config: json!({}),
            },
            &format!("register-{device_code}"),
        )
        .await
        .expect("register warehouse device");
    service
        .heartbeat(context, device.id, &format!("heartbeat-{device_code}"))
        .await
        .expect("heartbeat warehouse device");
    device.id
}

#[sqlx::test(migrations = "../../migrations")]
async fn simulator_api_dispatches_and_applies_idempotent_receipts(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let context = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id).await;
    let router =
        wcs_task_router(WcsTaskAppState::with_postgres(pool.clone())).layer(Extension(context));

    let (created_status, created) = post_json(
        &router,
        "/api/v1/wcs-tasks",
        "simulator-create",
        json!({
            "task_type": "pod_move",
            "device_id": device_id,
            "payload": {"pod_code": "POD-SIM-01", "workstation": "WS-01"}
        }),
    )
    .await;
    assert_eq!(created_status, StatusCode::CREATED);
    assert_eq!(created["status"], "pending");
    let task_id = created["id"].as_str().expect("created simulator task id");

    let dispatch_path = format!("/api/v1/wcs-tasks/{task_id}/dispatch");
    let (dispatch_status, dispatched) =
        post_json(&router, &dispatch_path, "simulator-dispatch", json!({})).await;
    assert_eq!(dispatch_status, StatusCode::OK);
    assert_eq!(dispatched["status"], "sent");
    let (_, dispatch_replay) =
        post_json(&router, &dispatch_path, "simulator-dispatch", json!({})).await;
    assert_eq!(dispatch_replay, dispatched);

    let receipt_path = format!("/api/v1/wcs-tasks/{task_id}/receipt");
    let (_, started) = post_json(
        &router,
        &receipt_path,
        "simulator-receipt-start",
        json!({"outcome": "start"}),
    )
    .await;
    assert_eq!(started["status"], "executing");

    let (_, succeeded) = post_json(
        &router,
        &receipt_path,
        "simulator-receipt-success",
        json!({"outcome": "success"}),
    )
    .await;
    assert_eq!(succeeded["status"], "succeeded");
    let (_, receipt_replay) = post_json(
        &router,
        &receipt_path,
        "simulator-receipt-success",
        json!({"outcome": "success"}),
    )
    .await;
    assert_eq!(receipt_replay, succeeded);

    let timestamps: (bool, bool) = sqlx::query_as(
        "SELECT sent_at IS NOT NULL, finished_at IS NOT NULL FROM wcs_tasks WHERE id = $1",
    )
    .bind(Uuid::parse_str(task_id).expect("simulator task id should be uuid"))
    .fetch_one(&pool)
    .await
    .expect("load simulator lifecycle timestamps");
    assert_eq!(timestamps, (true, true));

    let audit_actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_event WHERE resource_id = $1 ORDER BY occurred_at",
    )
    .bind(task_id)
    .fetch_all(&pool)
    .await
    .expect("load simulator audit events");
    assert!(audit_actions.contains(&"dispatch_wcs_task".to_string()));
    assert_eq!(
        audit_actions
            .iter()
            .filter(|action| action.as_str() == "apply_wcs_receipt")
            .count(),
        2
    );

    let (mismatch_status, mismatch_body) = post_json(
        &router,
        "/api/v1/wcs-tasks",
        "simulator-device-type-mismatch",
        json!({
            "task_type": "dws_weigh",
            "device_id": device_id,
            "payload": {"expected_weight_g": 1000}
        }),
    )
    .await;
    assert_eq!(mismatch_status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(mismatch_body["code"], "M1_DEVICE_TYPE_INVALID");

    let (_, pending) = post_json(
        &router,
        "/api/v1/wcs-tasks",
        "simulator-disable-create",
        json!({
            "task_type": "pod_move",
            "device_id": device_id,
            "payload": {"pod_code": "POD-SIM-02", "workstation": "WS-02"}
        }),
    )
    .await;
    sqlx::query("UPDATE iot_devices SET enabled = false, online_status = 'disabled' WHERE id = $1")
        .bind(device_id)
        .execute(&pool)
        .await
        .expect("disable simulator device");
    let disabled_path = format!(
        "/api/v1/wcs-tasks/{}/dispatch",
        pending["id"].as_str().expect("pending simulator task id")
    );
    let (disabled_status, disabled_body) = post_json(
        &router,
        &disabled_path,
        "simulator-disabled-dispatch",
        json!({}),
    )
    .await;
    assert_eq!(disabled_status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(disabled_body["code"], "M1_DEVICE_DISABLED");
}

#[sqlx::test(migrations = "../../migrations")]
async fn warehouse_scope_filters_reads_commands_and_dashboard(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let allowed_warehouse = Uuid::new_v4();
    let denied_warehouse = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'SIM-SCOPE', 'Simulator scope')",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert scoped owner");
    let unrestricted = ctx(owner_id);
    let allowed_device =
        register_device_in_warehouse(&pool, &unrestricted, allowed_warehouse, "SIM-SCOPE-ALLOWED")
            .await;
    let denied_device =
        register_device_in_warehouse(&pool, &unrestricted, denied_warehouse, "SIM-SCOPE-DENIED")
            .await;
    let service = WcsTaskService::new(pool.clone());
    let create = |device_id, pod_code: &str| CreateWcsTaskRequest {
        task_type: "pod_move".into(),
        device_id,
        location_id: None,
        business_ref_type: None,
        business_ref_no: None,
        payload: json!({"pod_code": pod_code, "workstation": "WS-SCOPE"}),
    };
    let allowed_task = service
        .create_task(
            &unrestricted,
            create(allowed_device, "POD-SCOPE-ALLOWED"),
            "scope-create-allowed",
        )
        .await
        .expect("create allowed task");
    let denied_task = service
        .create_task(
            &unrestricted,
            create(denied_device, "POD-SCOPE-DENIED"),
            "scope-create-denied",
        )
        .await
        .expect("create denied task");

    let mut limited = ctx(owner_id);
    limited.warehouse_scope = Some(allowed_warehouse);
    let router = wcs_task_router(WcsTaskAppState::with_postgres(pool)).layer(Extension(limited));

    let (list_status, list) = get_json(&router, "/api/v1/wcs-tasks").await;
    assert_eq!(list_status, StatusCode::OK);
    let ids: Vec<&str> = list
        .as_array()
        .expect("task list")
        .iter()
        .filter_map(|task| task["id"].as_str())
        .collect();
    assert!(ids.contains(&allowed_task.id.to_string().as_str()));
    assert!(!ids.contains(&denied_task.id.to_string().as_str()));

    let denied_path = format!("/api/v1/wcs-tasks/{}", denied_task.id);
    let (get_status, get_body) = get_json(&router, &denied_path).await;
    assert_eq!(get_status, StatusCode::FORBIDDEN, "{get_body}");
    for field in [
        "code",
        "message",
        "severity",
        "details",
        "trace_id",
        "retry_hint",
    ] {
        assert!(get_body.get(field).is_some(), "统一错误体缺少 {field}");
    }
    let (invalid_status, invalid_body) = get_json(&router, "/api/v1/wcs-tasks/not-a-uuid").await;
    assert_eq!(invalid_status, StatusCode::BAD_REQUEST, "{invalid_body}");
    assert_eq!(invalid_body["code"], "M1_DEVICE_INVALID_REQUEST");
    assert!(invalid_body.get("details").is_some());
    assert!(invalid_body.get("trace_id").is_some());
    for (suffix, key, body) in [
        ("dispatch", "scope-dispatch", json!({})),
        ("receipt", "scope-receipt", json!({"outcome": "start"})),
        ("resend", "scope-resend", json!({"reason": "test"})),
        ("void", "scope-void", json!({"reason": "test"})),
        (
            "confirm-skip",
            "scope-confirm-skip",
            json!({"reason": "test", "qty": null}),
        ),
    ] {
        let path = format!("{denied_path}/{suffix}");
        let (status, body) = post_json(&router, &path, key, body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{suffix}: {body}");
    }

    let dashboard_path = format!("/api/v1/device-dashboard?warehouse_id={allowed_warehouse}");
    let (dashboard_status, dashboard) = get_json(&router, &dashboard_path).await;
    assert_eq!(dashboard_status, StatusCode::OK, "{dashboard}");
    assert_eq!(dashboard["total_devices"], 1);
    assert_eq!(dashboard["pending_tasks"], 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn failed_skip_confirmation_is_audited_and_publishes_failure(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let context = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id).await;
    let router =
        wcs_task_router(WcsTaskAppState::with_postgres(pool.clone())).layer(Extension(context));
    let (_, created) = post_json(
        &router,
        "/api/v1/wcs-tasks",
        "skip-failure-create",
        json!({
            "task_type": "pod_move",
            "device_id": device_id,
            "business_ref_type": "putaway",
            "payload": {"pod_code": "POD-SKIP-FAIL", "workstation": "WS-SKIP"}
        }),
    )
    .await;
    let task_id = created["id"].as_str().expect("created task id");
    let dispatch_path = format!("/api/v1/wcs-tasks/{task_id}/dispatch");
    let (dispatch_status, _) =
        post_json(&router, &dispatch_path, "skip-failure-dispatch", json!({})).await;
    assert_eq!(dispatch_status, StatusCode::OK);

    let skip_path = format!("/api/v1/wcs-tasks/{task_id}/confirm-skip");
    let (skip_status, skip_body) = post_json(
        &router,
        &skip_path,
        "skip-failure-confirm",
        json!({"reason": "现场已处理但缺少落账证据", "qty": 1}),
    )
    .await;
    assert_eq!(skip_status, StatusCode::UNPROCESSABLE_ENTITY, "{skip_body}");

    let task_uuid = Uuid::parse_str(task_id).expect("task uuid");
    let task_status: String = sqlx::query_scalar("SELECT status FROM wcs_tasks WHERE id = $1")
        .bind(task_uuid)
        .fetch_one(&pool)
        .await
        .expect("load failed skip task");
    assert_eq!(task_status, "failed");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event WHERE resource_id = $1 AND action = 'confirm_skip_wcs_task_failed'",
    )
    .bind(task_id)
    .fetch_one(&pool)
    .await
    .expect("load failed skip audit");
    assert_eq!(audit_count, 1);
    let failure_events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_bus_event WHERE resource_id = $1 AND event_type = 'business.wcs_task_failed'",
    )
    .bind(task_id)
    .fetch_one(&pool)
    .await
    .expect("load failed skip event");
    assert_eq!(failure_events, 1);
    let (replay_status, replay_body) = post_json(
        &router,
        &skip_path,
        "skip-failure-confirm",
        json!({"reason": "现场已处理但缺少落账证据", "qty": 1}),
    )
    .await;
    assert_eq!(
        replay_status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{replay_body}"
    );
    let (conflict_status, conflict_body) = post_json(
        &router,
        &skip_path,
        "skip-failure-confirm",
        json!({"reason": "同键不同请求", "qty": 1}),
    )
    .await;
    assert_eq!(conflict_status, StatusCode::CONFLICT, "{conflict_body}");
    let audit_count_after_replay: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event WHERE resource_id = $1 AND action = 'confirm_skip_wcs_task_failed'",
    )
    .bind(task_id)
    .fetch_one(&pool)
    .await
    .expect("load failed skip audit after replay");
    assert_eq!(audit_count_after_replay, 1, "失败重放不得重复审计");

    let resend_path = format!("/api/v1/wcs-tasks/{task_id}/resend");
    let (resend_status, resend_body) = post_json(
        &router,
        &resend_path,
        "skip-failure-resend",
        json!({"reason": "修正现场证据后重发"}),
    )
    .await;
    assert_eq!(resend_status, StatusCode::OK, "{resend_body}");
    let receipt_path = format!("/api/v1/wcs-tasks/{task_id}/receipt");
    for attempt in 1..=3 {
        let (status, body) = post_json(
            &router,
            &receipt_path,
            &format!("skip-failure-receipt-{attempt}"),
            json!({"outcome": "fail", "error_code": "SIM_RETRY_FAILED"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "attempt {attempt}: {body}");
    }
    let failure_events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_bus_event WHERE resource_id = $1 AND event_type = 'business.wcs_task_failed'",
    )
    .bind(task_id)
    .fetch_one(&pool)
    .await
    .expect("load both failed lifecycle events");
    assert_eq!(failure_events, 2, "每次重新进入 failed 都应产生独立告警");
}
