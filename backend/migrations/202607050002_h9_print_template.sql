-- H9 print template first backend slice: field library metadata and immutable versions.

CREATE TABLE IF NOT EXISTS print_field_libraries (
    id             UUID PRIMARY KEY,
    library_code   TEXT NOT NULL UNIQUE,
    library_name   TEXT NOT NULL,
    source_schema  TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    version        BIGINT NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS print_field_library_versions (
    id            UUID PRIMARY KEY,
    library_id    UUID NOT NULL REFERENCES print_field_libraries(id) ON DELETE RESTRICT,
    version_no    INT NOT NULL CHECK (version_no > 0),
    status        TEXT NOT NULL DEFAULT 'published',
    published_at  TIMESTAMPTZ NOT NULL,
    published_by  UUID NOT NULL,
    request_hash  TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status = 'published'),
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
    metadata            JSONB NOT NULL DEFAULT '{}'::jsonb,
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
    RAISE EXCEPTION 'published print field library versions are immutable: % attempted by %', TG_OP, current_user;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_print_field_library_versions_no_update ON print_field_library_versions;
CREATE TRIGGER trg_print_field_library_versions_no_update
    BEFORE UPDATE OR DELETE OR TRUNCATE ON print_field_library_versions
    FOR EACH STATEMENT EXECUTE FUNCTION print_field_library_version_immutable();

DROP TRIGGER IF EXISTS trg_print_field_definitions_no_update ON print_field_definitions;
CREATE TRIGGER trg_print_field_definitions_no_update
    BEFORE UPDATE OR DELETE OR TRUNCATE ON print_field_definitions
    FOR EACH STATEMENT EXECUTE FUNCTION print_field_library_version_immutable();

GRANT SELECT, INSERT, UPDATE ON print_field_libraries TO wms_app;
GRANT SELECT, INSERT ON print_field_library_versions, print_field_definitions TO wms_app;
