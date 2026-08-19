-- US-H8-003 AC10/AC12：查询索引补强 + 消息与尝试月分区维护。

CREATE INDEX IF NOT EXISTS h8_erp_messages_owner_warehouse_created_idx
    ON h8_erp_messages (owner_id, warehouse_id, created_at DESC);

CREATE INDEX IF NOT EXISTS h8_erp_messages_owner_connector_created_idx
    ON h8_erp_messages (owner_id, connector_id, created_at DESC);

CREATE INDEX IF NOT EXISTS h8_erp_message_attempts_owner_started_idx
    ON h8_erp_message_attempts (owner_id, started_at DESC);

-- 复用 H2 的运维模式：迁移创建当前月/下月，维护任务持续提前创建下月。
CREATE OR REPLACE FUNCTION h8_erp_messages_ensure_month_partition(target_month date)
RETURNS text
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    message_part_name text;
    attempt_part_name text;
    start_ts timestamptz;
    end_ts timestamptz;
BEGIN
    IF target_month IS NULL THEN
        RAISE EXCEPTION 'target_month required';
    END IF;
    start_ts := date_trunc('month', target_month::timestamp) AT TIME ZONE 'UTC';
    end_ts := start_ts + interval '1 month';
    PERFORM pg_advisory_xact_lock(hashtextextended('h8-erp-partition:' || start_ts::text, 0));
    message_part_name := format(
        'h8_erp_messages_%s', to_char(start_ts AT TIME ZONE 'UTC', 'YYYYMM')
    );
    attempt_part_name := format(
        'h8_erp_message_attempts_%s', to_char(start_ts AT TIME ZONE 'UTC', 'YYYYMM')
    );
    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS public.%I PARTITION OF public.h8_erp_messages
         FOR VALUES FROM (%L) TO (%L)',
        message_part_name, start_ts, end_ts
    );
    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS public.%I PARTITION OF public.h8_erp_message_attempts
         FOR VALUES FROM (%L) TO (%L)',
        attempt_part_name, start_ts, end_ts
    );
    RETURN format('%s,%s', message_part_name, attempt_part_name);
END;
$$;

COMMENT ON FUNCTION h8_erp_messages_ensure_month_partition(date) IS
  'US-H8-003：同时创建消息 created_at 与尝试 started_at 的月分区。';

REVOKE ALL ON FUNCTION h8_erp_messages_ensure_month_partition(date) FROM PUBLIC;

SELECT h8_erp_messages_ensure_month_partition(
    (CURRENT_TIMESTAMP AT TIME ZONE 'UTC')::date
);
SELECT h8_erp_messages_ensure_month_partition(
    ((CURRENT_TIMESTAMP AT TIME ZONE 'UTC') + INTERVAL '1 month')::date
);

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        GRANT EXECUTE ON FUNCTION h8_erp_messages_ensure_month_partition(date) TO wms_app;
    END IF;
END $$;
