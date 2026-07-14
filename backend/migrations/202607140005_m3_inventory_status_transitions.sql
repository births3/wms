-- US-M3-003：库存质量状态转换规则可维护，货主配置覆盖全局默认。

CREATE TABLE IF NOT EXISTS inventory_status_transitions (
    id                UUID PRIMARY KEY,
    owner_id          UUID,
    from_status       TEXT NOT NULL,
    to_status         TEXT NOT NULL,
    approval_sources  TEXT[] NOT NULL,
    enabled           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(from_status) <> ''),
    CHECK (btrim(to_status) <> ''),
    CHECK (from_status <> to_status),
    CHECK (cardinality(approval_sources) > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS inventory_status_transitions_scope_uidx
    ON inventory_status_transitions (
        COALESCE(owner_id, '00000000-0000-0000-0000-000000000000'::uuid),
        from_status,
        to_status
    );

CREATE INDEX IF NOT EXISTS inventory_status_transitions_owner_lookup_idx
    ON inventory_status_transitions (owner_id, from_status, to_status, enabled);

GRANT SELECT, INSERT, UPDATE ON inventory_status_transitions TO wms_app;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:m3.inventory_status.global.write')::uuid, 'm3.inventory_status.global.write', 'M3 库存状态全局规则维护')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'm3.inventory_status.global.write'
 WHERE role.role_code IN ('system_admin', 'warehouse_manager')
ON CONFLICT DO NOTHING;

INSERT INTO inventory_status_transitions (
    id, owner_id, from_status, to_status, approval_sources, enabled
)
VALUES
    (
        '10000000-0000-0000-0000-000000000061', NULL, 'qualified', 'quarantined',
        ARRAY['质量联系单', '对账差异', '养护异常', '温度超标事件', 'M-QL', 'M-RC', 'M3-MAINT', 'M5-TEMP_EXCURSION', 'M-TC'], TRUE
    ),
    (
        '10000000-0000-0000-0000-000000000062', NULL, 'qualified', 'unqualified',
        ARRAY['M3-002-EXPIRY'], TRUE
    ),
    (
        '10000000-0000-0000-0000-000000000063', NULL, 'quarantined', 'qualified',
        ARRAY['验收结论', 'M2-INSPECTION'], TRUE
    ),
    (
        '10000000-0000-0000-0000-000000000064', NULL, 'quarantined', 'unqualified',
        ARRAY['验收结论', 'M2-INSPECTION'], TRUE
    ),
    (
        '10000000-0000-0000-0000-000000000065', NULL, 'unqualified', 'pending_destruction',
        ARRAY['质量联系单', 'M-QL'], TRUE
    ),
    (
        '10000000-0000-0000-0000-000000000066', NULL, 'qualified', 'loss_deducted',
        ARRAY['报损报溢单', '质量联系单', 'M-SA', 'M-QL'], TRUE
    ),
    (
        '10000000-0000-0000-0000-000000000067', NULL, 'quarantined', 'loss_deducted',
        ARRAY['报损报溢单', '质量联系单', 'M-SA', 'M-QL'], TRUE
    ),
    (
        '10000000-0000-0000-0000-000000000068', NULL, 'unqualified', 'loss_deducted',
        ARRAY['报损报溢单', '质量联系单', 'M-SA', 'M-QL'], TRUE
    ),
    (
        '10000000-0000-0000-0000-000000000069', NULL, 'pending_destruction', 'loss_deducted',
        ARRAY['报损报溢单', '质量联系单', 'M-SA', 'M-QL'], TRUE
    )
ON CONFLICT DO NOTHING;
