-- US-DI-001 drug inspection platform configuration.

CREATE TABLE IF NOT EXISTS drug_inspection_platforms (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    platform_code       TEXT NOT NULL,
    platform_name       TEXT NOT NULL,
    api_url             TEXT NOT NULL,
    auth_method         TEXT NOT NULL,
    api_key_alias       TEXT,
    username            TEXT,
    password_alias      TEXT,
    timeout_seconds     INT NOT NULL DEFAULT 30,
    status              TEXT NOT NULL DEFAULT 'testing',
    created_by          UUID NOT NULL,
    updated_by          UUID NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    version             BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, platform_code),
    CHECK (length(trim(platform_code)) > 0),
    CHECK (length(trim(platform_name)) > 0),
    CHECK (auth_method IN ('api_key', 'username_password')),
    CHECK (status IN ('connected', 'testing', 'disabled')),
    CHECK (timeout_seconds BETWEEN 1 AND 300),
    CHECK (api_key_alias IS NULL OR api_key_alias LIKE 'vault://%'),
    CHECK (password_alias IS NULL OR password_alias LIKE 'vault://%'),
    CHECK (
        (auth_method = 'api_key'
            AND api_key_alias IS NOT NULL
            AND username IS NULL
            AND password_alias IS NULL)
        OR
        (auth_method = 'username_password'
            AND api_key_alias IS NULL
            AND username IS NOT NULL
            AND password_alias IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS drug_inspection_platforms_owner_status_idx
    ON drug_inspection_platforms (owner_id, status, platform_code);

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:m-di.platform.read')::uuid, 'm-di.platform.read', 'M-DI 药检平台读取'),
    (md5('auth_permission:m-di.platform.write')::uuid, 'm-di.platform.write', 'M-DI 药检平台维护')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN ('m-di.platform.read', 'm-di.platform.write')
 WHERE role.role_code = 'system_admin'
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE ON drug_inspection_platforms TO wms_app;
