ALTER TABLE inventory_batches
    ADD COLUMN IF NOT EXISTS container_lpn TEXT;

CREATE INDEX IF NOT EXISTS inventory_batches_owner_container_lpn_idx
    ON inventory_batches (owner_id, container_lpn)
    WHERE container_lpn IS NOT NULL;
