-- US-CG-001 管理端入口和规则维护权限。

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:mcg.document_numbering.read')::uuid, 'mcg.document_numbering.read', 'M-CG 单据号读取'),
    (md5('auth_permission:mcg.document_numbering.write')::uuid, 'mcg.document_numbering.write', 'M-CG 单据号维护')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN ('mcg.document_numbering.read', 'mcg.document_numbering.write')
 WHERE role.role_code = 'system_admin'
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    ('00000000-0000-0000-0000-000000120014', '00000000-0000-0000-0000-000000110006', 2, 'platform.mcg', 'platform/mcg', 'M-CG 编码能力', NULL, 'KeyRound', 'menu.platform.mcg', 100, TRUE),
    ('00000000-0000-0000-0000-000000130024', '00000000-0000-0000-0000-000000120014', 3, 'platform.mcg.numbering', 'platform/mcg/numbering', 'M-CG 单据号规则', 'mcg-numbering', 'KeyRound', 'mcg.document_numbering.read', 10, TRUE)
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5('00000000-0000-0000-0000-000000130024:' || action.key)::uuid,
    '00000000-0000-0000-0000-000000130024',
    action.key,
    action.label,
    'standard',
    TRUE,
    action.sort_order
FROM (VALUES
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
    '00000000-0000-0000-0000-000000120014',
    '00000000-0000-0000-0000-000000130024'
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
 WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130024'
ON CONFLICT DO NOTHING;
