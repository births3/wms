-- US-H9-006 delivery-note route snapshots and atomic cutoff aggregation.

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (
        md5('auth_permission:h9.print_orchestration.read')::uuid,
        'h9.print_orchestration.read',
        'H9 打印编排读取'
    ),
    (
        md5('auth_permission:h9.print_orchestration.write')::uuid,
        'h9.print_orchestration.write',
        'H9 打印编排维护'
    )
ON CONFLICT (lower(permission_code)) DO UPDATE
SET permission_name = EXCLUDED.permission_name;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
 CROSS JOIN auth_permissions permission
 WHERE lower(role.role_code) IN ('system_admin', 'warehouse_manager')
   AND permission.permission_code IN (
       'h9.print_orchestration.read',
       'h9.print_orchestration.write'
   )
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES (
    '00000000-0000-0000-0000-000000130061',
    '00000000-0000-0000-0000-000000120013',
    3,
    'platform.h9.delivery_note_aggregation',
    'platform/h9/delivery_note_aggregation',
    '作业·随货同行单归集',
    'h9-delivery-note-aggregation',
    'Printer',
    'h9.print_orchestration.read',
    5,
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
        ('manual_cutoff', '人工截单', 'private', 30),
        ('publish_route', '发布线路', 'private', 40),
        ('create_plan', '新建计划', 'private', 50),
        ('publish_plan', '发布计划', 'private', 60),
        ('field', '字段', 'standard', 80),
        ('view', '视图', 'standard', 90)
) AS action(key, label, kind, sort_order)
WHERE node.id = '00000000-0000-0000-0000-000000130061'
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
WHERE node.id = '00000000-0000-0000-0000-000000130061'
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
WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130061'
ON CONFLICT DO NOTHING;

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
    'print_document_category',
    '打印单据分类',
    TRUE,
    'controlled',
    '{
        "required": ["source_mode"],
        "properties": {
            "source_mode": {
                "type": "string",
                "enum": ["rendered", "external_file"]
            }
        }
    }'::jsonb,
    'global_only',
    '{}'::jsonb,
    45,
    'H9 打印编排受控单据分类'
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
    id,
    dict_code,
    item_code,
    item_name,
    enabled,
    owner_id,
    params,
    source,
    created_at,
    updated_at
)
SELECT
    '10000000-0000-0000-0000-000000000061'::uuid,
    'print_document_category',
    'delivery_note',
    '随货同行单',
    TRUE,
    NULL,
    '{"source_mode": "rendered"}'::jsonb,
    'global',
    now(),
    now()
