-- US-H8-001：ERP 连接配置专用表（不入通用键值配置表）

CREATE TABLE IF NOT EXISTS h8_erp_connectors (
    id                          UUID PRIMARY KEY,
    owner_id                    UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    connector_code              TEXT NOT NULL,
    connector_name              TEXT NOT NULL,
    warehouse_ids               UUID[] NOT NULL DEFAULT '{}',
    directions                  TEXT[] NOT NULL,
    message_types               TEXT[] NOT NULL,
    channel_mode                TEXT NOT NULL
        CHECK (channel_mode IN ('rest', 'interface_table', 'rest_primary_table_fallback')),
    api_base_url                TEXT,
    interface_db_host           TEXT,
    interface_db_port           INT,
    interface_db_name           TEXT,
    interface_db_username       TEXT,
    api_key_id                  UUID,
    bearer_secret_alias         TEXT,
    interface_db_password_alias TEXT,
    status                      TEXT NOT NULL DEFAULT 'testing'
        CHECK (status IN ('testing', 'active', 'disabled')),
    config_version              BIGINT NOT NULL DEFAULT 1 CHECK (config_version >= 1),
    first_activated_at          TIMESTAMPTZ,
    last_tested_version         BIGINT,
    last_tested_at              TIMESTAMPTZ,
    last_tested_succeeded       BOOLEAN,
    last_tested_error_summary   TEXT,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_h8_erp_connectors_owner_code UNIQUE (owner_id, connector_code),
    CONSTRAINT ck_h8_erp_directions_nonempty CHECK (cardinality(directions) > 0),
    CONSTRAINT ck_h8_erp_message_types_nonempty CHECK (cardinality(message_types) > 0)
);

CREATE INDEX IF NOT EXISTS h8_erp_connectors_owner_status_idx
    ON h8_erp_connectors (owner_id, status, updated_at DESC);

-- 在途消息绑定（停用后续传）
CREATE TABLE IF NOT EXISTS h8_erp_in_flight_messages (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    connector_id        UUID NOT NULL REFERENCES h8_erp_connectors(id) ON DELETE RESTRICT,
    idempotency_key     TEXT NOT NULL,
    direction           TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound')),
    message_type        TEXT NOT NULL,
    channel_stage       TEXT NOT NULL DEFAULT 'rest'
        CHECK (channel_stage IN ('rest', 'interface_table')),
    status              TEXT NOT NULL DEFAULT 'paused'
        CHECK (status IN ('paused', 'running', 'succeeded', 'failed', 'dead')),
    payload_ref         TEXT,
    last_error          TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_h8_erp_inflight_idem UNIQUE (owner_id, idempotency_key)
);

CREATE INDEX IF NOT EXISTS h8_erp_inflight_connector_status_idx
    ON h8_erp_in_flight_messages (connector_id, status);

INSERT INTO auth_permissions (id, permission_code, permission_name)
SELECT gen_random_uuid(), 'm1.config.read', 'm1.config.read'
WHERE NOT EXISTS (
    SELECT 1 FROM auth_permissions WHERE lower(permission_code) = lower('m1.config.read')
);

INSERT INTO auth_permissions (id, permission_code, permission_name)
SELECT gen_random_uuid(), 'm1.config.write', 'm1.config.write'
WHERE NOT EXISTS (
    SELECT 1 FROM auth_permissions WHERE lower(permission_code) = lower('m1.config.write')
);

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON h8_erp_connectors TO wms_app;
        GRANT SELECT, INSERT, UPDATE, DELETE ON h8_erp_in_flight_messages TO wms_app;
    END IF;
END $$;
