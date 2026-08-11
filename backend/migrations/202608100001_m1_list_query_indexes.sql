-- M1 列表查询索引：owner_id 过滤 + updated_at 排序的组合索引（列表接口全量拉取的常用路径）
CREATE INDEX IF NOT EXISTS idx_products_owner_updated_at ON products (owner_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_suppliers_owner_updated_at ON suppliers (owner_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_customers_owner_updated_at ON customers (owner_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_warehouses_owner_updated_at ON warehouses (owner_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_warehouse_zones_owner_updated_at ON warehouse_zones (owner_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_warehouse_locations_owner_updated_at ON warehouse_locations (owner_id, updated_at DESC);
