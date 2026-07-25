-- US-M1-001 complete typed product contract and packaging hierarchy.

ALTER TABLE products
    ADD COLUMN udi_code TEXT,
    ADD COLUMN electronic_regulatory_code TEXT,
    ADD COLUMN length_mm DOUBLE PRECISION,
    ADD COLUMN width_mm DOUBLE PRECISION,
    ADD COLUMN height_mm DOUBLE PRECISION,
    ADD COLUMN volume_cm3 DOUBLE PRECISION,
    ADD COLUMN weight_g DOUBLE PRECISION,
    ADD CONSTRAINT products_length_positive
        CHECK (length_mm IS NULL OR length_mm > 0 AND length_mm < 'Infinity'::DOUBLE PRECISION),
    ADD CONSTRAINT products_width_positive
        CHECK (width_mm IS NULL OR width_mm > 0 AND width_mm < 'Infinity'::DOUBLE PRECISION),
    ADD CONSTRAINT products_height_positive
        CHECK (height_mm IS NULL OR height_mm > 0 AND height_mm < 'Infinity'::DOUBLE PRECISION),
    ADD CONSTRAINT products_volume_positive
        CHECK (volume_cm3 IS NULL OR volume_cm3 > 0 AND volume_cm3 < 'Infinity'::DOUBLE PRECISION),
    ADD CONSTRAINT products_weight_positive
        CHECK (weight_g IS NULL OR weight_g > 0 AND weight_g < 'Infinity'::DOUBLE PRECISION);

CREATE UNIQUE INDEX products_owner_id_uidx
    ON products (owner_id, id);

CREATE UNIQUE INDEX products_owner_udi_uidx
    ON products (owner_id, udi_code)
    WHERE udi_code IS NOT NULL AND btrim(udi_code) <> '';

CREATE TABLE product_packaging_levels (
    id             UUID PRIMARY KEY,
    owner_id       UUID NOT NULL,
    product_id     UUID NOT NULL,
    unit_code      TEXT NOT NULL,
    unit_name      TEXT NOT NULL,
    ratio_to_base  BIGINT NOT NULL CHECK (ratio_to_base > 0),
    is_base        BOOLEAN NOT NULL DEFAULT FALSE,
    is_default     BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order     INT NOT NULL CHECK (sort_order >= 0),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (owner_id, product_id)
        REFERENCES products(owner_id, id) ON DELETE CASCADE,
    UNIQUE (owner_id, product_id, unit_code),
    UNIQUE (owner_id, product_id, sort_order),
    CHECK (NOT is_base OR ratio_to_base = 1)
);

CREATE UNIQUE INDEX product_packaging_levels_one_base_uidx
    ON product_packaging_levels (owner_id, product_id)
    WHERE is_base;

CREATE UNIQUE INDEX product_packaging_levels_one_default_uidx
    ON product_packaging_levels (owner_id, product_id)
    WHERE is_default;

CREATE INDEX product_packaging_levels_owner_product_idx
    ON product_packaging_levels (owner_id, product_id, sort_order);

CREATE TABLE product_mapping_traces (
    id             UUID PRIMARY KEY,
    owner_id       UUID NOT NULL,
    product_id     UUID NOT NULL,
    field_name     TEXT NOT NULL,
    rule_id        UUID,
    source_system  TEXT NOT NULL,
    source_value   TEXT NOT NULL,
    target_value   TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (owner_id, product_id)
        REFERENCES products(owner_id, id) ON DELETE CASCADE,
    FOREIGN KEY (rule_id) REFERENCES parameter_mapping_rules(id),
    CHECK (btrim(field_name) <> ''),
    CHECK (btrim(source_system) <> ''),
    CHECK (btrim(source_value) <> ''),
    CHECK (target_value IS NULL OR btrim(target_value) <> '')
);

CREATE INDEX product_mapping_traces_owner_product_idx
    ON product_mapping_traces (owner_id, product_id, created_at DESC);

CREATE INDEX product_mapping_traces_rule_idx
    ON product_mapping_traces (rule_id, created_at DESC)
    WHERE rule_id IS NOT NULL;

CREATE OR REPLACE FUNCTION product_mapping_trace_immutable() RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'product_mapping_traces is append-only: % attempted by %', TG_OP, current_user;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER product_mapping_traces_no_update
    BEFORE UPDATE OR DELETE OR TRUNCATE ON product_mapping_traces
    FOR EACH STATEMENT EXECUTE FUNCTION product_mapping_trace_immutable();

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON product_packaging_levels TO wms_app;
        GRANT SELECT, INSERT ON product_mapping_traces TO wms_app;
    END IF;
END
$$;
