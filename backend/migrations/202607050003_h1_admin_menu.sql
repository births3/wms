-- US-H1-007 PC admin three-level menu, draft publishing, versions, and button permission points.

CREATE TABLE IF NOT EXISTS admin_menu_draft_nodes (
    id              UUID PRIMARY KEY,
    parent_id       UUID REFERENCES admin_menu_draft_nodes(id) ON DELETE CASCADE,
    level           INT NOT NULL CHECK (level BETWEEN 1 AND 3),
    code            TEXT NOT NULL,
    path            TEXT NOT NULL,
    title           TEXT NOT NULL,
    view_id         TEXT,
    icon_key        TEXT NOT NULL,
    permission_key  TEXT NOT NULL,
    sort_order      INT NOT NULL DEFAULT 0,
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    version         BIGINT NOT NULL DEFAULT 1,
    CHECK ((level = 1 AND parent_id IS NULL) OR (level > 1 AND parent_id IS NOT NULL)),
    CHECK ((level = 3 AND view_id IS NOT NULL) OR (level < 3 AND view_id IS NULL))
);

CREATE UNIQUE INDEX IF NOT EXISTS admin_menu_draft_nodes_code_uidx
    ON admin_menu_draft_nodes (lower(code));

CREATE UNIQUE INDEX IF NOT EXISTS admin_menu_draft_nodes_path_uidx
    ON admin_menu_draft_nodes (lower(path));

CREATE UNIQUE INDEX IF NOT EXISTS admin_menu_draft_nodes_view_uidx
    ON admin_menu_draft_nodes (lower(view_id))
    WHERE view_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS admin_menu_draft_nodes_parent_order_idx
    ON admin_menu_draft_nodes (parent_id, sort_order, title);

CREATE TABLE IF NOT EXISTS admin_menu_draft_button_permissions (
    id              UUID PRIMARY KEY,
    menu_node_id    UUID NOT NULL REFERENCES admin_menu_draft_nodes(id) ON DELETE CASCADE,
    action_key      TEXT NOT NULL,
    action_label    TEXT NOT NULL,
    action_kind     TEXT NOT NULL CHECK (action_kind IN ('standard', 'private')),
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order      INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (menu_node_id, action_key)
);

