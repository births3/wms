-- US-DOCK-002 预约数据库结构基础（兼容最小 dock_appointments 表）

CREATE TABLE IF NOT EXISTS dock_appointments (
    id                 UUID PRIMARY KEY,
    dock_id            UUID NOT NULL,
    status             TEXT NOT NULL DEFAULT 'pending',
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status IN ('pending', 'confirmed', 'arrived', 'completed', 'timed_out', 'cancelled'))
) ; -- 与旧版本兼容的兜底定义，后续字段补齐时幂等执行

CREATE UNIQUE INDEX IF NOT EXISTS ux_dock_appointments_id ON dock_appointments (id);

ALTER TABLE dock_appointments
    ADD COLUMN IF NOT EXISTS owner_id            UUID,
    ADD COLUMN IF NOT EXISTS warehouse_id        UUID,
    ADD COLUMN IF NOT EXISTS appointment_no      TEXT,
    ADD COLUMN IF NOT EXISTS document_type       TEXT,
    ADD COLUMN IF NOT EXISTS document_no         TEXT,
    ADD COLUMN IF NOT EXISTS window_start_at      TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS window_end_at        TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS vehicle_plate_no     TEXT,
    ADD COLUMN IF NOT EXISTS vehicle_type         TEXT,
    ADD COLUMN IF NOT EXISTS driver_name          TEXT,
    ADD COLUMN IF NOT EXISTS driver_phone         TEXT,
    ADD COLUMN IF NOT EXISTS updated_at           TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS version              BIGINT;

ALTER TABLE dock_appointments DROP CONSTRAINT IF EXISTS dock_appointments_status_check;
ALTER TABLE dock_appointments
    ADD CONSTRAINT dock_appointments_status_check
    CHECK (status IN ('pending', 'confirmed', 'arrived', 'completed', 'timed_out', 'cancelled'));

UPDATE dock_appointments a
SET
    owner_id = COALESCE(a.owner_id, w.owner_id),
    warehouse_id = COALESCE(a.warehouse_id, d.warehouse_id),
    appointment_no = COALESCE(a.appointment_no, 'APP-' || replace(a.id::text, '-', '')),
    document_type = COALESCE(a.document_type, 'inbound'),
    document_no = COALESCE(a.document_no, 'DOC-' || replace(a.id::text, '-', '')),
    window_start_at = COALESCE(a.window_start_at, now()),
    window_end_at = COALESCE(a.window_end_at, now() + interval '1 hour'),
    vehicle_plate_no = COALESCE(a.vehicle_plate_no, ''),
    vehicle_type = COALESCE(a.vehicle_type, 'truck'),
    driver_name = COALESCE(a.driver_name, 'unknown'),
    driver_phone = COALESCE(a.driver_phone, '00000000000'),
    updated_at = COALESCE(a.updated_at, a.created_at, now()),
    version = COALESCE(a.version, 1)
FROM warehouse_docks d
JOIN warehouses w ON d.warehouse_id = w.id
WHERE a.dock_id = d.id;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
      WHERE conrelid = 'warehouses'::regclass AND conname = 'uq_warehouses_owner_id_id'
  ) THEN
    ALTER TABLE warehouses
      ADD CONSTRAINT uq_warehouses_owner_id_id UNIQUE (owner_id, id);
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
      WHERE conrelid = 'warehouse_docks'::regclass AND conname = 'uq_warehouse_docks_id_warehouse_id'
  ) THEN
    ALTER TABLE warehouse_docks
      ADD CONSTRAINT uq_warehouse_docks_id_warehouse_id UNIQUE (id, warehouse_id);
  END IF;
END $$;

ALTER TABLE dock_appointments
    DROP CONSTRAINT IF EXISTS dock_appointments_window_order_check,
    DROP CONSTRAINT IF EXISTS dock_appointments_warehouse_fk,
    DROP CONSTRAINT IF EXISTS dock_appointments_owner_dock_fk,
    DROP CONSTRAINT IF EXISTS dock_appointments_warehouse_owner_fk;

