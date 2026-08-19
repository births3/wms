-- 容器软删除：作业状态增加 disabled；菜单补编辑/删除。

ALTER TABLE lpn_containers
    DROP CONSTRAINT IF EXISTS lpn_containers_status_valid;

ALTER TABLE lpn_containers
    ADD CONSTRAINT lpn_containers_status_valid CHECK (status IN (
        'idle',
        'in_use',
        'in_transit',
        'recycling',
        'shipped',
        'disabled'
    ));

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5('00000000-0000-0000-0000-000000130045:' || action.key)::uuid,
    '00000000-0000-0000-0000-000000130045',
    action.key,
    action.label,
    'standard',
    TRUE,
    action.sort_order
FROM (VALUES
    ('edit', '编辑', 40),
    ('delete', '删除', 50)
) AS action(key, label, sort_order)
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
WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130045'
  AND button.action_key IN ('edit', 'delete')
ON CONFLICT DO NOTHING;
