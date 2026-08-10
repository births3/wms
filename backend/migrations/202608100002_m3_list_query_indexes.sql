-- M3 列表查询索引：owner_id 过滤 + 排序键的组合索引（养护任务/记录分页列表的常用路径）
CREATE INDEX IF NOT EXISTS idx_inventory_maintenance_tasks_owner_planned_at
    ON inventory_maintenance_tasks (owner_id, planned_at ASC, id ASC);

CREATE INDEX IF NOT EXISTS idx_inventory_maintenance_records_owner_performed_at
    ON inventory_maintenance_records (owner_id, performed_at DESC, id DESC);

-- 库存批次列表分页：默认排序 updated_at DESC, id；近效期过滤路径已有 inventory_batches_owner_expiry_idx (owner_id, expiry_date)
CREATE INDEX IF NOT EXISTS idx_inventory_batches_owner_updated_at
    ON inventory_batches (owner_id, updated_at DESC);
