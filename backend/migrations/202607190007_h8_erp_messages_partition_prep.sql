-- US-H8-003 AC10/AC12：查询索引补强 + 月分区挂载准备（开发期可无数据迁移）
-- 正式环境可将 h8_erp_messages 切换为 RANGE(created_at) 父表；当前以索引裁剪 + 保留策略清理保证边界。

CREATE INDEX IF NOT EXISTS h8_erp_messages_owner_warehouse_created_idx
    ON h8_erp_messages (owner_id, warehouse_id, created_at DESC);

CREATE INDEX IF NOT EXISTS h8_erp_messages_owner_connector_created_idx
    ON h8_erp_messages (owner_id, connector_id, created_at DESC);

CREATE INDEX IF NOT EXISTS h8_erp_message_attempts_owner_started_idx
    ON h8_erp_message_attempts (owner_id, started_at DESC);

-- 按月创建分区的辅助函数（可选调用；不自动改表结构）
CREATE OR REPLACE FUNCTION h8_erp_messages_ensure_month_partition(target_month date)
RETURNS text
LANGUAGE plpgsql
AS $$
DECLARE
    part_name text;
    start_ts timestamptz;
    end_ts timestamptz;
BEGIN
    IF target_month IS NULL THEN
        RAISE EXCEPTION 'target_month required';
    END IF;
    start_ts := date_trunc('month', target_month::timestamptz);
    end_ts := start_ts + interval '1 month';
    part_name := format('h8_erp_messages_%s', to_char(start_ts, 'YYYYMM'));
    -- 仅当主表已是分区表时生效；否则返回提示
    IF EXISTS (
        SELECT 1 FROM pg_partitioned_table
        WHERE partrelid = 'h8_erp_messages'::regclass
    ) THEN
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I PARTITION OF h8_erp_messages
             FOR VALUES FROM (%L) TO (%L)',
            part_name, start_ts, end_ts
        );
        RETURN part_name;
    END IF;
    RETURN 'skipped_not_partitioned';
END;
$$;

COMMENT ON FUNCTION h8_erp_messages_ensure_month_partition(date) IS
  'US-H8-003：在 h8_erp_messages 已声明 RANGE 分区时创建当月分区；未分区则跳过。';
