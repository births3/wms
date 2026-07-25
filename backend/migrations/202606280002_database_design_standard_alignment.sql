-- Align PostgreSQL schema with database design standards.

CREATE TABLE IF NOT EXISTS products (
    id                     UUID PRIMARY KEY,
    owner_id               UUID NOT NULL,
    product_code           TEXT NOT NULL,
    product_name           TEXT NOT NULL,
    specification          TEXT NOT NULL,
    dosage_form            TEXT,
    storage_condition      TEXT,
    special_drug_category  TEXT,
    approval_no            TEXT,
    manufacturer           TEXT,
    status                 TEXT NOT NULL DEFAULT 'active',
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                BIGINT NOT NULL DEFAULT 1,
    CHECK (storage_condition IS NULL OR storage_condition IN ('frozen', 'cold', 'cool', 'normal')),
    CHECK (status IN ('active', 'disabled', 'pending_mapping')),
    CHECK (
        status = 'pending_mapping'
        OR (storage_condition IS NOT NULL AND special_drug_category IS NOT NULL)
    ),
    UNIQUE (owner_id, product_code)
);

CREATE INDEX IF NOT EXISTS products_owner_status_idx
    ON products (owner_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS suppliers (
    id             UUID PRIMARY KEY,
    owner_id       UUID NOT NULL,
    supplier_code  TEXT NOT NULL,
    supplier_name  TEXT NOT NULL,
    uscc           TEXT NOT NULL,
    contact_name   TEXT,
    contact_phone  TEXT,
    status         TEXT NOT NULL DEFAULT 'active',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    version        BIGINT NOT NULL DEFAULT 1,
    CHECK (status IN ('active', 'disabled')),
    UNIQUE (owner_id, supplier_code),
    UNIQUE (owner_id, uscc)
);

CREATE INDEX IF NOT EXISTS suppliers_owner_status_idx
    ON suppliers (owner_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS customers (
    id             UUID PRIMARY KEY,
    owner_id       UUID NOT NULL,
    customer_code  TEXT NOT NULL,
    customer_name  TEXT NOT NULL,
    customer_type  TEXT NOT NULL DEFAULT 'customer',
    contact_name   TEXT,
    contact_phone  TEXT,
    status         TEXT NOT NULL DEFAULT 'active',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    version        BIGINT NOT NULL DEFAULT 1,
    CHECK (customer_type IN ('customer', 'store')),
    CHECK (status IN ('active', 'disabled')),
    UNIQUE (owner_id, customer_code)
);

CREATE INDEX IF NOT EXISTS customers_owner_status_idx
    ON customers (owner_id, status, updated_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS customers_owner_id_uidx
    ON customers (owner_id, id);

CREATE TABLE IF NOT EXISTS customer_addresses (
    id                UUID PRIMARY KEY,
    owner_id          UUID NOT NULL,
    customer_id       UUID NOT NULL,
    province          TEXT NOT NULL,
    city              TEXT NOT NULL,
    district          TEXT NOT NULL,
    detail_address    TEXT NOT NULL,
    contact_name      TEXT NOT NULL,
    contact_phone     TEXT NOT NULL,
    is_default        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    version           BIGINT NOT NULL DEFAULT 1,
    FOREIGN KEY (owner_id, customer_id) REFERENCES customers(owner_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS customer_addresses_owner_customer_idx
    ON customer_addresses (owner_id, customer_id);

CREATE TABLE IF NOT EXISTS warehouses (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL,
    warehouse_code  TEXT NOT NULL,
    warehouse_name  TEXT NOT NULL,
    warehouse_type  TEXT NOT NULL,
    address         TEXT,
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    version         BIGINT NOT NULL DEFAULT 1,
    CHECK (status IN ('active', 'disabled')),
    UNIQUE (owner_id, warehouse_code)
);

CREATE INDEX IF NOT EXISTS warehouses_owner_status_idx
    ON warehouses (owner_id, status, updated_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS warehouses_owner_id_uidx
    ON warehouses (owner_id, id);

CREATE TABLE IF NOT EXISTS warehouse_zones (
    id                UUID PRIMARY KEY,
    owner_id          UUID NOT NULL,
    warehouse_id      UUID NOT NULL,
    zone_code         TEXT NOT NULL,
    zone_name         TEXT NOT NULL,
    temperature_zone  TEXT NOT NULL,
    quality_color     TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'active',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    version           BIGINT NOT NULL DEFAULT 1,
    CHECK (temperature_zone IN ('frozen', 'cold', 'cool', 'normal')),
    CHECK (quality_color IN ('qualified_green', 'quarantine_yellow', 'unqualified_red')),
    CHECK (status IN ('active', 'disabled')),
    UNIQUE (owner_id, warehouse_id, zone_code),
    FOREIGN KEY (owner_id, warehouse_id) REFERENCES warehouses(owner_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS warehouse_zones_owner_warehouse_idx
    ON warehouse_zones (owner_id, warehouse_id);

CREATE UNIQUE INDEX IF NOT EXISTS warehouse_zones_owner_id_uidx
    ON warehouse_zones (owner_id, id);

CREATE TABLE IF NOT EXISTS warehouse_locations (
    id                   UUID PRIMARY KEY,
    owner_id             UUID NOT NULL,
    warehouse_id         UUID NOT NULL,
    zone_id              UUID NOT NULL,
    location_code        TEXT NOT NULL,
    row_no               INT NOT NULL CHECK (row_no > 0),
    column_no            INT NOT NULL CHECK (column_no > 0),
    layer_no             INT NOT NULL CHECK (layer_no > 0),
    max_volume_cm3       BIGINT NOT NULL CHECK (max_volume_cm3 >= 0),
    used_volume_cm3      BIGINT NOT NULL DEFAULT 0 CHECK (used_volume_cm3 >= 0),
    max_sku_count        INT NOT NULL DEFAULT 1 CHECK (max_sku_count > 0),
    location_type        TEXT NOT NULL,
    bound_owner_id       UUID,
    status               TEXT NOT NULL DEFAULT 'available',
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    version              BIGINT NOT NULL DEFAULT 1,
    CHECK (used_volume_cm3 <= max_volume_cm3),
    CHECK (location_type IN ('storage', 'case_pick', 'piece_pick')),
    CHECK (status IN ('available', 'occupied', 'locked', 'disabled')),
    UNIQUE (owner_id, location_code),
    FOREIGN KEY (owner_id, warehouse_id) REFERENCES warehouses(owner_id, id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id, zone_id) REFERENCES warehouse_zones(owner_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS warehouse_locations_owner_zone_status_idx
    ON warehouse_locations (owner_id, zone_id, status);

CREATE UNIQUE INDEX IF NOT EXISTS receiving_orders_owner_id_uidx
    ON receiving_orders (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS inventory_batches_owner_id_uidx
    ON inventory_batches (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS billing_accounts_owner_id_uidx
    ON billing_accounts (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS billing_contracts_owner_id_uidx
    ON billing_contracts (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS outbound_orders_owner_id_uidx
    ON outbound_orders (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS outbound_waves_owner_id_uidx
    ON outbound_waves (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS traceability_outbound_reports_owner_id_uidx
    ON traceability_outbound_reports (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS packing_stations_owner_id_uidx
    ON packing_stations (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS billing_charge_calculations_owner_id_uidx
    ON billing_charge_calculations (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS billing_statements_owner_id_uidx
    ON billing_statements (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS tms_dispatches_owner_id_uidx
    ON tms_dispatches (owner_id, id);

ALTER TABLE receiving_order_lines
    ADD CONSTRAINT receiving_order_lines_owner_order_fk
    FOREIGN KEY (owner_id, receiving_order_id) REFERENCES receiving_orders(owner_id, id) ON DELETE CASCADE;

ALTER TABLE receiving_order_receipts
    ADD CONSTRAINT receiving_order_receipts_owner_order_fk
    FOREIGN KEY (owner_id, receiving_order_id) REFERENCES receiving_orders(owner_id, id) ON DELETE CASCADE;

ALTER TABLE receiving_inspections
    ADD CONSTRAINT receiving_inspections_owner_order_fk
    FOREIGN KEY (owner_id, receiving_order_id) REFERENCES receiving_orders(owner_id, id) ON DELETE CASCADE;

ALTER TABLE receiving_inspection_signatures
    ADD CONSTRAINT receiving_inspection_signatures_owner_order_fk
    FOREIGN KEY (owner_id, receiving_order_id) REFERENCES receiving_orders(owner_id, id) ON DELETE CASCADE;

ALTER TABLE receiving_putaways
    ADD CONSTRAINT receiving_putaways_owner_order_fk
    FOREIGN KEY (owner_id, receiving_order_id) REFERENCES receiving_orders(owner_id, id) ON DELETE CASCADE;

ALTER TABLE inventory_movements
    ADD CONSTRAINT inventory_movements_owner_batch_fk
    FOREIGN KEY (owner_id, batch_id) REFERENCES inventory_batches(owner_id, id);

ALTER TABLE inventory_status_changes
    ADD CONSTRAINT inventory_status_changes_owner_batch_fk
    FOREIGN KEY (owner_id, batch_id) REFERENCES inventory_batches(owner_id, id);

ALTER TABLE billing_contracts
    ADD CONSTRAINT billing_contracts_owner_account_fk
    FOREIGN KEY (owner_id, account_id) REFERENCES billing_accounts(owner_id, id);

ALTER TABLE billing_rules
    ADD CONSTRAINT billing_rules_owner_contract_fk
    FOREIGN KEY (owner_id, contract_id) REFERENCES billing_contracts(owner_id, id);

ALTER TABLE outbound_order_lines
    ADD CONSTRAINT outbound_order_lines_owner_order_fk
    FOREIGN KEY (owner_id, outbound_order_id) REFERENCES outbound_orders(owner_id, id) ON DELETE CASCADE;

ALTER TABLE outbound_wave_orders
    ADD CONSTRAINT outbound_wave_orders_owner_wave_fk
    FOREIGN KEY (owner_id, wave_id) REFERENCES outbound_waves(owner_id, id) ON DELETE CASCADE;

ALTER TABLE outbound_wave_orders
    ADD CONSTRAINT outbound_wave_orders_owner_order_fk
    FOREIGN KEY (owner_id, outbound_order_id) REFERENCES outbound_orders(owner_id, id) ON DELETE CASCADE;

ALTER TABLE outbound_shipments
    ADD CONSTRAINT outbound_shipments_owner_order_fk
    FOREIGN KEY (owner_id, outbound_order_id) REFERENCES outbound_orders(owner_id, id) ON DELETE CASCADE;

ALTER TABLE traceability_outbound_report_events
    ADD CONSTRAINT traceability_outbound_report_events_owner_report_fk
    FOREIGN KEY (owner_id, report_id) REFERENCES traceability_outbound_reports(owner_id, id) ON DELETE CASCADE;

ALTER TABLE packing_jobs
    ADD CONSTRAINT packing_jobs_owner_order_fk
    FOREIGN KEY (owner_id, outbound_order_id) REFERENCES outbound_orders(owner_id, id) ON DELETE CASCADE;

ALTER TABLE packing_jobs
    ADD CONSTRAINT packing_jobs_owner_station_fk
    FOREIGN KEY (owner_id, station_id) REFERENCES packing_stations(owner_id, id);

ALTER TABLE crossdock_plans
    ADD CONSTRAINT crossdock_plans_owner_order_fk
    FOREIGN KEY (owner_id, outbound_order_id) REFERENCES outbound_orders(owner_id, id) ON DELETE CASCADE;

ALTER TABLE billing_charge_calculations
    ADD CONSTRAINT billing_charge_calculations_owner_contract_fk
    FOREIGN KEY (owner_id, contract_id) REFERENCES billing_contracts(owner_id, id) ON DELETE CASCADE;

ALTER TABLE billing_statements
    ADD CONSTRAINT billing_statements_owner_contract_fk
    FOREIGN KEY (owner_id, contract_id) REFERENCES billing_contracts(owner_id, id) ON DELETE CASCADE;

ALTER TABLE billing_statement_charges
    ADD CONSTRAINT billing_statement_charges_owner_statement_fk
    FOREIGN KEY (owner_id, statement_id) REFERENCES billing_statements(owner_id, id) ON DELETE CASCADE;

ALTER TABLE billing_statement_charges
    ADD CONSTRAINT billing_statement_charges_owner_charge_fk
    FOREIGN KEY (owner_id, charge_id) REFERENCES billing_charge_calculations(owner_id, id) ON DELETE RESTRICT;

ALTER TABLE tms_dispatches
    ADD CONSTRAINT tms_dispatches_owner_order_fk
    FOREIGN KEY (owner_id, outbound_order_id) REFERENCES outbound_orders(owner_id, id) ON DELETE CASCADE;

ALTER TABLE transit_temperature_readings
    ADD CONSTRAINT transit_temperature_readings_owner_dispatch_fk
    FOREIGN KEY (owner_id, dispatch_id) REFERENCES tms_dispatches(owner_id, id) ON DELETE CASCADE;

ALTER TABLE billing_charge_calculations
    ALTER COLUMN period_start TYPE DATE USING period_start::DATE,
    ALTER COLUMN period_end TYPE DATE USING period_end::DATE;

ALTER TABLE billing_statements
    ALTER COLUMN period_start TYPE DATE USING period_start::DATE,
    ALTER COLUMN period_end TYPE DATE USING period_end::DATE;

GRANT SELECT, INSERT, UPDATE, DELETE ON
    products,
    suppliers,
    customers,
    customer_addresses,
    warehouses,
    warehouse_zones,
    warehouse_locations
TO wms_app;
