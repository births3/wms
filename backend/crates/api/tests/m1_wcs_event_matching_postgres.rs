//! 设备事件与任务匹配、终态重放幂等回归测试。

use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    device_service::DeviceError,
    wcs_task_service::{CreateWcsTaskRequest, DeviceEventRequest, WcsTaskService},
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "event-match-op".into(),
        permissions: vec!["m1.device.manage".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_device(pool: &PgPool, warehouse_id: Uuid, code: &str, device_type: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO iot_devices (
            id, warehouse_id, device_code, device_type, protocol, online_status, enabled
        ) VALUES ($1, $2, $3, $4, 'http', 'online', true)
        "#,
    )
    .bind(id)
    .bind(warehouse_id)
    .bind(code)
    .bind(device_type)
    .execute(pool)
    .await
    .expect("seed device");
    id
}

fn event(task_id: Uuid, event_type: &str, payload: serde_json::Value) -> DeviceEventRequest {
    DeviceEventRequest {
        event_id: Uuid::new_v4(),
        event_type: event_type.into(),
        task_id: Some(task_id),
        location_id: None,
        payload,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn events_require_matching_device_and_ignore_terminal_replay(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'EVENT-OWNER', 'Event owner')",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("seed owner");
    let dws_id = seed_device(&pool, owner_id, "DWS-MATCH", "dws").await;
    let wrong_id = seed_device(&pool, owner_id, "DWS-WRONG", "dws").await;
    let rfid_id = seed_device(&pool, owner_id, "RFID-MATCH", "rfid_antenna").await;
    let ctx = ctx(owner_id);
    let service = WcsTaskService::new(pool.clone());

    let dws = service
        .create_task(
            &ctx,
            CreateWcsTaskRequest {
                task_type: "dws_weigh".into(),
                device_id: dws_id,
                location_id: None,
                business_ref_type: None,
                business_ref_no: None,
                payload: json!({"expected_weight_g": 1000}),
            },
            "dws-device-match",
        )
        .await
        .expect("create DWS task");
    service
        .dispatch(&ctx, dws.id)
        .await
        .expect("dispatch DWS task");
    assert!(matches!(
        service
            .handle_event(
                &ctx,
                wrong_id,
                event(
                    dws.id,
                    "dws_result",
                    json!({"pass": true, "weight_g": 1000})
                ),
            )
            .await,
        Err(DeviceError::EventTaskMismatch)
    ));
    assert_eq!(
        service
            .get(&ctx, dws.id)
            .await
            .expect("load DWS task")
            .status,
        "sent"
    );
    service
        .handle_event(
            &ctx,
            dws_id,
            event(
                dws.id,
                "dws_result",
                json!({"pass": true, "weight_g": 1000}),
            ),
        )
        .await
        .expect("matching DWS event");
    service
        .handle_event(
            &ctx,
            dws_id,
            event(
                dws.id,
                "dws_result",
                json!({"pass": true, "weight_g": 1000}),
            ),
        )
        .await
        .expect("terminal DWS replay");

    let rfid = service
        .create_task(
            &ctx,
            CreateWcsTaskRequest {
                task_type: "rfid_scan".into(),
                device_id: rfid_id,
                location_id: None,
                business_ref_type: None,
                business_ref_no: None,
                payload: json!({"target_epcs": ["EPC-A"]}),
            },
            "rfid-device-match",
        )
        .await
        .expect("create RFID task");
    service
        .dispatch(&ctx, rfid.id)
        .await
        .expect("dispatch RFID task");
    assert!(matches!(
        service
            .handle_event(
                &ctx,
                wrong_id,
                event(rfid.id, "rfid_batch", json!({"epcs": ["EPC-A"]})),
            )
            .await,
        Err(DeviceError::EventTaskMismatch)
    ));
    service
        .handle_event(
            &ctx,
            rfid_id,
            event(rfid.id, "rfid_batch", json!({"epcs": ["EPC-A"]})),
        )
        .await
        .expect("matching RFID event");
    service
        .handle_event(
            &ctx,
            rfid_id,
            event(rfid.id, "rfid_batch", json!({"epcs": ["EPC-A"]})),
        )
        .await
        .expect("terminal RFID replay");

    let recovery = service
        .create_task(
            &ctx,
            CreateWcsTaskRequest {
                task_type: "dws_weigh".into(),
                device_id: dws_id,
                location_id: None,
                business_ref_type: None,
                business_ref_no: None,
                payload: json!({"expected_weight_g": 1000}),
            },
            "dws-event-recovery",
        )
        .await
        .expect("create recovery task");
    service
        .dispatch(&ctx, recovery.id)
        .await
        .expect("dispatch recovery task");
    let event_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO iot_event_logs (
            id, warehouse_id, device_id, event_type, task_id, payload, received_at
        ) VALUES ($1, $2, $3, 'dws_result', $4, $5, now())
        "#,
    )
    .bind(event_id)
    .bind(owner_id)
    .bind(dws_id)
    .bind(recovery.id)
    .bind(json!({"pass": true, "weight_g": 1000}))
    .execute(&pool)
    .await
    .expect("simulate event audit committed before processing");
    service
        .handle_event(
            &ctx,
            dws_id,
            DeviceEventRequest {
                event_id,
                event_type: "dws_result".into(),
                task_id: Some(recovery.id),
                location_id: None,
                payload: json!({"pass": true, "weight_g": 1000}),
            },
        )
        .await
        .expect("retry should resume processing an existing event");
    assert_eq!(
        service
            .get(&ctx, recovery.id)
            .await
            .expect("load recovered task")
            .status,
        "succeeded"
    );
    assert!(matches!(
        service
            .handle_event(
                &ctx,
                dws_id,
                DeviceEventRequest {
                    event_id,
                    event_type: "dws_result".into(),
                    task_id: Some(dws.id),
                    location_id: None,
                    payload: json!({"pass": true, "weight_g": 1000}),
                },
            )
            .await,
        Err(DeviceError::EventTaskMismatch)
    ));
}
