-- 202608180001_phase1_storage_location_and_inventory_baseline.sql
-- Phase 1 Ticket 01: 基础档案数据模型基线同步与多货主/温区/库位形态字段落地

-- 1. 库区 warehouse_zones: 5 温区收敛、品类准入、外用/易串味/特药专区
UPDATE warehouse_zones SET temperature_zone = 'normal_10_30' WHERE temperature_zone = 'normal';
UPDATE warehouse_zones SET temperature_zone = 'cool_le_20' WHERE temperature_zone = 'cool';
UPDATE warehouse_zones SET temperature_zone = 'cold_2_8' WHERE temperature_zone = 'cold';
UPDATE warehouse_zones SET temperature_zone = 'freeze_le_minus_20' WHERE temperature_zone = 'frozen';

ALTER TABLE warehouse_zones DROP CONSTRAINT IF EXISTS warehouse_zones_temperature_zone_check;
ALTER TABLE warehouse_zones ADD CONSTRAINT warehouse_zones_temperature_zone_check
    CHECK (temperature_zone IN ('normal_10_30', 'cool_le_20', 'cold_2_8', 'freeze_le_minus_20', 'ultra_cold_minus_80'));

ALTER TABLE warehouse_zones
    ADD COLUMN IF NOT EXISTS allowed_categories JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS is_external_use_zone BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS is_fragrant_zone BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS is_special_drug_zone BOOLEAN NOT NULL DEFAULT FALSE;

-- 2. 商品 products: 5 温区收敛、外用药/易串味商品标记
UPDATE products SET storage_condition = 'normal_10_30' WHERE storage_condition = 'normal';
UPDATE products SET storage_condition = 'cool_le_20' WHERE storage_condition = 'cool';
UPDATE products SET storage_condition = 'cold_2_8' WHERE storage_condition = 'cold';
UPDATE products SET storage_condition = 'freeze_le_minus_20' WHERE storage_condition = 'frozen';

ALTER TABLE products DROP CONSTRAINT IF EXISTS products_storage_condition_check;
ALTER TABLE products ADD CONSTRAINT products_storage_condition_check
    CHECK (storage_condition IS NULL OR storage_condition IN ('normal_10_30', 'cool_le_20', 'cold_2_8', 'freeze_le_minus_20', 'ultra_cold_minus_80'));

ALTER TABLE products
    ADD COLUMN IF NOT EXISTS is_external_use BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS is_fragrant BOOLEAN NOT NULL DEFAULT FALSE;

-- 3. 系统字典 items: 更新 5 温区项与 staging 库位类型
UPDATE system_dictionary_items SET item_code = 'normal_10_30', item_name = '常温 (10-30℃)', params = '{"min_celsius": 10, "max_celsius": 30}'::jsonb WHERE dict_code = 'temperature_zone' AND item_code = 'normal';
UPDATE system_dictionary_items SET item_code = 'cool_le_20', item_name = '阴凉 (≤20℃)', params = '{"min_celsius": 0, "max_celsius": 20}'::jsonb WHERE dict_code = 'temperature_zone' AND item_code = 'cool';
UPDATE system_dictionary_items SET item_code = 'cold_2_8', item_name = '冷藏 (2-8℃)', params = '{"min_celsius": 2, "max_celsius": 8}'::jsonb WHERE dict_code = 'temperature_zone' AND item_code = 'cold';
UPDATE system_dictionary_items SET item_code = 'freeze_le_minus_20', item_name = '冷冻 (≤-20℃)', params = '{"max_celsius": -20}'::jsonb WHERE dict_code = 'temperature_zone' AND item_code = 'frozen';

INSERT INTO system_dictionary_items (id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at)
VALUES ('10000000-0000-0000-0000-000000000030'::uuid, 'temperature_zone', 'ultra_cold_minus_80', '超低温 (≤-80℃)', true, null, '{"max_celsius": -80}'::jsonb, 'global', now(), now())
ON CONFLICT (id) DO NOTHING;

INSERT INTO system_dictionary_items (id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at)
VALUES ('10000000-0000-0000-0000-000000000044'::uuid, 'location_type', 'staging', '集货位', true, null, '{}'::jsonb, 'global', now(), now())
ON CONFLICT (id) DO NOTHING;

