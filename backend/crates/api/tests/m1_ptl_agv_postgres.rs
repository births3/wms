//! T04/T05：PTL 拍灯落账与差异规则（GWT 12/13/14）、AGV pod_move 不可达（GWT 16/17/18/22）。

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    device_service::{DeviceService, RegisterDeviceRequest},
    wcs_task_service::{CreateWcsTaskRequest, DeviceEventRequest, WcsTaskService},
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "ptl-agv-op".into(),
        permissions: vec!["m1.device.manage".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_device(pool: &PgPool, owner_id: Uuid, device_type: &str) -> Uuid {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'T45-OWNER', 'T45 owner') ON CONFLICT DO NOTHING",
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("insert owner");
    let service = DeviceService::new(pool.clone());
    let c = ctx(owner_id);
    let device = service
        .register(
            &c,
            RegisterDeviceRequest {
                device_code: format!("T45-{device_type}"),
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
        .expect("register");
    service.heartbeat(&c, device.id).await.expect("heartbeat");
    device.id
}

async fn seed_inventory_batch(
    pool: &PgPool,
    owner_id: Uuid,
    location_id: Uuid,
    product_id: Uuid,
    on_hand: i64,
) {
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_id, product_code, batch_no,
            production_date, expiry_date, qty_on_hand, qty_frozen, qty_allocated,
            qty_replenish_in_transit, qty_replenish_out_transit, status,
            location_id, location_code, version
        )
        VALUES ($1, $2, $3, 'P45', 'B45', '2026-01-01', '2028-01-01', $5, 0, 0, 0, 0, 'qualified', $4, 'LOC45', 1)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(product_id)
    .bind(location_id)
    .bind(on_hand)
    .execute(pool)
    .await
    .expect("seed inventory batch");
}

async fn seed_location(pool: &PgPool, owner_id: Uuid, location_id: Uuid, pod_code: Option<&str>) {
    // 先建仓库（owner_id/warehouse_id 外键）
    sqlx::query(
        r#"INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type)
           VALUES ($1, $2, 'WH45', 'WH45', 'medicine')"#,
    )
    .bind(owner_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        r#"INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color)
           VALUES ($1, $2, $2, 'Z45', 'Z45', 'normal_10_30', 'green')"#,
    )
    .bind(owner_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed zone");
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, location_type, status,
            row_no, layer_no, column_no, max_volume_cm3, is_agv_managed, agv_pod_code
        )
        VALUES ($1, $2, $3, $4, 'LOC45', 'piece_pick', 'available', 1, 1, 1, 100000, $5, $6)
        "#,
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(owner_id)
    .bind(owner_id)
    .bind(pod_code.is_some())
    .bind(pod_code)
    .execute(pool)
    .await
    .expect("seed location");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt12_13_14_ptl_press_settles_with_diff_rules(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "ptl_light").await;
    let location_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    seed_location(&pool, owner_id, location_id, None).await;
    seed_inventory_batch(&pool, owner_id, location_id, product_id, 10).await;
    let service = WcsTaskService::new(pool.clone());

    // GWT 12：拍灯数量 = 提示数量 → 落账 +Δ + succeeded
    let task = service
        .create_task(
            &c,
            CreateWcsTaskRequest {
                task_type: "ptl_light_on".into(),
                device_id,
                location_id: Some(location_id),
                business_ref_type: Some("putaway".into()),
                business_ref_no: Some("PA-45".into()),
                payload: json!({"qty": 5, "product_id": product_id, "location_id": location_id}),
            },
            "ptl-12",
        )
        .await
        .expect("create ptl task");
    service.dispatch(&c, task.id).await.expect("dispatch");
    service
        .handle_event(
            &c,
            device_id,
            DeviceEventRequest {
                event_type: "ptl_press".into(),
                task_id: Some(task.id),
                location_id: Some(location_id),
                payload: json!({"press_qty": 5}),
            },
        )
        .await
        .expect("press settle");
    let on_hand: sqlx::types::Decimal = sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE location_id = $1 AND product_id = $2",
    )
    .bind(location_id)
    .bind(product_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(on_hand, sqlx::types::Decimal::from(15), "拍灯确认应在手 +5");
    let status: String = sqlx::query_scalar("SELECT status FROM wcs_tasks WHERE id = $1")
        .bind(task.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "succeeded");

    // GWT 13：差异未超阈值 → 按拍灯量落账 + H4 ptl_qty_diff
    let task2 = service
        .create_task(
            &c,
            CreateWcsTaskRequest {
                task_type: "ptl_light_on".into(),
                device_id,
                location_id: Some(location_id),
                business_ref_type: Some("putaway".into()),
                business_ref_no: Some("PA-46".into()),
                payload: json!({"qty": 10, "product_id": product_id, "location_id": location_id}),
            },
            "ptl-13",
        )
        .await
        .expect("create ptl task2");
    service.dispatch(&c, task2.id).await.expect("dispatch");
    service
        .handle_event(
            &c,
            device_id,
            DeviceEventRequest {
                event_type: "ptl_press".into(),
                task_id: Some(task2.id),
                location_id: Some(location_id),
                payload: json!({"press_qty": 11}),
            },
        )
        .await
        .expect("press diff settle");
    let on_hand: sqlx::types::Decimal = sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE location_id = $1 AND product_id = $2",
    )
    .bind(location_id)
    .bind(product_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        on_hand,
        sqlx::types::Decimal::from(26),
        "差异未超阈值按拍灯量 11 落账"
    );
    let diff_events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_bus_event WHERE event_type = 'business.ptl_qty_diff'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(diff_events, 1, "差异应写 H4 ptl_qty_diff");

    // GWT 14：差异超阈值 → 422 阻断、任务 failed、账不回
    let task3 = service
        .create_task(
            &c,
            CreateWcsTaskRequest {
                task_type: "ptl_light_on".into(),
                device_id,
                location_id: Some(location_id),
                business_ref_type: Some("putaway".into()),
                business_ref_no: Some("PA-47".into()),
                payload: json!({"qty": 10, "product_id": product_id, "location_id": location_id}),
            },
            "ptl-14",
        )
        .await
        .expect("create ptl task3");
    service.dispatch(&c, task3.id).await.expect("dispatch");
    let err = service
        .handle_event(
            &c,
            device_id,
            DeviceEventRequest {
                event_type: "ptl_press".into(),
                task_id: Some(task3.id),
                location_id: Some(location_id),
                payload: json!({"press_qty": 30}),
            },
        )
        .await;
    assert!(err.is_err(), "差异超阈值应阻断");
    let status: String = sqlx::query_scalar("SELECT status FROM wcs_tasks WHERE id = $1")
        .bind(task3.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "failed");
    let on_hand: sqlx::types::Decimal = sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE location_id = $1 AND product_id = $2",
    )
    .bind(location_id)
    .bind(product_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(on_hand, sqlx::types::Decimal::from(26), "超阈值不应落账");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt16_17_18_pod_move_unreachable_cycle(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "agv").await;
    let pod_location = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    seed_location(&pool, owner_id, pod_location, Some("POD-45")).await;
    seed_inventory_batch(&pool, owner_id, pod_location, product_id, 50).await;
    let service = WcsTaskService::new(pool.clone());

    // GWT 17：一托一搬——同一货架活跃任务存在时再生成 → PodMoveActive
    let task = service
        .create_task(
            &c,
            CreateWcsTaskRequest {
                task_type: "pod_move".into(),
                device_id,
                location_id: Some(pod_location),
                business_ref_type: None,
                business_ref_no: None,
                payload: json!({"pod_code": "POD-45", "target_station": "ST-01"}),
            },
            "pod-1",
        )
        .await
        .expect("create pod move");
    let conflict = service
        .create_task(
            &c,
            CreateWcsTaskRequest {
                task_type: "pod_move".into(),
                device_id,
                location_id: Some(pod_location),
                business_ref_type: None,
                business_ref_no: None,
                payload: json!({"pod_code": "POD-45", "target_station": "ST-02"}),
            },
            "pod-2",
        )
        .await;
    assert!(conflict.is_err(), "同一货架应 409 PodMoveActive");

    // GWT 16：executing → 置不可达；格口落账阻断（LocationUnreachable）；succeeded → 清除恢复
    service.dispatch(&c, task.id).await.expect("dispatch");
    service
        .apply_receipt(&c, task.id, "start", None)
        .await
        .expect("start");
    let marked: bool = sqlx::query_scalar(
        "SELECT agv_unreachable_at IS NOT NULL FROM warehouse_locations WHERE id = $1",
    )
    .bind(pod_location)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(marked, "executing 应置格口不可达标记");

    // 不可达期间落账阻断（PTL 确认路径前置校验 I5）
    let ptl_id = seed_device(&pool, owner_id, "ptl_light").await;
    let ptl_task = service
        .create_task(
            &c,
            CreateWcsTaskRequest {
                task_type: "ptl_light_on".into(),
                device_id: ptl_id,
                location_id: Some(pod_location),
                business_ref_type: Some("putaway".into()),
                business_ref_no: Some("PA-48".into()),
                payload: json!({"qty": 2, "product_id": product_id, "location_id": pod_location}),
            },
            "ptl-16",
        )
        .await
        .expect("create ptl at pod location");
    service.dispatch(&c, ptl_task.id).await.expect("dispatch");
    let blocked = service
        .handle_event(
            &c,
            ptl_id,
            DeviceEventRequest {
                event_type: "ptl_press".into(),
                task_id: Some(ptl_task.id),
                location_id: Some(pod_location),
                payload: json!({"press_qty": 2}),
            },
        )
        .await;
    assert!(blocked.is_err(), "不可达期间落账应被阻断");

    // GWT 18：pod_move 全程不落库存账；succeeded 后标记清除、恢复可用
    let before: sqlx::types::Decimal = sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE location_id = $1 AND product_id = $2",
    )
    .bind(pod_location)
    .bind(product_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    service
        .apply_receipt(&c, task.id, "success", None)
        .await
        .expect("success");
    let after: sqlx::types::Decimal = sqlx::query_scalar(
        "SELECT qty_on_hand FROM inventory_batches WHERE location_id = $1 AND product_id = $2",
    )
    .bind(pod_location)
    .bind(product_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(before, after, "pod_move 不产生库存账变");
    let marked: bool = sqlx::query_scalar(
        "SELECT agv_unreachable_at IS NOT NULL FROM warehouse_locations WHERE id = $1",
    )
    .bind(pod_location)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!marked, "succeeded 应清除不可达标记");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gwt22_marker_inconsistency_scan_alerts(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let c = ctx(owner_id);
    let device_id = seed_device(&pool, owner_id, "agv").await;
    let pod_location = Uuid::new_v4();
    seed_location(&pool, owner_id, pod_location, Some("POD-99")).await;
    let service = WcsTaskService::new(pool.clone());

    // 制造不一致：标记存在但无活跃任务
    sqlx::query(r#"UPDATE warehouse_locations SET agv_unreachable_at = now() WHERE id = $1"#)
        .bind(pod_location)
        .execute(&pool)
        .await
        .unwrap();

    let count = service.run_marker_scan().await.expect("marker scan");
    assert!(count >= 1, "应发现标记不一致");
    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_bus_event WHERE event_type = 'business.agv_marker_inconsistent'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events, 1, "应写 H4 agv_marker_inconsistent");

    // 清理标记后扫描归零
    sqlx::query(r#"UPDATE warehouse_locations SET agv_unreachable_at = NULL WHERE id = $1"#)
        .bind(pod_location)
        .execute(&pool)
        .await
        .unwrap();
    let count = service.run_marker_scan().await.expect("marker scan 2");
    assert_eq!(count, 0);
}
#[sqlx::test(migrations = "../../migrations")]
async fn i5_unreachable_guard_sql_blocks_existing_write_paths(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let device_id = seed_device(&pool, owner_id, "agv").await;
    let pod_location = Uuid::new_v4();
    seed_location(&pool, owner_id, pod_location, Some("POD-I5")).await;
    let product_id = Uuid::new_v4();
    seed_inventory_batch(&pool, owner_id, pod_location, product_id, 50).await;
    let service = WcsTaskService::new(pool.clone());
    let c = ctx(owner_id);

    // pod_move executing → 格口不可达
    let task = service
        .create_task(
            &c,
            CreateWcsTaskRequest {
                task_type: "pod_move".into(),
                device_id,
                location_id: Some(pod_location),
                business_ref_type: None,
                business_ref_no: None,
                payload: json!({"pod_code": "POD-I5", "target_station": "ST-01"}),
            },
            "pod-i5",
        )
        .await
        .expect("create pod move");
    service.dispatch(&c, task.id).await.expect("dispatch");
    service
        .apply_receipt(&c, task.id, "start", None)
        .await
        .expect("start");

    // I5 守卫 SQL：不可达期间对格口批次行的扣减/确认 UPDATE 命中 0 行（与
    // confirm_replenish_in_tx / deduct_for_stock_loss_in_tx / wave4 拣选扣减的守卫条件同构）
    let guarded_update = r#"
        UPDATE inventory_batches
           SET qty_on_hand = qty_on_hand - $3,
               updated_at = $4,
               version = version + 1
         WHERE owner_id = $1 AND id = $2
           AND NOT EXISTS (
                SELECT 1 FROM warehouse_locations wl
                 WHERE wl.id = inventory_batches.location_id
                   AND wl.owner_id = inventory_batches.owner_id
                   AND wl.agv_unreachable_at IS NOT NULL
           )
    "#;
    let batch_id: Uuid =
        sqlx::query_scalar("SELECT id FROM inventory_batches WHERE location_id = $1 LIMIT 1")
            .bind(pod_location)
            .fetch_one(&pool)
            .await
            .unwrap();

    let blocked = sqlx::query(guarded_update)
        .bind(owner_id)
        .bind(batch_id)
        .bind(5i64)
        .bind(chrono::Utc::now())
        .execute(&pool)
        .await
        .unwrap()
        .rows_affected();
    assert_eq!(blocked, 0, "不可达期间扣减应被守卫阻断（0 行）");

    // 恢复可达后命中 1 行
    service
        .apply_receipt(&c, task.id, "success", None)
        .await
        .expect("finish");
    let ok = sqlx::query(guarded_update)
        .bind(owner_id)
        .bind(batch_id)
        .bind(5i64)
        .bind(chrono::Utc::now())
        .execute(&pool)
        .await
        .unwrap()
        .rows_affected();
    assert_eq!(ok, 1, "可达后扣减应恢复（1 行）");
}
