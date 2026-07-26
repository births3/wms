-- H9 print template first backend slice: field library metadata and immutable versions.

CREATE TABLE IF NOT EXISTS print_field_libraries (
    id             UUID PRIMARY KEY,
    library_code   TEXT NOT NULL UNIQUE,
    library_name   TEXT NOT NULL,
    business_module TEXT NOT NULL,
    source_schema  TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    version        BIGINT NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS print_field_library_versions (
    id            UUID PRIMARY KEY,
    library_id    UUID NOT NULL REFERENCES print_field_libraries(id) ON DELETE RESTRICT,
    version_no    INT NOT NULL CHECK (version_no > 0),
    status        TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published')),
    source_schema TEXT NOT NULL,
    business_module TEXT NOT NULL,
    request_hash  TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by    UUID NOT NULL,
    published_at  TIMESTAMPTZ,
    published_by  UUID,
    CHECK (
        (status = 'draft' AND published_at IS NULL AND published_by IS NULL)
        OR (status = 'published' AND published_at IS NOT NULL AND published_by IS NOT NULL)
    ),
    UNIQUE (library_id, version_no)
);

CREATE TABLE IF NOT EXISTS print_field_definitions (
    id                  UUID PRIMARY KEY,
    library_version_id  UUID NOT NULL REFERENCES print_field_library_versions(id) ON DELETE RESTRICT,
    field_path          TEXT NOT NULL,
    field_type          TEXT NOT NULL,
    source_schema       TEXT NOT NULL,
    display_name        TEXT NOT NULL,
    group_code          TEXT NOT NULL,
    group_name          TEXT NOT NULL,
    description         TEXT NOT NULL DEFAULT '',
    example_value       JSONB,
    printable           BOOLEAN NOT NULL DEFAULT TRUE,
    sensitive           BOOLEAN NOT NULL DEFAULT FALSE,
    masking_rule        TEXT,
    formatting_rule     TEXT,
    supports_barcode    BOOLEAN NOT NULL DEFAULT FALSE,
    supports_qrcode     BOOLEAN NOT NULL DEFAULT FALSE,
    is_table_detail     BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order          INT NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (library_version_id, field_path)
);

CREATE INDEX IF NOT EXISTS print_field_library_versions_lookup_idx
    ON print_field_library_versions (library_id, version_no DESC);

CREATE INDEX IF NOT EXISTS print_field_definitions_version_order_idx
    ON print_field_definitions (library_version_id, sort_order, field_path);

CREATE OR REPLACE FUNCTION print_field_library_version_immutable() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' OR OLD.status = 'published' THEN
        RAISE EXCEPTION 'published print field library versions are immutable: % attempted by %', TG_OP, current_user;
    END IF;
    IF NEW.id <> OLD.id
        OR NEW.library_id <> OLD.library_id
        OR NEW.version_no <> OLD.version_no
        OR NEW.source_schema <> OLD.source_schema
        OR NEW.business_module <> OLD.business_module
        OR NEW.request_hash <> OLD.request_hash
        OR NEW.created_at <> OLD.created_at
        OR NEW.created_by <> OLD.created_by
        OR NEW.status NOT IN ('draft', 'published') THEN
        RAISE EXCEPTION 'print field library draft identity is immutable';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_print_field_library_versions_no_update ON print_field_library_versions;
CREATE TRIGGER trg_print_field_library_versions_no_update
    BEFORE UPDATE OR DELETE ON print_field_library_versions
    FOR EACH ROW EXECUTE FUNCTION print_field_library_version_immutable();

CREATE OR REPLACE FUNCTION print_field_definition_draft_only() RETURNS TRIGGER AS $$
DECLARE
    target_version_id UUID;
    target_status TEXT;
BEGIN
    target_version_id := CASE WHEN TG_OP = 'DELETE' THEN OLD.library_version_id ELSE NEW.library_version_id END;
    SELECT status INTO target_status
      FROM print_field_library_versions
     WHERE id = target_version_id;
    IF target_status IS DISTINCT FROM 'draft' THEN
        RAISE EXCEPTION 'published print field definitions are immutable: % attempted by %', TG_OP, current_user;
    END IF;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_print_field_definitions_no_update ON print_field_definitions;
CREATE TRIGGER trg_print_field_definitions_no_update
    BEFORE INSERT OR UPDATE OR DELETE ON print_field_definitions
    FOR EACH ROW EXECUTE FUNCTION print_field_definition_draft_only();

GRANT SELECT, INSERT, UPDATE ON print_field_libraries TO wms_app;
GRANT SELECT, INSERT, UPDATE ON print_field_library_versions, print_field_definitions TO wms_app;
