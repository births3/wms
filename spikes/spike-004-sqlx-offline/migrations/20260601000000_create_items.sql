-- 20260601000000_create_items.sql
-- SPIKE-004 H4 验证：sqlx::migrate! 加载多 migration

CREATE TABLE items (
    id          UUID PRIMARY KEY,
    code        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    expiry      DATE NOT NULL,
    stock       INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_items_expiry ON items (expiry);
