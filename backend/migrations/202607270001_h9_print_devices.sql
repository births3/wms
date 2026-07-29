-- US-H9-011 printers, trays and device leases scoped to physical print sites.
-- 物理打印站点是 US-H9-012 Print Agent 的执行资源边界；本迁移即为 012 预留
-- 站点与货主仓显式映射，Agent/租约/打印机全部绑定站点，站点外不可引用。

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (
        md5('auth_permission:h9.print_device.read')::uuid,
        'h9.print_device.read',
        'H9 打印设备读取'
    ),
    (
        md5('auth_permission:h9.print_device.write')::uuid,
        'h9.print_device.write',
        'H9 打印设备维护'
    ),
    (
        md5('auth_permission:h9.device_lease.release')::uuid,
        'h9.device_lease.release',
        'H9 设备租约人工释放'
    )
ON CONFLICT (lower(permission_code)) DO UPDATE
SET permission_name = EXCLUDED.permission_name;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
 CROSS JOIN auth_permissions permission
 WHERE lower(role.role_code) IN ('system_admin', 'warehouse_manager')
   AND permission.permission_code IN (
       'h9.print_device.read',
       'h9.print_device.write'
   )
ON CONFLICT DO NOTHING;

-- 人工释放租约是专用权限，仅系统管理员默认持有。
INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
 CROSS JOIN auth_permissions permission
 WHERE lower(role.role_code) = 'system_admin'
   AND permission.permission_code = 'h9.device_lease.release'
ON CONFLICT DO NOTHING;

-- 菜单：设备·Print Agent 管理（US-H9-012 复用同一节点维护站点与 Agent）。
INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES (
    '00000000-0000-0000-0000-000000130062',
    '00000000-0000-0000-0000-000000120013',
    3,
    'platform.h9.print_devices',
    'platform/h9/print_devices',
    '设备·Print Agent 管理',
    'h9-print-devices',
    'Printer',
    'h9.print_device.read',
    15,
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
    action.kind,
    TRUE,
    action.sort_order
FROM admin_menu_draft_nodes node
CROSS JOIN (
    VALUES
        ('query', '查询', 'standard', 10),
        ('refresh', '刷新', 'standard', 20),
        ('create_site', '新建站点', 'private', 30),
        ('map_owner', '映射货主仓', 'private', 40),
        ('create_printer', '新建打印机', 'private', 50),
        ('test_print', '测试打印', 'private', 60),
        ('release_lease', '人工释放租约', 'private', 70),
        ('field', '字段', 'standard', 80),
        ('view', '视图', 'standard', 90)
) AS action(key, label, kind, sort_order)
WHERE node.id = '00000000-0000-0000-0000-000000130062'
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions ORDER BY version_no DESC LIMIT 1
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
WHERE node.id = '00000000-0000-0000-0000-000000130062'
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions ORDER BY version_no DESC LIMIT 1
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
WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130062'
ON CONFLICT DO NOTHING;

-- 全局默认租约释放策略：复用受控系统字典参数惯例，默认仅人工释放；
-- 打印机可用 release_mode_override 单机覆盖；运行中的租约冻结快照。
INSERT INTO system_dictionary_categories (
    dict_code,
    dict_name,
    enabled,
    control_level,
    param_schema,
    scope_mode,
    override_policy,
    sort_order,
    remark
)
VALUES (
    'h9_device_lease_release',
    'H9 设备租约释放策略',
    TRUE,
    'controlled',
    '{
        "required": ["release_mode"],
        "properties": {
            "release_mode": {
                "type": "string",
                "enum": ["manual_only", "safe_auto"]
            }
        }
    }'::jsonb,
    'global_only',
    '{}'::jsonb,
    46,
    'H9 设备租约全局默认释放模式；打印机维护页可单机覆盖'
)
ON CONFLICT (dict_code) DO UPDATE
SET dict_name = EXCLUDED.dict_name,
    enabled = EXCLUDED.enabled,
    control_level = EXCLUDED.control_level,
    param_schema = EXCLUDED.param_schema,
    scope_mode = EXCLUDED.scope_mode,
    override_policy = EXCLUDED.override_policy,
    sort_order = EXCLUDED.sort_order,
    remark = EXCLUDED.remark,
    updated_at = now();

