-- US-AL-004：看板仓库范围、统计/GSP 查询审计和 Excel/PDF 异步导出。

CREATE TABLE IF NOT EXISTS auth_user_warehouse_scopes (
    user_id      UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    owner_id     UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    warehouse_id UUID NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, owner_id, warehouse_id),
    FOREIGN KEY (user_id, owner_id)
        REFERENCES auth_user_owner_bindings(user_id, owner_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS alert_report_exports (
    id                         UUID PRIMARY KEY,
    owner_id                   UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    requested_by               UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT,
    format                     TEXT NOT NULL CHECK (format IN ('excel', 'pdf')),
    status                     TEXT NOT NULL CHECK (status IN ('queued', 'processing', 'ready', 'failed')),
    filters                    JSONB NOT NULL,
    row_count                  BIGINT NOT NULL CHECK (row_count >= 0),
    content                    BYTEA,
    content_type               TEXT,
    filename                   TEXT,
    download_token             UUID NOT NULL UNIQUE,
    recipient_email            TEXT,
    email_notification_status  TEXT,
    error_message              TEXT,
    created_at                 TIMESTAMPTZ NOT NULL,
    updated_at                 TIMESTAMPTZ NOT NULL,
    completed_at               TIMESTAMPTZ,
    expires_at                 TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS alert_statistics_snapshots (
    owner_id     UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    filter_key   TEXT NOT NULL,
    filters      JSONB NOT NULL,
    payload      JSONB NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (owner_id, filter_key)
);

CREATE INDEX IF NOT EXISTS alert_report_exports_owner_status_idx
    ON alert_report_exports (owner_id, status, created_at DESC);

GRANT SELECT, INSERT, DELETE ON auth_user_warehouse_scopes TO wms_app;
GRANT SELECT, INSERT, UPDATE ON alert_report_exports TO wms_app;
GRANT SELECT, INSERT, UPDATE ON alert_statistics_snapshots TO wms_app;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:hal.alert.read.all')::uuid, 'hal.alert.read.all', 'H-AL 全仓告警查询'),
    (md5('auth_permission:hal.alert.report')::uuid, 'hal.alert.report', 'H-AL 告警统计与导出')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission ON (
      (lower(role.role_code) = 'system_admin')
      OR (lower(role.role_code) IN ('warehouse_manager', 'gsp_auditor')
          AND permission.permission_code = 'hal.alert.report')
      OR (lower(role.role_code) = 'gsp_auditor'
          AND permission.permission_code IN ('hal.alert.read', 'hal.alert.read.all'))
  )
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_hal_permissions_to_warehouse_manager()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT NEW.id, permission.id
      FROM auth_permissions permission
     WHERE (lower(NEW.role_code) = 'system_admin')
        OR (lower(NEW.role_code) = 'warehouse_manager' AND permission.permission_code IN (
            'menu.platform.hal', 'hal.alert-definition.read', 'hal.alert-definition.write',
            'hal.alert.read', 'hal.alert.handle', 'hal.escalation.read',
            'hal.escalation.write', 'hal.alert.report'
        ))
        OR (lower(NEW.role_code) = 'gsp_auditor' AND permission.permission_code IN (
            'menu.platform.hal', 'hal.alert.read', 'hal.alert.read.all', 'hal.alert.report'
        ))
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION seed_hal_export_notification_config(target_owner_id UUID)
RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO h4_notification_configs (
        id, owner_id, event_type, enabled, template, recipient_rule,
        channels, created_by, updated_by
    ) VALUES (
        md5(target_owner_id::text || ':hal-alert-export-ready')::uuid,
        target_owner_id, 'hal.alert.export.ready', TRUE,
        '告警报表已生成：{{download_url}}', '{}'::jsonb,
        ARRAY['wechat', 'email']::text[],
        '00000000-0000-0000-0000-000000000000'::uuid,
        '00000000-0000-0000-0000-000000000000'::uuid
    )
    ON CONFLICT (owner_id, event_type) DO NOTHING;
END;
$$;

CREATE OR REPLACE FUNCTION seed_hal_export_notification_config_for_new_owner()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    PERFORM seed_hal_export_notification_config(NEW.id);
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_owners_seed_hal_export_notification_config ON auth_owners;
CREATE TRIGGER auth_owners_seed_hal_export_notification_config
AFTER INSERT ON auth_owners FOR EACH ROW
EXECUTE FUNCTION seed_hal_export_notification_config_for_new_owner();

SELECT seed_hal_export_notification_config(id) FROM auth_owners;
