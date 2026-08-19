-- US-H8-003：ERP 消息主记录 + append-only 尝试记录（运行日志，非 H2 审计替代）
-- PostgreSQL 分区表的唯一键必须包含分区键；两个极小登记表负责跨月全局唯一性。

CREATE TABLE IF NOT EXISTS h8_erp_message_registry (
    id                      UUID PRIMARY KEY,
    owner_id                UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    message_type            TEXT NOT NULL,
    external_ref            TEXT NOT NULL,
    idempotency_key         TEXT NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL,
    CONSTRAINT uq_h8_erp_message_registry_owner_id UNIQUE (owner_id, id),
    CONSTRAINT uq_h8_erp_message_registry_idem
        UNIQUE (owner_id, message_type, external_ref, idempotency_key)
);

CREATE TABLE IF NOT EXISTS h8_erp_messages (
    id                      UUID NOT NULL,
    owner_id                UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    warehouse_id            UUID,
    connector_id            UUID REFERENCES h8_erp_connectors(id) ON DELETE RESTRICT,
    connector_code          TEXT,
    config_version          BIGINT,
    direction               TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound')),
    message_type            TEXT NOT NULL,
    schema_version          TEXT NOT NULL DEFAULT '1',
    channel                 TEXT NOT NULL CHECK (channel IN ('rest', 'interface_table')),
    external_ref            TEXT NOT NULL,
    wms_resource_id         TEXT,
    idempotency_key         TEXT NOT NULL,
    correlation_id          TEXT NOT NULL,
    sync_status             TEXT NOT NULL DEFAULT 'pending'
        CHECK (sync_status IN ('pending', 'processing', 'succeeded', 'failed', 'dead', 'acked')),
    retry_count             INT NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
    next_retry_at           TIMESTAMPTZ,
    last_error_summary      TEXT,
    payload_digest          TEXT NOT NULL,
    claimed_by              TEXT,
    lease_expires_at        TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at            TIMESTAMPTZ,
    acked_at                TIMESTAMPTZ,
    PRIMARY KEY (id, created_at),
    FOREIGN KEY (id) REFERENCES h8_erp_message_registry(id) ON DELETE RESTRICT
) PARTITION BY RANGE (created_at);

CREATE OR REPLACE FUNCTION h8_erp_message_register()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    registered public.h8_erp_message_registry%ROWTYPE;
BEGIN
    INSERT INTO public.h8_erp_message_registry
        (id, owner_id, message_type, external_ref, idempotency_key, created_at)
    VALUES
        (NEW.id, NEW.owner_id, NEW.message_type, NEW.external_ref,
         NEW.idempotency_key, NEW.created_at)
    ON CONFLICT DO NOTHING;

    SELECT * INTO registered
    FROM public.h8_erp_message_registry
    WHERE id = NEW.id;

    IF NOT FOUND
       OR registered.owner_id IS DISTINCT FROM NEW.owner_id
       OR registered.message_type IS DISTINCT FROM NEW.message_type
       OR registered.external_ref IS DISTINCT FROM NEW.external_ref
       OR registered.idempotency_key IS DISTINCT FROM NEW.idempotency_key
       OR registered.created_at IS DISTINCT FROM NEW.created_at THEN
        RAISE unique_violation
            USING MESSAGE = 'H8 message identity conflicts with an existing month',
                  CONSTRAINT = 'uq_h8_erp_message_registry_idem';
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION h8_erp_message_unregister()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    DELETE FROM public.h8_erp_message_registry registry
    WHERE registry.id = OLD.id
      AND NOT EXISTS (SELECT 1 FROM public.h8_erp_messages WHERE id = OLD.id);
    RETURN OLD;
END;
$$;

CREATE TRIGGER h8_erp_messages_register
    BEFORE INSERT ON h8_erp_messages
    FOR EACH ROW EXECUTE FUNCTION h8_erp_message_register();

CREATE TRIGGER h8_erp_messages_unregister
    AFTER DELETE ON h8_erp_messages
    FOR EACH ROW EXECUTE FUNCTION h8_erp_message_unregister();