ALTER TABLE dock_appointments
    ADD CONSTRAINT dock_appointments_window_order_check
    CHECK (window_end_at > window_start_at);

ALTER TABLE dock_appointments
    ADD CONSTRAINT dock_appointments_owner_fk
    FOREIGN KEY (owner_id, warehouse_id) REFERENCES warehouses (owner_id, id) ON DELETE RESTRICT;

ALTER TABLE dock_appointments
    ADD CONSTRAINT dock_appointments_dock_fk
    FOREIGN KEY (dock_id, warehouse_id) REFERENCES warehouse_docks (id, warehouse_id) ON DELETE RESTRICT;

CREATE OR REPLACE FUNCTION dock_appointment_core_defaults()
RETURNS TRIGGER AS
$$
DECLARE
    v_warehouse_id uuid;
    v_owner_id uuid;
BEGIN
    SELECT d.warehouse_id, w.owner_id INTO v_warehouse_id, v_owner_id
      FROM warehouse_docks d
      JOIN warehouses w ON w.id = d.warehouse_id
     WHERE d.id = NEW.dock_id;
    NEW.warehouse_id := COALESCE(NEW.warehouse_id, v_warehouse_id);
    NEW.owner_id := COALESCE(NEW.owner_id, v_owner_id);
    IF NEW.warehouse_id IS NULL OR NEW.owner_id IS NULL THEN
        RAISE EXCEPTION 'dock_id % not found', NEW.dock_id USING ERRCODE = '23503';
    END IF;

    NEW.appointment_no = COALESCE(NEW.appointment_no, 'APP-' || replace(NEW.id::text, '-', ''));
    NEW.document_type = COALESCE(NEW.document_type, 'inbound');
    NEW.document_no = COALESCE(NEW.document_no, 'DOC-' || replace(NEW.id::text, '-', ''));
    NEW.window_start_at = COALESCE(NEW.window_start_at, now());
    NEW.window_end_at = COALESCE(NEW.window_end_at, now() + interval '1 hour');
    NEW.vehicle_plate_no = COALESCE(NEW.vehicle_plate_no, '');
    NEW.vehicle_type = COALESCE(NEW.vehicle_type, 'truck');
    NEW.driver_name = COALESCE(NEW.driver_name, 'unknown');
    NEW.driver_phone = COALESCE(NEW.driver_phone, '00000000000');
    NEW.updated_at = COALESCE(NEW.updated_at, now());
    NEW.version = COALESCE(NEW.version, 1);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_dock_appointments_defaults ON dock_appointments;
CREATE TRIGGER trg_dock_appointments_defaults
BEFORE INSERT ON dock_appointments
FOR EACH ROW
EXECUTE FUNCTION dock_appointment_core_defaults();

DO $$
BEGIN
    ALTER TABLE dock_appointments
        ALTER COLUMN owner_id SET NOT NULL,
        ALTER COLUMN warehouse_id SET NOT NULL,
        ALTER COLUMN appointment_no SET NOT NULL,
        ALTER COLUMN document_type SET NOT NULL,
        ALTER COLUMN document_no SET NOT NULL,
        ALTER COLUMN window_start_at SET NOT NULL,
        ALTER COLUMN window_end_at SET NOT NULL,
        ALTER COLUMN vehicle_plate_no SET NOT NULL,
        ALTER COLUMN vehicle_type SET NOT NULL,
        ALTER COLUMN driver_name SET NOT NULL,
        ALTER COLUMN driver_phone SET NOT NULL,
        ALTER COLUMN updated_at SET NOT NULL,
        ALTER COLUMN version SET NOT NULL;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS ux_dock_appointments_appointment_no
    ON dock_appointments (appointment_no);

CREATE UNIQUE INDEX IF NOT EXISTS ux_dock_appointments_active
    ON dock_appointments (owner_id, document_type, document_no)
WHERE status IN ('pending', 'confirmed', 'arrived');

GRANT SELECT, INSERT, UPDATE ON dock_appointments TO wms_app;
