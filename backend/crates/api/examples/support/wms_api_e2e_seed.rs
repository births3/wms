use sqlx::PgPool;
use std::env;
use uuid::Uuid;

pub async fn seed_mvr_matrix_approver(
    pool: &PgPool,
    password_hash: &str,
    system_admin_role_id: Uuid,
) -> Result<(), sqlx::Error> {
    let owner_id = Uuid::from_u128(1);
    let user_id = Uuid::from_u128(0x103);
    sqlx::query(
        r#"
        INSERT INTO auth_users (id, username, display_name, password_hash, status)
        VALUES ($1, 'mvr-matrix-approver', 'M-VR 矩阵确认人', $2, 'active')
        ON CONFLICT (id) DO UPDATE
        SET display_name = EXCLUDED.display_name,
            password_hash = EXCLUDED.password_hash,
            status = 'active',
            updated_at = now()
        "#,
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary)
        VALUES ($1, $2, TRUE, FALSE)
        ON CONFLICT (user_id, owner_id) DO UPDATE SET is_active = TRUE
        "#,
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_roles (user_id, owner_id, role_id)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(owner_id)
    .bind(system_admin_role_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn seed_quality_approver(pool: &PgPool, password_hash: &str) -> Result<(), sqlx::Error> {
    let owner_id = Uuid::from_u128(1);
    let user_id = Uuid::from_u128(0x201);
    let role_id = Uuid::from_u128(0x202);
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, 'm3-quality-approver', 'M3 质量审批人', $2, 'active') ON CONFLICT (id) DO UPDATE SET status = 'active', updated_at = now()",
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, FALSE) ON CONFLICT (user_id, owner_id) DO UPDATE SET is_active = TRUE",
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO auth_roles (id, owner_id, role_code, role_name) VALUES ($1, $2, 'm3_quality_approver', 'M3 质量审批人') ON CONFLICT (owner_id, lower(role_code)) DO UPDATE SET role_name = EXCLUDED.role_name",
    )
    .bind(role_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO auth_user_roles (user_id, owner_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(owner_id)
    .bind(role_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO auth_role_permissions (role_id, permission_id) SELECT $1, id FROM auth_permissions WHERE permission_code = 'm3.recall.approve' ON CONFLICT DO NOTHING",
    )
    .bind(role_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 为真实管理端 E2E 提供 M9/M10 所需的权限和最小关联业务数据。
pub async fn seed_m9_m10_capabilities(pool: &PgPool) -> Result<(), sqlx::Error> {
    let owner_id = Uuid::from_u128(1);
    let admin_role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND lower(role_code) = 'system_admin'",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await?;
    for (id, code, name) in [
        (Uuid::from_u128(0x128), "m9.billing.read", "M9 计费读取"),
        (Uuid::from_u128(0x129), "m9.write", "M9 计费维护"),
        (Uuid::from_u128(0x12a), "m10.tms.read", "M10 TMS 读取"),
        (Uuid::from_u128(0x12b), "m10.write", "M10 TMS 写入"),
    ] {
        let permission_id: Uuid = sqlx::query_scalar(
            "INSERT INTO auth_permissions (id, permission_code, permission_name) VALUES ($1, $2, $3) ON CONFLICT (lower(permission_code)) DO UPDATE SET permission_name = EXCLUDED.permission_name RETURNING id",
        )
        .bind(id)
        .bind(code)
        .bind(name)
        .fetch_one(pool)
        .await?;
        sqlx::query(
            "INSERT INTO auth_role_permissions (role_id, permission_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(admin_role_id)
        .bind(permission_id)
        .execute(pool)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, wms_order_no, erp_order_no, customer_id, warehouse_id,
            required_ship_at, status, short_pick, document_type
        )
        VALUES (
            '00000000-0000-0000-0000-000000001701', $1, 'OUT-M10-E2E-001', 'ERP-M10-E2E-001',
            '00000000-0000-0000-0000-000000001201', '00000000-0000-0000-0000-000000001301',
            '2026-07-14T12:00:00Z', 'created', FALSE, 'sales_outbound'
        )
        ON CONFLICT (owner_id, wms_order_no) DO UPDATE
        SET customer_id = EXCLUDED.customer_id,
            warehouse_id = EXCLUDED.warehouse_id,
            status = EXCLUDED.status,
            short_pick = EXCLUDED.short_pick,
            document_type = EXCLUDED.document_type,
            updated_at = now()
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 为真实管理端 E2E 提供一条可复核的已拣货出库单。
pub async fn seed_m4_review_data(pool: &PgPool) -> Result<(), sqlx::Error> {
    let owner_id = Uuid::from_u128(1);
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification,
            storage_condition, special_drug_category, status
        )
        VALUES (
            '00000000-0000-0000-0000-000000001704', $1,
            'P-M4-REVIEW-E2E-001', 'M4 复核策略 E2E 商品', '1 unit',
            'normal', 'none', 'active'
        )
        ON CONFLICT (owner_id, product_code) DO UPDATE
        SET special_drug_category = 'none', status = 'active', updated_at = now()
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_users (id, username, display_name, password_hash, status)
        VALUES ('00000000-0000-4000-8000-000000000104', 'm4-review-second', 'M4 第二复核员', 'test-hash', 'active')
        ON CONFLICT (id) DO UPDATE SET status = 'active', updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary)
        VALUES ('00000000-0000-4000-8000-000000000104', $1, TRUE, FALSE)
        ON CONFLICT (user_id, owner_id) DO UPDATE SET is_active = TRUE
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_roles (user_id, owner_id, role_id)
        SELECT '00000000-0000-4000-8000-000000000104', $1, id
          FROM auth_roles
         WHERE owner_id = $1 AND role_code = 'custodian'
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, document_type, wms_order_no, erp_order_no, customer_id,
            warehouse_id, required_ship_at, status, short_pick
        )
        VALUES (
            '00000000-0000-0000-0000-000000001702', $1, 'sales_outbound',
            'OUT-M4-REVIEW-E2E-001', 'ERP-M4-REVIEW-E2E-001',
            '00000000-0000-0000-0000-000000001201',
            '00000000-0000-0000-0000-000000001301',
            '2026-07-14T12:00:00Z', 'picked', FALSE
        )
        ON CONFLICT (id) DO UPDATE
        SET status = EXCLUDED.status,
            short_pick = EXCLUDED.short_pick,
            updated_at = now()
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO outbound_order_lines (
            id, outbound_order_id, owner_id, line_no, product_code,
            batch_no, planned_qty, picked_qty, reviewed_qty, shipped_qty,
            short_pick_qty
        )
        VALUES (
            '00000000-0000-0000-0000-000000001703',
            '00000000-0000-0000-0000-000000001702', $1, 1,
            'P-M4-REVIEW-E2E-001', 'B-M4-REVIEW-E2E-001', 8, 8, 0, 0, 0
        )
        ON CONFLICT (id) DO UPDATE
        SET planned_qty = EXCLUDED.planned_qty,
            picked_qty = EXCLUDED.picked_qty,
            reviewed_qty = EXCLUDED.reviewed_qty,
            shipped_qty = EXCLUDED.shipped_qty,
            short_pick_qty = EXCLUDED.short_pick_qty
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn seed_hal_alert_capabilities(pool: &PgPool) -> Result<(), sqlx::Error> {
    let owner_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")
        .expect("static H-AL owner UUID should parse");
    sqlx::query(
        r#"
        INSERT INTO h4_notification_configs (
            id, owner_id, event_type, enabled, template, recipient_rule,
            channels, created_by, updated_by
        ) VALUES (
            '00000000-0000-0000-0000-000000006001', $1,
            'business.inventory.changed', TRUE, '库存低于阈值：{{product_code}}',
            '{}'::jsonb, ARRAY['wechat']::text[],
            '00000000-0000-0000-0000-000000000101',
            '00000000-0000-0000-0000-000000000101'
        )
        ON CONFLICT (owner_id, event_type) DO UPDATE
        SET enabled = TRUE, channels = EXCLUDED.channels, updated_at = now()
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO alert_escalation_rules (
            id, owner_id, rule_code, rule_name, notify_lower_levels,
            off_hours_start, off_hours_end, off_hours_handler_roles,
            holiday_dates, enabled, created_by, updated_by, created_at, updated_at
        ) VALUES (
            '00000000-0000-0000-0000-000000006010', $1,
            'gsp-critical-default', 'GSP 严重告警三级升级', TRUE,
            '18:00', '08:00', ARRAY['warehouse_manager','system_admin']::text[],
            ARRAY['2026-10-01'::date], TRUE,
            '00000000-0000-0000-0000-000000000101',
            '00000000-0000-0000-0000-000000000101', now(), now()
        )
        ON CONFLICT (owner_id, rule_code) DO UPDATE
        SET rule_name = EXCLUDED.rule_name,
            notify_lower_levels = EXCLUDED.notify_lower_levels,
            off_hours_start = EXCLUDED.off_hours_start,
            off_hours_end = EXCLUDED.off_hours_end,
            off_hours_handler_roles = EXCLUDED.off_hours_handler_roles,
            holiday_dates = EXCLUDED.holiday_dates,
            enabled = TRUE,
            updated_at = now()
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "DELETE FROM alert_escalation_levels WHERE rule_id = '00000000-0000-0000-0000-000000006010'",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO alert_escalation_levels (
            id, owner_id, rule_id, level_no, threshold_seconds,
            recipient_roles, created_at, updated_at
        ) VALUES
            ('00000000-0000-0000-0000-000000006011', $1, '00000000-0000-0000-0000-000000006010', 1, 1800, ARRAY['warehouse_manager']::text[], now(), now()),
            ('00000000-0000-0000-0000-000000006012', $1, '00000000-0000-0000-0000-000000006010', 2, 7200, ARRAY['warehouse_manager','system_admin']::text[], now(), now()),
            ('00000000-0000-0000-0000-000000006013', $1, '00000000-0000-0000-0000-000000006010', 3, 86400, ARRAY['system_admin']::text[], now(), now())
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;

    for (event_id, idempotency_key, event_type, resource_type, resource_id) in [
        (
            "00000000-0000-0000-0000-000000006101",
            "hal-e2e-event-1",
            "maintenance.overdue",
            "maintenance_task",
            "MT-E2E-001",
        ),
        (
            "00000000-0000-0000-0000-000000006102",
            "hal-e2e-event-2",
            "cold_chain.break",
            "cold_chain_event",
            "CC-E2E-001",
        ),
        (
            "00000000-0000-0000-0000-000000006103",
            "hal-e2e-event-3",
            "qualification.expiry",
            "supplier",
            "SUP-E2E-001",
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO event_bus_event (
                id, owner_id, idempotency_key, event_type, source_module,
                resource_type, resource_id, payload, created_at
            ) VALUES ($1, $2, $3, $4, 'H-AL-E2E', $5, $6, '{}'::jsonb, now())
            ON CONFLICT (id) DO UPDATE SET created_at = now()
            "#,
        )
        .bind(Uuid::parse_str(event_id).expect("static H-AL event UUID should parse"))
        .bind(owner_id)
        .bind(idempotency_key)
        .bind(event_type)
        .bind(resource_type)
        .bind(resource_id)
        .execute(pool)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO alert_instances (
            id, owner_id, alert_definition_id, alert_code, severity, event_id,
            event_type, resource_type, resource_id, resource_path, warehouse_id,
            event_payload, recipients, status, dedup_key, escalation_level,
            action_description, triggered_at, notified_at, acknowledged_at,
            handled_at, created_at, updated_at
        )
        SELECT seed.id, $1, definition.id, seed.alert_code, seed.severity,
               seed.event_id, definition.event_type, seed.resource_type,
               seed.resource_id, seed.resource_path,
               '00000000-0000-0000-0000-000000001301', '{}'::jsonb,
               seed.recipients, seed.status, seed.dedup_key,
               seed.escalation_level, seed.action_description,
               seed.triggered_at, seed.triggered_at, seed.acknowledged_at,
               seed.handled_at, seed.triggered_at, now()
          FROM (VALUES
            ('00000000-0000-0000-0000-000000006201'::uuid, 'maintenance_overdue_3d', 'critical', '00000000-0000-0000-0000-000000006101'::uuid, 'maintenance_task', 'MT-E2E-001', '/inventory/maintenance/MT-E2E-001', ARRAY['系统管理员']::text[], 'escalated', 'hal-e2e-alert-1', 2, '已联系养护负责人', now() - interval '3 hours', NULL::timestamptz, NULL::timestamptz),
            ('00000000-0000-0000-0000-000000006202'::uuid, 'cold_chain_break_received', 'critical', '00000000-0000-0000-0000-000000006102'::uuid, 'cold_chain_event', 'CC-E2E-001', '/cold-chain/events/CC-E2E-001', ARRAY['仓库经理','系统管理员']::text[], 'handling', 'hal-e2e-alert-2', 1, '正在核对温控记录', now() - interval '40 minutes', now() - interval '35 minutes', now() - interval '20 minutes'),
            ('00000000-0000-0000-0000-000000006203'::uuid, 'qualification_expiry_30d', 'warning', '00000000-0000-0000-0000-000000006103'::uuid, 'supplier', 'SUP-E2E-001', '/master-data/suppliers/SUP-E2E-001', ARRAY['仓库经理']::text[], 'notified', 'hal-e2e-alert-3', 0, NULL::text, now() - interval '10 minutes', NULL::timestamptz, NULL::timestamptz)
          ) AS seed(id, alert_code, severity, event_id, resource_type, resource_id,
                    resource_path, recipients, status, dedup_key, escalation_level,
                    action_description, triggered_at, acknowledged_at, handled_at)
          JOIN alert_definitions definition
            ON definition.owner_id = $1 AND definition.alert_code = seed.alert_code
        ON CONFLICT (id) DO UPDATE
        SET status = EXCLUDED.status,
            escalation_level = EXCLUDED.escalation_level,
            action_description = EXCLUDED.action_description,
            triggered_at = EXCLUDED.triggered_at,
            notified_at = EXCLUDED.notified_at,
            acknowledged_at = EXCLUDED.acknowledged_at,
            handled_at = EXCLUDED.handled_at,
            closed_at = NULL,
            close_reason = NULL,
            updated_at = now()
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO alert_lifecycle_events (
            id, owner_id, alert_instance_id, from_status, to_status,
            action_description, actor_id, actor_name, occurred_at, created_at
        ) VALUES
            ('00000000-0000-0000-0000-000000006301', $1, '00000000-0000-0000-0000-000000006201', NULL, 'escalated', '升级到 L2', NULL, 'H-AL E2E', now() - interval '3 hours', now() - interval '3 hours'),
            ('00000000-0000-0000-0000-000000006302', $1, '00000000-0000-0000-0000-000000006202', NULL, 'handling', '正在核对温控记录', '00000000-0000-0000-0000-000000000101', '系统管理员', now() - interval '20 minutes', now() - interval '20 minutes'),
            ('00000000-0000-0000-0000-000000006303', $1, '00000000-0000-0000-0000-000000006203', NULL, 'notified', NULL, NULL, 'H-AL E2E', now() - interval '10 minutes', now() - interval '10 minutes')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(owner_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// US-H8-001：仓库主管仅具备 h8.erp_connector.read，用于只读 E2E。
pub async fn seed_h8_warehouse_manager(
    pool: &PgPool,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    let owner_id = Uuid::from_u128(1);
    let user_id = Uuid::from_u128(0x130);
    sqlx::query(
        r#"
        INSERT INTO auth_users (id, username, display_name, password_hash, status)
        VALUES ($1, 'wh-manager', '仓库主管', $2, 'active')
        ON CONFLICT (id) DO UPDATE
        SET username = EXCLUDED.username,
            display_name = EXCLUDED.display_name,
            password_hash = EXCLUDED.password_hash,
            status = 'active',
            updated_at = now()
        "#,
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary)
        VALUES ($1, $2, TRUE, TRUE)
        ON CONFLICT (user_id, owner_id) DO UPDATE
        SET is_active = TRUE, is_primary = TRUE
        "#,
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_permissions (id, permission_code, permission_name)
        VALUES
            (md5('auth_permission:h8.erp_connector.read')::uuid, 'h8.erp_connector.read', 'H8 ERP 连接只读'),
            (md5('auth_permission:h8.erp_connector.write')::uuid, 'h8.erp_connector.write', 'H8 ERP 连接维护')
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;
    let role_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO auth_roles (id, owner_id, role_code, role_name)
        VALUES ($1, $2, 'warehouse_manager', '仓库主管')
        ON CONFLICT (owner_id, lower(role_code)) DO UPDATE
        SET role_name = EXCLUDED.role_name
        RETURNING id
        "#,
    )
    .bind(Uuid::from_u128(0x131))
    .bind(owner_id)
    .fetch_one(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_roles (user_id, owner_id, role_id)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(owner_id)
    .bind(role_id)
    .execute(pool)
    .await?;
    // 仅读权限（写权限不得授予仓库主管）
    sqlx::query(
        r#"
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT $1, id FROM auth_permissions
         WHERE permission_code = 'h8.erp_connector.read'
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(role_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// US-H8-004：真实 MSSQL 接口表 E2E 使用的当前连接基线，仅保存 secret alias。
pub async fn seed_h8_interface_connector(pool: &PgPool) -> Result<(), sqlx::Error> {
    let host = env::var("WMS_E2E_H8_INTERFACE_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("WMS_E2E_H8_INTERFACE_DB_PORT")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(14333);
    sqlx::query(
        r#"
        INSERT INTO h8_erp_connectors (
            id, owner_id, connector_code, connector_name, warehouse_ids, directions,
            message_types, channel_mode, interface_db_host, interface_db_port,
            interface_db_name, interface_probe_db_username,
            interface_probe_db_password_alias, status
        )
        VALUES (
            '00000000-0000-0000-0000-000000008801',
            '00000000-0000-0000-0000-000000000001',
            'H8-IF-E2E', 'H8 接口表真实 E2E', ARRAY[]::uuid[],
            ARRAY['inbound', 'outbound'], ARRAY['asn'], 'interface_table',
            $1, $2, 'wms_erp_if', 'wms_h8_probe',
            'vault://wms/e2e/h8/probe', 'testing'
        )
        ON CONFLICT (owner_id, connector_code) DO UPDATE
        SET connector_name = EXCLUDED.connector_name,
            warehouse_ids = EXCLUDED.warehouse_ids,
            directions = EXCLUDED.directions,
            message_types = EXCLUDED.message_types,
            channel_mode = EXCLUDED.channel_mode,
            interface_db_host = EXCLUDED.interface_db_host,
            interface_db_port = EXCLUDED.interface_db_port,
            interface_db_name = EXCLUDED.interface_db_name,
            interface_probe_db_username = EXCLUDED.interface_probe_db_username,
            interface_probe_db_password_alias = EXCLUDED.interface_probe_db_password_alias,
            status = EXCLUDED.status,
            updated_at = now()
        "#,
    )
    .bind(host)
    .bind(port)
    .execute(pool)
    .await?;
    Ok(())
}
