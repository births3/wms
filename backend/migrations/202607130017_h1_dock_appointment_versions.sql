-- US-DOCK-004 预约变更版本关联。

ALTER TABLE dock_appointments
    ADD COLUMN IF NOT EXISTS supersedes_id UUID;

ALTER TABLE dock_appointments
    DROP CONSTRAINT IF EXISTS dock_appointments_supersedes_fk;

ALTER TABLE dock_appointments
    ADD CONSTRAINT dock_appointments_supersedes_fk
    FOREIGN KEY (supersedes_id) REFERENCES dock_appointments (id) ON DELETE RESTRICT;

CREATE INDEX IF NOT EXISTS idx_dock_appointments_supersedes
    ON dock_appointments (supersedes_id);

GRANT SELECT, INSERT, UPDATE ON dock_appointments TO wms_app;
