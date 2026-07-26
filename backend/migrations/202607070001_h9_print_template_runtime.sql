-- H9 print template runtime slice: template versions and browser print records.

CREATE TABLE IF NOT EXISTS print_templates (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL,
    template_code       TEXT NOT NULL,
    template_name       TEXT NOT NULL,
    template_type_code  TEXT NOT NULL,
    scope               TEXT NOT NULL CHECK (scope IN ('global', 'owner')),
    enabled             BOOLEAN NOT NULL DEFAULT TRUE,
    is_default          BOOLEAN NOT NULL DEFAULT FALSE,
    remark              TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by          UUID NOT NULL,
    updated_by          UUID NOT NULL,
    version             BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, template_code)
);

CREATE INDEX IF NOT EXISTS print_templates_owner_type_lookup_idx
    ON print_templates (owner_id, template_type_code, enabled, is_default, scope, updated_at DESC);

CREATE TABLE IF NOT EXISTS print_template_versions (
    id                        UUID PRIMARY KEY,
    template_id               UUID NOT NULL REFERENCES print_templates(id) ON DELETE RESTRICT,
    field_library_version_id  UUID NOT NULL REFERENCES print_field_library_versions(id) ON DELETE RESTRICT,
    template_name             TEXT NOT NULL,
    template_type_code        TEXT NOT NULL,
    scope                     TEXT NOT NULL CHECK (scope IN ('global', 'owner')),
    is_default                BOOLEAN NOT NULL DEFAULT FALSE,
    remark                    TEXT,
    version_no                INT NOT NULL CHECK (version_no > 0),
    status                    TEXT NOT NULL CHECK (status IN ('draft', 'published')),
    hiprint_json              JSONB NOT NULL,
    field_bindings            JSONB NOT NULL DEFAULT '[]'::jsonb,
    paper                     JSONB NOT NULL DEFAULT '{}'::jsonb,
    designer_version          TEXT NOT NULL,
    request_hash              TEXT NOT NULL,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by                UUID NOT NULL,
    published_at              TIMESTAMPTZ,
    published_by              UUID,
    UNIQUE (template_id, version_no)
);

CREATE INDEX IF NOT EXISTS print_template_versions_template_lookup_idx
    ON print_template_versions (template_id, version_no DESC);

CREATE INDEX IF NOT EXISTS print_template_versions_published_lookup_idx
    ON print_template_versions (template_id, status, version_no DESC);

CREATE TABLE IF NOT EXISTS print_records (
    id                      UUID PRIMARY KEY,
    owner_id                UUID NOT NULL,
    template_version_id     UUID NOT NULL REFERENCES print_template_versions(id) ON DELETE RESTRICT,
    business_module         TEXT NOT NULL,
    business_document_type  TEXT NOT NULL,
    business_document_id    TEXT NOT NULL,
    status                  TEXT NOT NULL CHECK (status IN ('printed', 'cancelled', 'failed')),
    failure_reason          TEXT,
    retry_count             INT NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
    printed_at              TIMESTAMPTZ NOT NULL,
    operator_id             UUID NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS print_records_owner_document_idx
    ON print_records (owner_id, business_document_type, business_document_id, printed_at DESC);

CREATE OR REPLACE FUNCTION print_template_version_immutable() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'TRUNCATE' OR TG_OP = 'DELETE' OR OLD.status = 'published' THEN
        RAISE EXCEPTION 'published print template versions are immutable: % attempted by %', TG_OP, current_user;
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id
        OR NEW.template_id IS DISTINCT FROM OLD.template_id
        OR NEW.field_library_version_id IS DISTINCT FROM OLD.field_library_version_id
        OR NEW.template_name IS DISTINCT FROM OLD.template_name
        OR NEW.template_type_code IS DISTINCT FROM OLD.template_type_code
        OR NEW.scope IS DISTINCT FROM OLD.scope
        OR NEW.is_default IS DISTINCT FROM OLD.is_default
        OR NEW.remark IS DISTINCT FROM OLD.remark
        OR NEW.version_no IS DISTINCT FROM OLD.version_no
        OR NEW.hiprint_json IS DISTINCT FROM OLD.hiprint_json
        OR NEW.field_bindings IS DISTINCT FROM OLD.field_bindings
        OR NEW.paper IS DISTINCT FROM OLD.paper
        OR NEW.designer_version IS DISTINCT FROM OLD.designer_version
        OR NEW.request_hash IS DISTINCT FROM OLD.request_hash
        OR NEW.created_at IS DISTINCT FROM OLD.created_at
        OR NEW.created_by IS DISTINCT FROM OLD.created_by
        OR NEW.status <> 'published'
        OR NEW.published_at IS NULL
        OR NEW.published_by IS NULL
    THEN
        RAISE EXCEPTION 'print template draft may only transition to published';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_print_template_versions_no_update ON print_template_versions;
CREATE TRIGGER trg_print_template_versions_no_update
    BEFORE UPDATE OR DELETE ON print_template_versions
    FOR EACH ROW EXECUTE FUNCTION print_template_version_immutable();

DROP TRIGGER IF EXISTS trg_print_template_versions_no_truncate ON print_template_versions;
CREATE TRIGGER trg_print_template_versions_no_truncate
    BEFORE TRUNCATE ON print_template_versions
    FOR EACH STATEMENT EXECUTE FUNCTION print_template_version_immutable();

CREATE OR REPLACE FUNCTION print_record_immutable() RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'print records are append-only: % attempted by %', TG_OP, current_user;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_print_records_no_update ON print_records;
CREATE TRIGGER trg_print_records_no_update
    BEFORE UPDATE OR DELETE OR TRUNCATE ON print_records
    FOR EACH STATEMENT EXECUTE FUNCTION print_record_immutable();

GRANT SELECT, INSERT, UPDATE ON print_templates TO wms_app;
GRANT SELECT, INSERT, UPDATE ON print_template_versions TO wms_app;
GRANT SELECT, INSERT ON print_records TO wms_app;
