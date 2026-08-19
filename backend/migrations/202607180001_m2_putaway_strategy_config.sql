-- US-M2-010：上架策略方案绑定字段 + 无库位通知开关 + 管理端菜单

ALTER TABLE putaway_strategy_profiles
    ADD COLUMN IF NOT EXISTS warehouse_id UUID,
    ADD COLUMN IF NOT EXISTS product_category TEXT,
    ADD COLUMN IF NOT EXISTS notify_on_no_location BOOLEAN NOT NULL DEFAULT TRUE;

COMMENT ON COLUMN putaway_strategy_profiles.warehouse_id IS
    '可选仓库绑定；NULL 表示货主级通用方案。';
COMMENT ON COLUMN putaway_strategy_profiles.product_category IS
    '可选商品类别绑定；NULL 表示不限类别。';
COMMENT ON COLUMN putaway_strategy_profiles.notify_on_no_location IS
    '无可用库位时是否登记企业微信/H4 通知（仓库主管）。';

CREATE INDEX IF NOT EXISTS putaway_strategy_profiles_owner_bind_idx
    ON putaway_strategy_profiles (owner_id, warehouse_id, product_category, status);

-- 管理端入口：入库作业 → 上架策略
INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    ('00000000-0000-0000-0000-000000130040', '00000000-0000-0000-0000-000000120005', 3,
     'inbound.putaway_strategy', 'inbound/operation/putaway_strategy', 'M2 上架策略',
     'm2-putaway-strategy', 'Settings', 'm2.putaway.write', 35, TRUE)
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
        ('edit', '修改', 40),
        ('export', '导出', 50),
        ('field', '字段', 60),
        ('view', '视图', 70)
) AS action(key, label, sort_order)
WHERE node.view_id = 'm2-putaway-strategy'
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
WHERE node.view_id = 'm2-putaway-strategy'
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
WHERE node.view_id = 'm2-putaway-strategy'
ON CONFLICT DO NOTHING;
