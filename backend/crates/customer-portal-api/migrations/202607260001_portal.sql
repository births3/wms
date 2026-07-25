-- US-DI-003：独立客户药检单平台查询库、认证、投影、下载、导出与审计。

CREATE TABLE portal_customers (
    id              UUID PRIMARY KEY,
    customer_code   TEXT NOT NULL UNIQUE,
    customer_name   TEXT NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL
);

CREATE TABLE portal_customer_addresses (
    id              UUID PRIMARY KEY,
    customer_id     UUID NOT NULL REFERENCES portal_customers(id),
    address_code    TEXT NOT NULL,
    address_name    TEXT NOT NULL,
    address_snapshot JSONB NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL,
    UNIQUE (customer_id, address_code)
);

CREATE TABLE portal_users (
    id                      UUID PRIMARY KEY,
    customer_id             UUID NOT NULL REFERENCES portal_customers(id),
    username                TEXT NOT NULL UNIQUE,
    display_name            TEXT NOT NULL,
    password_hash           TEXT NOT NULL,
    role                    TEXT NOT NULL CHECK (role IN ('customer_admin', 'customer_user')),
    status                  TEXT NOT NULL CHECK (status IN ('active', 'disabled', 'locked')),
    can_view_report_history BOOLEAN NOT NULL DEFAULT FALSE,
    failed_login_count      INT NOT NULL DEFAULT 0 CHECK (failed_login_count >= 0),
    locked_until            TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE portal_user_addresses (
    user_id         UUID NOT NULL REFERENCES portal_users(id) ON DELETE CASCADE,
    address_id      UUID NOT NULL REFERENCES portal_customer_addresses(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, address_id)
);

CREATE TABLE portal_orders (
    id                  UUID PRIMARY KEY,
    customer_id         UUID NOT NULL REFERENCES portal_customers(id),
    order_no            TEXT NOT NULL,
    status              TEXT NOT NULL CHECK (status IN ('shipped', 'signed')),
    delivery_address_id UUID NOT NULL REFERENCES portal_customer_addresses(id),
    address_snapshot    JSONB NOT NULL,
    shipped_at          TIMESTAMPTZ NOT NULL,
    signed_at           TIMESTAMPTZ,
    updated_at          TIMESTAMPTZ NOT NULL,
    UNIQUE (customer_id, order_no)
);

CREATE INDEX portal_orders_scope_idx
    ON portal_orders (customer_id, delivery_address_id, shipped_at DESC);

CREATE TABLE portal_order_lines (
    id              UUID PRIMARY KEY,
    order_id        UUID NOT NULL REFERENCES portal_orders(id) ON DELETE CASCADE,
    product_id      UUID NOT NULL,
    product_code    TEXT NOT NULL,
    product_name    TEXT NOT NULL,
    batch_no        TEXT NOT NULL,
    quantity        NUMERIC(18, 4) NOT NULL CHECK (quantity > 0),
    UNIQUE (order_id, product_id, batch_no)
);

CREATE TABLE portal_report_versions (
    id                   UUID PRIMARY KEY,
    report_id            UUID NOT NULL,
    owner_id             UUID NOT NULL,
    product_id           UUID NOT NULL,
    batch_no             TEXT NOT NULL,
    version_number       INT NOT NULL CHECK (version_number > 0),
    report_no            TEXT NOT NULL,
    status               TEXT NOT NULL CHECK (status IN ('confirmed', 'superseded')),
    is_current           BOOLEAN NOT NULL,
    modification_reason  TEXT,
    customer_copy_status TEXT NOT NULL CHECK (
        customer_copy_status IN ('queued', 'processing', 'available', 'failed')
    ),
    customer_copy_storage_key TEXT,
    customer_copy_file_name TEXT,
    customer_copy_size   BIGINT CHECK (customer_copy_size IS NULL OR customer_copy_size > 0),
    customer_copy_hash   TEXT,
    digitally_signed_original BOOLEAN NOT NULL DEFAULT FALSE,
    confirmed_at         TIMESTAMPTZ NOT NULL,
    updated_at           TIMESTAMPTZ NOT NULL,
    UNIQUE (report_id, version_number),
    CHECK (
        customer_copy_status <> 'available'
        OR (
            customer_copy_storage_key IS NOT NULL
            AND customer_copy_file_name IS NOT NULL
            AND customer_copy_size IS NOT NULL
            AND customer_copy_hash IS NOT NULL
        )
    )
);

CREATE UNIQUE INDEX portal_report_one_current_idx
    ON portal_report_versions (report_id)
    WHERE is_current;

CREATE INDEX portal_report_lookup_idx
    ON portal_report_versions (product_id, batch_no, is_current, version_number DESC);

CREATE TABLE portal_projection_events (
    event_id         UUID PRIMARY KEY,
    event_type       TEXT NOT NULL,
    occurred_at      TIMESTAMPTZ NOT NULL,
    payload          JSONB NOT NULL,
    status           TEXT NOT NULL CHECK (status IN ('processing', 'succeeded', 'failed', 'dead_letter')),
    attempt_count    INT NOT NULL DEFAULT 1 CHECK (attempt_count > 0),
    last_error       TEXT,
    processed_at     TIMESTAMPTZ,
    next_attempt_at  TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX portal_projection_retry_idx
    ON portal_projection_events (status, next_attempt_at)
    WHERE status IN ('failed', 'dead_letter');

CREATE TABLE portal_download_sessions (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES portal_users(id),
    resource_type   TEXT NOT NULL CHECK (resource_type IN ('report', 'export')),
    resource_id     UUID NOT NULL,
    storage_key     TEXT NOT NULL,
    file_name       TEXT NOT NULL,
    token_hash      TEXT NOT NULL UNIQUE,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE portal_export_jobs (
    id                  UUID PRIMARY KEY,
    customer_id         UUID NOT NULL REFERENCES portal_customers(id),
    created_by          UUID NOT NULL REFERENCES portal_users(id),
    include_history     BOOLEAN NOT NULL DEFAULT FALSE,
    status              TEXT NOT NULL CHECK (
        status IN ('queued', 'processing', 'completed', 'failed')
    ),
    requested_order_count INT NOT NULL CHECK (requested_order_count > 0),
    report_file_count   INT NOT NULL DEFAULT 0 CHECK (report_file_count >= 0),
    missing_count       INT NOT NULL DEFAULT 0 CHECK (missing_count >= 0),
    total_size          BIGINT NOT NULL DEFAULT 0 CHECK (total_size >= 0),
    result_storage_key  TEXT,
    result_file_name    TEXT,
    last_error          TEXT,
    expires_at          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at          TIMESTAMPTZ,
    finished_at         TIMESTAMPTZ,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX portal_export_jobs_user_idx
    ON portal_export_jobs (created_by, created_at DESC);

CREATE TABLE portal_export_job_orders (
    export_job_id   UUID NOT NULL REFERENCES portal_export_jobs(id) ON DELETE CASCADE,
    order_id        UUID NOT NULL REFERENCES portal_orders(id),
    PRIMARY KEY (export_job_id, order_id)
);

CREATE TABLE portal_audit_events (
    id              UUID PRIMARY KEY,
    occurred_at     TIMESTAMPTZ NOT NULL,
    user_id         UUID REFERENCES portal_users(id),
    customer_id     UUID,
    action          TEXT NOT NULL,
    resource_type   TEXT NOT NULL,
    resource_id     TEXT NOT NULL,
    detail          JSONB NOT NULL,
    request_id      UUID
);

CREATE INDEX portal_audit_events_scope_idx
    ON portal_audit_events (customer_id, occurred_at DESC);

CREATE OR REPLACE FUNCTION portal_audit_append_only_guard()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'portal_audit_events is append-only';
END;
$$;

CREATE TRIGGER portal_audit_events_append_only
BEFORE UPDATE OR DELETE ON portal_audit_events
FOR EACH ROW EXECUTE FUNCTION portal_audit_append_only_guard();
