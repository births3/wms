-- H2 audit archive, event bus and business retention baseline.

CREATE TABLE IF NOT EXISTS audit_archive_partition_state (
    partition_name  TEXT PRIMARY KEY,
    partition_start DATE NOT NULL,
    partition_end   DATE NOT NULL,
    storage_tier    TEXT NOT NULL CHECK (storage_tier IN ('online', 'archive', 'deep_archive')),
    target_tier     TEXT NOT NULL CHECK (target_tier IN ('online', 'archive', 'deep_archive')),
    archived_at     TIMESTAMPTZ,
    last_run_id     UUID,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS audit_archive_partition_state_tier_idx
    ON audit_archive_partition_state (target_tier, partition_start);

CREATE TABLE IF NOT EXISTS audit_archive_run (
    id                     UUID PRIMARY KEY,
    owner_id               UUID NOT NULL,
    idempotency_key        TEXT NOT NULL,
    reference_date         DATE NOT NULL,
    online_quarters        INT NOT NULL CHECK (online_quarters > 0),
    retention_years        INT NOT NULL CHECK (retention_years >= 5),
    partitions_seen        INT NOT NULL CHECK (partitions_seen >= 0),
    partitions_archived    INT NOT NULL CHECK (partitions_archived >= 0),
    status                 TEXT NOT NULL CHECK (status IN ('completed', 'failed')),
    created_at             TIMESTAMPTZ NOT NULL,
    UNIQUE (owner_id, idempotency_key)
);

CREATE TABLE IF NOT EXISTS event_bus_subscription (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL,
    subscriber_key  TEXT NOT NULL,
    event_pattern   TEXT NOT NULL,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, subscriber_key)
);

CREATE INDEX IF NOT EXISTS event_bus_subscription_owner_active_idx
    ON event_bus_subscription (owner_id, active);

CREATE TABLE IF NOT EXISTS event_bus_event (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL,
    idempotency_key  TEXT NOT NULL,
    event_type       TEXT NOT NULL,
    source_module    TEXT NOT NULL,
    resource_type    TEXT NOT NULL,
    resource_id      TEXT NOT NULL,
    payload          JSONB NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    UNIQUE (owner_id, idempotency_key)
);

CREATE INDEX IF NOT EXISTS event_bus_event_owner_type_idx
    ON event_bus_event (owner_id, event_type, created_at DESC);

CREATE TABLE IF NOT EXISTS event_bus_delivery (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL,
    event_id         UUID NOT NULL REFERENCES event_bus_event(id) ON DELETE CASCADE,
    subscription_id  UUID NOT NULL REFERENCES event_bus_subscription(id) ON DELETE CASCADE,
    status           TEXT NOT NULL CHECK (status IN ('pending', 'delivered', 'dead_letter')),
    attempt_count    INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    last_error       TEXT,
    next_attempt_at  TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (event_id, subscription_id)
);

CREATE INDEX IF NOT EXISTS event_bus_delivery_owner_status_idx
    ON event_bus_delivery (owner_id, status, next_attempt_at);

CREATE TABLE IF NOT EXISTS event_bus_dead_letter (
    id           UUID PRIMARY KEY,
    owner_id     UUID NOT NULL,
    delivery_id  UUID NOT NULL REFERENCES event_bus_delivery(id) ON DELETE CASCADE,
    event_id     UUID NOT NULL REFERENCES event_bus_event(id) ON DELETE CASCADE,
    reason       TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL,
    UNIQUE (delivery_id)
);

CREATE TABLE IF NOT EXISTS business_retention_policy (
    id                       UUID PRIMARY KEY,
    owner_id                 UUID NOT NULL,
    policy_code              TEXT NOT NULL,
    policy_name              TEXT NOT NULL,
    retention_years          INT,
    online_retention_months  INT NOT NULL CHECK (online_retention_months > 0),
    permanent                BOOLEAN NOT NULL DEFAULT FALSE,
    special_drug             BOOLEAN NOT NULL DEFAULT FALSE,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (permanent OR retention_years IS NOT NULL),
    UNIQUE (owner_id, policy_code)
);

CREATE TABLE IF NOT EXISTS business_archive_job (
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL,
    idempotency_key    TEXT NOT NULL,
    policy_id          UUID NOT NULL REFERENCES business_retention_policy(id),
    table_name         TEXT NOT NULL,
    target_layer       TEXT NOT NULL CHECK (target_layer IN ('archive', 'deep_archive', 'skip')),
    cutoff_date        DATE,
    status             TEXT NOT NULL CHECK (status IN ('planned', 'skipped')),
    delete_allowed     BOOLEAN NOT NULL DEFAULT FALSE,
    skip_reason        TEXT,
    created_at         TIMESTAMPTZ NOT NULL,
    UNIQUE (owner_id, idempotency_key)
);

CREATE INDEX IF NOT EXISTS business_archive_job_owner_status_idx
    ON business_archive_job (owner_id, status, created_at DESC);
