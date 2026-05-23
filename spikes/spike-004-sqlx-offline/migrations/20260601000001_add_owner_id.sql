-- 20260601000001_add_owner_id.sql
-- SPIKE-004 H3 验证：schema 漂移检测
-- 多 migration 演示：起初的 items 表无 owner_id，本 migration 加上以模拟多租户隔离

ALTER TABLE items ADD COLUMN owner_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
CREATE INDEX idx_items_owner_id ON items (owner_id);

-- 先去掉默认值（避免新插入用全 0；强制业务层显式赋值）
ALTER TABLE items ALTER COLUMN owner_id DROP DEFAULT;
