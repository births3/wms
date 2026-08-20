-- US-M1-010：设备档案与设备指令大盘管理端入口（Phase 3 治理收口 T07 核漏补登）。

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    (
        '00000000-0000-0000-0000-000000130095',
        '00000000-0000-0000-0000-000000120002',
        3,
        'master_data.devices',
        'master_data/main/devices',
        'M1 设备档案',
        'm1-devices',
        'Cpu',
        'm1.device.manage',
        30,
        TRUE
    ),
    (
        '00000000-0000-0000-0000-000000130096',
        '00000000-0000-0000-0000-000000120002',
        3,
        'master_data.device_dashboard',
        'master_data/main/device_dashboard',
        'M1 设备指令大盘',
        'm1-device-dashboard',
        'Activity',
        'm1.device.monitor',
        40,
        TRUE
    )
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5(node.id::text || ':' || action.key)::uuid,
    node.id,
    action.key,
    action.label,
    'standard',
    TRUE,
    action.sort_order
FROM admin_menu_draft_nodes node
CROSS JOIN (
    VALUES
        ('query', '查询', 10),
        ('refresh', '刷新', 20),
        ('export', '导出', 50),
        ('field', '字段', 60),
        ('view', '视图', 70)
) AS action(key, label, sort_order)
WHERE node.view_id IN ('m1-devices', 'm1-device-dashboard')
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5(node.id::text || ':' || action.key)::uuid,
    node.id,
    action.key,
    action.label,
    'standard',
    TRUE,
    action.sort_order
FROM admin_menu_draft_nodes node
CROSS JOIN (
    VALUES
        ('device_register', '注册设备', 30),
        ('device_bind', '库位绑定', 31),
        ('device_toggle', '启停', 32),
        ('task_resend', '重发', 33),
        ('task_void', '作废', 34),
        ('task_confirm_skip', '跳过确认', 35)
) AS action(key, label, sort_order)
WHERE node.view_id = 'm1-devices'
   OR node.view_id = 'm1-device-dashboard'
ON CONFLICT DO NOTHING;

-- 发布到 version 1（与 m3 菜单迁移同款：draft → versions）
WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
)
INSERT INTO admin_menu_version_nodes (
    id, version_id, source_node_id, parent_source_id, level, code, path, title,
    view_id, icon_key, permission_key, sort_order, enabled, created_at, updated_at
)
SELECT
    md5((SELECT id::text FROM version_row) || ':' || node.id::text)::uuid,
    (SELECT id FROM version_row),
    node.id,
    node.parent_id,
    node.level,
    node.code,
    node.path,
    node.title,
    node.view_id,
    node.icon_key,
    node.permission_key,
    node.sort_order,
    node.enabled,
    node.created_at,
    node.updated_at
FROM admin_menu_draft_nodes node
WHERE node.view_id IN ('m1-devices', 'm1-device-dashboard')
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5((SELECT id::text FROM version_row) || ':' || button.id::text)::uuid,
    (SELECT id FROM version_row),
    button.menu_node_id,
    button.action_key,
    button.action_label,
    button.action_kind,
    button.enabled,
    button.sort_order
FROM admin_menu_draft_button_permissions button
JOIN admin_menu_draft_nodes node ON node.id = button.menu_node_id
WHERE node.view_id IN ('m1-devices', 'm1-device-dashboard')
ON CONFLICT DO NOTHING;