CREATE OR REPLACE FUNCTION h8_erp_message_identity_immutable()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.id IS DISTINCT FROM NEW.id
       OR OLD.owner_id IS DISTINCT FROM NEW.owner_id
       OR OLD.warehouse_id IS DISTINCT FROM NEW.warehouse_id
       OR OLD.connector_id IS DISTINCT FROM NEW.connector_id
       OR OLD.connector_code IS DISTINCT FROM NEW.connector_code
       OR OLD.config_version IS DISTINCT FROM NEW.config_version
       OR OLD.direction IS DISTINCT FROM NEW.direction
       OR OLD.message_type IS DISTINCT FROM NEW.message_type
       OR OLD.schema_version IS DISTINCT FROM NEW.schema_version
       OR OLD.channel IS DISTINCT FROM NEW.channel
       OR OLD.external_ref IS DISTINCT FROM NEW.external_ref
       OR OLD.idempotency_key IS DISTINCT FROM NEW.idempotency_key
       OR OLD.correlation_id IS DISTINCT FROM NEW.correlation_id
       OR OLD.created_at IS DISTINCT FROM NEW.created_at THEN
        RAISE check_violation USING MESSAGE = 'H8 message identity is immutable';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER h8_erp_messages_identity_immutable
    BEFORE UPDATE ON h8_erp_messages
    FOR EACH ROW EXECUTE FUNCTION h8_erp_message_identity_immutable();

CREATE TABLE IF NOT EXISTS h8_erp_message_stats_daily (
    -- ponytail: 日维度原生汇总先满足 10M/月；仅在实测出现热行锁等待时再拆小时桶。
    owner_id                UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    stat_date               DATE NOT NULL,
    connector_code          TEXT NOT NULL,
    channel                 TEXT NOT NULL,
    message_type            TEXT NOT NULL,
    warehouse_id            TEXT NOT NULL,
    total                   BIGINT NOT NULL DEFAULT 0 CHECK (total >= 0),
    succeeded               BIGINT NOT NULL DEFAULT 0 CHECK (succeeded >= 0),
    failed                  BIGINT NOT NULL DEFAULT 0 CHECK (failed >= 0),
    dead                    BIGINT NOT NULL DEFAULT 0 CHECK (dead >= 0),
    processing              BIGINT NOT NULL DEFAULT 0 CHECK (processing >= 0),
    pending                 BIGINT NOT NULL DEFAULT 0 CHECK (pending >= 0),
    retry_total             BIGINT NOT NULL DEFAULT 0 CHECK (retry_total >= 0),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, stat_date, connector_code, channel, message_type, warehouse_id)
);

CREATE OR REPLACE FUNCTION h8_erp_message_stats_sync()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    source public.h8_erp_messages%ROWTYPE;
    total_delta BIGINT := 0;
    succeeded_delta BIGINT := 0;
    failed_delta BIGINT := 0;
    dead_delta BIGINT := 0;
    processing_delta BIGINT := 0;
    pending_delta BIGINT := 0;
    retry_delta BIGINT := 0;
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO public.h8_erp_message_stats_daily (
            owner_id, stat_date, connector_code, channel, message_type, warehouse_id,
            total, succeeded, failed, dead, processing, pending, retry_total, updated_at
        ) VALUES (
            NEW.owner_id, (NEW.created_at AT TIME ZONE 'UTC')::date,
            COALESCE(NEW.connector_code, ''),
            NEW.channel, NEW.message_type, COALESCE(NEW.warehouse_id::text, ''), 1,
            CASE WHEN NEW.sync_status IN ('succeeded', 'acked') THEN 1 ELSE 0 END,
            CASE WHEN NEW.sync_status = 'failed' THEN 1 ELSE 0 END,
            CASE WHEN NEW.sync_status = 'dead' THEN 1 ELSE 0 END,
            CASE WHEN NEW.sync_status = 'processing' THEN 1 ELSE 0 END,
            CASE WHEN NEW.sync_status = 'pending' THEN 1 ELSE 0 END,
            NEW.retry_count, now()
        )
        ON CONFLICT (owner_id, stat_date, connector_code, channel, message_type, warehouse_id)
        DO UPDATE SET
            total = h8_erp_message_stats_daily.total + 1,
            succeeded = h8_erp_message_stats_daily.succeeded + EXCLUDED.succeeded,
            failed = h8_erp_message_stats_daily.failed + EXCLUDED.failed,
            dead = h8_erp_message_stats_daily.dead + EXCLUDED.dead,
            processing = h8_erp_message_stats_daily.processing + EXCLUDED.processing,
            pending = h8_erp_message_stats_daily.pending + EXCLUDED.pending,
            retry_total = h8_erp_message_stats_daily.retry_total + EXCLUDED.retry_total,
            updated_at = EXCLUDED.updated_at;
        RETURN NEW;
    END IF;

    IF TG_OP = 'UPDATE' THEN
        source := NEW;
        succeeded_delta :=
            CASE WHEN NEW.sync_status IN ('succeeded', 'acked') THEN 1 ELSE 0 END
            - CASE WHEN OLD.sync_status IN ('succeeded', 'acked') THEN 1 ELSE 0 END;
        failed_delta := CASE WHEN NEW.sync_status = 'failed' THEN 1 ELSE 0 END
            - CASE WHEN OLD.sync_status = 'failed' THEN 1 ELSE 0 END;
        dead_delta := CASE WHEN NEW.sync_status = 'dead' THEN 1 ELSE 0 END
            - CASE WHEN OLD.sync_status = 'dead' THEN 1 ELSE 0 END;
        processing_delta := CASE WHEN NEW.sync_status = 'processing' THEN 1 ELSE 0 END
            - CASE WHEN OLD.sync_status = 'processing' THEN 1 ELSE 0 END;
        pending_delta := CASE WHEN NEW.sync_status = 'pending' THEN 1 ELSE 0 END
            - CASE WHEN OLD.sync_status = 'pending' THEN 1 ELSE 0 END;
        retry_delta := NEW.retry_count - OLD.retry_count;
    ELSE
        source := OLD;
        total_delta := -1;
        succeeded_delta := -CASE WHEN OLD.sync_status IN ('succeeded', 'acked') THEN 1 ELSE 0 END;
        failed_delta := -CASE WHEN OLD.sync_status = 'failed' THEN 1 ELSE 0 END;
        dead_delta := -CASE WHEN OLD.sync_status = 'dead' THEN 1 ELSE 0 END;
        processing_delta := -CASE WHEN OLD.sync_status = 'processing' THEN 1 ELSE 0 END;
        pending_delta := -CASE WHEN OLD.sync_status = 'pending' THEN 1 ELSE 0 END;
        retry_delta := -OLD.retry_count;
    END IF;

    UPDATE public.h8_erp_message_stats_daily SET
        total = total + total_delta,
        succeeded = succeeded + succeeded_delta,
        failed = failed + failed_delta,
        dead = dead + dead_delta,
        processing = processing + processing_delta,
        pending = pending + pending_delta,
        retry_total = retry_total + retry_delta,
        updated_at = now()
    WHERE owner_id = source.owner_id
      AND stat_date = (source.created_at AT TIME ZONE 'UTC')::date
      AND connector_code = COALESCE(source.connector_code, '')
      AND channel = source.channel
      AND message_type = source.message_type
      AND warehouse_id = COALESCE(source.warehouse_id::text, '');

    DELETE FROM public.h8_erp_message_stats_daily
    WHERE owner_id = source.owner_id
      AND stat_date = (source.created_at AT TIME ZONE 'UTC')::date
      AND connector_code = COALESCE(source.connector_code, '')
      AND channel = source.channel
      AND message_type = source.message_type
      AND warehouse_id = COALESCE(source.warehouse_id::text, '')
      AND total = 0;
    RETURN source;
