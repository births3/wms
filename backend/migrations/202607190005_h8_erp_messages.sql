-- US-H8-003：ERP 消息主记录 + append-only 尝试记录（运行日志，非 H2 审计替代）

CREATE TABLE IF NOT EXISTS h8_erp_messages (
    id                      UUID PRIMARY KEY,
    owner_id                UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    warehouse_id            UUID,
    connector_id            UUID REFERENCES h8_erp_connectors(id) ON DELETE RESTRICT,
    connector_code          TEXT,
    config_version          BIGINT,
    direction               TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound')),
    message_type            TEXT NOT NULL,
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
    CONSTRAINT uq_h8_erp_messages_idem UNIQUE (owner_id, message_type, external_ref, idempotency_key)
);

-- 查询必须命中货主 + 时间（AC12）；月分区可后续挂载，首版以复合索引保证裁剪
CREATE INDEX IF NOT EXISTS h8_erp_messages_owner_created_idx
    ON h8_erp_messages (owner_id, created_at DESC);
CREATE INDEX IF NOT EXISTS h8_erp_messages_owner_status_idx
    ON h8_erp_messages (owner_id, sync_status, created_at DESC);
CREATE INDEX IF NOT EXISTS h8_erp_messages_owner_type_idx
    ON h8_erp_messages (owner_id, direction, message_type, created_at DESC);
CREATE INDEX IF NOT EXISTS h8_erp_messages_correlation_idx
    ON h8_erp_messages (owner_id, correlation_id);

CREATE TABLE IF NOT EXISTS h8_erp_message_attempts (
    id                  UUID PRIMARY KEY,
    message_id          UUID NOT NULL REFERENCES h8_erp_messages(id) ON DELETE RESTRICT,
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
    CONSTRAINT uq_h8_erp_message_attempts UNIQUE (message_id, attempt_no)
);

CREATE INDEX IF NOT EXISTS h8_erp_message_attempts_msg_idx
    ON h8_erp_message_attempts (message_id, attempt_no);

-- 受控保留策略：未配置时禁止自动清理（AC10）
CREATE TABLE IF NOT EXISTS h8_erp_message_retention_policy (
    owner_id            UUID PRIMARY KEY REFERENCES auth_owners(id) ON DELETE RESTRICT,
    retention_days      INT NOT NULL CHECK (retention_days > 0),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON h8_erp_messages TO wms_app;
        GRANT SELECT, INSERT ON h8_erp_message_attempts TO wms_app;
        GRANT SELECT, INSERT, UPDATE, DELETE ON h8_erp_message_retention_policy TO wms_app;
    END IF;
END $$;