-- 4. 补货策略与库位组表
CREATE TABLE IF NOT EXISTS replenishment_strategies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL,
    strategy_code VARCHAR(64) NOT NULL,
    strategy_name VARCHAR(128) NOT NULL,
    scope_type VARCHAR(16) NOT NULL,
    scope_ref UUID NOT NULL,
    location_type VARCHAR(16) NOT NULL,
    source_type VARCHAR(16) NOT NULL DEFAULT 'storage',
    target_type VARCHAR(16) NOT NULL,
    min_safety_threshold NUMERIC(19, 4) NOT NULL,
    max_replenish_target NUMERIC(19, 4) NOT NULL,
    trigger_modes TEXT[] NOT NULL DEFAULT '{min_max, wave_gap}',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, strategy_code),
    CHECK (scope_type IN ('location_group', 'category', 'product')),
    CHECK (source_type IN ('storage', 'case_pick')),
    CHECK (target_type IN ('case_pick', 'piece_pick'))
);

CREATE INDEX IF NOT EXISTS replenishment_strategies_owner_idx
    ON replenishment_strategies (owner_id, enabled);

CREATE TABLE IF NOT EXISTS replenishment_location_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL,
    group_code VARCHAR(64) NOT NULL,
    group_name VARCHAR(128) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, group_code)
);

CREATE TABLE IF NOT EXISTS replenishment_location_group_members (
    group_id UUID NOT NULL REFERENCES replenishment_location_groups(id) ON DELETE CASCADE,
    location_id UUID NOT NULL,
    PRIMARY KEY (group_id, location_id)
);

-- 5. 库位 warehouse_locations: 字段重命名、3 值域状态、四重锁定、作业形态与动线字段
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'warehouse_locations' AND column_name = 'bound_owner_id'
    ) THEN
        ALTER TABLE warehouse_locations RENAME COLUMN bound_owner_id TO current_owner_id;
    END IF;
END $$;

ALTER TABLE warehouse_locations
    ADD COLUMN IF NOT EXISTS allows_container BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS mix_product_policy VARCHAR(32) NOT NULL DEFAULT 'single_product_only',
    ADD COLUMN IF NOT EXISTS mix_batch_policy VARCHAR(32) NOT NULL DEFAULT 'single_batch',
    ADD COLUMN IF NOT EXISTS lock_status VARCHAR(16) NOT NULL DEFAULT 'normal',
    ADD COLUMN IF NOT EXISTS pick_zone_level VARCHAR(16) DEFAULT 'normal',
    ADD COLUMN IF NOT EXISTS pick_sequence_no INT,
    ADD COLUMN IF NOT EXISTS putaway_sequence_no INT,
    ADD COLUMN IF NOT EXISTS is_agv_managed BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS agv_pod_code VARCHAR(64),
    ADD COLUMN IF NOT EXISTS agv_unreachable_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS replenish_strategy_id UUID REFERENCES replenishment_strategies(id) ON DELETE SET NULL;

ALTER TABLE warehouse_locations DROP CONSTRAINT IF EXISTS warehouse_locations_mix_product_policy_check;
ALTER TABLE warehouse_locations ADD CONSTRAINT warehouse_locations_mix_product_policy_check
    CHECK (mix_product_policy IN ('single_product_only', 'restricted_mix'));

ALTER TABLE warehouse_locations DROP CONSTRAINT IF EXISTS warehouse_locations_mix_batch_policy_check;
ALTER TABLE warehouse_locations ADD CONSTRAINT warehouse_locations_mix_batch_policy_check
    CHECK (mix_batch_policy IN ('single_batch', 'multi_batch'));

ALTER TABLE warehouse_locations DROP CONSTRAINT IF EXISTS warehouse_locations_lock_status_check;
ALTER TABLE warehouse_locations ADD CONSTRAINT warehouse_locations_lock_status_check
    CHECK (lock_status IN ('normal', 'lock_in', 'lock_out', 'lock_all'));