INSERT INTO system_dictionary_items (
    id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at
)
SELECT
    '10000000-0000-0000-0000-00000000006a'::uuid,
    'h9_device_lease_release',
    'default',
    '全局默认释放模式',
    TRUE,
    NULL,
    '{"release_mode": "manual_only"}'::jsonb,
    'global',
    now(),
    now()
WHERE NOT EXISTS (
    SELECT 1
      FROM system_dictionary_items
     WHERE dict_code = 'h9_device_lease_release'
       AND item_code = 'default'
       AND owner_id IS NULL
);

-- 物理打印站点：打印机、纸盒、租约与（012）Agent 的共同资源边界。
CREATE TABLE IF NOT EXISTS h9_print_sites (
    id          UUID PRIMARY KEY,
    site_code   TEXT NOT NULL UNIQUE,
    site_name   TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active',
    created_by  UUID NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status IN ('active', 'disabled')),
    CHECK (length(btrim(site_code)) BETWEEN 1 AND 64),
    CHECK (length(btrim(site_name)) BETWEEN 1 AND 100)
);

-- 站点 ↔ 货主+仓库显式映射；停用为软删，012 站点激活前置条件复用本表。
CREATE TABLE IF NOT EXISTS h9_print_site_owner_mappings (
    id            UUID PRIMARY KEY,
    site_id       UUID NOT NULL REFERENCES h9_print_sites(id) ON DELETE RESTRICT,
    owner_id      UUID NOT NULL,
    warehouse_id  UUID NOT NULL,
    status        TEXT NOT NULL DEFAULT 'active',
    created_by    UUID NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    disabled_by   UUID,
    disabled_at   TIMESTAMPTZ,
    CHECK (status IN ('active', 'disabled')),
    CHECK (
        (status = 'active' AND disabled_by IS NULL AND disabled_at IS NULL)
        OR (status = 'disabled' AND disabled_by IS NOT NULL AND disabled_at IS NOT NULL)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS h9_print_site_owner_mappings_active_uidx
    ON h9_print_site_owner_mappings (site_id, owner_id, warehouse_id)
    WHERE status = 'active';

-- 打印机：归属唯一站点；(site_id, id) 复合唯一键是"站点外不可引用"的引用锚点。
-- connection_type = 'usb' 表示租约语义单机：只能由实际连接它的本机 Agent 持有租约；
-- 'network' 打印机可被多个 Agent 登记，但同一时点仅一个活动租约（见部分唯一索引）。
CREATE TABLE IF NOT EXISTS h9_printers (
    id                     UUID PRIMARY KEY,
    site_id                UUID NOT NULL REFERENCES h9_print_sites(id) ON DELETE RESTRICT,
    printer_name           TEXT NOT NULL,
    printer_model          TEXT,
    connection_type        TEXT NOT NULL,
    status                 TEXT NOT NULL DEFAULT 'active',
    release_mode_override  TEXT,
    created_by             UUID NOT NULL,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (site_id, id),
    UNIQUE (site_id, printer_name),
    CHECK (connection_type IN ('network', 'usb')),
    CHECK (status IN ('active', 'disabled')),
    CHECK (release_mode_override IS NULL OR release_mode_override IN ('manual_only', 'safe_auto')),
    CHECK (length(btrim(printer_name)) BETWEEN 1 AND 100),
    CHECK (printer_model IS NULL OR length(btrim(printer_model)) BETWEEN 1 AND 100)
);

-- 纸盒：纸张能力 + 启用状态 + 设备标识；复合外键保证纸盒不越出打印机站点。
CREATE TABLE IF NOT EXISTS h9_printer_trays (
    id          UUID PRIMARY KEY,
    site_id     UUID NOT NULL,
    printer_id  UUID NOT NULL,
    tray_code   TEXT NOT NULL,
    paper_size  TEXT NOT NULL,
    paper_type  TEXT NOT NULL,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    created_by  UUID NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (printer_id, tray_code),
    UNIQUE (printer_id, id),
    FOREIGN KEY (site_id, printer_id)
        REFERENCES h9_printers(site_id, id)
        ON DELETE RESTRICT,
    CHECK (length(btrim(tray_code)) BETWEEN 1 AND 64),
    CHECK (length(btrim(paper_size)) BETWEEN 1 AND 32),
    CHECK (length(btrim(paper_type)) BETWEEN 1 AND 64)
);

-- 设备租约：holder_agent_id 在 US-H9-012 之前允许 NULL 占位；release_mode 为
-- 租约创建时刻的策略快照；busy_state 的真实来源在 US-H9-010/012（打印执行与对账），
-- 本故事先落字段并实现"printing/result_unknown/reconciling 禁止任何人释放"的硬校验。
CREATE TABLE IF NOT EXISTS h9_device_leases (
    id               UUID PRIMARY KEY,
    site_id          UUID NOT NULL,
    printer_id       UUID NOT NULL,
    holder_agent_id  UUID,
    lease_token      TEXT NOT NULL,
    release_mode     TEXT NOT NULL,
    busy_state       TEXT NOT NULL DEFAULT 'idle',
    status           TEXT NOT NULL DEFAULT 'active',
    assigned_at      TIMESTAMPTZ NOT NULL,
    acquired_at      TIMESTAMPTZ,
    released_at      TIMESTAMPTZ,
    released_by      UUID,
    release_reason   TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (site_id, printer_id)
        REFERENCES h9_printers(site_id, id)
        ON DELETE RESTRICT,
    CHECK (release_mode IN ('manual_only', 'safe_auto')),
    CHECK (busy_state IN ('idle', 'printing', 'result_unknown', 'reconciling')),
    CHECK (status IN ('active', 'released')),
    CHECK (length(btrim(lease_token)) BETWEEN 1 AND 128),
    CHECK (
        (status = 'active' AND released_at IS NULL AND released_by IS NULL)
        OR (status = 'released' AND released_at IS NOT NULL)
    ),
    CHECK (release_reason IS NULL OR length(btrim(release_reason)) BETWEEN 1 AND 500)
);

-- 同一打印机同一时点仅一个活动租约。
CREATE UNIQUE INDEX IF NOT EXISTS h9_device_leases_one_active_uidx
    ON h9_device_leases (printer_id)
    WHERE status = 'active';

CREATE INDEX IF NOT EXISTS h9_device_leases_site_idx
    ON h9_device_leases (site_id, printer_id, assigned_at DESC);

-- 测试打印结果：真实硬件在本机不可达时先落"已下发测试指令"，
-- result/result_at/result_note 作为回执字段等待 Agent（012）或人工登记。
CREATE TABLE IF NOT EXISTS h9_printer_test_prints (
    id            UUID PRIMARY KEY,
    site_id       UUID NOT NULL,
    printer_id    UUID NOT NULL,
    tray_id       UUID NOT NULL,
    result        TEXT NOT NULL DEFAULT 'dispatched',
    result_note   TEXT,
    requested_by  UUID NOT NULL,
    requested_at  TIMESTAMPTZ NOT NULL,
    result_at     TIMESTAMPTZ,
    FOREIGN KEY (site_id, printer_id)
        REFERENCES h9_printers(site_id, id)
        ON DELETE RESTRICT,
    FOREIGN KEY (printer_id, tray_id)
        REFERENCES h9_printer_trays(printer_id, id)
        ON DELETE RESTRICT,
    CHECK (result IN ('dispatched', 'succeeded', 'failed')),
    CHECK (result_note IS NULL OR length(btrim(result_note)) BETWEEN 1 AND 500)
);

CREATE INDEX IF NOT EXISTS h9_printer_test_prints_printer_idx
    ON h9_printer_test_prints (printer_id, requested_at DESC);

GRANT SELECT, INSERT, UPDATE ON
    h9_print_sites,
    h9_print_site_owner_mappings,
    h9_printers,
    h9_printer_trays,
    h9_device_leases,
    h9_printer_test_prints
TO wms_app;
