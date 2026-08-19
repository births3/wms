-- US-AL-001：告警定义查询、启停版本控制、M-QL 变更审批与管理端入口。

ALTER TABLE alert_definitions
    ADD COLUMN IF NOT EXISTS enabled BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    ADD COLUMN IF NOT EXISTS message_templates JSONB NOT NULL DEFAULT '{}'::jsonb;

UPDATE alert_definitions
   SET message_templates = jsonb_build_object('zh-CN', message_template)
 WHERE message_templates = '{}'::jsonb;

ALTER TABLE alert_definitions
    ALTER COLUMN silence_period_seconds SET DEFAULT 300,
    ADD CONSTRAINT alert_definitions_message_templates_check
    CHECK (
        jsonb_typeof(message_templates) = 'object'
        AND message_templates ? 'zh-CN'
        AND btrim(message_templates ->> 'zh-CN') <> ''
    );

CREATE OR REPLACE FUNCTION fill_alert_definition_message_templates()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.message_templates = '{}'::jsonb THEN
        NEW.message_templates := jsonb_build_object('zh-CN', NEW.message_template);
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS alert_definitions_fill_message_templates ON alert_definitions;
CREATE TRIGGER alert_definitions_fill_message_templates
BEFORE INSERT OR UPDATE OF message_template, message_templates ON alert_definitions
FOR EACH ROW EXECUTE FUNCTION fill_alert_definition_message_templates();

UPDATE alert_definitions
   SET silence_period_seconds = 300
 WHERE is_gsp_forced AND silence_period_seconds = 0;

ALTER TABLE alert_definitions
    ADD CONSTRAINT alert_definitions_gsp_enabled_check
    CHECK (NOT is_gsp_forced OR (NOT is_disable_allowed AND enabled));

CREATE UNIQUE INDEX IF NOT EXISTS alert_definitions_owner_name_uq
    ON alert_definitions (owner_id, lower(btrim(name)));

CREATE OR REPLACE FUNCTION prevent_gsp_alert_definition_delete()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.is_gsp_forced THEN
        RAISE EXCEPTION 'GSP forced alert definition cannot be deleted'
            USING ERRCODE = '23514';
    END IF;
    RETURN OLD;
END;
$$;

DROP TRIGGER IF EXISTS alert_definitions_prevent_gsp_delete ON alert_definitions;
CREATE TRIGGER alert_definitions_prevent_gsp_delete
BEFORE DELETE ON alert_definitions
FOR EACH ROW EXECUTE FUNCTION prevent_gsp_alert_definition_delete();

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:menu.platform.hal')::uuid, 'menu.platform.hal', 'H-AL 告警能力菜单'),
    (md5('auth_permission:hal.alert-definition.read')::uuid, 'hal.alert-definition.read', 'H-AL 告警定义查询'),
    (md5('auth_permission:hal.alert-definition.write')::uuid, 'hal.alert-definition.write', 'H-AL 告警定义变更申请')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN (
        'menu.platform.hal',
        'hal.alert-definition.read',
        'hal.alert-definition.write'
    )
 WHERE lower(role.role_code) = 'warehouse_manager'
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_hal_permissions_to_warehouse_manager()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF lower(NEW.role_code) = 'warehouse_manager' THEN
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT NEW.id, permission.id
          FROM auth_permissions permission
         WHERE permission.permission_code IN (
             'menu.platform.hal',
             'hal.alert-definition.read',
             'hal.alert-definition.write'
         )
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_hal_permissions ON auth_roles;
CREATE TRIGGER auth_roles_grant_hal_permissions
AFTER INSERT OR UPDATE OF role_code ON auth_roles
FOR EACH ROW EXECUTE FUNCTION grant_hal_permissions_to_warehouse_manager();

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key,
    permission_key, sort_order, enabled
)
VALUES (
    '00000000-0000-0000-0000-000000120016',
    '00000000-0000-0000-0000-000000110006',
    2,
    'platform.hal',
    'platform/hal',
    'H-AL 告警能力',
    NULL,
    'Bell',
    'menu.platform.hal',
    60,
    TRUE
), (
    '00000000-0000-0000-0000-000000130033',
    '00000000-0000-0000-0000-000000120016',
    3,
    'platform.alert_definitions',
    'platform/capability/alert_definitions',
    'H-AL 告警定义',
    'hal-alert-definitions',
    'Bell',
    'hal.alert-definition.read',
    50,
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
    'standard',
    TRUE,
    action.sort_order
FROM admin_menu_draft_nodes node
CROSS JOIN (
    VALUES
        ('query', '查询', 10),
        ('refresh', '刷新', 20),
        ('create', '新增', 30),
        ('edit', '编辑', 40),
        ('disable', '启停', 50),
        ('delete', '删除', 60),
        ('export', '导出', 70),
        ('field', '字段', 80),
        ('view', '视图', 90)
) AS action(key, label, sort_order)
WHERE node.id = '00000000-0000-0000-0000-000000130033'
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
WHERE node.id IN (
    '00000000-0000-0000-0000-000000120016',
    '00000000-0000-0000-0000-000000130033'
)
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label,
    action_kind, enabled, sort_order
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
WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130033'
ON CONFLICT DO NOTHING;
