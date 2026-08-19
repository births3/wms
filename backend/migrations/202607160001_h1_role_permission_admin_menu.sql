-- 补齐 H1 角色权限生产菜单；页面和权限已存在，历史迁移未发布该节点。

WITH node_seed AS (
    SELECT
        md5('admin_menu_node:platform.h1.roles')::uuid AS id,
        '00000000-0000-0000-0000-000000120008'::uuid AS parent_id,
        'platform.h1.roles'::text AS code,
        'platform/h1/roles'::text AS path,
        'H1 角色权限'::text AS title,
        'h1-role-permission'::text AS view_id,
        'ShieldCheck'::text AS icon_key,
        'h1.roles.manage'::text AS permission_key
)
INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key,
    permission_key, sort_order, enabled
)
SELECT id, parent_id, 3, code, path, title, view_id, icon_key,
       permission_key, 15, TRUE
  FROM node_seed
ON CONFLICT (id) DO UPDATE
SET parent_id = EXCLUDED.parent_id,
    code = EXCLUDED.code,
    path = EXCLUDED.path,
    title = EXCLUDED.title,
    view_id = EXCLUDED.view_id,
    icon_key = EXCLUDED.icon_key,
    permission_key = EXCLUDED.permission_key,
    sort_order = EXCLUDED.sort_order,
    enabled = TRUE,
    updated_at = now();

WITH action_seed(action_key, action_label, action_kind, sort_order) AS (
    VALUES
        ('query', '查询', 'standard', 10),
        ('refresh', '刷新', 'standard', 20),
        ('create', '新增', 'standard', 30),
        ('edit', '修改', 'standard', 40),
        ('delete', '删除', 'standard', 50),
        ('field', '字段', 'standard', 110),
        ('view', '视图', 'standard', 120),
        ('create-user', '新增用户', 'private', 200),
        ('batch-assign', '批量授权', 'private', 210)
)
INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT md5(node.id::text || ':' || action.action_key)::uuid,
       node.id, action.action_key, action.action_label, action.action_kind, TRUE, action.sort_order
  FROM action_seed action
  JOIN admin_menu_draft_nodes node ON node.code = 'platform.h1.roles'
ON CONFLICT (id) DO UPDATE
SET action_label = EXCLUDED.action_label,
    action_kind = EXCLUDED.action_kind,
    enabled = TRUE,
    sort_order = EXCLUDED.sort_order;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
), node AS (
    SELECT * FROM admin_menu_draft_nodes WHERE code = 'platform.h1.roles'
)
INSERT INTO admin_menu_version_nodes (
    id, version_id, source_node_id, parent_source_id, level, code, path, title,
    view_id, icon_key, permission_key, sort_order, enabled, created_at, updated_at
)
SELECT md5(version.id::text || ':' || node.id::text)::uuid,
       version.id, node.id, node.parent_id, node.level, node.code, node.path, node.title,
       node.view_id, node.icon_key, node.permission_key, node.sort_order, node.enabled,
       node.created_at, node.updated_at
  FROM version_row version
 CROSS JOIN node
ON CONFLICT (id) DO UPDATE
SET parent_source_id = EXCLUDED.parent_source_id,
    title = EXCLUDED.title,
    view_id = EXCLUDED.view_id,
    icon_key = EXCLUDED.icon_key,
    permission_key = EXCLUDED.permission_key,
    sort_order = EXCLUDED.sort_order,
    enabled = EXCLUDED.enabled,
    updated_at = EXCLUDED.updated_at;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
), node AS (
    SELECT id FROM admin_menu_draft_nodes WHERE code = 'platform.h1.roles'
)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label,
    action_kind, enabled, sort_order
)
SELECT md5(version.id::text || ':' || button.id::text)::uuid,
       version.id, button.menu_node_id, button.action_key, button.action_label,
       button.action_kind, button.enabled, button.sort_order
  FROM version_row version
 CROSS JOIN node
  JOIN admin_menu_draft_button_permissions button ON button.menu_node_id = node.id
ON CONFLICT (id) DO UPDATE
SET action_label = EXCLUDED.action_label,
    action_kind = EXCLUDED.action_kind,
    enabled = EXCLUDED.enabled,
    sort_order = EXCLUDED.sort_order;
