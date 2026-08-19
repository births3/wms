-- US-M9-001 / US-M10-001 管理端入口、权限和标准按钮。

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:m9.billing.read')::uuid, 'm9.billing.read', 'M9 计费规则读取'),
    (md5('auth_permission:m9.write')::uuid, 'm9.write', 'M9 计费规则维护'),
    (md5('auth_permission:m10.tms.read')::uuid, 'm10.tms.read', 'M10 TMS 路径读取'),
    (md5('auth_permission:m10.write')::uuid, 'm10.write', 'M10 TMS 路径维护')
ON CONFLICT (lower(permission_code)) DO UPDATE
SET permission_name = EXCLUDED.permission_name;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN (
        'm9.billing.read', 'm9.write', 'm10.tms.read', 'm10.write'
    )
 WHERE role.role_code = 'system_admin'
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    (
        '00000000-0000-0000-0000-000000110007', NULL, 1,
        'value_added', 'value_added', '增值业务', NULL, 'KeyRound', 'menu.value_added', 70, TRUE
    ),
    (
        '00000000-0000-0000-0000-000000120015', '00000000-0000-0000-0000-000000110007', 2,
        'value_added.operation', 'value_added/operation', '增值作业', NULL, 'KeyRound',
        'menu.value_added.operation', 10, TRUE
    ),
    (
        '00000000-0000-0000-0000-000000130028', '00000000-0000-0000-0000-000000120015', 3,
        'value_added.billing_rules', 'value_added/operation/billing_rules', 'M9 计费规则',
        'm9-billing-rules', 'ClipboardList', 'm9.billing.read', 10, TRUE
    ),
    (
        '00000000-0000-0000-0000-000000130029', '00000000-0000-0000-0000-000000120015', 3,
        'value_added.route_plans', 'value_added/operation/route_plans', 'M10 路径规划接收',
        'm10-route-plans', 'Truck', 'm10.tms.read', 20, TRUE
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
        ('create', '新增', 30),
        ('edit', '编辑', 40),
        ('disable', '启停', 50),
        ('detail', '预览', 60),
        ('export', '导出', 70),
        ('field', '字段', 80),
        ('view', '视图', 90)
) AS action(key, label, sort_order)
WHERE node.id IN (
    '00000000-0000-0000-0000-000000130028',
    '00000000-0000-0000-0000-000000130029'
)
ON CONFLICT DO NOTHING;

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
WHERE node.id IN (
    '00000000-0000-0000-0000-000000110007',
    '00000000-0000-0000-0000-000000120015',
    '00000000-0000-0000-0000-000000130028',
    '00000000-0000-0000-0000-000000130029'
)
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
WHERE button.menu_node_id IN (
    '00000000-0000-0000-0000-000000130028',
    '00000000-0000-0000-0000-000000130029'
)
ON CONFLICT DO NOTHING;
