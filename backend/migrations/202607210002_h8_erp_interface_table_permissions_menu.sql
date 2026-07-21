-- US-H8-004：接口表探查专用权限与独立菜单（系统管理员默认授予）。

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:h8.erp_interface_table.read')::uuid,
    'h8.erp_interface_table.read',
    'H8 ERP 接口表探查只读'
)
ON CONFLICT DO NOTHING;

-- 探查是系统管理员专属能力；仓库主管不会因既有 connector.read 自动获得该权限。
INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'h8.erp_interface_table.read'
 WHERE lower(role.role_code) = 'system_admin'
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES (
    '00000000-0000-0000-0000-000000130043',
    '00000000-0000-0000-0000-000000120017',
    3,
    'platform.h8.erp_interface_tables',
    'platform/h8/erp_interface_tables',
    'H8 接口表探查',
    'h8-erp-interface-tables',
    'Database',
    'h8.erp_interface_table.read',
    30,
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
        ('detail', '详情', 30),
        ('field', '字段', 80),
        ('view', '视图', 90)
) AS action(key, label, sort_order)
WHERE node.id = '00000000-0000-0000-0000-000000130043'
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
WHERE node.id = '00000000-0000-0000-0000-000000130043'
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5((SELECT id::text FROM version_row) || ':' || bp.id::text)::uuid,
    (SELECT id FROM version_row),
    bp.menu_node_id,
    bp.action_key,
    bp.action_label,
    bp.action_kind,
    bp.enabled,
    bp.sort_order
FROM admin_menu_draft_button_permissions bp
WHERE bp.menu_node_id = '00000000-0000-0000-0000-000000130043'
ON CONFLICT DO NOTHING;
