-- US-M3-011：库位历史追踪事实字段（兼容回填，不破坏既有流水写入）。

ALTER TABLE inventory_movements
    ADD COLUMN IF NOT EXISTS location_code TEXT,
    ADD COLUMN IF NOT EXISTS from_location_code TEXT,
    ADD COLUMN IF NOT EXISTS to_location_code TEXT,
    ADD COLUMN IF NOT EXISTS lpn_code TEXT,
    ADD COLUMN IF NOT EXISTS operator_user_id UUID,
    ADD COLUMN IF NOT EXISTS operator_name TEXT,
    ADD COLUMN IF NOT EXISTS volume_delta_cm3 BIGINT;

-- 历史流水按当前批次库位回填，便于库位查询入口可用。
UPDATE inventory_movements movement
   SET location_code = batch.location_code
  FROM inventory_batches batch
 WHERE movement.batch_id = batch.id
   AND movement.owner_id = batch.owner_id
   AND movement.location_code IS NULL
   AND batch.location_code IS NOT NULL
   AND batch.location_code <> '';

CREATE INDEX IF NOT EXISTS inventory_movements_owner_location_occurred_idx
    ON inventory_movements (owner_id, location_code, occurred_at DESC)
    WHERE location_code IS NOT NULL;

CREATE INDEX IF NOT EXISTS inventory_movements_owner_from_location_occurred_idx
    ON inventory_movements (owner_id, from_location_code, occurred_at DESC)
    WHERE from_location_code IS NOT NULL;

CREATE INDEX IF NOT EXISTS inventory_movements_owner_to_location_occurred_idx
    ON inventory_movements (owner_id, to_location_code, occurred_at DESC)
    WHERE to_location_code IS NOT NULL;
