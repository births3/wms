-- US-AL-003：最多三级告警升级规则、夜间/节假日路由与幂等升级事件。

CREATE TABLE IF NOT EXISTS alert_escalation_rules (
    id                       UUID PRIMARY KEY,
    owner_id                 UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    rule_code                TEXT NOT NULL,
    rule_name                TEXT NOT NULL,
    notify_lower_levels      BOOLEAN NOT NULL DEFAULT TRUE,
    off_hours_start          TIME NOT NULL DEFAULT '18:00',
    off_hours_end            TIME NOT NULL DEFAULT '08:00',
    off_hours_handler_roles  TEXT[] NOT NULL DEFAULT '{}',
    holiday_dates            DATE[] NOT NULL DEFAULT '{}',
    enabled                  BOOLEAN NOT NULL DEFAULT TRUE,
    version                  BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    created_by               UUID,
    updated_by               UUID,
    created_at               TIMESTAMPTZ NOT NULL,
    updated_at               TIMESTAMPTZ NOT NULL,
    CHECK (btrim(rule_code) <> ''),
    CHECK (btrim(rule_name) <> ''),
    UNIQUE (owner_id, rule_code)
);

CREATE TABLE IF NOT EXISTS alert_escalation_levels (
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    rule_id            UUID NOT NULL REFERENCES alert_escalation_rules(id) ON DELETE CASCADE,
    level_no           INT NOT NULL CHECK (level_no BETWEEN 1 AND 3),
    threshold_seconds  BIGINT NOT NULL CHECK (threshold_seconds > 0),
    recipient_roles    TEXT[] NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL,
    updated_at         TIMESTAMPTZ NOT NULL,
    CHECK (cardinality(recipient_roles) > 0),
    UNIQUE (rule_id, level_no)
);

CREATE TABLE IF NOT EXISTS alert_escalation_events (
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    alert_instance_id  UUID NOT NULL REFERENCES alert_instances(id) ON DELETE RESTRICT,
    level_no           INT NOT NULL CHECK (level_no BETWEEN 1 AND 3),
    repeat_key         TEXT NOT NULL,
    recipients         TEXT[] NOT NULL,
    elapsed_seconds    BIGINT NOT NULL CHECK (elapsed_seconds >= 0),
    reason             TEXT NOT NULL,
    occurred_at        TIMESTAMPTZ NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL,
    UNIQUE (alert_instance_id, repeat_key)
);

CREATE INDEX IF NOT EXISTS alert_escalation_events_alert_idx
    ON alert_escalation_events (alert_instance_id, occurred_at DESC);

CREATE OR REPLACE FUNCTION seed_hal_default_escalation_rule(target_owner_id UUID)
RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE
    default_rule_id UUID := md5(target_owner_id::text || ':hal-escalation:gsp-default')::uuid;
BEGIN
    INSERT INTO alert_escalation_rules (
        id, owner_id, rule_code, rule_name, notify_lower_levels,
        off_hours_start, off_hours_end, off_hours_handler_roles,
        holiday_dates, enabled, created_at, updated_at
    ) VALUES (
        default_rule_id, target_owner_id, 'gsp-default', 'GSP 默认三级升级', TRUE,
        '18:00', '08:00', ARRAY['warehouse_manager','system_admin']::text[],
        '{}'::date[], TRUE, now(), now()
    ) ON CONFLICT (owner_id, rule_code) DO NOTHING;

    INSERT INTO alert_escalation_levels (
        id, owner_id, rule_id, level_no, threshold_seconds,
        recipient_roles, created_at, updated_at
    ) VALUES
        (md5(default_rule_id::text || ':1')::uuid, target_owner_id, default_rule_id, 1, 1800, ARRAY['warehouse_manager']::text[], now(), now()),
        (md5(default_rule_id::text || ':2')::uuid, target_owner_id, default_rule_id, 2, 7200, ARRAY['warehouse_manager','system_admin']::text[], now(), now()),
        (md5(default_rule_id::text || ':3')::uuid, target_owner_id, default_rule_id, 3, 86400, ARRAY['system_admin','owner_contact']::text[], now(), now())
    ON CONFLICT (rule_id, level_no) DO NOTHING;
