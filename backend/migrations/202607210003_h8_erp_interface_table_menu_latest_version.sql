-- US-H8-004：运行时读取最新已发布菜单版本；把 H8 菜单子树补到历史最新版本。
-- 旧迁移只写 version_no=1，已有发布版本较新时会导致页面不出现在菜单树。

WITH latest_version AS (
    SELECT id
      FROM admin_menu_versions
     ORDER BY version_no DESC
     LIMIT 1
), h8_nodes AS (
    SELECT id, parent_id, level, code, path, title, view_id, icon_key,
           permission_key, sort_order, enabled, created_at, updated_at
      FROM admin_menu_draft_nodes
     WHERE id IN (
         '00000000-0000-0000-0000-000000120017',
         '00000000-0000-0000-0000-000000130041',
         '00000000-0000-0000-0000-000000130042',
         '00000000-0000-0000-0000-000000130043'
     )
)
INSERT INTO admin_menu_version_nodes (
    id, version_id, source_node_id, parent_source_id, level, code, path, title,
    view_id, icon_key, permission_key, sort_order, enabled, created_at, updated_at
)
SELECT
    md5(latest.id::text || ':' || node.id::text)::uuid,
    latest.id,
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
FROM latest_version latest
CROSS JOIN h8_nodes node
ON CONFLICT (version_id, source_node_id) DO NOTHING;

WITH latest_version AS (
    SELECT id
      FROM admin_menu_versions
     ORDER BY version_no DESC
     LIMIT 1
)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label,
    action_kind, enabled, sort_order
)
SELECT
    md5(latest.id::text || ':' || button.id::text)::uuid,
    latest.id,
    button.menu_node_id,
    button.action_key,
    button.action_label,
    button.action_kind,
    button.enabled,
    button.sort_order
FROM latest_version latest
JOIN admin_menu_draft_button_permissions button
  ON button.menu_node_id IN (
      '00000000-0000-0000-0000-000000130041',
      '00000000-0000-0000-0000-000000130042',
      '00000000-0000-0000-0000-000000130043'
  )
ON CONFLICT (version_id, menu_source_node_id, action_key) DO NOTHING;
