use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    device_service::{DeviceService, RegisterDeviceRequest},
    wcs_task_service::{CreateWcsTaskRequest, VoidRequest, WcsTaskService},
};

mod postgres_test_support;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "wcs-t3".to_string(),
        permissions: vec!["m1.device.manage".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn resend_wcs_task_and_void_wcs_task_replay_with_audit(pool: PgPool) {
    // POST /api/v1/wcs-tasks/{id}/resend
    // POST /api/v1/wcs-tasks/{id}/void
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'WCS T3 测试货主')",
    )
    .bind(owner_id)
    .bind(format!("W3-{}", &owner_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed owner");
    postgres_test_support::ensure_audit_partition(&pool, Utc::now()).await;

    let actor = ctx(owner_id);
    let device_service = DeviceService::new(pool.clone());
    let device = device_service
        .register(
            &actor,
            RegisterDeviceRequest {
                warehouse_id: owner_id,
                device_code: "PTL-WCS-T3".to_string(),
                device_type: "ptl_light".to_string(),
                vendor: None,
                model: None,
                protocol: "http".to_string(),
                ip_address: None,
                port: None,
                extra_config: json!({}),
            },
            "wcs-t3-register",
        )
        .await
        .expect("register device");
    device_service
        .heartbeat(&actor, device.id, "wcs-t3-heartbeat")
        .await
        .expect("device online");

    let service = WcsTaskService::new(pool.clone());
    let created = service
        .create_task(
            &actor,
            CreateWcsTaskRequest {
                task_type: "ptl_light_on".to_string(),
                device_id: device.id,
                location_id: None,
                business_ref_type: None,
                business_ref_no: None,
                payload: json!({"qty": 5}),
            },
            "wcs-t3-create",
        )
        .await
        .expect("create wcs task");
    let task_id = created.task.id;
    service.dispatch(&actor, task_id).await.expect("dispatch");

    for _ in 0..3 {
        service
            .apply_receipt(&actor, task_id, "fail", Some("DEV_ERR"))
            .await
            .expect("failed receipt");
    }

    let resent = service
        .resend(&actor, task_id, "T3 重发".to_string(), "wcs-t3-resend")
        .await
        .expect("resend task");
    assert_eq!(resent.status, "sent");
    let resend_replay = service
        .resend(&actor, task_id, "T3 重发".to_string(), "wcs-t3-resend")
        .await
        .expect("resend replay");
    assert_eq!(resend_replay.id, resent.id);

    let voided = service
        .void(
            &actor,
            task_id,
            VoidRequest {
                reason: "T3 作废".to_string(),
            },
            "wcs-t3-void",
        )
        .await
        .expect("void task");
    assert_eq!(voided.status, "failed");
    let void_replay = service
        .void(
            &actor,
            task_id,
            VoidRequest {
                reason: "T3 作废".to_string(),
            },
            "wcs-t3-void",
        )
        .await
        .expect("void replay");
    assert_eq!(void_replay.id, voided.id);

    postgres_test_support::audit_event(&pool, owner_id, 5).await;
    postgres_test_support::idempotency_request(&pool, owner_id, "wcs-t3-resend").await;
    postgres_test_support::idempotency_request(&pool, owner_id, "wcs-t3-void").await;
}
