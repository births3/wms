-- US-H8-003 AC9/15/16：Worker 心跳、暂停认领与完整报文短期加密保留。

CREATE EXTENSION IF NOT EXISTS pgcrypto;

ALTER TABLE h8_erp_connectors
    ADD CONSTRAINT uq_h8_erp_connectors_owner_id UNIQUE (owner_id, id);

CREATE TABLE h8_erp_worker_heartbeats (
    owner_id                 UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    worker_id                TEXT NOT NULL CHECK (btrim(worker_id) <> '' AND length(worker_id) <= 128),
    worker_version           TEXT NOT NULL CHECK (btrim(worker_version) <> '' AND length(worker_version) <= 64),
    connector_id             UUID NOT NULL,
    directions               TEXT[] NOT NULL,
    current_claims           INT NOT NULL CHECK (current_claims >= 0),
    created_at               TIMESTAMPTZ NOT NULL,
    last_heartbeat_at        TIMESTAMPTZ NOT NULL,
    heartbeat_expires_at     TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (owner_id, worker_id),
    CHECK (cardinality(directions) BETWEEN 1 AND 2),
    CHECK (array_position(directions, NULL) IS NULL),
    CHECK (cardinality(directions) = 1 OR directions[1] <> directions[2]),
    CHECK (directions <@ ARRAY['inbound', 'outbound']::TEXT[]),
    FOREIGN KEY (owner_id, connector_id)
        REFERENCES h8_erp_connectors(owner_id, id) ON DELETE RESTRICT
);

CREATE INDEX h8_erp_worker_heartbeats_owner_expiry_idx
    ON h8_erp_worker_heartbeats (owner_id, heartbeat_expires_at DESC);

CREATE TABLE h8_erp_worker_claim_controls (
    owner_id                 UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    connector_id             UUID NOT NULL,
    direction                TEXT NOT NULL CHECK (direction IN ('inbound', 'outbound')),
    paused                   BOOLEAN NOT NULL,
    reason                   TEXT NOT NULL CHECK (btrim(reason) <> '' AND length(reason) <= 500),
    paused_until             TIMESTAMPTZ,
    updated_by               TEXT NOT NULL,
    updated_at               TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (owner_id, connector_id, direction),
    FOREIGN KEY (owner_id, connector_id)
        REFERENCES h8_erp_connectors(owner_id, id) ON DELETE RESTRICT
);

CREATE TABLE h8_erp_payload_retention_policies (
    owner_id                 UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    connector_id             UUID NOT NULL,
    enabled                  BOOLEAN NOT NULL DEFAULT FALSE,
    retention_days           INT NOT NULL DEFAULT 7 CHECK (retention_days BETWEEN 1 AND 30),
    updated_by               TEXT NOT NULL,
    updated_at               TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (owner_id, connector_id),
    FOREIGN KEY (owner_id, connector_id)
        REFERENCES h8_erp_connectors(owner_id, id) ON DELETE RESTRICT
);

ALTER TABLE h8_erp_messages
    ADD COLUMN encrypted_payload BYTEA,
    ADD COLUMN payload_key_version TEXT,
    ADD COLUMN payload_expires_at TIMESTAMPTZ;

ALTER TABLE h8_erp_messages
    ADD CONSTRAINT h8_erp_messages_payload_cipher_check CHECK (
        (encrypted_payload IS NULL AND payload_key_version IS NULL AND payload_expires_at IS NULL)
        OR
        (encrypted_payload IS NOT NULL AND btrim(payload_key_version) <> ''
         AND length(payload_key_version) <= 64 AND payload_expires_at IS NOT NULL)
    );

CREATE INDEX h8_erp_messages_payload_expiry_idx
    ON h8_erp_messages (owner_id, payload_expires_at)
    WHERE encrypted_payload IS NOT NULL;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON h8_erp_worker_heartbeats TO wms_app;
        GRANT SELECT, INSERT, UPDATE, DELETE ON h8_erp_worker_claim_controls TO wms_app;
        GRANT SELECT, INSERT, UPDATE, DELETE ON h8_erp_payload_retention_policies TO wms_app;
    END IF;
END $$;
