-- M-RC 库存对账：配置、执行批次、差异明细和权限。

CREATE TABLE reconciliation_rules (
    owner_id       UUID PRIMARY KEY REFERENCES auth_owners(id) ON DELETE RESTRICT,
    interval_hours INT NOT NULL DEFAULT 24 CHECK (interval_hours BETWEEN 1 AND 168),
    enabled        BOOLEAN NOT NULL DEFAULT TRUE,
    updated_by     UUID NOT NULL,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE reconciliation_runs (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    window_key      TEXT NOT NULL,
    request_hash    TEXT NOT NULL,
    snapshot_at     TIMESTAMPTZ NOT NULL,
    status          TEXT NOT NULL DEFAULT 'completed' CHECK (status IN ('completed', 'failed')),
    matched_count   INT NOT NULL DEFAULT 0,
    wms_more_count  INT NOT NULL DEFAULT 0,
    erp_more_count  INT NOT NULL DEFAULT 0,
    created_by      UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, window_key)
);

CREATE TABLE reconciliation_schedule_claims (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    window_key          TEXT NOT NULL,
    claim_token         UUID NOT NULL UNIQUE,
    worker_id           TEXT NOT NULL CHECK (length(btrim(worker_id)) BETWEEN 1 AND 128),
    attempt_no          INT NOT NULL CHECK (attempt_no > 0),
    status              TEXT NOT NULL CHECK (status IN ('active', 'completed', 'failed', 'expired')),
    lease_expires_at    TIMESTAMPTZ NOT NULL,
    run_id              UUID REFERENCES reconciliation_runs(id) ON DELETE RESTRICT,
    failure_stage       TEXT CHECK (failure_stage IN ('pull', 'submit', 'lease')),
    failure_code        TEXT CHECK (
        failure_code IS NULL
        OR failure_code IN ('erp_pull_failed', 'snapshot_submit_failed', 'lease_expired')
    ),
    claimed_at          TIMESTAMPTZ NOT NULL,
    updated_at          TIMESTAMPTZ NOT NULL,
    completed_at        TIMESTAMPTZ,
    failed_at           TIMESTAMPTZ,
    UNIQUE (owner_id, window_key, attempt_no),
    CHECK (
        (failure_stage IS NULL AND failure_code IS NULL)
        OR (failure_stage = 'pull' AND failure_code = 'erp_pull_failed')
        OR (failure_stage = 'submit' AND failure_code = 'snapshot_submit_failed')
        OR (failure_stage = 'lease' AND failure_code = 'lease_expired')
    ),
    CHECK (
        (
            status = 'active'
            AND run_id IS NULL
            AND completed_at IS NULL
            AND failed_at IS NULL
            AND failure_stage IS NULL
            AND failure_code IS NULL
        )
        OR (
            status = 'completed'
            AND run_id IS NOT NULL
            AND completed_at IS NOT NULL
            AND failed_at IS NULL
            AND failure_stage IS NULL
            AND failure_code IS NULL
        )
        OR (
            status = 'failed'
            AND run_id IS NULL
            AND completed_at IS NULL
            AND failed_at IS NOT NULL
            AND failure_stage IN ('pull', 'submit')
        )
        OR (
            status = 'expired'
            AND run_id IS NULL
            AND completed_at IS NULL
            AND failed_at IS NOT NULL
            AND failure_stage = 'lease'
            AND failure_code = 'lease_expired'
        )
    )
);

CREATE UNIQUE INDEX reconciliation_schedule_claims_active_window_idx
    ON reconciliation_schedule_claims (owner_id, window_key)
    WHERE status = 'active';

CREATE INDEX reconciliation_schedule_claims_owner_status_idx
    ON reconciliation_schedule_claims (owner_id, status, lease_expires_at);

CREATE TABLE reconciliation_items (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    run_id              UUID NOT NULL REFERENCES reconciliation_runs(id) ON DELETE RESTRICT,
    product_code        TEXT NOT NULL,
    batch_no            TEXT NOT NULL,
    wms_qty             BIGINT NOT NULL CHECK (wms_qty >= 0),
    erp_qty             BIGINT NOT NULL CHECK (erp_qty >= 0),
    difference_qty      BIGINT NOT NULL,
    difference_type     TEXT NOT NULL CHECK (difference_type IN ('matched', 'wms_more', 'erp_more')),
    resolution_status   TEXT NOT NULL CHECK (resolution_status IN (
        'matched', 'open', 'adjustment_pending', 'erp_feedback_pending',
        'exception', 'resolved', 'known_difference'
    )),
    disposition         TEXT CHECK (disposition IN ('wms_truth', 'erp_truth', 'known_difference')),
    resolved_by         UUID,
    resolved_at         TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (run_id, product_code, batch_no)
);