ALTER TABLE warehouse_locations DROP CONSTRAINT IF EXISTS warehouse_locations_pick_zone_level_check;
ALTER TABLE warehouse_locations ADD CONSTRAINT warehouse_locations_pick_zone_level_check
    CHECK (pick_zone_level IS NULL OR pick_zone_level IN ('gold', 'normal', 'deep'));

-- 清洗存量 locked 状态至 lock_status='lock_all'
UPDATE warehouse_locations SET lock_status = 'lock_all', status = 'available' WHERE status = 'locked';

ALTER TABLE warehouse_locations DROP CONSTRAINT IF EXISTS warehouse_locations_status_check;
ALTER TABLE warehouse_locations ADD CONSTRAINT warehouse_locations_status_check
    CHECK (status IN ('available', 'occupied', 'disabled'));

ALTER TABLE warehouse_locations DROP CONSTRAINT IF EXISTS warehouse_locations_location_type_check;
ALTER TABLE warehouse_locations ADD CONSTRAINT warehouse_locations_location_type_check
    CHECK (location_type IN ('storage', 'case_pick', 'piece_pick', 'staging'));

-- 6. 库位设备绑定表（设备中台四表属 Phase 2，Phase 1 只建绑定表；device_id 外键待 Phase 2 建表后补充）
CREATE TABLE IF NOT EXISTS location_device_bindings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    warehouse_id UUID NOT NULL,
    location_id UUID NOT NULL,
    device_id UUID NOT NULL,
    binding_role VARCHAR(16) NOT NULL,
    point_address VARCHAR(64),
    valid_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_to TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (binding_role IN ('ptl_light', 'rfid_antenna'))
);

CREATE UNIQUE INDEX IF NOT EXISTS location_device_bindings_active_uidx
    ON location_device_bindings (location_id, binding_role)
    WHERE valid_to IS NULL;
CREATE INDEX IF NOT EXISTS location_device_bindings_device_idx
    ON location_device_bindings (device_id, valid_to);

-- 7. 容器质量锁事件表、当前锁冗余与上架拒绝日志表
ALTER TABLE lpn_containers
    ADD COLUMN IF NOT EXISTS current_lock_category VARCHAR(32) DEFAULT 'qualified',
    ADD COLUMN IF NOT EXISTS current_lock_reason_item_code VARCHAR(64);

CREATE TABLE IF NOT EXISTS container_quality_lock_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL,
    container_id UUID NOT NULL REFERENCES lpn_containers(id),
    lpn_code VARCHAR(64) NOT NULL,
    event_type VARCHAR(16) NOT NULL,
    lock_category VARCHAR(32),
    reason_dict_item_code VARCHAR(64),
    reason_desc TEXT,
    evidence_urls JSONB DEFAULT '[]',
    quality_liaison_id UUID REFERENCES quality_liaison_orders(id),
    operated_by UUID NOT NULL,
    witness_id UUID,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    note TEXT
);

CREATE INDEX IF NOT EXISTS container_quality_lock_events_container_idx
    ON container_quality_lock_events (owner_id, container_id, occurred_at DESC);

CREATE TABLE IF NOT EXISTS putaway_validation_rejection_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL,
    operated_by UUID NOT NULL,
    container_code VARCHAR(64),
    product_id UUID,
    target_location_id UUID NOT NULL,
    rejection_dimension VARCHAR(32) NOT NULL,
    error_code VARCHAR(64) NOT NULL,
    reason TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS putaway_rejection_logs_owner_time_idx
    ON putaway_validation_rejection_logs (owner_id, occurred_at DESC);

-- 8. 库存表 inventory_batches: product_id 回填、status 改名、qty_frozen 改名、在途补货双字段
ALTER TABLE inventory_batches
    ADD COLUMN IF NOT EXISTS product_id UUID,
    ADD COLUMN IF NOT EXISTS warehouse_id UUID,
    ADD COLUMN IF NOT EXISTS zone_id UUID;

ALTER TABLE inventory_batches
    ALTER COLUMN product_code DROP NOT NULL,
    ALTER COLUMN location_code DROP NOT NULL;

