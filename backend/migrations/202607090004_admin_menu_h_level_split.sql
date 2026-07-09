-- Split 基础能力 into per-H second-level menu groups.

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:audit.read')::uuid, 'audit.read', '审计查询'),
    (md5('auth_permission:h3.contract.read')::uuid, 'h3.contract.read', 'H3 契约读取')
ON CONFLICT DO NOTHING;

WITH menu_permissions AS (
    SELECT DISTINCT permission_key AS permission_code
      FROM admin_menu_draft_nodes
     WHERE permission_key IS NOT NULL
       AND trim(permission_key) <> ''
    UNION
    SELECT permission_code
      FROM (VALUES
        ('audit.read'),
        ('h3.contract.read')
      ) AS extra(permission_code)
),
inserted_permissions AS (
    INSERT INTO auth_permissions (id, permission_code, permission_name)
    SELECT
        md5('auth_permission:' || permission_code)::uuid,
        permission_code,
        permission_code
      FROM menu_permissions
    ON CONFLICT DO NOTHING
    RETURNING id
)
INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN (SELECT permission_code FROM menu_permissions)
 WHERE role.role_code = 'system_admin'
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    ('00000000-0000-0000-0000-000000120009', '00000000-0000-0000-0000-000000110006', 2, 'platform.h2', 'platform/h2', 'H2 审计能力', NULL, 'ClipboardList', 'menu.platform.h2', 20, TRUE),
    ('00000000-0000-0000-0000-000000120010', '00000000-0000-0000-0000-000000110006', 2, 'platform.h3', 'platform/h3', 'H3 契约能力', NULL, 'KeyRound', 'menu.platform.h3', 30, TRUE),
    ('00000000-0000-0000-0000-000000120011', '00000000-0000-0000-0000-000000110006', 2, 'platform.h4', 'platform/h4', 'H4 企业微信', NULL, 'Bell', 'menu.platform.h4', 40, TRUE),
    ('00000000-0000-0000-0000-000000120012', '00000000-0000-0000-0000-000000110006', 2, 'platform.h5', 'platform/h5', 'H5 快递能力', NULL, 'Truck', 'menu.platform.h5', 50, TRUE),
    ('00000000-0000-0000-0000-000000120013', '00000000-0000-0000-0000-000000110006', 2, 'platform.h9', 'platform/h9', 'H9 打印能力', NULL, 'Printer', 'menu.platform.h9', 90, TRUE),
    ('00000000-0000-0000-0000-000000130022', '00000000-0000-0000-0000-000000120009', 3, 'platform.h2.audit_trail', 'platform/h2/audit_trail', 'H2 审计追踪', 'h2-audit-trail', 'ClipboardList', 'audit.read', 10, TRUE),
    ('00000000-0000-0000-0000-000000130023', '00000000-0000-0000-0000-000000120010', 3, 'platform.h3.api_contract', 'platform/h3/api_contract', 'H3 OpenAPI', 'h3-api-contract', 'KeyRound', 'h3.contract.read', 10, TRUE)
ON CONFLICT DO NOTHING;

UPDATE admin_menu_draft_nodes
   SET code = 'platform.h1',
       path = 'platform/h1',
       title = 'H1 权限租户',
       icon_key = 'ShieldCheck',
       permission_key = 'menu.platform.h1',
       sort_order = 10,
       updated_at = now()
 WHERE id = '00000000-0000-0000-0000-000000120008';

UPDATE admin_menu_draft_nodes
   SET parent_id = '00000000-0000-0000-0000-000000120008',
       code = 'platform.h1.menu_management',
       path = 'platform/h1/menu_management',
       sort_order = 10,
       updated_at = now()
 WHERE id = '00000000-0000-0000-0000-000000130017';

UPDATE admin_menu_draft_nodes
   SET parent_id = '00000000-0000-0000-0000-000000120013',
       code = 'platform.h9.print_templates',
       path = 'platform/h9/print_templates',
       sort_order = 10,
       updated_at = now()
 WHERE id = '00000000-0000-0000-0000-000000130018';

UPDATE admin_menu_draft_nodes
   SET parent_id = '00000000-0000-0000-0000-000000120011',
       code = 'platform.h4.wechat_notify_configs',
       path = 'platform/h4/wechat_notify_configs',
       sort_order = 10,
       updated_at = now()
 WHERE id = '00000000-0000-0000-0000-000000130019';

UPDATE admin_menu_draft_nodes
   SET parent_id = '00000000-0000-0000-0000-000000120011',
       code = 'platform.h4.wechat_notify_records',
       path = 'platform/h4/wechat_notify_records',
       sort_order = 20,
       updated_at = now()
 WHERE id = '00000000-0000-0000-0000-000000130020';

UPDATE admin_menu_draft_nodes
   SET parent_id = '00000000-0000-0000-0000-000000120012',
       code = 'platform.h5.express',
       path = 'platform/h5/express',
       sort_order = 10,
       updated_at = now()
 WHERE id = '00000000-0000-0000-0000-000000130021';

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5(node.id::text || ':' || action.key)::uuid,
    node.id,
    action.key,
    action.label,
    action.kind,
    TRUE,
    action.sort_order
FROM admin_menu_draft_nodes node
CROSS JOIN (
    VALUES
        ('query', '查询', 'standard', 10),
        ('refresh', '刷新', 'standard', 20),
        ('detail', '详情', 'standard', 70),
        ('export', '导出', 'standard', 80),
        ('field', '字段', 'standard', 110),
        ('view', '视图', 'standard', 120)
) AS action(key, label, kind, sort_order)
WHERE node.id IN (
    '00000000-0000-0000-0000-000000130022',
    '00000000-0000-0000-0000-000000130023'
)
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
),
affected_nodes AS (
    SELECT *
      FROM admin_menu_draft_nodes
     WHERE id IN (
        '00000000-0000-0000-0000-000000120008',
        '00000000-0000-0000-0000-000000120009',
        '00000000-0000-0000-0000-000000120010',
        '00000000-0000-0000-0000-000000120011',
        '00000000-0000-0000-0000-000000120012',
        '00000000-0000-0000-0000-000000120013',
        '00000000-0000-0000-0000-000000130017',
        '00000000-0000-0000-0000-000000130018',
        '00000000-0000-0000-0000-000000130019',
        '00000000-0000-0000-0000-000000130020',
        '00000000-0000-0000-0000-000000130021',
        '00000000-0000-0000-0000-000000130022',
        '00000000-0000-0000-0000-000000130023'
     )
)
UPDATE admin_menu_version_nodes version_node
   SET parent_source_id = node.parent_id,
       code = node.code,
       path = node.path,
       title = node.title,
       view_id = node.view_id,
       icon_key = node.icon_key,
       permission_key = node.permission_key,
       sort_order = node.sort_order,
       enabled = node.enabled,
       updated_at = now()
  FROM affected_nodes node, version_row
 WHERE version_node.version_id = version_row.id
   AND version_node.source_node_id = node.id;

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
    '00000000-0000-0000-0000-000000120009',
    '00000000-0000-0000-0000-000000120010',
    '00000000-0000-0000-0000-000000120011',
    '00000000-0000-0000-0000-000000120012',
    '00000000-0000-0000-0000-000000120013',
    '00000000-0000-0000-0000-000000130022',
    '00000000-0000-0000-0000-000000130023'
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
    '00000000-0000-0000-0000-000000130022',
    '00000000-0000-0000-0000-000000130023'
)
ON CONFLICT DO NOTHING;
