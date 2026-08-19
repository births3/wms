-- Wave 1 W1.B audit trail baseline, aligned with ADR-0025.

DO $$
BEGIN
    CREATE ROLE wms_app NOLOGIN;
EXCEPTION
    WHEN duplicate_object OR unique_violation THEN NULL;
END
$$;

CREATE TABLE IF NOT EXISTS audit_event (
    id            BIGSERIAL,
    occurred_at   TIMESTAMPTZ NOT NULL,
    actor_id      UUID NOT NULL,
    actor_name    TEXT NOT NULL,
    owner_id      UUID NOT NULL,
    jti           TEXT NOT NULL,
    action        TEXT NOT NULL,
    module        TEXT NOT NULL,
    resource_type TEXT,
    resource_id   TEXT,
    diff          JSONB,
    request_id    UUID,
    ip            INET,
    user_agent    TEXT,
    prev_hash     TEXT,
    self_hash     TEXT NOT NULL,
    PRIMARY KEY (id, occurred_at)
) PARTITION BY RANGE (occurred_at);

CREATE TABLE IF NOT EXISTS audit_event_2026_06 PARTITION OF audit_event
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');

CREATE INDEX IF NOT EXISTS audit_event_owner_idx
    ON audit_event (owner_id, occurred_at DESC);

CREATE INDEX IF NOT EXISTS audit_event_actor_idx
    ON audit_event (actor_id, occurred_at DESC);

CREATE INDEX IF NOT EXISTS audit_event_module_idx
    ON audit_event (module, occurred_at DESC);

CREATE INDEX IF NOT EXISTS audit_event_diff_changed_keys_idx
    ON audit_event USING gin (diff jsonb_path_ops);

CREATE OR REPLACE FUNCTION audit_event_immutable() RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_event is append-only: % attempted by %', TG_OP, current_user;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_audit_event_no_update ON audit_event;
CREATE TRIGGER trg_audit_event_no_update
    BEFORE UPDATE OR DELETE OR TRUNCATE ON audit_event
    FOR EACH STATEMENT EXECUTE FUNCTION audit_event_immutable();

DROP TRIGGER IF EXISTS trg_audit_event_2026_06_no_update ON audit_event_2026_06;
CREATE TRIGGER trg_audit_event_2026_06_no_update
    BEFORE UPDATE OR DELETE OR TRUNCATE ON audit_event_2026_06
    FOR EACH STATEMENT EXECUTE FUNCTION audit_event_immutable();

CREATE OR REPLACE FUNCTION create_audit_partition(partition_start DATE)
RETURNS TEXT AS $$
DECLARE
    normalized_start DATE := date_trunc('month', partition_start)::DATE;
    partition_end DATE := (normalized_start + INTERVAL '1 month')::DATE;
    partition_name TEXT := format('audit_event_%s', to_char(normalized_start, 'YYYY_MM'));
    trigger_name TEXT := format('trg_%s_no_update', partition_name);
BEGIN
    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF audit_event FOR VALUES FROM (%L) TO (%L)',
        partition_name,
        normalized_start,
        partition_end
    );
    EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', trigger_name, partition_name);
    EXECUTE format(
        'CREATE TRIGGER %I BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION audit_event_immutable()',
        trigger_name,
        partition_name
    );
    RETURN partition_name;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION create_current_partition(reference_date DATE DEFAULT CURRENT_DATE)
RETURNS TEXT AS $$
    SELECT create_audit_partition(date_trunc('month', reference_date)::DATE);
$$ LANGUAGE sql;

CREATE OR REPLACE FUNCTION create_next_partition(reference_date DATE DEFAULT CURRENT_DATE)
RETURNS TEXT AS $$
    SELECT create_audit_partition(date_trunc('month', reference_date + INTERVAL '1 month')::DATE);
$$ LANGUAGE sql;

SELECT create_current_partition(CURRENT_DATE);
SELECT create_next_partition(CURRENT_DATE);

CREATE TABLE IF NOT EXISTS audit_chain_seal (
    seal_date      DATE PRIMARY KEY,
    last_id        BIGINT NOT NULL,
    last_self_hash TEXT NOT NULL,
    sealed_at      TIMESTAMPTZ NOT NULL
);

CREATE OR REPLACE FUNCTION audit_chain_seal_immutable() RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_chain_seal is read-only after insert: % attempted by %', TG_OP, current_user;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_audit_chain_seal_no_update ON audit_chain_seal;
CREATE TRIGGER trg_audit_chain_seal_no_update
    BEFORE UPDATE OR DELETE OR TRUNCATE ON audit_chain_seal
    FOR EACH STATEMENT EXECUTE FUNCTION audit_chain_seal_immutable();

GRANT INSERT, SELECT ON audit_event TO wms_app;
GRANT USAGE, SELECT ON SEQUENCE audit_event_id_seq TO wms_app;
GRANT INSERT, SELECT ON audit_chain_seal TO wms_app;
GRANT EXECUTE ON FUNCTION create_audit_partition(DATE) TO wms_app;
GRANT EXECUTE ON FUNCTION create_current_partition(DATE) TO wms_app;
GRANT EXECUTE ON FUNCTION create_next_partition(DATE) TO wms_app;