-- 回填 product_id, warehouse_id, zone_id
UPDATE inventory_batches b
SET product_id = p.id
FROM products p
WHERE p.owner_id = b.owner_id AND p.product_code = b.product_code AND b.product_id IS NULL;

UPDATE inventory_batches b
SET warehouse_id = l.warehouse_id, zone_id = l.zone_id
FROM warehouse_locations l
WHERE l.id = b.location_id AND (b.warehouse_id IS NULL OR b.zone_id IS NULL);

-- 改名 quality_status -> status (若 status 未存在)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'inventory_batches' AND column_name = 'quality_status'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'inventory_batches' AND column_name = 'status'
    ) THEN
        ALTER TABLE inventory_batches RENAME COLUMN quality_status TO status;
    ELSIF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'inventory_batches' AND column_name = 'status'
    ) THEN
        ALTER TABLE inventory_batches ADD COLUMN status VARCHAR(32) NOT NULL DEFAULT 'qualified';
    END IF;
END $$;

-- 改名 qty_locked -> qty_frozen (若 qty_frozen 未存在)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'inventory_batches' AND column_name = 'qty_locked'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'inventory_batches' AND column_name = 'qty_frozen'
    ) THEN
        ALTER TABLE inventory_batches RENAME COLUMN qty_locked TO qty_frozen;
    ELSIF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'inventory_batches' AND column_name = 'qty_frozen'
    ) THEN
        ALTER TABLE inventory_batches ADD COLUMN qty_frozen NUMERIC(19, 4) NOT NULL DEFAULT 0;
    END IF;
END $$;

ALTER TABLE inventory_batches
    ADD COLUMN IF NOT EXISTS qty_allocated NUMERIC(19, 4) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS qty_replenish_in_transit NUMERIC(19, 4) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS qty_replenish_out_transit NUMERIC(19, 4) NOT NULL DEFAULT 0;

-- 回填已分配数量
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables WHERE table_name = 'inventory_allocations'
    ) THEN
        UPDATE inventory_batches b
        SET qty_allocated = COALESCE(a.allocated_sum, 0)
        FROM (
            SELECT batch_id, SUM(allocated_qty) AS allocated_sum
            FROM inventory_allocations
            WHERE status = 'locked'
            GROUP BY batch_id
        ) a
        WHERE b.id = a.batch_id;
    END IF;
END $$;

-- 重构唯一约束与索引
ALTER TABLE inventory_batches DROP CONSTRAINT IF EXISTS inventory_batches_owner_id_product_code_batch_no_location_id_qu_key;
ALTER TABLE inventory_batches DROP CONSTRAINT IF EXISTS inventory_batches_owner_product_batch_location_status_key;
ALTER TABLE inventory_batches DROP CONSTRAINT IF EXISTS inventory_batches_unique_inventory;

DROP INDEX IF EXISTS inventory_batches_owner_product_batch_idx;
DROP INDEX IF EXISTS inventory_batches_owner_location_status_idx;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'inventory_batches_owner_product_batch_location_status_uidx'
    ) THEN
        ALTER TABLE inventory_batches
            ADD CONSTRAINT inventory_batches_owner_product_batch_location_status_uidx
            UNIQUE (owner_id, product_id, batch_no, location_id, status);
    END IF;
EXCEPTION
    WHEN duplicate_object THEN NULL;
    WHEN duplicate_table THEN NULL;
END $$;

CREATE INDEX IF NOT EXISTS inventory_batches_owner_product_batch_idx
    ON inventory_batches (owner_id, product_id, batch_no);
CREATE INDEX IF NOT EXISTS inventory_batches_owner_location_status_idx
    ON inventory_batches (owner_id, location_id, status);
CREATE INDEX IF NOT EXISTS inventory_batches_owner_zone_status_idx
    ON inventory_batches (owner_id, zone_id, status);

-- 9. 赋权应用账号
GRANT SELECT, INSERT, UPDATE, DELETE ON replenishment_strategies TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON replenishment_location_groups TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON replenishment_location_group_members TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON location_device_bindings TO wms_app;
GRANT SELECT, INSERT ON container_quality_lock_events TO wms_app;
GRANT SELECT, INSERT ON putaway_validation_rejection_logs TO wms_app;