END;
$$;

CREATE TRIGGER h8_erp_messages_stats_insert
    AFTER INSERT ON h8_erp_messages
    FOR EACH ROW EXECUTE FUNCTION h8_erp_message_stats_sync();

CREATE TRIGGER h8_erp_messages_stats_update
    AFTER UPDATE OF sync_status, retry_count ON h8_erp_messages
    FOR EACH ROW EXECUTE FUNCTION h8_erp_message_stats_sync();

CREATE TRIGGER h8_erp_messages_stats_delete
    AFTER DELETE ON h8_erp_messages
    FOR EACH ROW EXECUTE FUNCTION h8_erp_message_stats_sync();

-- 查询同时使用月分区裁剪与货主/时间复合索引（AC12）。
CREATE INDEX IF NOT EXISTS h8_erp_messages_owner_created_idx
    ON h8_erp_messages (owner_id, created_at DESC);
CREATE INDEX IF NOT EXISTS h8_erp_messages_owner_status_idx
    ON h8_erp_messages (owner_id, sync_status, created_at DESC);
CREATE INDEX IF NOT EXISTS h8_erp_messages_owner_type_idx
    ON h8_erp_messages (owner_id, direction, message_type, created_at DESC);
CREATE INDEX IF NOT EXISTS h8_erp_messages_correlation_idx
    ON h8_erp_messages (owner_id, correlation_id);