END;
$$;

CREATE OR REPLACE FUNCTION seed_hal_default_escalation_rule_for_new_owner()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    PERFORM seed_hal_default_escalation_rule(NEW.id);
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_owners_seed_hal_default_escalation_rule ON auth_owners;
CREATE TRIGGER auth_owners_seed_hal_default_escalation_rule
AFTER INSERT ON auth_owners FOR EACH ROW
EXECUTE FUNCTION seed_hal_default_escalation_rule_for_new_owner();

SELECT seed_hal_default_escalation_rule(id) FROM auth_owners;

UPDATE alert_definitions
   SET escalation_ref = 'gsp-default', updated_at = now()
 WHERE is_gsp_forced
   AND escalation_ref IS DISTINCT FROM 'gsp-default';

GRANT SELECT, INSERT, UPDATE ON alert_escalation_rules TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON alert_escalation_levels TO wms_app;
GRANT SELECT, INSERT ON alert_escalation_events TO wms_app;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:hal.escalation.read')::uuid, 'hal.escalation.read', 'H-AL 升级规则查询'),
    (md5('auth_permission:hal.escalation.write')::uuid, 'hal.escalation.write', 'H-AL 升级规则维护')
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_hal_permissions_to_warehouse_manager()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF lower(NEW.role_code) IN ('warehouse_manager', 'system_admin') THEN
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT NEW.id, permission.id
          FROM auth_permissions permission
         WHERE permission.permission_code IN (
             'menu.platform.hal',
             'hal.alert-definition.read',
             'hal.alert-definition.write',
             'hal.alert.read',
             'hal.alert.handle',
             'hal.escalation.read',
             'hal.escalation.write'
         )
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key,
    permission_key, sort_order, enabled
) VALUES (
    '00000000-0000-0000-0000-000000130035',
    '00000000-0000-0000-0000-000000120016',
    3, 'platform.alert_escalations', 'platform/capability/alert_escalations',
    'H-AL 升级规则', 'hal-alert-escalations', 'ArrowUpCircle',
    'hal.escalation.read', 70, TRUE
)
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT md5(node.id::text || ':' || action.key)::uuid, node.id,
       action.key, action.label, 'standard', TRUE, action.sort_order
  FROM admin_menu_draft_nodes node
 CROSS JOIN (VALUES
    ('query', '查询', 10), ('refresh', '刷新', 20), ('create', '新增', 30),
    ('edit', '编辑', 40), ('enable', '启停', 50), ('field', '字段', 80),
    ('view', '视图', 90)
 ) AS action(key, label, sort_order)
 WHERE node.id = '00000000-0000-0000-0000-000000130035'
ON CONFLICT DO NOTHING;

WITH version_row AS (SELECT id FROM admin_menu_versions WHERE version_no = 1)
INSERT INTO admin_menu_version_nodes (
    id, version_id, source_node_id, parent_source_id, level, code, path, title,
    view_id, icon_key, permission_key, sort_order, enabled, created_at, updated_at
)
SELECT md5((SELECT id::text FROM version_row) || ':' || node.id::text)::uuid,
       (SELECT id FROM version_row), node.id, node.parent_id, node.level,
       node.code, node.path, node.title, node.view_id, node.icon_key,
       node.permission_key, node.sort_order, node.enabled, node.created_at, node.updated_at
  FROM admin_menu_draft_nodes node
 WHERE node.id = '00000000-0000-0000-0000-000000130035'
ON CONFLICT DO NOTHING;

WITH version_row AS (SELECT id FROM admin_menu_versions WHERE version_no = 1)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label,
    action_kind, enabled, sort_order
)
SELECT md5((SELECT id::text FROM version_row) || ':' || button.id::text)::uuid,
       (SELECT id FROM version_row), button.menu_node_id, button.action_key,
       button.action_label, button.action_kind, button.enabled, button.sort_order
  FROM admin_menu_draft_button_permissions button
 WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130035'
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN ('hal.escalation.read', 'hal.escalation.write')
 WHERE lower(role.role_code) IN ('warehouse_manager', 'system_admin')
ON CONFLICT DO NOTHING;
