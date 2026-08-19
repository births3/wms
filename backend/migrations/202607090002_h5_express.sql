-- H5 express integration: carrier config, routing rule, waybill and tracking cache.

CREATE TABLE IF NOT EXISTS h5_express_carriers (
    id                UUID PRIMARY KEY,
    owner_id          UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    carrier_code      TEXT NOT NULL,
    carrier_name      TEXT NOT NULL,
    api_url           TEXT NOT NULL,
    api_key_alias     TEXT,
    api_secret_alias  TEXT,
    account_no        TEXT,
    enabled           BOOLEAN NOT NULL DEFAULT TRUE,
    priority          INT NOT NULL DEFAULT 100 CHECK (priority >= 0),
    conditions        JSONB NOT NULL DEFAULT '{}'::jsonb,
    status            TEXT NOT NULL DEFAULT 'testing',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, carrier_code),
    CHECK (status IN ('testing', 'connected', 'disabled'))
);

CREATE INDEX IF NOT EXISTS h5_express_carriers_owner_status_idx
    ON h5_express_carriers (owner_id, enabled, priority, carrier_code);

CREATE TABLE IF NOT EXISTS h5_express_routing_rules (
    id                      UUID PRIMARY KEY,
    owner_id                UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    rule_code               TEXT NOT NULL,
    rule_name               TEXT NOT NULL,
    delivery_provider_type  TEXT NOT NULL,
    carrier_code            TEXT,
    priority                INT NOT NULL DEFAULT 100 CHECK (priority >= 0),
    conditions              JSONB NOT NULL DEFAULT '{}'::jsonb,
    fallback_strategy       TEXT,
    enabled                 BOOLEAN NOT NULL DEFAULT TRUE,
    effective_from          TIMESTAMPTZ,
    effective_to            TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, rule_code),
    CHECK (delivery_provider_type IN ('own_fleet', 'third_party_express')),
    CHECK (delivery_provider_type <> 'third_party_express' OR carrier_code IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS h5_express_routing_rules_owner_idx
    ON h5_express_routing_rules (owner_id, enabled, priority, rule_code);

CREATE TABLE IF NOT EXISTS h5_express_waybills (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    outbound_order_id   UUID REFERENCES outbound_orders(id) ON DELETE SET NULL,
    package_no          TEXT NOT NULL,
    carrier_code        TEXT NOT NULL,
    waybill_no          TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'created',
    sender_name         TEXT NOT NULL,
    sender_mobile       TEXT NOT NULL,
    sender_address      TEXT NOT NULL,
    receiver_name       TEXT NOT NULL,
    receiver_mobile     TEXT NOT NULL,
    receiver_address    TEXT NOT NULL,
    weight_grams        BIGINT NOT NULL CHECK (weight_grams > 0),
    volume_cm3          BIGINT NOT NULL CHECK (volume_cm3 >= 0),
    package_count       INT NOT NULL CHECK (package_count > 0),
    eta_at              TIMESTAMPTZ,
    idempotency_key     TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, waybill_no),
    UNIQUE (owner_id, idempotency_key),
    CHECK (status IN ('created', 'pushed', 'in_transit', 'signed', 'exception', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS h5_express_waybills_owner_status_idx
    ON h5_express_waybills (owner_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS h5_express_tracking_events (
    id             UUID PRIMARY KEY,
    owner_id       UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    waybill_id     UUID NOT NULL REFERENCES h5_express_waybills(id) ON DELETE CASCADE,
    waybill_no     TEXT NOT NULL,
    event_time     TIMESTAMPTZ NOT NULL,
    status         TEXT NOT NULL,
    location       TEXT,
    description    TEXT NOT NULL,
    source         TEXT NOT NULL DEFAULT 'carrier_cache',
    cached_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, waybill_id, event_time, status, description)
);

CREATE INDEX IF NOT EXISTS h5_express_tracking_events_waybill_idx
    ON h5_express_tracking_events (owner_id, waybill_no, event_time DESC);

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    ('00000000-0000-0000-0000-000000005501', 'h5.express.read', 'H5 快递读取'),
    ('00000000-0000-0000-0000-000000005502', 'h5.express.write', 'H5 快递维护')
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    ('00000000-0000-0000-0000-000000130021', '00000000-0000-0000-0000-000000120008', 3, 'platform.express', 'platform/capability/express', 'H5 快递对接', 'h5-express', 'Truck', 'h5.express.read', 50, TRUE)
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
        ('detail', '详情', 'standard', 70),
        ('export', '导出', 'standard', 80),
        ('print', '打印', 'standard', 90),
        ('field', '字段', 'standard', 110),
        ('view', '视图', 'standard', 120)
) AS action(key, label, kind, sort_order)
WHERE node.id = '00000000-0000-0000-0000-000000130021'
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
VALUES
    (md5('00000000-0000-0000-0000-000000130021:waybill')::uuid, '00000000-0000-0000-0000-000000130021', 'waybill', '下单', 'private', TRUE, 200),
    (md5('00000000-0000-0000-0000-000000130021:tracking')::uuid, '00000000-0000-0000-0000-000000130021', 'tracking', '轨迹', 'private', TRUE, 210)
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
WHERE node.id = '00000000-0000-0000-0000-000000130021'
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
WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130021'
ON CONFLICT DO NOTHING;

WITH h5_permissions AS (
    SELECT id FROM auth_permissions WHERE permission_code IN ('h5.express.read', 'h5.express.write')
)
INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
 CROSS JOIN h5_permissions permission
 WHERE role.role_code = 'system_admin'
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE ON
    h5_express_carriers,
    h5_express_routing_rules,
    h5_express_waybills,
    h5_express_tracking_events
TO wms_app;
