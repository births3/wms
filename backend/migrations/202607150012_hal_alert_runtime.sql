-- US-AL-002：H2 事件驱动的告警实例、生命周期、静默去重与 H4 通知状态。

CREATE TABLE IF NOT EXISTS alert_instances (
    id                     UUID PRIMARY KEY,
    owner_id               UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    alert_definition_id    UUID NOT NULL REFERENCES alert_definitions(id) ON DELETE RESTRICT,
    alert_code             TEXT NOT NULL,
    severity               TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    event_id               UUID NOT NULL REFERENCES event_bus_event(id) ON DELETE RESTRICT,
    event_type             TEXT NOT NULL,
    resource_type          TEXT NOT NULL,
    resource_id            TEXT NOT NULL,
    resource_path          TEXT,
    warehouse_id           UUID,
    event_payload          JSONB NOT NULL,
    recipients             TEXT[] NOT NULL DEFAULT '{}',
    status                 TEXT NOT NULL CHECK (status IN (
        'triggered', 'notified', 'acknowledged', 'handling', 'closed', 'ignored',
        'timed_out', 'escalated', 'notification_failed'
    )),
    dedup_key              TEXT NOT NULL,
    escalation_level       INT NOT NULL DEFAULT 0 CHECK (escalation_level BETWEEN 0 AND 3),
    action_description     TEXT,
    ignored_reason         TEXT,
    close_reason           TEXT,
    triggered_at           TIMESTAMPTZ NOT NULL,
    notified_at            TIMESTAMPTZ,
    acknowledged_at        TIMESTAMPTZ,
    handled_at             TIMESTAMPTZ,
    closed_at              TIMESTAMPTZ,
    last_escalated_at      TIMESTAMPTZ,
    created_at             TIMESTAMPTZ NOT NULL,
    updated_at             TIMESTAMPTZ NOT NULL,
    UNIQUE (owner_id, dedup_key)
);

CREATE INDEX IF NOT EXISTS alert_instances_owner_active_idx
    ON alert_instances (owner_id, status, severity, triggered_at DESC);
CREATE INDEX IF NOT EXISTS alert_instances_owner_resource_idx
    ON alert_instances (owner_id, alert_code, resource_type, resource_id, triggered_at DESC);

CREATE TABLE IF NOT EXISTS alert_lifecycle_events (
    event_sequence     BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    alert_instance_id  UUID NOT NULL REFERENCES alert_instances(id) ON DELETE RESTRICT,
    from_status        TEXT,
    to_status          TEXT NOT NULL,
    action_description TEXT,
    actor_id           UUID,
    actor_name         TEXT NOT NULL,
    occurred_at        TIMESTAMPTZ NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS alert_lifecycle_events_instance_idx
    ON alert_lifecycle_events (alert_instance_id, occurred_at, event_sequence);

CREATE OR REPLACE FUNCTION prevent_alert_lifecycle_event_mutation()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'alert_lifecycle_events is append-only'
        USING ERRCODE = '42501';
END;
$$;

DROP TRIGGER IF EXISTS alert_lifecycle_events_append_only ON alert_lifecycle_events;
CREATE TRIGGER alert_lifecycle_events_append_only
BEFORE UPDATE OR DELETE ON alert_lifecycle_events
FOR EACH ROW EXECUTE FUNCTION prevent_alert_lifecycle_event_mutation();

GRANT SELECT, INSERT, UPDATE ON alert_instances TO wms_app;
GRANT SELECT, INSERT ON alert_lifecycle_events TO wms_app;
GRANT USAGE, SELECT ON SEQUENCE alert_lifecycle_events_event_sequence_seq TO wms_app;

CREATE OR REPLACE FUNCTION seed_hal_event_subscription(target_owner_id UUID)
RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO event_bus_subscription (
        id, owner_id, subscriber_key, event_pattern, active
    ) VALUES (
        md5(target_owner_id::text || ':hal-alert-engine')::uuid,
        target_owner_id, 'hal-alert-engine', 'business.*', TRUE
    )
    ON CONFLICT (owner_id, subscriber_key)
    DO UPDATE SET event_pattern = EXCLUDED.event_pattern, active = TRUE, updated_at = now();
END;
$$;

CREATE OR REPLACE FUNCTION seed_hal_event_subscription_for_new_owner()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    PERFORM seed_hal_event_subscription(NEW.id);
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_owners_seed_hal_event_subscription ON auth_owners;
CREATE TRIGGER auth_owners_seed_hal_event_subscription
AFTER INSERT ON auth_owners FOR EACH ROW
EXECUTE FUNCTION seed_hal_event_subscription_for_new_owner();

SELECT seed_hal_event_subscription(id) FROM auth_owners;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:hal.alert.read')::uuid, 'hal.alert.read', 'H-AL 告警实例查询'),
    (md5('auth_permission:hal.alert.handle')::uuid, 'hal.alert.handle', 'H-AL 告警确认与处置')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN ('hal.alert.read', 'hal.alert.handle')
 WHERE lower(role.role_code) IN ('warehouse_manager', 'system_admin')
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
             'hal.alert.handle'
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
    '00000000-0000-0000-0000-000000130034',
    '00000000-0000-0000-0000-000000120016',
    3,
    'platform.alert_dashboard',
    'platform/capability/alert_dashboard',
    'H-AL 告警看板',
    'hal-alert-dashboard',
    'BellRing',
    'hal.alert.read',
    60,
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
        ('acknowledge', '确认', 30),
        ('handle', '处理', 40),
        ('close', '关闭', 50),
        ('ignore', '忽略', 60),
        ('export', '导出', 70),
        ('field', '字段', 80),
        ('view', '视图', 90)
) AS action(key, label, sort_order)
WHERE node.id = '00000000-0000-0000-0000-000000130034'
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
    (SELECT id FROM version_row), node.id, node.parent_id, node.level,
    node.code, node.path, node.title, node.view_id, node.icon_key,
    node.permission_key, node.sort_order, node.enabled, node.created_at, node.updated_at
FROM admin_menu_draft_nodes node
WHERE node.id = '00000000-0000-0000-0000-000000130034'
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
    (SELECT id FROM version_row), button.menu_node_id, button.action_key,
    button.action_label, button.action_kind, button.enabled, button.sort_order
FROM admin_menu_draft_button_permissions button
WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130034'
ON CONFLICT DO NOTHING;
