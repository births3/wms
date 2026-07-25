-- H-FILE attachment metadata and short-lived upload/download authorizations.

CREATE TABLE IF NOT EXISTS h_file_upload_sessions (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES auth_owners(id),
    module          TEXT NOT NULL CHECK (length(btrim(module)) BETWEEN 1 AND 32),
    entity_type     TEXT NOT NULL CHECK (length(btrim(entity_type)) BETWEEN 1 AND 64),
    entity_id       UUID NOT NULL,
    file_name       TEXT NOT NULL CHECK (length(btrim(file_name)) BETWEEN 1 AND 255),
    content_type    TEXT NOT NULL CHECK (content_type IN (
        'image/jpeg',
        'image/png',
        'application/pdf',
        'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
        'text/csv'
    )),
    expected_size   BIGINT NOT NULL CHECK (expected_size BETWEEN 1 AND 52428800),
    uploaded_size   BIGINT,
    storage_key     TEXT NOT NULL UNIQUE,
    token_hash      TEXT NOT NULL,
    sha256          TEXT,
    status          TEXT NOT NULL CHECK (status IN ('created', 'uploaded', 'confirmed')),
    uploaded_by     UUID NOT NULL REFERENCES auth_users(id),
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (
        module <> 'M-DI'
        OR content_type NOT IN ('image/jpeg', 'image/png')
        OR expected_size <= 5242880
    )
);

CREATE INDEX IF NOT EXISTS h_file_upload_sessions_owner_entity_idx
    ON h_file_upload_sessions (owner_id, entity_type, entity_id, created_at DESC);

CREATE INDEX IF NOT EXISTS h_file_upload_sessions_expires_idx
    ON h_file_upload_sessions (expires_at)
    WHERE status <> 'confirmed';

CREATE TABLE IF NOT EXISTS attachments (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES auth_owners(id),
    module          TEXT NOT NULL,
    entity_type     TEXT NOT NULL,
    entity_id       UUID NOT NULL,
    file_name       TEXT NOT NULL,
    content_type    TEXT NOT NULL,
    size_bytes      BIGINT NOT NULL CHECK (size_bytes > 0),
    storage_key     TEXT NOT NULL,
    sha256          TEXT NOT NULL,
    uploaded_by     UUID NOT NULL REFERENCES auth_users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, storage_key)
);

CREATE INDEX IF NOT EXISTS attachments_owner_entity_idx
    ON attachments (owner_id, entity_type, entity_id, created_at DESC);

CREATE TABLE IF NOT EXISTS h_file_download_sessions (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES auth_owners(id),
    attachment_id   UUID NOT NULL REFERENCES attachments(id),
    token_hash      TEXT NOT NULL,
    created_by      UUID NOT NULL REFERENCES auth_users(id),
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS h_file_download_sessions_expires_idx
    ON h_file_download_sessions (expires_at);

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:h-file.attachment.read')::uuid, 'h-file.attachment.read', '附件查看与下载'),
    (md5('auth_permission:h-file.attachment.write')::uuid, 'h-file.attachment.write', '附件上传与确认')
ON CONFLICT (lower(permission_code)) DO UPDATE
SET permission_name = EXCLUDED.permission_name;

GRANT SELECT, INSERT, UPDATE ON h_file_upload_sessions TO wms_app;
GRANT SELECT, INSERT ON attachments TO wms_app;
GRANT SELECT, INSERT ON h_file_download_sessions TO wms_app;
