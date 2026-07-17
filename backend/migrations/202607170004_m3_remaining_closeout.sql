-- US-M3-002/003/006/009/010 剩余闭环：移库、状态 ERP outbox、预警事件、ABC 分类。

-- 移库单
CREATE TABLE IF NOT EXISTS inventory_relocations (
    id                   UUID PRIMARY KEY,
    owner_id             UUID NOT NULL,
    batch_id             UUID NOT NULL,
    product_code         TEXT NOT NULL,
    batch_no             TEXT NOT NULL,
    qty                  BIGINT NOT NULL CHECK (qty > 0),
    from_location_id     UUID NOT NULL,
    from_location_code   TEXT NOT NULL,
    to_location_id       UUID NOT NULL,
    to_location_code     TEXT NOT NULL,
    relocation_mode      TEXT NOT NULL DEFAULT 'direct'
        CHECK (relocation_mode IN ('direct', 'lpn_full', 'partial', 'piece')),
    lpn_code             TEXT,
    quality_status       TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'completed'
        CHECK (status IN ('completed', 'pending_supervisor', 'failed')),
    reason               TEXT,
    created_by           UUID NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS inventory_relocations_owner_created_idx
    ON inventory_relocations (owner_id, created_at DESC);

-- 库存状态变更 ERP 异步反馈
CREATE TABLE IF NOT EXISTS inventory_status_erp_feedback_outbox (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL,
    batch_id         UUID NOT NULL,
    status_change_id UUID,
    event_type       TEXT NOT NULL DEFAULT 'inventory_status_changed',
    payload          JSONB NOT NULL,
    status           TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'sending', 'succeeded', 'failed')),
    attempt_count    INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at  TIMESTAMPTZ NOT NULL,
    last_error       TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS inventory_status_erp_feedback_pending_idx
    ON inventory_status_erp_feedback_outbox (status, next_attempt_at)
    WHERE status IN ('pending', 'failed');

-- 库存统一预警事件
CREATE TABLE IF NOT EXISTS inventory_alert_events (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL,
    alert_type       TEXT NOT NULL
        CHECK (alert_type IN (
            'near_expiry', 'expired', 'safety_stock', 'overstock',
            'maintenance_overdue', 'temperature'
        )),
    product_code     TEXT,
    batch_id         UUID,
    batch_no         TEXT,
    location_code    TEXT,
    severity         TEXT NOT NULL DEFAULT 'medium'
        CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    title            TEXT NOT NULL,
    message          TEXT NOT NULL,
    lifecycle_status TEXT NOT NULL DEFAULT 'open'
        CHECK (lifecycle_status IN ('open', 'handled', 'ignored')),
    handled_by       UUID,
    handled_at       TIMESTAMPTZ,
    payload          JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS inventory_alert_events_owner_status_idx
    ON inventory_alert_events (owner_id, lifecycle_status, created_at DESC);

CREATE INDEX IF NOT EXISTS inventory_alert_events_owner_type_idx
    ON inventory_alert_events (owner_id, alert_type, created_at DESC);

-- ABC 分类
CREATE TABLE IF NOT EXISTS inventory_abc_classifications (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL,
    product_code     TEXT NOT NULL,
    abc_class        TEXT NOT NULL CHECK (abc_class IN ('A', 'B', 'C')),
    score            NUMERIC(18, 4) NOT NULL DEFAULT 0,
    outbound_qty     BIGINT NOT NULL DEFAULT 0,
    period_start     DATE NOT NULL,
    period_end       DATE NOT NULL,
    source           TEXT NOT NULL DEFAULT 'system'
        CHECK (source IN ('system', 'manual')),
    override_reason  TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, product_code, period_end)
);

CREATE INDEX IF NOT EXISTS inventory_abc_owner_class_idx
    ON inventory_abc_classifications (owner_id, abc_class, product_code);

-- 权限
INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:m3.relocation.write')::uuid, 'm3.relocation.write', 'M3 库内移库'),
    (md5('auth_permission:m3.alert.read')::uuid, 'm3.alert.read', 'M3 库存预警查询'),
    (md5('auth_permission:m3.alert.write')::uuid, 'm3.alert.write', 'M3 库存预警处理'),
    (md5('auth_permission:m3.abc.read')::uuid, 'm3.abc.read', 'M3 ABC 分类查询'),
    (md5('auth_permission:m3.abc.write')::uuid, 'm3.abc.write', 'M3 ABC 分类维护')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN (
        'm3.relocation.write', 'm3.alert.read', 'm3.alert.write', 'm3.abc.read', 'm3.abc.write'
    )
 WHERE role.role_code IN ('warehouse_manager', 'admin', 'system_admin')
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE ON inventory_relocations TO wms_app;
GRANT SELECT, INSERT, UPDATE ON inventory_status_erp_feedback_outbox TO wms_app;
GRANT SELECT, INSERT, UPDATE ON inventory_alert_events TO wms_app;
GRANT SELECT, INSERT, UPDATE ON inventory_abc_classifications TO wms_app;
