-- 修复历史菜单固定 UUID 冲突：补回被 ON CONFLICT 静默丢弃的节点和按钮。

DELETE FROM admin_menu_version_button_permissions
 WHERE menu_source_node_id = '00000000-0000-0000-0000-000000130024'
   AND action_key IN ('disable', 'revoke_others', 'kick_user');

DELETE FROM admin_menu_draft_button_permissions
 WHERE menu_node_id = '00000000-0000-0000-0000-000000130024'
   AND action_key IN ('disable', 'revoke_others', 'kick_user');

DELETE FROM admin_menu_version_button_permissions
 WHERE menu_source_node_id = '00000000-0000-0000-0000-000000130025'
   AND action_key IN ('edit', 'disable', 'export');

DELETE FROM admin_menu_draft_button_permissions
 WHERE menu_node_id = '00000000-0000-0000-0000-000000130025'
   AND action_key IN ('edit', 'disable', 'export');

WITH node_seed(id, parent_id, code, path, title, view_id, icon_key, permission_key, sort_order) AS (
    VALUES
        (
            md5('admin_menu_node:platform.h1.sessions')::uuid,
            '00000000-0000-0000-0000-000000120008'::uuid,
            'platform.h1.sessions', 'platform/h1/sessions', 'H1 登录会话',
            'h1-session-management', 'ShieldCheck', 'h1.sessions.manage', 20
        ),
        (
            md5('admin_menu_node:platform.mcg.numbering')::uuid,
            '00000000-0000-0000-0000-000000120014'::uuid,
            'platform.mcg.numbering', 'platform/mcg/numbering', 'M-CG 单据号规则',
            'mcg-numbering', 'KeyRound', 'mcg.document_numbering.read', 10
        ),
        (
            md5('admin_menu_node:master_data.docks')::uuid,
            '00000000-0000-0000-0000-000000120003'::uuid,
            'master_data.docks', 'master_data/warehouse/docks', 'M1 月台管理',
            'dock-management', 'MapPinned', 'm1.master_data.read', 40
        )
)
INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key,
    permission_key, sort_order, enabled
)
SELECT id, parent_id, 3, code, path, title, view_id, icon_key,
       permission_key, sort_order, TRUE
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

WITH action_seed(code, action_key, action_label, action_kind, sort_order) AS (
    VALUES
        ('platform.h1.sessions', 'query', '查询', 'standard', 10),
        ('platform.h1.sessions', 'refresh', '刷新', 'standard', 20),
        ('platform.h1.sessions', 'disable', '失效设备', 'standard', 60),
        ('platform.h1.sessions', 'field', '字段', 'standard', 110),
        ('platform.h1.sessions', 'view', '视图', 'standard', 120),
        ('platform.h1.sessions', 'revoke_others', '登出其他设备', 'private', 200),
        ('platform.h1.sessions', 'kick_user', '踢出用户', 'private', 210),
        ('platform.mcg.numbering', 'query', '查询', 'standard', 10),
        ('platform.mcg.numbering', 'refresh', '刷新', 'standard', 20),
        ('platform.mcg.numbering', 'create', '新增', 'standard', 30),
        ('platform.mcg.numbering', 'edit', '编辑', 'standard', 40),
        ('platform.mcg.numbering', 'disable', '启停', 'standard', 50),
        ('platform.mcg.numbering', 'detail', '预览', 'standard', 60),
        ('platform.mcg.numbering', 'export', '导出', 'standard', 70),
        ('platform.mcg.numbering', 'field', '字段', 'standard', 80),
        ('platform.mcg.numbering', 'view', '视图', 'standard', 90),
        ('master_data.docks', 'query', '查询', 'standard', 10),
        ('master_data.docks', 'refresh', '刷新', 'standard', 20),
        ('master_data.docks', 'create', '新增', 'standard', 30),
        ('master_data.docks', 'edit', '编辑', 'standard', 40),
        ('master_data.docks', 'disable', '启停', 'standard', 50),
        ('master_data.docks', 'export', '导出', 'standard', 70),
        ('master_data.docks', 'field', '字段', 'standard', 110),
        ('master_data.docks', 'view', '视图', 'standard', 120)
)
INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT md5(node.id::text || ':' || action.action_key)::uuid,
       node.id, action.action_key, action.action_label, action.action_kind, TRUE, action.sort_order
  FROM action_seed action
  JOIN admin_menu_draft_nodes node ON node.code = action.code
ON CONFLICT (id) DO UPDATE
SET action_label = EXCLUDED.action_label,
    action_kind = EXCLUDED.action_kind,
    enabled = TRUE,
    sort_order = EXCLUDED.sort_order;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
), repaired_nodes AS (
    SELECT *
      FROM admin_menu_draft_nodes
     WHERE code IN ('platform.h1.sessions', 'platform.mcg.numbering', 'master_data.docks')
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
 CROSS JOIN repaired_nodes node
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
), repaired_nodes AS (
    SELECT id
      FROM admin_menu_draft_nodes
     WHERE code IN ('platform.h1.sessions', 'platform.mcg.numbering', 'master_data.docks')
)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label,
    action_kind, enabled, sort_order
)
SELECT md5(version.id::text || ':' || button.id::text)::uuid,
       version.id, button.menu_node_id, button.action_key, button.action_label,
       button.action_kind, button.enabled, button.sort_order
  FROM version_row version
 CROSS JOIN repaired_nodes node
  JOIN admin_menu_draft_button_permissions button ON button.menu_node_id = node.id
ON CONFLICT (id) DO UPDATE
SET action_label = EXCLUDED.action_label,
    action_kind = EXCLUDED.action_kind,
    enabled = EXCLUDED.enabled,
    sort_order = EXCLUDED.sort_order;
