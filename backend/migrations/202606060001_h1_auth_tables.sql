-- Wave 1 W1.A auth users, owner bindings, roles, and permissions.

CREATE TABLE IF NOT EXISTS auth_owners (
    id            UUID PRIMARY KEY,
    owner_code    TEXT NOT NULL,
    owner_name    TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS auth_owners_owner_code_lower_idx
    ON auth_owners (lower(owner_code));

CREATE TABLE IF NOT EXISTS auth_users (
    id                    UUID PRIMARY KEY,
    username              TEXT NOT NULL,
    display_name          TEXT NOT NULL,
    password_hash         TEXT NOT NULL,
    status                TEXT NOT NULL DEFAULT 'active',
    failed_login_count    INT NOT NULL DEFAULT 0 CHECK (failed_login_count >= 0),
    locked_until          TIMESTAMPTZ,
    permissions_changed_at TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status IN ('active', 'disabled', 'locked'))
);

CREATE UNIQUE INDEX IF NOT EXISTS auth_users_username_lower_idx
    ON auth_users (lower(username));

CREATE TABLE IF NOT EXISTS auth_user_owner_bindings (
    user_id     UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    owner_id    UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    is_primary  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, owner_id)
);

CREATE INDEX IF NOT EXISTS auth_user_owner_bindings_owner_idx
    ON auth_user_owner_bindings (owner_id, user_id);

CREATE TABLE IF NOT EXISTS auth_roles (
    id          UUID PRIMARY KEY,
    owner_id    UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    role_code   TEXT NOT NULL,
    role_name   TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS auth_roles_owner_role_code_lower_idx
    ON auth_roles (owner_id, lower(role_code));

CREATE TABLE IF NOT EXISTS auth_permissions (
    id               UUID PRIMARY KEY,
    permission_code  TEXT NOT NULL,
    permission_name  TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS auth_permissions_code_lower_idx
    ON auth_permissions (lower(permission_code));

CREATE TABLE IF NOT EXISTS auth_user_roles (
    user_id     UUID NOT NULL,
    owner_id    UUID NOT NULL,
    role_id     UUID NOT NULL REFERENCES auth_roles(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, owner_id, role_id),
    FOREIGN KEY (user_id, owner_id)
        REFERENCES auth_user_owner_bindings(user_id, owner_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS auth_user_roles_role_idx
    ON auth_user_roles (role_id);

CREATE TABLE IF NOT EXISTS auth_role_permissions (
    role_id        UUID NOT NULL REFERENCES auth_roles(id) ON DELETE CASCADE,
    permission_id  UUID NOT NULL REFERENCES auth_permissions(id) ON DELETE CASCADE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (role_id, permission_id)
);

CREATE INDEX IF NOT EXISTS auth_role_permissions_permission_idx
    ON auth_role_permissions (permission_id);

GRANT SELECT, INSERT, UPDATE, DELETE ON auth_owners TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON auth_users TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON auth_user_owner_bindings TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON auth_roles TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON auth_permissions TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON auth_user_roles TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON auth_role_permissions TO wms_app;
