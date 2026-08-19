-- H4 企业微信通知与审批通道。

CREATE TABLE IF NOT EXISTS h4_notification_configs (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL,
    event_type      TEXT NOT NULL,
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    template        TEXT NOT NULL,
    recipient_rule  JSONB NOT NULL,
    channels        TEXT[] NOT NULL DEFAULT ARRAY['wechat']::TEXT[],
    created_by      UUID NOT NULL,
    updated_by      UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    version         BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, event_type),
    CHECK (length(trim(event_type)) > 0),
    CHECK (array_position(channels, 'wechat') IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS h4_notification_configs_owner_event_idx
    ON h4_notification_configs (owner_id, event_type, enabled);

CREATE TABLE IF NOT EXISTS h4_notification_records (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL,
    config_id        UUID REFERENCES h4_notification_configs(id),
    event_type       TEXT NOT NULL,
    dedupe_key       TEXT NOT NULL,
    recipient        TEXT NOT NULL,
    channel          TEXT NOT NULL,
    content          TEXT NOT NULL,
    content_summary  TEXT NOT NULL,
    status           TEXT NOT NULL CHECK (status IN ('success', 'failed', 'retrying')),
    retry_count      INT NOT NULL DEFAULT 0,
    failure_reason   TEXT,
    sent_at          TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, event_type, recipient, dedupe_key)
);

CREATE INDEX IF NOT EXISTS h4_notification_records_query_idx
    ON h4_notification_records (owner_id, created_at DESC, event_type, status);

CREATE TABLE IF NOT EXISTS h4_approval_records (
    id                    UUID PRIMARY KEY,
    owner_id              UUID NOT NULL,
    scenario              TEXT NOT NULL,
    business_ref          TEXT NOT NULL,
    dedupe_key            TEXT NOT NULL,
    approver_user         TEXT NOT NULL,
    process_id            TEXT NOT NULL,
    callback_path         TEXT NOT NULL,
    summary               TEXT NOT NULL,
    status                TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'rejected', 'failed')),
    opinion               TEXT,
    external_approval_id  TEXT,
    approved_by           TEXT,
    approved_at           TIMESTAMPTZ,
    failure_reason        TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, scenario, business_ref, dedupe_key)
);

CREATE INDEX IF NOT EXISTS h4_approval_records_query_idx
    ON h4_approval_records (owner_id, created_at DESC, scenario, status);

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    ('00000000-0000-0000-0000-000000004401', 'h4.notify.read', 'H4 通知读取'),
    ('00000000-0000-0000-0000-000000004402', 'h4.notify.write', 'H4 通知配置维护'),
    ('00000000-0000-0000-0000-000000004403', 'h4.notify.send', 'H4 通知发送'),
    ('00000000-0000-0000-0000-000000004404', 'h4.approval.write', 'H4 审批回写')
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    ('00000000-0000-0000-0000-000000130019', '00000000-0000-0000-0000-000000120008', 3, 'platform.wechat_notify_configs', 'platform/capability/wechat_notify_configs', 'H4 通知配置', 'h4-notify-configs', 'Bell', 'h4.notify.read', 30, TRUE),
    ('00000000-0000-0000-0000-000000130020', '00000000-0000-0000-0000-000000120008', 3, 'platform.wechat_notify_records', 'platform/capability/wechat_notify_records', 'H4 发送记录', 'h4-notify-records', 'History', 'h4.notify.read', 40, TRUE)
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
        ('edit', '编辑', 'standard', 40),
        ('disable', '启停', 'standard', 60),
        ('detail', '详情', 'standard', 70),
        ('export', '导出', 'standard', 80),
        ('field', '字段', 'standard', 110),
        ('view', '视图', 'standard', 120)
) AS action(key, label, kind, sort_order)
WHERE node.id IN (
    '00000000-0000-0000-0000-000000130019',
    '00000000-0000-0000-0000-000000130020'
)
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
VALUES
    (md5('00000000-0000-0000-0000-000000130019:test_send')::uuid, '00000000-0000-0000-0000-000000130019', 'test_send', '试发', 'private', TRUE, 200),
    (md5('00000000-0000-0000-0000-000000130020:resend')::uuid, '00000000-0000-0000-0000-000000130020', 'resend', '重发', 'private', TRUE, 200)
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
    '00000000-0000-0000-0000-000000130019',
    '00000000-0000-0000-0000-000000130020'
)
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
WHERE button.menu_node_id IN (
    '00000000-0000-0000-0000-000000130019',
    '00000000-0000-0000-0000-000000130020'
)
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE ON
    h4_notification_configs,
    h4_notification_records,
    h4_approval_records
TO wms_app;