WHERE NOT EXISTS (
    SELECT 1
      FROM system_dictionary_items
     WHERE dict_code = 'print_document_category'
       AND item_code = 'delivery_note'
       AND owner_id IS NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS customer_addresses_owner_id_uidx
    ON customer_addresses (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS customer_addresses_owner_id_customer_uidx
    ON customer_addresses (owner_id, id, customer_id);

CREATE UNIQUE INDEX IF NOT EXISTS outbound_orders_owner_id_boundary_uidx
    ON outbound_orders (owner_id, id, warehouse_id, customer_id);

CREATE TABLE IF NOT EXISTS h9_route_bindings (
    id                   UUID PRIMARY KEY,
    owner_id             UUID NOT NULL,
    warehouse_id         UUID NOT NULL,
    customer_id          UUID NOT NULL,
    delivery_address_id  UUID NOT NULL,
    route_code           TEXT NOT NULL,
    effective_from       TIMESTAMPTZ NOT NULL,
    effective_to         TIMESTAMPTZ,
    created_by           UUID NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, id),
    FOREIGN KEY (owner_id, delivery_address_id, customer_id)
        REFERENCES customer_addresses(owner_id, id, customer_id)
        ON DELETE RESTRICT,
    CHECK (length(btrim(route_code)) BETWEEN 1 AND 64),
    CHECK (effective_to IS NULL OR effective_to > effective_from)
);

CREATE INDEX IF NOT EXISTS h9_route_bindings_resolution_idx
    ON h9_route_bindings (
        owner_id,
        warehouse_id,
        delivery_address_id,
        effective_from,
        effective_to
    );

CREATE TABLE IF NOT EXISTS h9_cutoff_plans (
    id                   UUID PRIMARY KEY,
    owner_id             UUID NOT NULL,
    name                 TEXT NOT NULL,
    warehouse_id         UUID NOT NULL,
    scope_type           TEXT NOT NULL,
    customer_id          UUID,
    route_code           TEXT,
    utc_offset_minutes   SMALLINT NOT NULL,
    weekly_schedule      JSONB NOT NULL,
    exceptions           JSONB NOT NULL DEFAULT '[]'::jsonb,
    effective_from       TIMESTAMPTZ NOT NULL,
    effective_to         TIMESTAMPTZ,
    status               TEXT NOT NULL DEFAULT 'draft',
    created_by           UUID NOT NULL,
    published_by         UUID,
    published_at         TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, id),
    CHECK (length(btrim(name)) BETWEEN 1 AND 100),
    CHECK (scope_type IN ('customer', 'route', 'owner_warehouse')),
    CHECK (
        (scope_type = 'customer' AND customer_id IS NOT NULL AND route_code IS NULL)
        OR (scope_type = 'route' AND customer_id IS NULL AND route_code IS NOT NULL)
        OR (scope_type = 'owner_warehouse' AND customer_id IS NULL AND route_code IS NULL)
    ),
    CHECK (route_code IS NULL OR length(btrim(route_code)) BETWEEN 1 AND 64),
    CHECK (utc_offset_minutes BETWEEN -720 AND 840),
    CHECK (jsonb_typeof(weekly_schedule) = 'array' AND jsonb_array_length(weekly_schedule) > 0),
    CHECK (jsonb_typeof(exceptions) = 'array'),
    CHECK (effective_to IS NULL OR effective_to > effective_from),
    CHECK (status IN ('draft', 'published', 'disabled')),
    CHECK (
        (status = 'published' AND published_by IS NOT NULL AND published_at IS NOT NULL)
        OR (status <> 'published')
    )
);

CREATE INDEX IF NOT EXISTS h9_cutoff_plans_resolution_idx
    ON h9_cutoff_plans (
        owner_id,
        warehouse_id,
        status,
        scope_type,
        effective_from,
        effective_to
    );

CREATE TABLE IF NOT EXISTS h9_outbound_route_snapshots (
    outbound_order_id    UUID PRIMARY KEY,
    owner_id             UUID NOT NULL,
    warehouse_id         UUID NOT NULL,
    customer_id          UUID NOT NULL,
    delivery_address_id  UUID NOT NULL,
    route_code           TEXT NOT NULL,
    frozen_at            TIMESTAMPTZ NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (
        owner_id,
        outbound_order_id,
        warehouse_id,
        customer_id,
        delivery_address_id,
        route_code
    ),
    FOREIGN KEY (owner_id, outbound_order_id, warehouse_id, customer_id)
        REFERENCES outbound_orders(owner_id, id, warehouse_id, customer_id)
        ON DELETE RESTRICT,
    FOREIGN KEY (owner_id, delivery_address_id, customer_id)
        REFERENCES customer_addresses(owner_id, id, customer_id)
        ON DELETE RESTRICT,
    CHECK (length(btrim(route_code)) BETWEEN 1 AND 64)
);

CREATE INDEX IF NOT EXISTS h9_outbound_route_snapshots_boundary_idx
    ON h9_outbound_route_snapshots (
        owner_id,
        warehouse_id,
        delivery_address_id,
        route_code
    );

