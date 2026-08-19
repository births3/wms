-- US-H8-001：基础能力 / H8 集成中心 / H8 ERP 连接（方案 C 独立菜单树）

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    (
        '00000000-0000-0000-0000-000000120017',
        '00000000-0000-0000-0000-000000110006',
        2,
        'platform.h8',
        'platform/h8',
        'H8 集成中心',
        NULL,
        'KeyRound',
        'menu.platform.h8',
        55,
        TRUE
    ),
    (
        '00000000-0000-0000-0000-000000130041',
        '00000000-0000-0000-0000-000000120017',
        3,
        'platform.h8.erp_connectors',
        'platform/h8/erp_connectors',
        'H8 ERP 连接',
        'h8-erp-connectors',
        'KeyRound',
        'h8.erp_connector.read',
        10,
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
        ('disable', '启停', 50),
        ('detail', '预览', 60),
        ('export', '导出', 70),
        ('field', '字段', 80),
        ('view', '视图', 90)
) AS action(key, label, sort_order)
WHERE node.id = '00000000-0000-0000-0000-000000130041'
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
    '00000000-0000-0000-0000-000000120017',
    '00000000-0000-0000-0000-000000130041'
)
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
WHERE bp.menu_node_id = '00000000-0000-0000-0000-000000130041'
ON CONFLICT DO NOTHING;
