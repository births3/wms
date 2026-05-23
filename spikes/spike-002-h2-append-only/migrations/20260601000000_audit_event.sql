-- 20260601000000_audit_event.sql
-- SPIKE-002 H1 + H2 + H3：append-only 审计表 + 按月 RANGE 分区 + JSONB diff + hash chain
--
-- 设计要点：
--   - PARTITION BY RANGE (occurred_at)：按月分区，partition pruning 生效
--   - PRIMARY KEY 必须含 partition key（PG 限制）
--   - diff JSONB：{before, after, changed_keys}；用 jsonb_path_ops 索引快速过滤变化字段
--   - prev_hash + self_hash：哈希链；单测验证完整性
--   - trigger 阻止 UPDATE/DELETE/TRUNCATE：append-only 强制（DB 层而非应用层）
--   - 角色 wms_app：业务连接用，仅 INSERT/SELECT 权限（DBA 不许给 UPDATE）

-- 主表（声明分区）
CREATE TABLE audit_event (
    id            BIGSERIAL,
    occurred_at   TIMESTAMPTZ NOT NULL,
    actor_id      UUID NOT NULL,
    actor_name    TEXT NOT NULL,
    owner_id      UUID NOT NULL,
    action        TEXT NOT NULL,           -- create/update/delete/login/...
    module        TEXT NOT NULL,           -- M1/M2/H1/...
    resource_type TEXT,
    resource_id   TEXT,
    diff          JSONB,                   -- {before, after, changed_keys}
    request_id    UUID,
    ip            INET,
    user_agent    TEXT,
    prev_hash     TEXT,                    -- 上一条 self_hash（哈希链）
    self_hash     TEXT NOT NULL,           -- sha256(prev_hash || canonical(row))
    PRIMARY KEY (id, occurred_at)
) PARTITION BY RANGE (occurred_at);

-- JSONB 索引：按变化字段过滤
CREATE INDEX audit_event_diff_changed_keys_idx
    ON audit_event USING gin (diff jsonb_path_ops);

-- 常规过滤索引
CREATE INDEX audit_event_actor_idx ON audit_event (actor_id, occurred_at DESC);
CREATE INDEX audit_event_module_idx ON audit_event (module, occurred_at DESC);
CREATE INDEX audit_event_owner_idx ON audit_event (owner_id, occurred_at DESC);

-- 12 个月分区（spike 演示；生产由 cron 滚动维护）
CREATE TABLE audit_event_2026_01 PARTITION OF audit_event
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
CREATE TABLE audit_event_2026_02 PARTITION OF audit_event
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');
CREATE TABLE audit_event_2026_03 PARTITION OF audit_event
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
CREATE TABLE audit_event_2026_04 PARTITION OF audit_event
    FOR VALUES FROM ('2026-04-01') TO ('2026-05-01');
CREATE TABLE audit_event_2026_05 PARTITION OF audit_event
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE audit_event_2026_06 PARTITION OF audit_event
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE audit_event_2026_07 PARTITION OF audit_event
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');
CREATE TABLE audit_event_2026_08 PARTITION OF audit_event
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE audit_event_2026_09 PARTITION OF audit_event
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');
CREATE TABLE audit_event_2026_10 PARTITION OF audit_event
    FOR VALUES FROM ('2026-10-01') TO ('2026-11-01');
CREATE TABLE audit_event_2026_11 PARTITION OF audit_event
    FOR VALUES FROM ('2026-11-01') TO ('2026-12-01');
CREATE TABLE audit_event_2026_12 PARTITION OF audit_event
    FOR VALUES FROM ('2026-12-01') TO ('2027-01-01');

-- ============================================================
-- H1: trigger 阻止 UPDATE / DELETE / TRUNCATE
-- 即使应用层有 bug 写出 UPDATE 语句，PG 层仍拒绝
-- ============================================================
CREATE OR REPLACE FUNCTION audit_event_immutable() RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_event is append-only: % attempted by %', TG_OP, current_user
        USING ERRCODE = 'insufficient_privilege';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_audit_event_no_update
    BEFORE UPDATE OR DELETE OR TRUNCATE ON audit_event
    FOR EACH STATEMENT EXECUTE FUNCTION audit_event_immutable();

-- 子分区也要挂同样 trigger（PG 不会自动继承 partition trigger）
DO $$
DECLARE
    p text;
BEGIN
    FOR p IN
        SELECT inhrelid::regclass::text
        FROM pg_inherits
        WHERE inhparent = 'audit_event'::regclass
    LOOP
        EXECUTE format(
            'CREATE TRIGGER trg_no_update BEFORE UPDATE OR DELETE OR TRUNCATE ON %s '
            'FOR EACH STATEMENT EXECUTE FUNCTION audit_event_immutable()',
            p
        );
    END LOOP;
END $$;