CREATE TABLE IF NOT EXISTS h9_delivery_note_groups (
    id                   UUID PRIMARY KEY,
    owner_id             UUID NOT NULL,
    warehouse_id         UUID NOT NULL,
    customer_id          UUID NOT NULL,
    delivery_address_id  UUID NOT NULL,
    route_code           TEXT NOT NULL,
    delivery_note_no     TEXT NOT NULL,
    cutoff_mode          TEXT NOT NULL,
    cutoff_reason        TEXT,
    cutoff_plan_id       UUID,
    scheduled_cutoff_at  TIMESTAMPTZ,
    cutoff_at            TIMESTAMPTZ NOT NULL,
    created_by           UUID NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (
        owner_id,
        id,
        warehouse_id,
        customer_id,
        delivery_address_id,
        route_code
    ),
    UNIQUE (delivery_note_no),
    FOREIGN KEY (owner_id, delivery_address_id, customer_id)
        REFERENCES customer_addresses(owner_id, id, customer_id)
        ON DELETE RESTRICT,
    FOREIGN KEY (owner_id, cutoff_plan_id)
        REFERENCES h9_cutoff_plans(owner_id, id)
        ON DELETE RESTRICT,
    CHECK (cutoff_mode IN ('scheduled', 'manual')),
    CHECK (length(btrim(route_code)) BETWEEN 1 AND 64),
    CHECK (
        cutoff_mode <> 'manual'
        OR cutoff_reason IS NOT NULL
        AND length(btrim(cutoff_reason)) BETWEEN 1 AND 500
    ),
    CHECK (
        (cutoff_mode = 'scheduled'
            AND cutoff_reason IS NULL
            AND cutoff_plan_id IS NOT NULL
            AND scheduled_cutoff_at IS NOT NULL)
        OR
        (cutoff_mode = 'manual'
            AND cutoff_plan_id IS NULL
            AND scheduled_cutoff_at IS NULL)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS h9_delivery_note_groups_scheduled_once_uidx
    ON h9_delivery_note_groups (
        owner_id,
        warehouse_id,
        delivery_address_id,
        cutoff_plan_id,
        scheduled_cutoff_at
    )
    WHERE cutoff_mode = 'scheduled';

CREATE INDEX IF NOT EXISTS h9_delivery_note_groups_boundary_idx
    ON h9_delivery_note_groups (
        owner_id,
        warehouse_id,
        delivery_address_id,
        cutoff_at DESC
    );

CREATE TABLE IF NOT EXISTS h9_delivery_note_group_orders (
    group_id             UUID NOT NULL,
    owner_id             UUID NOT NULL,
    outbound_order_id    UUID NOT NULL,
    warehouse_id         UUID NOT NULL,
    customer_id          UUID NOT NULL,
    delivery_address_id  UUID NOT NULL,
    route_code           TEXT NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, group_id, outbound_order_id),
    UNIQUE (owner_id, outbound_order_id),
    FOREIGN KEY (
        owner_id,
        group_id,
        warehouse_id,
        customer_id,
        delivery_address_id,
        route_code
    )
        REFERENCES h9_delivery_note_groups(
            owner_id,
            id,
            warehouse_id,
            customer_id,
            delivery_address_id,
            route_code
        )
        ON DELETE RESTRICT,
    FOREIGN KEY (
        owner_id,
        outbound_order_id,
        warehouse_id,
        customer_id,
        delivery_address_id,
        route_code
    )
        REFERENCES h9_outbound_route_snapshots(
            owner_id,
            outbound_order_id,
            warehouse_id,
            customer_id,
            delivery_address_id,
            route_code
        )
        ON DELETE RESTRICT,
    CHECK (length(btrim(route_code)) BETWEEN 1 AND 64)
);

GRANT SELECT, INSERT ON
    h9_route_bindings,
    h9_cutoff_plans,
    h9_outbound_route_snapshots,
    h9_delivery_note_groups,
    h9_delivery_note_group_orders
TO wms_app;

GRANT UPDATE ON h9_cutoff_plans TO wms_app;
