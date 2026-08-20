//! T01：设备中台三表基线、CHECK/GRANT、权限/字典/M-CG/H4 种子验证。

use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn iot_tables_exist_with_expected_columns(pool: PgPool) {
    for (table, required_columns) in [
        (
            "iot_devices",
            vec![
                "id",
                "warehouse_id",
                "device_code",
                "device_type",
                "vendor",
                "model",
                "protocol",
                "ip_address",
                "port",
                "extra_config",
                "online_status",
                "last_heartbeat_at",
                "enabled",
                "version",
            ],
        ),
        (
            "wcs_tasks",
            vec![
                "id",
                "owner_id",
                "task_no",
                "task_type",
                "device_id",
                "location_id",
                "business_ref_type",
                "business_ref_no",
                "payload",
                "status",
                "ack_payload",
                "error_code",
                "error_message",
                "retry_count",
                "max_retries",
                "idempotency_key",
                "version",
            ],
        ),
        (
            "iot_event_logs",
            vec![
                "id",
                "warehouse_id",
                "device_id",
                "event_type",
                "task_id",
                "location_id",
                "payload",
                "occurred_at",
                "received_at",
            ],
        ),
    ] {
        let columns: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT column_name
              FROM information_schema.columns
             WHERE table_schema = 'public'
               AND table_name = $1
            "#,
        )
        .bind(table)
        .fetch_all(&pool)
        .await
        .expect("query columns");

        for required in required_columns {
            assert!(
                columns.iter().any(|c| c == required),
                "表 {table} 缺列 {required}"
            );
        }
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn warehouse_locations_has_agv_unreachable_at(pool: PgPool) {
    let has: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
             WHERE table_schema = 'public'
               AND table_name = 'warehouse_locations'
               AND column_name = 'agv_unreachable_at'
        )
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("query agv_unreachable_at column");

    assert!(has, "warehouse_locations 缺 agv_unreachable_at 列");
}

#[sqlx::test(migrations = "../../migrations")]
async fn device_type_and_event_type_checks_reject_invalid(pool: PgPool) {
    let warehouse_id = Uuid::new_v4();

    let bad_device = sqlx::query(
        r#"
        INSERT INTO iot_devices (warehouse_id, device_code, device_type, protocol)
        VALUES ($1, 'BAD-01', 'robot', 'http')
        "#,
    )
    .bind(warehouse_id)
    .execute(&pool)
    .await;

    assert!(bad_device.is_err(), "非法 device_type 应被 CHECK 拒绝");

    let ok_device: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO iot_devices (warehouse_id, device_code, device_type, protocol)
        VALUES ($1, 'PTL-01', 'ptl_light', 'http')
        RETURNING id
        "#,
    )
    .bind(warehouse_id)
    .fetch_one(&pool)
    .await
    .expect("合法设备应插入成功");

    let bad_event = sqlx::query(
        r#"
        INSERT INTO iot_event_logs (warehouse_id, device_id, event_type)
        VALUES ($1, $2, 'nope')
        "#,
    )
    .bind(warehouse_id)
    .bind(ok_device)
    .execute(&pool)
    .await;

    assert!(bad_event.is_err(), "非法 event_type 应被 CHECK 拒绝");

    let ok_event = sqlx::query(
        r#"
        INSERT INTO iot_event_logs (warehouse_id, device_id, event_type)
        VALUES ($1, $2, 'ptl_press')
        "#,
    )
    .bind(warehouse_id)
    .bind(ok_device)
    .execute(&pool)
    .await;

    assert!(ok_event.is_ok(), "合法事件应插入成功");
}

#[sqlx::test(migrations = "../../migrations")]
async fn iot_event_logs_grant_is_insert_only_for_wms_app(pool: PgPool) {
    let privileges: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT privilege_type
          FROM information_schema.table_privileges
         WHERE table_name = 'iot_event_logs'
           AND grantee = 'wms_app'
        "#,
    )
    .fetch_all(&pool)
    .await
    .expect("query iot_event_logs grants");

    assert!(
        privileges.contains(&"INSERT".to_string()),
        "wms_app 应可 INSERT iot_event_logs"
    );
    assert!(
        privileges.contains(&"SELECT".to_string()),
        "wms_app 应可 SELECT iot_event_logs"
    );
    assert!(
        !privileges.contains(&"UPDATE".to_string()),
        "iot_event_logs 为纯审计流，禁止 UPDATE"
    );
    assert!(
        !privileges.contains(&"DELETE".to_string()),
        "iot_event_logs 为纯审计流，禁止 DELETE"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn permission_dictionary_and_numbering_seeds_exist(pool: PgPool) {
    let permissions: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT permission_code FROM auth_permissions
         WHERE permission_code IN ('m1.device.manage', 'm1.device.monitor', 'm1.device-bind.manage')
         ORDER BY 1
        "#,
    )
    .fetch_all(&pool)
    .await
    .expect("query device permissions");

    assert_eq!(permissions.len(), 3, "应存在三个设备权限点");

    let dict_item: Option<String> = sqlx::query_scalar(
        r#"
        SELECT item_code FROM system_dictionary_items
         WHERE dict_code = 'document_type' AND item_code = 'wcs_task'
        "#,
    )
    .fetch_optional(&pool)
    .await
    .expect("query wcs_task dictionary item");

    assert_eq!(
        dict_item.as_deref(),
        Some("wcs_task"),
        "document_type 应有 wcs_task"
    );

    let rule: Option<String> = sqlx::query_scalar(
        r#"
        SELECT document_type FROM document_number_rules
         WHERE document_type = 'wcs_task'
        "#,
    )
    .fetch_optional(&pool)
    .await
    .expect("query wcs_task numbering rule");

    assert_eq!(
        rule.as_deref(),
        Some("wcs_task"),
        "M-CG 应有 wcs_task 编号规则"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_alert_seeds_exist_per_owner(pool: PgPool) {
    // 告警种子按 owner 触发（auth_owners 触发器路径）：先建 owner 再断言。
    let owner_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO auth_owners (id, owner_code, owner_name)
        VALUES ($1, 'T01-SEED', 'T01 seed owner')
        "#,
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert owner");

    let alerts: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT alert_code FROM alert_definitions
         WHERE owner_id = $1
           AND alert_code IN ('device_offline', 'device_event_orphan', 'wcs_task_failed',
                              'wcs_task_stalled', 'ptl_qty_diff', 'agv_marker_inconsistent')
        "#,
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("query device alert seeds");

    for expected in [
        "device_offline",
        "device_event_orphan",
        "wcs_task_failed",
        "wcs_task_stalled",
        "ptl_qty_diff",
        "agv_marker_inconsistent",
    ] {
        assert!(
            alerts.iter().any(|a| a == expected),
            "缺少 H4 告警种子 {expected}"
        );
    }
}
