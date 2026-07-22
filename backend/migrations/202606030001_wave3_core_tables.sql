-- Wave 3 core business tables, aligned with ADR-0034.

CREATE TABLE IF NOT EXISTS idempotency_request (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL,
    idempotency_key  TEXT NOT NULL,
    request_hash     TEXT NOT NULL,
    method           TEXT NOT NULL,
    path             TEXT NOT NULL,
    status_code      INT NOT NULL,
    response_body    JSONB NOT NULL,
    resource_type    TEXT NOT NULL,
    resource_id      TEXT NOT NULL,
    expires_at       TIMESTAMPTZ NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idempotency_request_expires_at_idx
    ON idempotency_request (expires_at);

CREATE TABLE IF NOT EXISTS receiving_orders (
    id                   UUID PRIMARY KEY,
    owner_id             UUID NOT NULL,
    receipt_no           TEXT NOT NULL,
    document_type        TEXT NOT NULL,
    supplier_id          UUID,
    warehouse_id         UUID NOT NULL,
    external_ref         TEXT,
    status               TEXT NOT NULL,
    expected_arrival_at  TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    version              BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, receipt_no)
);

CREATE INDEX IF NOT EXISTS receiving_orders_owner_status_idx
    ON receiving_orders (owner_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS receiving_order_lines (
    id                  UUID PRIMARY KEY,
    receiving_order_id  UUID NOT NULL REFERENCES receiving_orders(id) ON DELETE CASCADE,
    owner_id            UUID NOT NULL,
    line_no             INT NOT NULL CHECK (line_no > 0),
    product_id          UUID,
    product_code        TEXT NOT NULL,
    expected_qty        BIGINT NOT NULL CHECK (expected_qty > 0),
    batch_no            TEXT,
    production_date     DATE,
    expiry_date         DATE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (receiving_order_id, line_no)
);

CREATE INDEX IF NOT EXISTS receiving_order_lines_owner_product_idx
    ON receiving_order_lines (owner_id, product_code);

CREATE TABLE IF NOT EXISTS receiving_order_receipts (
    id                           UUID PRIMARY KEY,
    receiving_order_id           UUID NOT NULL REFERENCES receiving_orders(id) ON DELETE CASCADE,
    owner_id                     UUID NOT NULL,
    actual_qty                   BIGINT NOT NULL CHECK (actual_qty >= 0),
    shortage_qty                 BIGINT NOT NULL CHECK (shortage_qty >= 0),
    rejected_qty                 BIGINT NOT NULL CHECK (rejected_qty >= 0),
    arrival_temperature_celsius  DOUBLE PRECISION,
    exception_note               TEXT,
    occurred_at                  TIMESTAMPTZ NOT NULL,
    created_at                   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (receiving_order_id)
);

CREATE INDEX IF NOT EXISTS receiving_order_receipts_owner_occurred_idx
    ON receiving_order_receipts (owner_id, occurred_at DESC);

CREATE TABLE IF NOT EXISTS receiving_inspections (
    id                  UUID PRIMARY KEY,
    receiving_order_id  UUID NOT NULL REFERENCES receiving_orders(id) ON DELETE CASCADE,
    owner_id            UUID NOT NULL,
    batch_no            TEXT NOT NULL,
    accepted_qty        BIGINT NOT NULL CHECK (accepted_qty >= 0),
    rejected_qty        BIGINT NOT NULL CHECK (rejected_qty >= 0),
    production_date     DATE NOT NULL,
    expiry_date         DATE NOT NULL,
    quality_status      TEXT NOT NULL,
    trace_codes         TEXT[] NOT NULL DEFAULT '{}',
    occurred_at         TIMESTAMPTZ NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS receiving_inspections_owner_batch_idx
    ON receiving_inspections (owner_id, batch_no, occurred_at DESC);

CREATE TABLE IF NOT EXISTS receiving_inspection_signatures (
    id                  UUID PRIMARY KEY,
    receiving_order_id  UUID NOT NULL REFERENCES receiving_orders(id) ON DELETE CASCADE,
    owner_id            UUID NOT NULL,
    dual_required       BOOLEAN NOT NULL,
    first_signer_id     UUID NOT NULL,
    second_signer_id    UUID,
    strategy_rule_id    UUID,
    signed_at           TIMESTAMPTZ NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (second_signer_id IS NULL OR second_signer_id <> first_signer_id)
);

CREATE INDEX IF NOT EXISTS receiving_inspection_signatures_owner_signed_idx
    ON receiving_inspection_signatures (owner_id, signed_at DESC);

CREATE TABLE IF NOT EXISTS receiving_putaways (
    id                  UUID PRIMARY KEY,
    receiving_order_id  UUID NOT NULL REFERENCES receiving_orders(id) ON DELETE CASCADE,
    owner_id            UUID NOT NULL,
    batch_no            TEXT NOT NULL,
    product_code        TEXT NOT NULL,
    qty                 BIGINT NOT NULL CHECK (qty > 0),
    location_id         UUID NOT NULL,
    location_code       TEXT NOT NULL,
    quality_status      TEXT NOT NULL,
    occurred_at         TIMESTAMPTZ NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS receiving_putaways_owner_batch_idx
    ON receiving_putaways (owner_id, product_code, batch_no, occurred_at DESC);

CREATE TABLE IF NOT EXISTS inventory_batches (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL,
    product_code     TEXT NOT NULL,
    batch_no         TEXT NOT NULL,
    production_date  DATE NOT NULL,
    expiry_date      DATE NOT NULL,
    qty_on_hand      BIGINT NOT NULL CHECK (qty_on_hand >= 0),
    qty_locked       BIGINT NOT NULL DEFAULT 0 CHECK (qty_locked >= 0),
    quality_status   TEXT NOT NULL,
    location_id      UUID NOT NULL,
    location_code    TEXT NOT NULL,
    recall_flag      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    version          BIGINT NOT NULL DEFAULT 1,
    CHECK (qty_locked <= qty_on_hand),
    UNIQUE (owner_id, product_code, batch_no, location_id, quality_status)
);

CREATE INDEX IF NOT EXISTS inventory_batches_owner_product_batch_idx
    ON inventory_batches (owner_id, product_code, batch_no);

CREATE INDEX IF NOT EXISTS inventory_batches_owner_location_status_idx
    ON inventory_batches (owner_id, location_id, quality_status);

CREATE INDEX IF NOT EXISTS inventory_batches_owner_expiry_idx
    ON inventory_batches (owner_id, expiry_date);

CREATE TABLE IF NOT EXISTS inventory_movements (
    id                    UUID PRIMARY KEY,
    owner_id              UUID NOT NULL,
    batch_id              UUID NOT NULL REFERENCES inventory_batches(id),
    movement_type         TEXT NOT NULL,
    qty_delta             BIGINT NOT NULL,
    source_document_type  TEXT NOT NULL,
    source_document_id    UUID NOT NULL,
    location_code         TEXT,
    from_location_code    TEXT,
    to_location_code      TEXT,
    lpn_code              TEXT,
    operator_user_id      UUID,
    operator_name         TEXT,
    volume_delta_cm3      BIGINT,
    occurred_at           TIMESTAMPTZ NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS inventory_movements_owner_batch_idx
    ON inventory_movements (owner_id, batch_id, occurred_at DESC);

CREATE INDEX IF NOT EXISTS inventory_movements_owner_location_occurred_idx
    ON inventory_movements (owner_id, location_code, occurred_at DESC)
    WHERE location_code IS NOT NULL;

CREATE INDEX IF NOT EXISTS inventory_movements_owner_from_location_occurred_idx
    ON inventory_movements (owner_id, from_location_code, occurred_at DESC)
    WHERE from_location_code IS NOT NULL;

CREATE INDEX IF NOT EXISTS inventory_movements_owner_to_location_occurred_idx
    ON inventory_movements (owner_id, to_location_code, occurred_at DESC)
    WHERE to_location_code IS NOT NULL;

CREATE TABLE IF NOT EXISTS inventory_status_changes (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL,
    batch_id         UUID NOT NULL REFERENCES inventory_batches(id),
    from_status      TEXT NOT NULL,
    to_status        TEXT NOT NULL,
    reason           TEXT NOT NULL,
    approval_source  TEXT NOT NULL,
    approval_id      TEXT NOT NULL,
    occurred_at      TIMESTAMPTZ NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS inventory_status_changes_owner_batch_idx
    ON inventory_status_changes (owner_id, batch_id, occurred_at DESC);

CREATE TABLE IF NOT EXISTS cold_chain_devices (
    id                           UUID PRIMARY KEY,
    owner_id                     UUID NOT NULL,
    device_code                  TEXT NOT NULL,
    device_type                  TEXT NOT NULL,
    installed_at_location_code   TEXT,
    calibration_due_at           TIMESTAMPTZ,
    status                       TEXT NOT NULL,
    created_at                   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                   TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                      BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, device_code)
);

CREATE TABLE IF NOT EXISTS temperature_readings (
    id                       UUID PRIMARY KEY,
    owner_id                 UUID NOT NULL,
    device_code              TEXT NOT NULL,
    temperature_celsius      DOUBLE PRECISION NOT NULL,
    humidity_percent         DOUBLE PRECISION,
    captured_at              TIMESTAMPTZ NOT NULL,
    external_report_url      TEXT,
    out_of_range             BOOLEAN NOT NULL,
    source_system            TEXT,
    external_reading_id      TEXT,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, device_code, captured_at)
);

CREATE INDEX IF NOT EXISTS temperature_readings_owner_device_captured_idx
    ON temperature_readings (owner_id, device_code, captured_at DESC);

CREATE TABLE IF NOT EXISTS temperature_excursion_events (
    id                           UUID PRIMARY KEY,
    owner_id                     UUID NOT NULL,
    external_event_id            TEXT NOT NULL,
    device_code                  TEXT NOT NULL,
    location_code                TEXT,
    started_at                   TIMESTAMPTZ NOT NULL,
    ended_at                     TIMESTAMPTZ,
    min_temperature_celsius      DOUBLE PRECISION,
    max_temperature_celsius      DOUBLE PRECISION,
    affected_batch_ids           UUID[] NOT NULL DEFAULT '{}',
    status                       TEXT NOT NULL,
    created_at                   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, external_event_id)
);

CREATE INDEX IF NOT EXISTS temperature_excursion_events_owner_status_idx
    ON temperature_excursion_events (owner_id, status, created_at DESC);

CREATE TABLE IF NOT EXISTS billing_accounts (
    id             UUID PRIMARY KEY,
    owner_id       UUID NOT NULL,
    account_code   TEXT NOT NULL,
    account_name   TEXT NOT NULL,
    status         TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    version        BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, account_code)
);

CREATE TABLE IF NOT EXISTS billing_contracts (
    id           UUID PRIMARY KEY,
    owner_id     UUID NOT NULL,
    account_id   UUID NOT NULL REFERENCES billing_accounts(id),
    contract_no  TEXT NOT NULL,
    valid_from   DATE NOT NULL,
    valid_to     DATE NOT NULL,
    status       TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    version      BIGINT NOT NULL DEFAULT 1,
    CHECK (valid_to >= valid_from),
    UNIQUE (owner_id, contract_no)
);

CREATE INDEX IF NOT EXISTS billing_contracts_owner_account_idx
    ON billing_contracts (owner_id, account_id);

CREATE TABLE IF NOT EXISTS billing_rules (
    id                UUID PRIMARY KEY,
    owner_id          UUID NOT NULL,
    contract_id       UUID NOT NULL REFERENCES billing_contracts(id),
    charge_item       TEXT NOT NULL,
    unit              TEXT NOT NULL,
    unit_price_cents  BIGINT NOT NULL CHECK (unit_price_cents >= 0),
    billing_cycle     TEXT NOT NULL,
    effective_from    DATE NOT NULL,
    effective_to      DATE NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (effective_to >= effective_from)
);

CREATE INDEX IF NOT EXISTS billing_rules_owner_contract_idx
    ON billing_rules (owner_id, contract_id, charge_item, unit, billing_cycle);

CREATE INDEX IF NOT EXISTS billing_rules_owner_effective_idx
    ON billing_rules (owner_id, effective_from, effective_to);

GRANT SELECT, INSERT, UPDATE, DELETE ON
    idempotency_request,
    receiving_orders,
    receiving_order_lines,
    receiving_order_receipts,
    receiving_inspections,
    receiving_inspection_signatures,
    receiving_putaways,
    inventory_batches,
    inventory_movements,
    inventory_status_changes,
    cold_chain_devices,
    temperature_readings,
    temperature_excursion_events,
    billing_accounts,
    billing_contracts,
    billing_rules
TO wms_app;
