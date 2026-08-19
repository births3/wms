-- US-M3-004 最小养护任务/记录闭环。

CREATE TABLE IF NOT EXISTS inventory_maintenance_tasks (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL,
    inventory_batch_id  UUID NOT NULL REFERENCES inventory_batches(id),
    planned_at          TIMESTAMPTZ NOT NULL,
    status              TEXT NOT NULL DEFAULT 'pending',
    assigned_user_id    UUID,
    completed_at        TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status IN ('pending', 'completed')),
    CHECK (
        (status = 'pending' AND completed_at IS NULL)
        OR (status = 'completed' AND completed_at IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS inventory_maintenance_tasks_owner_status_idx
    ON inventory_maintenance_tasks (owner_id, status, planned_at, id);

CREATE INDEX IF NOT EXISTS inventory_maintenance_tasks_owner_batch_idx
    ON inventory_maintenance_tasks (owner_id, inventory_batch_id, planned_at DESC);

CREATE TABLE IF NOT EXISTS inventory_maintenance_records (
    id                  UUID PRIMARY KEY,
    task_id             UUID NOT NULL REFERENCES inventory_maintenance_tasks(id),
    owner_id            UUID NOT NULL,
    inventory_batch_id  UUID NOT NULL REFERENCES inventory_batches(id),
    product_code        TEXT NOT NULL,
    batch_no            TEXT NOT NULL,
    expiry_date         DATE NOT NULL,
    inventory_status    TEXT NOT NULL,
    temperature_celsius DOUBLE PRECISION NOT NULL,
    humidity_percent    DOUBLE PRECISION NOT NULL,
    appearance          TEXT NOT NULL,
    packaging           TEXT NOT NULL,
    pest                TEXT NOT NULL,
    rodent              TEXT NOT NULL,
    mildew              TEXT NOT NULL,
    conclusion          TEXT NOT NULL,
    exception_type      TEXT,
    notes               TEXT,
    performed_by        UUID NOT NULL,
    performed_at        TIMESTAMPTZ NOT NULL,
    performed_date      DATE NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (temperature_celsius BETWEEN -100 AND 100),
    CHECK (humidity_percent BETWEEN 0 AND 100),
    CHECK (appearance IN ('intact', 'damaged', 'discolored', 'damp')),
    CHECK (packaging IN ('intact', 'damaged', 'leaking', 'label_unclear')),
    CHECK (pest IN ('none', 'present')),
    CHECK (rodent IN ('none', 'present')),
    CHECK (mildew IN ('none', 'present')),
    CHECK (conclusion IN ('normal', 'abnormal')),
    CHECK (exception_type IS NULL OR exception_type IN (
        'quality_change', 'package_damage', 'temperature_excursion',
        'pest_rodent_mildew', 'other'
    )),
    CHECK (
        (conclusion = 'normal' AND exception_type IS NULL)
        OR (conclusion = 'abnormal' AND exception_type IS NOT NULL)
    ),
    UNIQUE (owner_id, task_id, performed_date)
);

CREATE INDEX IF NOT EXISTS inventory_maintenance_records_owner_batch_idx
    ON inventory_maintenance_records (owner_id, inventory_batch_id, performed_at DESC);

CREATE INDEX IF NOT EXISTS inventory_maintenance_records_owner_task_idx
    ON inventory_maintenance_records (owner_id, task_id, performed_at DESC);

CREATE OR REPLACE FUNCTION inventory_maintenance_record_immutable() RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'inventory_maintenance_records is append-only: % attempted by %', TG_OP, current_user;
END;
$$;

DROP TRIGGER IF EXISTS trg_inventory_maintenance_records_no_update ON inventory_maintenance_records;
CREATE TRIGGER trg_inventory_maintenance_records_no_update
    BEFORE UPDATE OR DELETE OR TRUNCATE ON inventory_maintenance_records
    FOR EACH STATEMENT EXECUTE FUNCTION inventory_maintenance_record_immutable();

GRANT SELECT, INSERT, UPDATE ON inventory_maintenance_tasks TO wms_app;
GRANT SELECT, INSERT ON inventory_maintenance_records TO wms_app;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:m3.maintenance.write')::uuid,
    'm3.maintenance.write',
    'M3 在库养护结果写入'
)
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'm3.maintenance.write'
 WHERE role.role_code IN ('system_admin', 'maintenance_clerk')
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_m3_maintenance_write_to_maintenance_clerk()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF lower(NEW.role_code) = 'maintenance_clerk' THEN
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT NEW.id, permission.id
          FROM auth_permissions permission
         WHERE permission.permission_code = 'm3.maintenance.write'
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_m3_maintenance_write ON auth_roles;
CREATE TRIGGER auth_roles_grant_m3_maintenance_write
AFTER INSERT ON auth_roles
FOR EACH ROW
EXECUTE FUNCTION grant_m3_maintenance_write_to_maintenance_clerk();
