-- US-TE-002/005/009：任务组资格与任务调度管理端入口。

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    (
        '00000000-0000-0000-0000-000000130031',
        '00000000-0000-0000-0000-000000120007',
        3,
        'inventory.task_dispatch',
        'inventory/operation/task_dispatch',
        'M-TE 任务调度',
        'mte-task-dispatch',
        'ClipboardList',
        'mte.task.read_all',
        20,
        TRUE
    ),
    (
        '00000000-0000-0000-0000-000000130032',
        '00000000-0000-0000-0000-000000120007',
        3,
        'inventory.task_groups',
        'inventory/management/task_groups',
        'M-TE 任务组资格',
        'mte-task-groups',
        'Users',
        'mte.task_group.write',
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
        ('create', '新增', 30),
        ('edit', '编辑', 40),
        ('detail', '详情', 50),
        ('export', '导出', 60),
        ('field', '字段', 70),
        ('view', '视图', 80)
) AS action(key, label, sort_order)
WHERE node.id IN (
    '00000000-0000-0000-0000-000000130031',
    '00000000-0000-0000-0000-000000130032'
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
    '00000000-0000-0000-0000-000000130031',
    '00000000-0000-0000-0000-000000130032'
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
    '00000000-0000-0000-0000-000000130031',
    '00000000-0000-0000-0000-000000130032'
)
ON CONFLICT DO NOTHING;
