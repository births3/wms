-- US-H1-006 API Key 生命周期、owner 隔离、幂等和数据库限流状态。

CREATE TABLE IF NOT EXISTS auth_api_keys (
    id                            UUID PRIMARY KEY,
    owner_id                      UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    caller_name                   TEXT NOT NULL,
    purpose                       TEXT NOT NULL,
    warehouse_ids                 UUID[] NOT NULL DEFAULT '{}'::uuid[],
    scopes                        TEXT[] NOT NULL,
    responsible_user_id           UUID NOT NULL REFERENCES auth_users(id),
    key_hash                      TEXT NOT NULL UNIQUE,
    status                        TEXT NOT NULL DEFAULT 'active',
    expires_at                    TIMESTAMPTZ NOT NULL,
    grace_expires_at              TIMESTAMPTZ,
    replaced_by_key_id            UUID REFERENCES auth_api_keys(id),
    revoked_at                    TIMESTAMPTZ,
    temporarily_disabled_until    TIMESTAMPTZ,
    failed_auth_count             INT NOT NULL DEFAULT 0 CHECK (failed_auth_count >= 0),
    failed_auth_window_started_at TIMESTAMPTZ,
    rate_limit_window_started_at  TIMESTAMPTZ,
    rate_limit_count              INT NOT NULL DEFAULT 0 CHECK (rate_limit_count >= 0),
    last_used_at                  TIMESTAMPTZ,
    created_at                    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                    TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                       BIGINT NOT NULL DEFAULT 1,
    CHECK (length(trim(caller_name)) > 0),
    CHECK (length(trim(purpose)) > 0),
    CHECK (cardinality(scopes) > 0),
    CHECK (status IN ('active', 'revoked', 'temporarily_disabled'))
);

CREATE INDEX IF NOT EXISTS auth_api_keys_owner_status_idx
    ON auth_api_keys (owner_id, status, expires_at);

CREATE INDEX IF NOT EXISTS auth_api_keys_responsible_user_idx
    ON auth_api_keys (owner_id, responsible_user_id);

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:h1.api_keys.manage')::uuid,
    'h1.api_keys.manage',
    'H1 API Key 生命周期管理'
)
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'h1.api_keys.manage'
 WHERE role.role_code = 'system_admin'
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES (
    '00000000-0000-0000-0000-000000130025',
    '00000000-0000-0000-0000-000000120008',
    3,
    'platform.h1.api_keys',
    'platform/h1/api_keys',
    'H1 API Key 管理',
    'h1-api-keys',
    'KeyRound',
    'h1.api_keys.manage',
    20,
    TRUE
)
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5(node.id::text || ':' || action.key)::uuid,
    node.id,
    action.key,
    action.label,
    action.kind,
    TRUE,
    action.sort_order
  FROM admin_menu_draft_nodes node
 CROSS JOIN (
    VALUES
        ('query', '查询', 'standard', 10),
        ('refresh', '刷新', 'standard', 20),
        ('create', '新增', 'standard', 30),
        ('field', '字段', 'standard', 110),
        ('view', '视图', 'standard', 120),
        ('rotate', '轮换', 'private', 200),
        ('revoke', '吊销', 'private', 210)
 ) AS action(key, label, kind, sort_order)
 WHERE node.id = '00000000-0000-0000-0000-000000130025'
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
)
INSERT INTO admin_menu_version_nodes (
    id, version_id, source_node_id, parent_source_id, level, code, path, title,
    view_id, icon_key, permission_key, sort_order, enabled, created_at, updated_at
)
SELECT
    md5((SELECT id::text FROM version_row) || ':' || node.id::text)::uuid,
    (SELECT id FROM version_row),
    node.id,
    node.parent_id,
    node.level,
    node.code,
    node.path,
    node.title,
    node.view_id,
    node.icon_key,
    node.permission_key,
    node.sort_order,
    node.enabled,
    node.created_at,
    node.updated_at
  FROM admin_menu_draft_nodes node
 WHERE node.id = '00000000-0000-0000-0000-000000130025'
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5((SELECT id::text FROM version_row) || ':' || button.id::text)::uuid,
    (SELECT id FROM version_row),
    button.menu_node_id,
    button.action_key,
    button.action_label,
    button.action_kind,
    button.enabled,
    button.sort_order
  FROM admin_menu_draft_button_permissions button
 WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130025'
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE ON auth_api_keys TO wms_app;
