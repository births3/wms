DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conname = 'warehouse_locations_bound_owner_fk'
    ) THEN
        ALTER TABLE warehouse_locations
            ADD CONSTRAINT warehouse_locations_bound_owner_fk
            FOREIGN KEY (bound_owner_id) REFERENCES auth_owners(id) ON DELETE RESTRICT;
    END IF;
END
$$;
