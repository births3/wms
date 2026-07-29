-- Wave 4 M4 outbound closure tables.

CREATE TABLE IF NOT EXISTS outbound_orders (
    id                UUID PRIMARY KEY,
    owner_id          UUID NOT NULL,
    wms_order_no      TEXT NOT NULL,
    erp_order_no      TEXT,
    customer_id       UUID NOT NULL,
    delivery_address_id UUID NOT NULL,
    delivery_address_snapshot JSONB NOT NULL
        CHECK (jsonb_typeof(delivery_address_snapshot) = 'object'),
    warehouse_id      UUID NOT NULL,
    required_ship_at  TIMESTAMPTZ,
    status            TEXT NOT NULL,
    short_pick        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    version           BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, wms_order_no)
);

CREATE INDEX IF NOT EXISTS outbound_orders_owner_status_idx
    ON outbound_orders (owner_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS outbound_order_lines (
    id                 UUID PRIMARY KEY,
    outbound_order_id  UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE,
    owner_id           UUID NOT NULL,
    line_no            INT NOT NULL CHECK (line_no > 0),
    product_code       TEXT NOT NULL,
    batch_no           TEXT NOT NULL,
    planned_qty        BIGINT NOT NULL CHECK (planned_qty > 0),
    picked_qty         BIGINT NOT NULL DEFAULT 0 CHECK (picked_qty >= 0),
    reviewed_qty       BIGINT NOT NULL DEFAULT 0 CHECK (reviewed_qty >= 0),
    shipped_qty        BIGINT NOT NULL DEFAULT 0 CHECK (shipped_qty >= 0),
    short_pick_qty     BIGINT NOT NULL DEFAULT 0 CHECK (short_pick_qty >= 0),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (outbound_order_id, line_no),
    CHECK (picked_qty <= planned_qty),
    CHECK (reviewed_qty <= planned_qty),
    CHECK (shipped_qty <= planned_qty),
    CHECK (short_pick_qty <= planned_qty)
);

CREATE INDEX IF NOT EXISTS outbound_order_lines_owner_product_batch_idx
    ON outbound_order_lines (owner_id, product_code, batch_no);

CREATE TABLE IF NOT EXISTS outbound_waves (
    id          UUID PRIMARY KEY,
    owner_id    UUID NOT NULL,
    wave_no     TEXT NOT NULL,
    status      TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    version     BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, wave_no)
);

CREATE TABLE IF NOT EXISTS outbound_wave_orders (
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL,
    wave_id            UUID NOT NULL REFERENCES outbound_waves(id) ON DELETE CASCADE,
    outbound_order_id  UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (outbound_order_id),
    UNIQUE (wave_id, outbound_order_id)
);

CREATE TABLE IF NOT EXISTS outbound_shipments (
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL,
    outbound_order_id  UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE,
    carrier_type       TEXT NOT NULL,
    handover_to        TEXT NOT NULL,
    package_count      INT NOT NULL CHECK (package_count > 0),
    shipped_at         TIMESTAMPTZ NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (outbound_order_id)
);

CREATE INDEX IF NOT EXISTS outbound_shipments_owner_shipped_idx
    ON outbound_shipments (owner_id, shipped_at DESC);

CREATE TABLE IF NOT EXISTS traceability_outbound_reports (
    id             UUID PRIMARY KEY,
    owner_id       UUID NOT NULL,
    platform       TEXT NOT NULL,
    status         TEXT NOT NULL,
    queued_count   INT NOT NULL CHECK (queued_count > 0),
    generated_at   TIMESTAMPTZ NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    version        BIGINT NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS traceability_outbound_reports_owner_status_idx
    ON traceability_outbound_reports (owner_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS traceability_outbound_report_events (
    event_id            UUID NOT NULL,
    owner_id            UUID NOT NULL,
    report_id           UUID NOT NULL REFERENCES traceability_outbound_reports(id) ON DELETE CASCADE,
    trace_code          TEXT NOT NULL,
    status_change_type  TEXT NOT NULL,
    occurred_at         TIMESTAMPTZ NOT NULL,
    report_status       TEXT NOT NULL DEFAULT 'queued',
    retry_count         INT NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
    last_error_code     TEXT,
    platform_receipt_id TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, event_id)
);

CREATE INDEX IF NOT EXISTS traceability_outbound_report_events_owner_status_idx
    ON traceability_outbound_report_events (owner_id, report_status, updated_at DESC);

CREATE INDEX IF NOT EXISTS traceability_outbound_report_events_trace_code_idx
    ON traceability_outbound_report_events (owner_id, trace_code);
