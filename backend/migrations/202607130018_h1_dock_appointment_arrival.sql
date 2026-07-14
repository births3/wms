ALTER TABLE dock_appointments
    ADD COLUMN IF NOT EXISTS arrived_at TIMESTAMPTZ;
