-- US-AL-001：告警定义注册，以及 GSP 强制告警的数据库约束。

CREATE TABLE IF NOT EXISTS alert_definitions (
    id                       UUID PRIMARY KEY,
    owner_id                 UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    alert_code               TEXT NOT NULL,
    name                     TEXT NOT NULL,
    event_type               TEXT NOT NULL,
    condition_expression     TEXT NOT NULL,
    default_severity         TEXT NOT NULL,
    recipient_roles          TEXT[] NOT NULL DEFAULT '{}',
    escalation_ref           TEXT,
    silence_period_seconds   BIGINT NOT NULL DEFAULT 0,
    is_disable_allowed       BOOLEAN NOT NULL DEFAULT TRUE,
    message_template         TEXT NOT NULL,
    is_gsp_forced            BOOLEAN NOT NULL DEFAULT FALSE,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(alert_code) <> ''),
    CHECK (btrim(name) <> ''),
    CHECK (btrim(event_type) <> ''),
    CHECK (btrim(condition_expression) <> ''),
    CHECK (btrim(default_severity) <> ''),
    CHECK (btrim(message_template) <> ''),
    CHECK (silence_period_seconds >= 0),
    CONSTRAINT alert_definitions_gsp_disable_check
        CHECK (NOT is_gsp_forced OR NOT is_disable_allowed),
    UNIQUE (owner_id, alert_code)
);

CREATE TABLE IF NOT EXISTS alert_definition_triggers (
    id                   UUID PRIMARY KEY,
    alert_definition_id  UUID NOT NULL
        REFERENCES alert_definitions(id) ON DELETE RESTRICT,
    event_type           TEXT NOT NULL,
    occurred_at          TIMESTAMPTZ NOT NULL,
    payload              JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(event_type) <> '')
);

CREATE INDEX IF NOT EXISTS alert_definitions_owner_idx
    ON alert_definitions (owner_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS alert_definition_triggers_definition_idx
    ON alert_definition_triggers (alert_definition_id, occurred_at DESC);

GRANT SELECT, INSERT, UPDATE, DELETE ON alert_definitions TO wms_app;
GRANT SELECT, INSERT ON alert_definition_triggers TO wms_app;

CREATE OR REPLACE FUNCTION seed_h1_gsp_alert_definitions(target_owner_id UUID)
RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO alert_definitions (
        id, owner_id, alert_code, name, event_type, condition_expression,
        default_severity, recipient_roles, escalation_ref, silence_period_seconds,
        is_disable_allowed, message_template, is_gsp_forced
    )
    SELECT md5(target_owner_id::text || ':gsp-alert:' || defaults.alert_code)::uuid,
           target_owner_id, defaults.*
      FROM (VALUES
        ('qualification_expiry_30d', '资质有效期不足30天', 'qualification.expiry', 'expiry < current_date + 30', 'warning', ARRAY['warehouse_manager']::TEXT[], 'GSP 6.79', 0::BIGINT, FALSE, '资质有效期不足30天：{{subject}}', TRUE),
        ('near_expiry_6m', '近效期不足6个月', 'inventory.near_expiry', 'expiry < current_date + 6 months', 'warning', ARRAY['warehouse_manager']::TEXT[], 'GSP 7.100', 0::BIGINT, FALSE, '近效期不足6个月：{{batch}}', TRUE),
        ('maintenance_overdue_3d', '养护超期超过3天', 'maintenance.overdue', 'planned_at + 3 days < now()', 'critical', ARRAY['warehouse_manager']::TEXT[], 'GSP 7.97', 0::BIGINT, FALSE, '养护任务已超期3天：{{task_id}}', TRUE),
        ('quarantine_overdue_24h', '不合格隔离超过24小时未处理', 'quarantine.overdue', 'isolated_at + 24 hours < now()', 'critical', ARRAY['warehouse_manager']::TEXT[], 'GSP 8.119 / 不-1', 0::BIGINT, FALSE, '不合格品隔离超过24小时未处理：{{batch}}', TRUE),
        ('cold_chain_break_received', '冷链断链事件', 'cold_chain.break', 'event_received = true', 'critical', ARRAY['warehouse_manager']::TEXT[], 'GSP 冷-7', 0::BIGINT, FALSE, '收到冷链断链事件：{{event_id}}', TRUE),
        ('destruction_approval_overdue_48h', '销毁审批超过48小时', 'destruction.approval', 'submitted_at + 48 hours < now()', 'critical', ARRAY['warehouse_manager']::TEXT[], 'GSP 7.103', 0::BIGINT, FALSE, '销毁审批已超过48小时：{{request_id}}', TRUE)
    ) AS defaults(alert_code, name, event_type, condition_expression, default_severity,
                  recipient_roles, escalation_ref, silence_period_seconds,
                  is_disable_allowed, message_template, is_gsp_forced)
    ON CONFLICT (owner_id, alert_code) DO NOTHING;
END;
$$;

CREATE OR REPLACE FUNCTION seed_h1_gsp_alert_definitions_for_new_owner()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    PERFORM seed_h1_gsp_alert_definitions(NEW.id);
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_owners_seed_h1_gsp_alert_definitions ON auth_owners;
CREATE TRIGGER auth_owners_seed_h1_gsp_alert_definitions
AFTER INSERT ON auth_owners FOR EACH ROW
EXECUTE FUNCTION seed_h1_gsp_alert_definitions_for_new_owner();

SELECT seed_h1_gsp_alert_definitions(id) FROM auth_owners;