CREATE TABLE IF NOT EXISTS h8_erp_message_attempt_registry (
    id                  UUID PRIMARY KEY,
    message_id          UUID NOT NULL,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    attempt_no          INT NOT NULL CHECK (attempt_no >= 1),
    started_at          TIMESTAMPTZ NOT NULL,
    CONSTRAINT uq_h8_erp_message_attempt_registry UNIQUE (message_id, attempt_no),
    FOREIGN KEY (owner_id, message_id)
        REFERENCES h8_erp_message_registry(owner_id, id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS h8_erp_message_attempts (
    id                  UUID NOT NULL,
    message_id          UUID NOT NULL,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    attempt_no          INT NOT NULL CHECK (attempt_no >= 1),
    channel             TEXT NOT NULL CHECK (channel IN ('rest', 'interface_table')),
    started_at          TIMESTAMPTZ NOT NULL,
    finished_at         TIMESTAMPTZ,
    result              TEXT NOT NULL
        CHECK (result IN ('succeeded', 'failed', 'dead', 'replayed', 'claimed')),
    error_summary       TEXT,
    actor               TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, started_at),
    FOREIGN KEY (id) REFERENCES h8_erp_message_attempt_registry(id) ON DELETE RESTRICT,
    FOREIGN KEY (owner_id, message_id)
        REFERENCES h8_erp_message_registry(owner_id, id) ON DELETE RESTRICT
) PARTITION BY RANGE (started_at);

CREATE OR REPLACE FUNCTION h8_erp_message_attempt_register()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    registered public.h8_erp_message_attempt_registry%ROWTYPE;
BEGIN
    INSERT INTO public.h8_erp_message_attempt_registry
        (id, message_id, owner_id, attempt_no, started_at)
    VALUES
        (NEW.id, NEW.message_id, NEW.owner_id, NEW.attempt_no, NEW.started_at)
    ON CONFLICT DO NOTHING;

    SELECT * INTO registered
    FROM public.h8_erp_message_attempt_registry
    WHERE id = NEW.id;

    IF NOT FOUND
       OR registered.message_id IS DISTINCT FROM NEW.message_id
       OR registered.owner_id IS DISTINCT FROM NEW.owner_id
       OR registered.attempt_no IS DISTINCT FROM NEW.attempt_no
       OR registered.started_at IS DISTINCT FROM NEW.started_at THEN
        RAISE unique_violation
            USING MESSAGE = 'H8 message attempt identity conflicts with an existing month',
                  CONSTRAINT = 'uq_h8_erp_message_attempt_registry';
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION h8_erp_message_attempt_unregister()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    DELETE FROM public.h8_erp_message_attempt_registry registry
    WHERE registry.id = OLD.id
      AND NOT EXISTS (SELECT 1 FROM public.h8_erp_message_attempts WHERE id = OLD.id);
    RETURN OLD;
END;
$$;

CREATE TRIGGER h8_erp_message_attempts_register
    BEFORE INSERT ON h8_erp_message_attempts
    FOR EACH ROW EXECUTE FUNCTION h8_erp_message_attempt_register();

CREATE TRIGGER h8_erp_message_attempts_unregister
    AFTER DELETE ON h8_erp_message_attempts
    FOR EACH ROW EXECUTE FUNCTION h8_erp_message_attempt_unregister();

CREATE OR REPLACE FUNCTION h8_erp_message_attempt_immutable()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE object_not_in_prerequisite_state
        USING MESSAGE = 'H8 message attempts are append-only';
END;
$$;

CREATE TRIGGER h8_erp_message_attempts_immutable
    BEFORE UPDATE ON h8_erp_message_attempts
    FOR EACH ROW EXECUTE FUNCTION h8_erp_message_attempt_immutable();

CREATE INDEX IF NOT EXISTS h8_erp_message_attempts_msg_idx
    ON h8_erp_message_attempts (message_id, attempt_no);

-- 受控保留策略：未配置时禁止自动清理（AC10）
CREATE TABLE IF NOT EXISTS h8_erp_message_retention_policy (
    owner_id            UUID PRIMARY KEY REFERENCES auth_owners(id) ON DELETE RESTRICT,
    retention_days      INT NOT NULL CHECK (retention_days > 0),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

REVOKE ALL ON FUNCTION h8_erp_message_register() FROM PUBLIC;
REVOKE ALL ON FUNCTION h8_erp_message_unregister() FROM PUBLIC;
REVOKE ALL ON FUNCTION h8_erp_message_identity_immutable() FROM PUBLIC;
REVOKE ALL ON FUNCTION h8_erp_message_stats_sync() FROM PUBLIC;
REVOKE ALL ON FUNCTION h8_erp_message_attempt_register() FROM PUBLIC;
REVOKE ALL ON FUNCTION h8_erp_message_attempt_unregister() FROM PUBLIC;
REVOKE ALL ON FUNCTION h8_erp_message_attempt_immutable() FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON h8_erp_messages TO wms_app;
        GRANT SELECT, INSERT ON h8_erp_message_attempts TO wms_app;
        GRANT SELECT ON h8_erp_message_registry TO wms_app;
        GRANT SELECT ON h8_erp_message_stats_daily TO wms_app;
        GRANT SELECT, INSERT, UPDATE, DELETE ON h8_erp_message_retention_policy TO wms_app;
    END IF;
END $$;
