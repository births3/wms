-- PR deep validation repair: repository update/disable paths already maintain updated_at.
-- Keep the schema contract aligned with replenishment_strategies and the runtime SQL.
ALTER TABLE replenishment_location_groups
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
