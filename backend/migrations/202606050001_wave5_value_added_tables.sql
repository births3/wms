-- Wave 5 value-added modules: M-PK, M8, M9, M10.

CREATE TABLE IF NOT EXISTS packing_stations (
    id                UUID PRIMARY KEY,
    owner_id          UUID NOT NULL,
    station_code      TEXT NOT NULL,
    station_name      TEXT NOT NULL,
    printer_code      TEXT,
    scale_code        TEXT,
    temperature_zone  TEXT NOT NULL,
    status            TEXT NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    version           BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, station_code)
);

CREATE TABLE IF NOT EXISTS packing_jobs (
    id                    UUID PRIMARY KEY,
    owner_id              UUID NOT NULL,
    outbound_order_id     UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE,
    station_id            UUID REFERENCES packing_stations(id),
    job_no                TEXT NOT NULL,
    pack_mode             TEXT NOT NULL,
    recommended_box_type  TEXT NOT NULL,
    actual_box_type       TEXT NOT NULL,
    adjustment_reason     TEXT,
    outbound_lpn          TEXT NOT NULL,
    trace_codes           TEXT[] NOT NULL,
    status                TEXT NOT NULL,
    weight_grams          BIGINT,
    waybill_no            TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    version               BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, job_no),
    UNIQUE (owner_id, outbound_lpn),
    CHECK (array_length(trace_codes, 1) > 0)
);

CREATE INDEX IF NOT EXISTS packing_jobs_owner_status_idx
    ON packing_jobs (owner_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS retail_replenishment_suggestions (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL,
    store_id         UUID NOT NULL,
    product_code     TEXT NOT NULL,
    period_key       TEXT NOT NULL,
    min_qty          BIGINT NOT NULL CHECK (min_qty >= 0),
    max_qty          BIGINT NOT NULL CHECK (max_qty >= min_qty),
    current_qty      BIGINT NOT NULL CHECK (current_qty >= 0),
    in_transit_qty   BIGINT NOT NULL CHECK (in_transit_qty >= 0),
    daily_sales_avg  BIGINT NOT NULL CHECK (daily_sales_avg >= 0),
    suggested_qty    BIGINT NOT NULL CHECK (suggested_qty >= 0),
    status           TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, store_id, product_code, period_key)
);

CREATE TABLE IF NOT EXISTS crossdock_plans (
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL,
    asn_id             UUID NOT NULL,
    outbound_order_id  UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE,
    store_id           UUID NOT NULL,
    product_code       TEXT NOT NULL,
    qty                BIGINT NOT NULL CHECK (qty > 0),
    status             TEXT NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS crossdock_plans_owner_store_idx
    ON crossdock_plans (owner_id, store_id, created_at DESC);

CREATE TABLE IF NOT EXISTS billing_charge_calculations (
    id             UUID PRIMARY KEY,
    owner_id       UUID NOT NULL,
    contract_id    UUID NOT NULL REFERENCES billing_contracts(id) ON DELETE CASCADE,
    period_start   TEXT NOT NULL,
    period_end     TEXT NOT NULL,
    charge_item    TEXT NOT NULL,
    quantity       BIGINT NOT NULL CHECK (quantity >= 0),
    amount_cents   BIGINT NOT NULL CHECK (amount_cents >= 0),
    source_refs    TEXT[] NOT NULL,
    status         TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, contract_id, period_start, period_end, charge_item)
);

CREATE TABLE IF NOT EXISTS billing_statements (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL,
    contract_id         UUID NOT NULL REFERENCES billing_contracts(id) ON DELETE CASCADE,
    period_start        TEXT NOT NULL,
    period_end          TEXT NOT NULL,
    status              TEXT NOT NULL,
    total_amount_cents  BIGINT NOT NULL CHECK (total_amount_cents >= 0),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    version             BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, contract_id, period_start, period_end)
);

CREATE TABLE IF NOT EXISTS billing_statement_charges (
    id            UUID PRIMARY KEY,
    owner_id      UUID NOT NULL,
    statement_id  UUID NOT NULL REFERENCES billing_statements(id) ON DELETE CASCADE,
    charge_id     UUID NOT NULL REFERENCES billing_charge_calculations(id) ON DELETE RESTRICT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (statement_id, charge_id)
);

CREATE TABLE IF NOT EXISTS tms_dispatches (
    id                      UUID PRIMARY KEY,
    owner_id                UUID NOT NULL,
    dispatch_no             TEXT NOT NULL,
    outbound_order_id        UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE CASCADE,
    delivery_provider_type  TEXT NOT NULL,
    vehicle_no              TEXT,
    plate_no                TEXT,
    driver_user_id          UUID,
    carrier_code            TEXT,
    waybill_no              TEXT,
    status                  TEXT NOT NULL,
    dispatch_version        INT NOT NULL CHECK (dispatch_version > 0),
    scheduled_load_at       TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                 BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, dispatch_no)
);

CREATE INDEX IF NOT EXISTS tms_dispatches_owner_order_idx
    ON tms_dispatches (owner_id, outbound_order_id);

CREATE TABLE IF NOT EXISTS transit_temperature_readings (
    id                    UUID PRIMARY KEY,
    owner_id              UUID NOT NULL,
    dispatch_id           UUID NOT NULL REFERENCES tms_dispatches(id) ON DELETE CASCADE,
    device_code           TEXT NOT NULL,
    plate_no              TEXT NOT NULL,
    measured_at           TIMESTAMPTZ NOT NULL,
    temperature_celsius   DOUBLE PRECISION NOT NULL,
    humidity_percent      DOUBLE PRECISION,
    is_exceeded           BOOLEAN NOT NULL DEFAULT FALSE,
    external_trace_url    TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, dispatch_id, device_code, measured_at)
);

CREATE TABLE IF NOT EXISTS container_recoveries (
    id                      UUID PRIMARY KEY,
    owner_id                UUID NOT NULL,
    container_lpn           TEXT NOT NULL,
    dispatch_id             UUID REFERENCES tms_dispatches(id) ON DELETE SET NULL,
    customer_id             UUID NOT NULL,
    delivery_provider_type  TEXT NOT NULL,
    status                  TEXT NOT NULL,
    shipped_at              TIMESTAMPTZ NOT NULL,
    recovered_at            TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                 BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, container_lpn, shipped_at)
);
