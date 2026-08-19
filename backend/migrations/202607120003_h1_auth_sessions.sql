-- US-H1-005 token revocation and multi-device session lifecycle.

CREATE TABLE IF NOT EXISTS auth_sessions (
    session_id   TEXT PRIMARY KEY,
    owner_id     UUID NOT NULL,
    user_id      UUID NOT NULL,
    device_name  TEXT NOT NULL,
    ip           INET,
    logged_in_at TIMESTAMPTZ NOT NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    revoked_at   TIMESTAMPTZ,
    revoke_reason TEXT,
    revoked_by   UUID,
    FOREIGN KEY (user_id, owner_id)
        REFERENCES auth_user_owner_bindings(user_id, owner_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS auth_sessions_owner_user_active_idx
    ON auth_sessions (owner_id, user_id, expires_at DESC)
    WHERE revoked_at IS NULL;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:h1.sessions.manage')::uuid,
    'h1.sessions.manage',
    'H1 会话强制失效'
)
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'h1.sessions.manage'
 WHERE role.role_code = 'system_admin'
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES (
    '00000000-0000-0000-0000-000000130024',
    '00000000-0000-0000-0000-000000120008',
    3,
    'platform.h1.sessions',
    'platform/h1/sessions',
    'H1 登录会话',
    'h1-session-management',
    'ShieldCheck',
    'h1.sessions.manage',
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
        ('disable', '失效设备', 'standard', 60),
        ('field', '字段', 'standard', 110),
        ('view', '视图', 'standard', 120),
        ('revoke_others', '登出其他设备', 'private', 200),
        ('kick_user', '踢出用户', 'private', 210)
 ) AS action(key, label, kind, sort_order)
 WHERE node.id = '00000000-0000-0000-0000-000000130024'
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
 WHERE node.id = '00000000-0000-0000-0000-000000130024'
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
 WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130024'
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE ON auth_sessions TO wms_app;
