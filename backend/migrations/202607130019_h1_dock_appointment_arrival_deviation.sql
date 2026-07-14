ALTER TABLE dock_appointments
    ADD COLUMN IF NOT EXISTS arrival_deviation_minutes BIGINT;
