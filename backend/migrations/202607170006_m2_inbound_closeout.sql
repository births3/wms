-- M2 入库收口：供应商资质有效期、上架策略方案（最小可配置模型）

ALTER TABLE suppliers
    ADD COLUMN IF NOT EXISTS qualification_valid_until TIMESTAMPTZ;

COMMENT ON COLUMN suppliers.qualification_valid_until IS
    '供应商资质有效期截止时间；NULL 表示未登记截止日期，仅校验 active 状态。';

CREATE TABLE IF NOT EXISTS putaway_strategy_profiles (
    id            UUID PRIMARY KEY,
    owner_id      UUID NOT NULL,
    profile_code  TEXT NOT NULL,
    profile_name  TEXT NOT NULL,
    is_default    BOOLEAN NOT NULL DEFAULT FALSE,
    top_n         INT NOT NULL DEFAULT 3
        CHECK (top_n > 0 AND top_n <= 50),
    enabled_rules JSONB NOT NULL DEFAULT '{
        "temperature_match": true,
        "owner_isolation": true,
        "capacity_match": true,
        "same_product_cluster": true,
        "quality_color_match": true
    }'::jsonb,
    rule_priority JSONB NOT NULL DEFAULT '[
        "temperature_match",
        "owner_isolation",
        "capacity_match",
        "quality_color_match",
        "same_product_cluster"
    ]'::jsonb,
    status        TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'disabled')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    version       BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, profile_code)
);

CREATE UNIQUE INDEX IF NOT EXISTS putaway_strategy_profiles_one_default_idx
    ON putaway_strategy_profiles (owner_id)
    WHERE is_default AND status = 'active';

CREATE INDEX IF NOT EXISTS putaway_strategy_profiles_owner_status_idx
    ON putaway_strategy_profiles (owner_id, status, updated_at DESC);

COMMENT ON TABLE putaway_strategy_profiles IS
    'M2 上架策略方案：按货主维护默认/多方案，驱动库位推荐 Top N 与规则启停。';
