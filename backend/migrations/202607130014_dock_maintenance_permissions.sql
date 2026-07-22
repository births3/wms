-- US-DOCK-001：只有系统管理员和仓库主管可维护月台。

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:dock.manage')::uuid,
    'dock.manage',
    '月台档案维护'
)
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'dock.manage'
 WHERE role.role_code IN ('system_admin', 'warehouse_manager')
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_dock_manage_to_warehouse_manager()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF lower(NEW.role_code) = 'warehouse_manager' THEN
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT NEW.id, permission.id
          FROM auth_permissions permission
         WHERE permission.permission_code = 'dock.manage'
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_dock_manage
    ON auth_roles;
CREATE TRIGGER auth_roles_grant_dock_manage
AFTER INSERT ON auth_roles
FOR EACH ROW
EXECUTE FUNCTION grant_dock_manage_to_warehouse_manager();

-- US-DOCK-002/004：预约当前完整基线。
CREATE TABLE IF NOT EXISTS dock_appointments (
    id                        UUID PRIMARY KEY,
    dock_id                   UUID NOT NULL,
    owner_id                  UUID NOT NULL,
    warehouse_id              UUID NOT NULL,
    appointment_no            TEXT NOT NULL,
    document_type             TEXT NOT NULL,
    document_no               TEXT NOT NULL,
    window_start_at           TIMESTAMPTZ NOT NULL,
    window_end_at             TIMESTAMPTZ NOT NULL,
    vehicle_plate_no          TEXT NOT NULL DEFAULT '',
    vehicle_type              TEXT NOT NULL,
    driver_name               TEXT NOT NULL,
    driver_phone              TEXT NOT NULL DEFAULT '',
    status                    TEXT NOT NULL DEFAULT 'pending',
    supersedes_id             UUID REFERENCES dock_appointments(id) ON DELETE RESTRICT,
    arrived_at                TIMESTAMPTZ,
    arrival_deviation_minutes BIGINT,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                   BIGINT NOT NULL DEFAULT 1,
    FOREIGN KEY (owner_id, warehouse_id) REFERENCES warehouses (owner_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (dock_id, warehouse_id) REFERENCES warehouse_docks (id, warehouse_id) ON DELETE RESTRICT,
    CHECK (status IN ('pending', 'confirmed', 'arrived', 'completed', 'timed_out', 'cancelled')),
    CHECK (window_end_at > window_start_at)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_dock_appointments_appointment_no
    ON dock_appointments (appointment_no);

CREATE UNIQUE INDEX IF NOT EXISTS ux_dock_appointments_active
    ON dock_appointments (owner_id, document_type, document_no)
    WHERE status IN ('pending', 'confirmed', 'arrived');

CREATE INDEX IF NOT EXISTS idx_dock_appointments_supersedes
    ON dock_appointments (supersedes_id);

GRANT SELECT, INSERT, UPDATE ON dock_appointments TO wms_app;
GRANT DELETE ON warehouse_docks TO wms_app;
