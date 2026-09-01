use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    device_service::{BindDeviceRequest, DeviceService, RegisterDeviceRequest, UnbindRequest},
};

mod postgres_test_support;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "device-t3".to_string(),
        permissions: vec!["m1.device.manage".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn heartbeat_bind_and_unbind_replay_with_audit(pool: PgPool) {
    // POST /api/v1/iot-devices/{id}/heartbeat
    // POST /api/v1/location-device-bindings
    // POST /api/v1/location-device-bindings/{id}/unbind
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '设备 T3 测试货主')",
    )
    .bind(owner_id)
    .bind(format!("D3-{}", &owner_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed owner");
    postgres_test_support::ensure_audit_partition(&pool, Utc::now()).await;

    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, '设备测试仓', 'physical', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("DWH-{}", &warehouse_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, 'DZ-01', '设备测试区', 'normal_10_30', 'qualified_green', 'active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await
    .expect("seed zone");
    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status) VALUES ($1, $2, $3, $4, 'DLOC-01', 1, 1, 1, 100000, 0, 10, 'piece_pick', 'available')",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(&pool)
    .await
    .expect("seed location");

    let actor = ctx(owner_id);
    let service = DeviceService::new(pool.clone());
    let device = service
        .register(
            &actor,
            RegisterDeviceRequest {
                warehouse_id,
                device_code: "PTL-T3-01".to_string(),
                device_type: "ptl_light".to_string(),
                vendor: Some("test".to_string()),
                model: Some("T3".to_string()),
                protocol: "http".to_string(),
                ip_address: None,
                port: None,
                extra_config: json!({}),
            },
            "device-t3-register",
        )
        .await
        .expect("register device");

    let heartbeat = service
        .heartbeat(&actor, device.id, "device-t3-heartbeat")
        .await
        .expect("heartbeat");
    let heartbeat_replay = service
        .heartbeat(&actor, device.id, "device-t3-heartbeat")
        .await
        .expect("heartbeat replay");
    assert_eq!(heartbeat.id, heartbeat_replay.id);
    assert_eq!(heartbeat.online_status, "online");

    let binding = service
        .bind(
            &actor,
            BindDeviceRequest {
                location_id,
                device_id: device.id,
                binding_role: "ptl_light".to_string(),
                point_address: Some("A-01".to_string()),
            },
            "device-t3-bind",
        )
        .await
        .expect("bind device");

    service
        .unbind(
            &actor,
            binding.id,
            UnbindRequest {
                reason: "T3 合同验证".to_string(),
            },
            "device-t3-unbind",
        )
        .await
        .expect("unbind device");
    service
        .unbind(
            &actor,
            binding.id,
            UnbindRequest {
                reason: "T3 合同验证".to_string(),
            },
            "device-t3-unbind",
        )
        .await
        .expect("unbind replay");

    postgres_test_support::audit_event(&pool, owner_id, 4).await;
    postgres_test_support::idempotency_request(&pool, owner_id, "device-t3-heartbeat").await;
    postgres_test_support::idempotency_request(&pool, owner_id, "device-t3-unbind").await;
}
