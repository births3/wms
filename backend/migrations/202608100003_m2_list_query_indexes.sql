-- M2 列表查询索引：P0 最大表 receiving_orders 分页列表（list_receiving_orders）的
-- owner_id 过滤 + 排序键（updated_at DESC, receipt_no）组合索引。
-- 既有 receiving_orders_owner_status_idx(owner_id, status, updated_at DESC)
-- 与 UNIQUE(owner_id, receipt_no) 均与排序键不对应；含 tiebreaker 列使 ORDER BY 完全走索引。
CREATE INDEX IF NOT EXISTS idx_receiving_orders_owner_updated_at
    ON receiving_orders (owner_id, updated_at DESC, receipt_no);