CREATE TABLE IF NOT EXISTS admin_menu_versions (
    id            UUID PRIMARY KEY,
    version_no    BIGINT NOT NULL UNIQUE,
    note          TEXT,
    published_by  UUID NOT NULL,
    published_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS admin_menu_version_nodes (
    id                  UUID PRIMARY KEY,
    version_id          UUID NOT NULL REFERENCES admin_menu_versions(id) ON DELETE CASCADE,
    source_node_id      UUID NOT NULL,
    parent_source_id    UUID,
    level               INT NOT NULL CHECK (level BETWEEN 1 AND 3),
    code                TEXT NOT NULL,
    path                TEXT NOT NULL,
    title               TEXT NOT NULL,
    view_id             TEXT,
    icon_key            TEXT NOT NULL,
    permission_key      TEXT NOT NULL,
    sort_order          INT NOT NULL DEFAULT 0,
    enabled             BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (version_id, source_node_id)
);

CREATE INDEX IF NOT EXISTS admin_menu_version_nodes_version_order_idx
    ON admin_menu_version_nodes (version_id, parent_source_id, sort_order, title);

CREATE TABLE IF NOT EXISTS admin_menu_version_button_permissions (
    id                  UUID PRIMARY KEY,
    version_id          UUID NOT NULL REFERENCES admin_menu_versions(id) ON DELETE CASCADE,
    menu_source_node_id UUID NOT NULL,
    action_key          TEXT NOT NULL,
    action_label        TEXT NOT NULL,
    action_kind         TEXT NOT NULL CHECK (action_kind IN ('standard', 'private')),
    enabled             BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order          INT NOT NULL DEFAULT 0,
    UNIQUE (version_id, menu_source_node_id, action_key)
);

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    ('00000000-0000-0000-0000-000000001901', 'h1.menu.read', 'H1 菜单读取'),
    ('00000000-0000-0000-0000-000000001902', 'h1.menu.write', 'H1 菜单维护'),
    ('00000000-0000-0000-0000-000000001903', 'h1.menu.publish', 'H1 菜单发布'),
    ('00000000-0000-0000-0000-000000002201', 'audit.read', '审计查询'),
    ('00000000-0000-0000-0000-000000002301', 'h3.contract.read', 'H3 契约读取')
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    ('00000000-0000-0000-0000-000000110001', NULL, 1, 'workspace', 'workspace', '工作台', NULL, 'Activity', 'menu.workspace', 10, TRUE),
    ('00000000-0000-0000-0000-000000110002', NULL, 1, 'master_data', 'master_data', '基础档案', NULL, 'PackageCheck', 'menu.master_data', 20, TRUE),
    ('00000000-0000-0000-0000-000000110003', NULL, 1, 'inbound', 'inbound', '入库业务', NULL, 'CheckCircle2', 'menu.inbound', 30, TRUE),
    ('00000000-0000-0000-0000-000000110004', NULL, 1, 'outbound', 'outbound', '出库业务', NULL, 'ClipboardList', 'menu.outbound', 40, TRUE),
    ('00000000-0000-0000-0000-000000110005', NULL, 1, 'inventory', 'inventory', '库内业务', NULL, 'Layers', 'menu.inventory', 50, TRUE),
    ('00000000-0000-0000-0000-000000110006', NULL, 1, 'platform', 'platform', '基础能力', NULL, 'ShieldCheck', 'menu.platform', 60, TRUE),
    ('00000000-0000-0000-0000-000000120001', '00000000-0000-0000-0000-000000110001', 2, 'workspace.overview', 'workspace/overview', '工作台概览', NULL, 'Activity', 'menu.workspace.overview', 10, TRUE),
    ('00000000-0000-0000-0000-000000120002', '00000000-0000-0000-0000-000000110002', 2, 'master_data.main', 'master_data/main', '主数据', NULL, 'PackageCheck', 'menu.master_data.main', 10, TRUE),
    ('00000000-0000-0000-0000-000000120003', '00000000-0000-0000-0000-000000110002', 2, 'master_data.warehouse', 'master_data/warehouse', '仓储资料', NULL, 'Warehouse', 'menu.master_data.warehouse', 20, TRUE),
    ('00000000-0000-0000-0000-000000120004', '00000000-0000-0000-0000-000000110002', 2, 'master_data.config', 'master_data/config', '系统配置', NULL, 'BookOpen', 'menu.master_data.config', 30, TRUE),
    ('00000000-0000-0000-0000-000000120005', '00000000-0000-0000-0000-000000110003', 2, 'inbound.operation', 'inbound/operation', '入库作业', NULL, 'CheckCircle2', 'menu.inbound.operation', 10, TRUE),
    ('00000000-0000-0000-0000-000000120018', '00000000-0000-0000-0000-000000110003', 2, 'inbound.documents', 'inbound/documents', '入库资料', NULL, 'ClipboardList', 'menu.inbound.documents', 20, TRUE),
    ('00000000-0000-0000-0000-000000120006', '00000000-0000-0000-0000-000000110004', 2, 'outbound.operation', 'outbound/operation', '出库作业', NULL, 'ClipboardList', 'menu.outbound.operation', 10, TRUE),
    ('00000000-0000-0000-0000-000000120007', '00000000-0000-0000-0000-000000110005', 2, 'inventory.management', 'inventory/management', '库存管理', NULL, 'Layers', 'menu.inventory.management', 10, TRUE),
    ('00000000-0000-0000-0000-000000120008', '00000000-0000-0000-0000-000000110006', 2, 'platform.h1', 'platform/h1', 'H1 权限租户', NULL, 'ShieldCheck', 'menu.platform.h1', 10, TRUE),
    ('00000000-0000-0000-0000-000000120009', '00000000-0000-0000-0000-000000110006', 2, 'platform.h2', 'platform/h2', 'H2 审计能力', NULL, 'ClipboardList', 'menu.platform.h2', 20, TRUE),
    ('00000000-0000-0000-0000-000000120010', '00000000-0000-0000-0000-000000110006', 2, 'platform.h3', 'platform/h3', 'H3 契约能力', NULL, 'KeyRound', 'menu.platform.h3', 30, TRUE),
    ('00000000-0000-0000-0000-000000120011', '00000000-0000-0000-0000-000000110006', 2, 'platform.h4', 'platform/h4', 'H4 企业微信', NULL, 'Bell', 'menu.platform.h4', 40, TRUE),
    ('00000000-0000-0000-0000-000000120012', '00000000-0000-0000-0000-000000110006', 2, 'platform.h5', 'platform/h5', 'H5 快递能力', NULL, 'Truck', 'menu.platform.h5', 50, TRUE),
    ('00000000-0000-0000-0000-000000120013', '00000000-0000-0000-0000-000000110006', 2, 'platform.h9', 'platform/h9', 'H9 打印能力', NULL, 'Printer', 'menu.platform.h9', 90, TRUE),
    ('00000000-0000-0000-0000-000000130001', '00000000-0000-0000-0000-000000120001', 3, 'workspace.dashboard', 'workspace/overview/dashboard', '运营总览', 'dashboard', 'Activity', 'h1.auth.me', 10, TRUE),
    ('00000000-0000-0000-0000-000000130002', '00000000-0000-0000-0000-000000120002', 3, 'master_data.products', 'master_data/main/products', 'M1 商品档案', 'm1-products', 'PackageCheck', 'm1.master_data.read', 10, TRUE),
    ('00000000-0000-0000-0000-000000130003', '00000000-0000-0000-0000-000000120002', 3, 'master_data.business_partners', 'master_data/main/business_partners', 'M1 客商档案', 'm1-business-partners', 'Users', 'm1.master_data.read', 20, TRUE),
    ('00000000-0000-0000-0000-000000130004', '00000000-0000-0000-0000-000000120003', 3, 'master_data.warehouses', 'master_data/warehouse/warehouses', 'M1 仓库管理', 'm1-warehouses', 'Warehouse', 'm1.master_data.read', 10, TRUE),
    ('00000000-0000-0000-0000-000000130005', '00000000-0000-0000-0000-000000120003', 3, 'master_data.zones', 'master_data/warehouse/zones', 'M1 库区管理', 'm1-zones', 'MapPinned', 'm1.master_data.read', 20, TRUE),
    ('00000000-0000-0000-0000-000000130006', '00000000-0000-0000-0000-000000120003', 3, 'master_data.locations', 'master_data/warehouse/locations', 'M1 库位管理', 'm1-locations', 'MapPinned', 'm1.master_data.read', 30, TRUE),
    ('00000000-0000-0000-0000-000000130007', '00000000-0000-0000-0000-000000120004', 3, 'master_data.system_dictionary', 'master_data/config/system_dictionary', 'M1 系统字典', 'm1-system-dictionary', 'BookOpen', 'm1.system_dictionary.read', 10, TRUE),
    ('00000000-0000-0000-0000-000000130008', '00000000-0000-0000-0000-000000120004', 3, 'master_data.feature_flags', 'master_data/config/feature_flags', 'M1 Feature Flag', 'm1-feature-flags', 'KeyRound', 'm1.config.write', 20, TRUE),
    ('00000000-0000-0000-0000-000000130009', '00000000-0000-0000-0000-000000120005', 3, 'inbound.receiving', 'inbound/operation/receiving', 'M2 收货管理', 'm2-receiving', 'CheckCircle2', 'm2.write', 10, TRUE),
    ('00000000-0000-0000-0000-000000130010', '00000000-0000-0000-0000-000000120005', 3, 'inbound.inspecting', 'inbound/operation/inspecting', 'M2 验收管理', 'm2-inspecting', 'ClipboardList', 'm2.write', 20, TRUE),
    ('00000000-0000-0000-0000-000000130011', '00000000-0000-0000-0000-000000120005', 3, 'inbound.putaway', 'inbound/operation/putaway', 'M2 上架管理', 'm2-putaway', 'PackageCheck', 'm2.write', 30, TRUE),
    ('00000000-0000-0000-0000-000000130012', '00000000-0000-0000-0000-000000120006', 3, 'outbound.orders', 'outbound/operation/orders', 'M4 出库订单管理', 'm4-orders', 'ClipboardList', 'm4.read', 10, TRUE),
    ('00000000-0000-0000-0000-000000130013', '00000000-0000-0000-0000-000000120006', 3, 'outbound.waves', 'outbound/operation/waves', 'M4 波次规划', 'm4-waves', 'PackageCheck', 'm4.read', 20, TRUE),
    ('00000000-0000-0000-0000-000000130014', '00000000-0000-0000-0000-000000120006', 3, 'outbound.review', 'outbound/operation/review', 'M4 复核发货', 'm4-review', 'CheckCircle2', 'm4.read', 30, TRUE),
    ('00000000-0000-0000-0000-000000130015', '00000000-0000-0000-0000-000000120006', 3, 'outbound.purchase_returns', 'outbound/operation/purchase_returns', 'M4 采购退货出库', 'm4-returns', 'ClipboardList', 'm4.read', 40, TRUE),
    ('00000000-0000-0000-0000-000000130016', '00000000-0000-0000-0000-000000120007', 3, 'inventory.batches', 'inventory/management/batches', 'M3 批号管理', 'm3-batches', 'Layers', 'm3.read', 10, TRUE),
    ('00000000-0000-0000-0000-000000130017', '00000000-0000-0000-0000-000000120008', 3, 'platform.h1.menu_management', 'platform/h1/menu_management', 'H1 菜单管理', 'h1-menu-management', 'PanelLeftOpen', 'h1.menu.read', 10, TRUE),
    ('00000000-0000-0000-0000-000000130018', '00000000-0000-0000-0000-000000120013', 3, 'platform.h9.print_templates', 'platform/h9/print_templates', 'H9 打印模板', 'h9-print-templates', 'Printer', 'h9.print_template.read', 10, TRUE),
    ('00000000-0000-0000-0000-000000130022', '00000000-0000-0000-0000-000000120009', 3, 'platform.h2.audit_trail', 'platform/h2/audit_trail', 'H2 审计追踪', 'h2-audit-trail', 'ClipboardList', 'audit.read', 10, TRUE),
    ('00000000-0000-0000-0000-000000130023', '00000000-0000-0000-0000-000000120010', 3, 'platform.h3.api_contract', 'platform/h3/api_contract', 'H3 OpenAPI', 'h3-api-contract', 'KeyRound', 'h3.contract.read', 10, TRUE)
ON CONFLICT DO NOTHING;

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
        ('create', '新增', 'standard', 30),
        ('edit', '编辑', 'standard', 40),
        ('delete', '删除', 'standard', 50),
        ('disable', '启停', 'standard', 60),
        ('detail', '详情', 'standard', 70),
        ('export', '导出', 'standard', 80),
        ('print', '打印', 'standard', 90),
        ('summary', '汇总', 'standard', 100),
        ('field', '字段', 'standard', 110),
        ('view', '视图', 'standard', 120)
) AS action(key, label, kind, sort_order)
WHERE node.level = 3
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
VALUES
    (md5('00000000-0000-0000-0000-000000130009:receive')::uuid, '00000000-0000-0000-0000-000000130009', 'receive', '收货', 'private', TRUE, 200),
    (md5('00000000-0000-0000-0000-000000130010:inspect')::uuid, '00000000-0000-0000-0000-000000130010', 'inspect', '验收', 'private', TRUE, 200),
    (md5('00000000-0000-0000-0000-000000130011:putaway')::uuid, '00000000-0000-0000-0000-000000130011', 'putaway', '上架', 'private', TRUE, 200),
    (md5('00000000-0000-0000-0000-000000130017:publish')::uuid, '00000000-0000-0000-0000-000000130017', 'publish', '发布', 'private', TRUE, 200),
    (md5('00000000-0000-0000-0000-000000130017:rollback')::uuid, '00000000-0000-0000-0000-000000130017', 'rollback', '回滚', 'private', TRUE, 210),
    (md5('00000000-0000-0000-0000-000000130018:template_publish')::uuid, '00000000-0000-0000-0000-000000130018', 'template_publish', '发布模板', 'private', TRUE, 200)
ON CONFLICT DO NOTHING;

WITH version_row AS (
    INSERT INTO admin_menu_versions (id, version_no, note, published_by, published_at)
    VALUES (
        '00000000-0000-0000-0000-000000119001',
        1,
        '系统初始三层菜单',
        '00000000-0000-0000-0000-000000000000',
        now()
    )
    ON CONFLICT (version_no) DO UPDATE SET note = EXCLUDED.note
    RETURNING id
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
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE, DELETE ON
    admin_menu_draft_nodes,
    admin_menu_draft_button_permissions
TO wms_app;

GRANT SELECT, INSERT ON
    admin_menu_versions,
    admin_menu_version_nodes,
    admin_menu_version_button_permissions
TO wms_app;