CREATE INDEX reconciliation_items_query_idx
    ON reconciliation_items (owner_id, created_at DESC, id DESC);

CREATE TABLE reconciliation_item_adjustments (
    item_id            UUID NOT NULL REFERENCES reconciliation_items(id) ON DELETE RESTRICT,
    owner_id           UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    inventory_batch_id UUID NOT NULL REFERENCES inventory_batches(id) ON DELETE RESTRICT,
    quantity           BIGINT NOT NULL CHECK (quantity > 0),
    adjustment_order_id UUID NOT NULL REFERENCES stock_adjustment_orders(id) ON DELETE RESTRICT,
    created_at         TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (item_id, adjustment_order_id),
    UNIQUE (item_id, inventory_batch_id)
);

CREATE INDEX reconciliation_item_adjustments_order_idx
    ON reconciliation_item_adjustments (owner_id, adjustment_order_id);

CREATE TABLE reconciliation_item_locks (
    item_id          UUID NOT NULL REFERENCES reconciliation_items(id) ON DELETE RESTRICT,
    inventory_batch_id UUID NOT NULL REFERENCES inventory_batches(id) ON DELETE RESTRICT,
    owner_id         UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    previous_status  TEXT NOT NULL,
    locked_at        TIMESTAMPTZ NOT NULL,
    released_at      TIMESTAMPTZ,
    PRIMARY KEY (item_id, inventory_batch_id)
);

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    ('00000000-0000-0000-0000-00000000c001', 'rc.reconciliation.read', '库存对账查询'),
    ('00000000-0000-0000-0000-00000000c002', 'rc.reconciliation.execute', '库存对账执行'),
    ('00000000-0000-0000-0000-00000000c003', 'rc.reconciliation.resolve', '库存对账差异处理'),
    -- 仅供 H-SCH/H8 服务账号提交 ERP 快照；不授予 warehouse_manager/admin。
    ('00000000-0000-0000-0000-00000000c004', 'rc.reconciliation.ingest', '库存对账服务写入')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN (
        'rc.reconciliation.read',
        'rc.reconciliation.execute',
        'rc.reconciliation.resolve'
    )
 WHERE lower(role.role_code) IN ('warehouse_manager', 'admin', 'system_admin')
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_mrc_permissions_to_operational_roles()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF lower(NEW.role_code) IN ('warehouse_manager', 'admin', 'system_admin') THEN
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT NEW.id, permission.id
          FROM auth_permissions permission
         WHERE permission.permission_code IN (
             'rc.reconciliation.read',
             'rc.reconciliation.execute',
             'rc.reconciliation.resolve'
         )
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_mrc_permissions ON auth_roles;
CREATE TRIGGER auth_roles_grant_mrc_permissions
AFTER INSERT ON auth_roles
FOR EACH ROW
EXECUTE FUNCTION grant_mrc_permissions_to_operational_roles();

GRANT SELECT, INSERT, UPDATE ON reconciliation_rules, reconciliation_runs,
    reconciliation_schedule_claims, reconciliation_items, reconciliation_item_locks,
    reconciliation_item_adjustments TO wms_app;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES (
    '00000000-0000-0000-0000-000000130070',
    '00000000-0000-0000-0000-000000120007',
    3,
    'inventory.reconciliation',
    'inventory/management/reconciliation',
    'M-RC 库存对账',
    'mrc-reconciliation',
    'ClipboardList',
    'rc.reconciliation.read',
    28,
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
        ('isolate', '对账隔离', 30),
        ('release', '释放隔离', 40),
        ('resolve', '处理差异', 50),
        ('rule', '对账频率', 60),
        ('export', '导出', 70),
        ('field', '字段', 80),
        ('view', '视图', 90)
) AS action(key, label, sort_order)
WHERE node.view_id = 'mrc-reconciliation'
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
WHERE node.view_id = 'mrc-reconciliation'
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
WHERE node.view_id = 'mrc-reconciliation'
ON CONFLICT DO NOTHING;
