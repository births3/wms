-- US-CG-001 / US-CG-002 no-gap document numbering first backend slice.

CREATE TABLE IF NOT EXISTS document_number_rules (
    id              UUID PRIMARY KEY,
    owner_id        UUID REFERENCES auth_owners(id) ON DELETE CASCADE,
    document_type   TEXT NOT NULL,
    rule_code       TEXT NOT NULL,
    rule_name       TEXT NOT NULL,
    template        TEXT NOT NULL,
    reset_policy    TEXT NOT NULL,
    sequence_width  INT NOT NULL CHECK (sequence_width > 0 AND sequence_width <= 18),
    sequence_mode   TEXT NOT NULL DEFAULT 'no_gap',
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    effective_from  TIMESTAMPTZ,
    effective_to    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    version         BIGINT NOT NULL DEFAULT 1,
    CHECK (reset_policy IN ('daily', 'continuous')),
    CHECK (sequence_mode = 'no_gap'),
    CHECK (effective_to IS NULL OR effective_from IS NULL OR effective_to > effective_from)
);

CREATE UNIQUE INDEX IF NOT EXISTS document_number_rules_scope_code_uidx
    ON document_number_rules (
        COALESCE(owner_id, '00000000-0000-0000-0000-000000000000'::uuid),
        rule_code
    );

CREATE INDEX IF NOT EXISTS document_number_rules_effective_idx
    ON document_number_rules (document_type, owner_id, enabled, effective_from, effective_to);

CREATE TABLE IF NOT EXISTS document_number_counters (
    id             UUID PRIMARY KEY,
    rule_id        UUID NOT NULL REFERENCES document_number_rules(id) ON DELETE RESTRICT,
    counter_key    TEXT NOT NULL,
    current_value  BIGINT NOT NULL DEFAULT 0 CHECK (current_value >= 0),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    version        BIGINT NOT NULL DEFAULT 1,
    UNIQUE (rule_id, counter_key)
);

CREATE TABLE IF NOT EXISTS document_number_allocations (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    rule_id             UUID NOT NULL REFERENCES document_number_rules(id) ON DELETE RESTRICT,
    document_type       TEXT NOT NULL,
    idempotency_key     TEXT NOT NULL,
    request_hash        TEXT NOT NULL,
    generated_no        TEXT NOT NULL,
    sequence_value      BIGINT NOT NULL CHECK (sequence_value > 0),
    counter_key         TEXT NOT NULL,
    source_module       TEXT NOT NULL,
    source_document_id  UUID,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, idempotency_key),
    UNIQUE (generated_no)
);

CREATE INDEX IF NOT EXISTS document_number_allocations_lookup_idx
    ON document_number_allocations (owner_id, document_type, created_at DESC);

GRANT SELECT, INSERT, UPDATE ON
    document_number_rules,
    document_number_counters
TO wms_app;

GRANT SELECT, INSERT ON document_number_allocations TO wms_app;
