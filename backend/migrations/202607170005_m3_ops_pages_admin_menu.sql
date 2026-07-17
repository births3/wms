-- M3 盘点 / 养护 / 移库管理端入口。

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    ('00000000-0000-0000-0000-000000130037', '00000000-0000-0000-0000-000000120007', 3,
     'inventory.counts', 'inventory/management/counts', 'M3 库存盘点', 'm3-counts', 'ClipboardList', 'm3.read', 25, TRUE),
    ('00000000-0000-0000-0000-000000130038', '00000000-0000-0000-0000-000000120007', 3,
     'inventory.maintenance', 'inventory/management/maintenance', 'M3 在库养护', 'm3-maintenance', 'ClipboardList', 'm3.read', 26, TRUE),
    ('00000000-0000-0000-0000-000000130039', '00000000-0000-0000-0000-000000120007', 3,
     'inventory.relocations', 'inventory/management/relocations', 'M3 库内移库', 'm3-relocations', 'Layers', 'm3.read', 27, TRUE)
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
        ('export', '导出', 50),
        ('field', '字段', 60),
        ('view', '视图', 70)
) AS action(key, label, sort_order)
WHERE node.view_id IN ('m3-counts', 'm3-maintenance', 'm3-relocations')
ON CONFLICT DO NOTHING;

WITH version_row AS (SELECT id FROM admin_menu_versions WHERE version_no = 1)
INSERT INTO admin_menu_version_nodes (
    id, version_id, source_node_id, parent_source_id, level, code, path, title,
    view_id, icon_key, permission_key, sort_order, enabled, created_at, updated_at
)
SELECT
    md5((SELECT id::text FROM version_row) || ':' || node.id::text)::uuid,
    (SELECT id FROM version_row),
    node.id, node.parent_id, node.level, node.code, node.path, node.title,
    node.view_id, node.icon_key, node.permission_key, node.sort_order, node.enabled,
    node.created_at, node.updated_at
FROM admin_menu_draft_nodes node
WHERE node.view_id IN ('m3-counts', 'm3-maintenance', 'm3-relocations')
ON CONFLICT DO NOTHING;

WITH version_row AS (SELECT id FROM admin_menu_versions WHERE version_no = 1)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5((SELECT id::text FROM version_row) || ':' || button.id::text)::uuid,
    (SELECT id FROM version_row),
    button.menu_node_id, button.action_key, button.action_label, button.action_kind,
    button.enabled, button.sort_order
FROM admin_menu_draft_button_permissions button
JOIN admin_menu_draft_nodes node ON node.id = button.menu_node_id
WHERE node.view_id IN ('m3-counts', 'm3-maintenance', 'm3-relocations')
ON CONFLICT DO NOTHING;
