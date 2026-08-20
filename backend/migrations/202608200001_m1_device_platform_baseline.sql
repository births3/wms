-- Phase 3 设备中台基线（ADR-0048 Phase 3 / device-platform-phase3-spec §7）。
-- iot_devices / wcs_tasks / iot_event_logs 三表建齐 + agv_unreachable_at 列
-- + 权限 / 字典 / M-CG 编号规则 / H4 告警种子。
-- v1 前直接改基线（ADR-0038）；iot_event_logs 纯审计追加流，只授予 INSERT。

-- ============================================================
-- 1. iot_devices（设备主档，仓库级共享资产，不按货主隔离）
-- ============================================================
CREATE TABLE IF NOT EXISTS iot_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    warehouse_id UUID NOT NULL,
    device_code VARCHAR(64) NOT NULL,
    device_type VARCHAR(16) NOT NULL,
    vendor VARCHAR(64),
    model VARCHAR(64),
    protocol VARCHAR(16) NOT NULL,
    ip_address VARCHAR(64),
    port INT,
    extra_config JSONB NOT NULL DEFAULT '{}',
    online_status VARCHAR(16) NOT NULL DEFAULT 'offline',
    last_heartbeat_at TIMESTAMPTZ,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (warehouse_id, device_code),
    CHECK (device_type IN ('agv', 'ptl_light', 'dws', 'rfid_antenna', 'stacker')),
    CHECK (online_status IN ('online', 'offline', 'disabled')),
    CHECK (port IS NULL OR (port > 0 AND port < 65536))
);

CREATE INDEX IF NOT EXISTS iot_devices_warehouse_type_idx
    ON iot_devices (warehouse_id, device_type);
CREATE INDEX IF NOT EXISTS iot_devices_warehouse_status_idx
    ON iot_devices (warehouse_id, online_status);

-- ============================================================
-- 2. wcs_tasks（指令任务，多货主隔离）
-- ============================================================
CREATE TABLE IF NOT EXISTS wcs_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL,
    task_no VARCHAR(64) NOT NULL,
    task_type VARCHAR(16) NOT NULL,
    device_id UUID NOT NULL REFERENCES iot_devices(id),
    location_id UUID,
    business_ref_type VARCHAR(32),
    business_ref_no VARCHAR(64),
    payload JSONB NOT NULL DEFAULT '{}',
    status VARCHAR(16) NOT NULL DEFAULT 'pending',
    ack_payload JSONB DEFAULT '{}',
    error_code VARCHAR(32),
    error_message TEXT,
    retry_count INT NOT NULL DEFAULT 0,
    max_retries INT NOT NULL DEFAULT 3,
    idempotency_key VARCHAR(128) NOT NULL,
    sent_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, task_no),
    UNIQUE (idempotency_key),
    CHECK (task_type IN ('pod_move', 'ptl_light_on', 'ptl_light_off', 'sorter_divert', 'dws_weigh', 'rfid_scan')),
    CHECK (status IN ('pending', 'sent', 'executing', 'succeeded', 'failed', 'timeout')),
    CHECK (retry_count >= 0 AND retry_count <= max_retries)
);

CREATE INDEX IF NOT EXISTS wcs_tasks_owner_status_idx
    ON wcs_tasks (owner_id, status, updated_at);
CREATE INDEX IF NOT EXISTS wcs_tasks_device_status_idx
    ON wcs_tasks (device_id, status);
CREATE INDEX IF NOT EXISTS wcs_tasks_business_ref_idx
    ON wcs_tasks (owner_id, business_ref_type, business_ref_no);
CREATE UNIQUE INDEX IF NOT EXISTS wcs_tasks_active_ptl_light_on_device_uq
    ON wcs_tasks (device_id)
    WHERE task_type = 'ptl_light_on'
      AND status IN ('pending', 'sent', 'executing', 'timeout');
CREATE UNIQUE INDEX IF NOT EXISTS wcs_tasks_active_pod_move_code_uq
    ON wcs_tasks ((payload->>'pod_code'))
    WHERE task_type = 'pod_move'
      AND status IN ('pending', 'sent', 'executing', 'timeout');

-- ============================================================
-- 3. iot_event_logs（硬件事件，纯审计追加流，只 INSERT）
-- ============================================================
CREATE TABLE IF NOT EXISTS iot_event_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    warehouse_id UUID NOT NULL,
    device_id UUID NOT NULL REFERENCES iot_devices(id),
    event_type VARCHAR(16) NOT NULL,
    task_id UUID REFERENCES wcs_tasks(id),
    location_id UUID,
    payload JSONB NOT NULL DEFAULT '{}',
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (event_type IN ('ptl_press', 'rfid_batch', 'dws_result', 'heartbeat'))
);

CREATE INDEX IF NOT EXISTS iot_event_logs_device_time_idx
    ON iot_event_logs (device_id, received_at);
CREATE INDEX IF NOT EXISTS iot_event_logs_task_idx
    ON iot_event_logs (task_id);
CREATE INDEX IF NOT EXISTS iot_event_logs_location_time_idx
    ON iot_event_logs (location_id, received_at);

-- ============================================================
-- 4. warehouse_locations.agv_unreachable_at（AGV 格口临时不可达标记）
-- ============================================================
ALTER TABLE warehouse_locations
    ADD COLUMN IF NOT EXISTS agv_unreachable_at TIMESTAMPTZ;

COMMENT ON COLUMN warehouse_locations.agv_unreachable_at IS
    'AGV 货架搬运中临时不可达标记：pod_move executing 置位，终态清除；不可达期间禁止格口账务动作';

-- ============================================================
-- 5. 权限种子（m1.device.manage / m1.device.monitor / m1.device-bind.manage）
-- ============================================================
INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:' || 'm1.device.manage')::uuid, 'm1.device.manage', 'M1 设备档案与指令管理'),
    (md5('auth_permission:' || 'm1.device.monitor')::uuid, 'm1.device.monitor', 'M1 设备大盘只读'),
    (md5('auth_permission:' || 'm1.device-bind.manage')::uuid, 'm1.device-bind.manage', 'M1 库位-设备点位绑定')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON (
        (
            lower(role.role_code) IN ('system_admin', 'warehouse_manager')
            AND permission.permission_code IN ('m1.device.manage', 'm1.device-bind.manage')
        )
        OR (
            lower(role.role_code) IN ('system_admin', 'warehouse_manager', 'custodian')
            AND permission.permission_code = 'm1.device.monitor'
        )
    )
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_m1_device_platform_permissions()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT NEW.id, permission.id
      FROM auth_permissions permission
     WHERE permission.permission_code IN ('m1.device.manage', 'm1.device-bind.manage')
       AND lower(NEW.role_code) IN ('system_admin', 'warehouse_manager')
    ON CONFLICT DO NOTHING;

    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT NEW.id, permission.id
      FROM auth_permissions permission
     WHERE permission.permission_code = 'm1.device.monitor'
       AND lower(NEW.role_code) IN ('system_admin', 'warehouse_manager', 'custodian')
    ON CONFLICT DO NOTHING;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_m1_device_platform_permissions ON auth_roles;
CREATE TRIGGER auth_roles_grant_m1_device_platform_permissions
AFTER INSERT ON auth_roles FOR EACH ROW
EXECUTE FUNCTION grant_m1_device_platform_permissions();

-- ============================================================
-- 6. 系统字典：document_type 增加 wcs_task
-- ============================================================
UPDATE system_dictionary_categories
   SET param_schema = jsonb_set(
           param_schema,
           '{properties,workflow_template,enum}',
           '["purchase_inbound", "sales_return", "other_inbound", "purchase_return_outbound", "sales_outbound", "sample_outbound", "other_outbound", "stock_loss", "stock_surplus", "quality_liaison", "lpn_container", "replenishment_task", "wcs_task"]'::jsonb
       ),
       updated_at = now()
 WHERE dict_code = 'document_type';

INSERT INTO system_dictionary_items (
    id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at
)
VALUES (
    '10000000-0000-0000-0000-00000000d001'::uuid,
    'document_type',
    'wcs_task',
    '设备指令任务',
    TRUE,
    NULL,
    '{"direction":"internal","workflow_template":"wcs_task","batch_policy":"none"}'::jsonb,
    'global',
    now(),
    now()
)
ON CONFLICT DO NOTHING;

-- ============================================================
-- 7. M-CG 编号规则：wcs_task
-- ============================================================
INSERT INTO document_number_rules (
    id, owner_id, document_type, rule_code, rule_name, template,
    reset_policy, sequence_width, sequence_mode, enabled, created_at, updated_at
)
VALUES (
    '10000000-0000-0000-0000-00000000d101'::uuid,
    NULL,
    'wcs_task',
    'GLOBAL-WCS-TASK',
    '设备指令任务默认编号',
    'WCST-{OWNER}-{YYYY}{MM}{DD}-{SEQ}',
    'daily',
    4,
    'no_gap',
    TRUE,
    now(),
    now()
)
ON CONFLICT DO NOTHING;

-- ============================================================
-- 8. H4 告警定义种子（六类）
-- ============================================================
CREATE OR REPLACE FUNCTION seed_m1_device_alert_definitions(target_owner_id UUID)
RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO alert_definitions (
        id, owner_id, alert_code, name, event_type, condition_expression,
        default_severity, recipient_roles, escalation_ref, silence_period_seconds,
        is_disable_allowed, message_template, is_gsp_forced
    )
    SELECT md5(target_owner_id::text || ':m1-device-alert:' || defaults.alert_code)::uuid,
           target_owner_id, defaults.*
      FROM (VALUES
        (
            'device_offline',
            '设备离线',
            'business.device_offline',
            '{"field":"offline_seconds","op":"gte","value":90}',
            'critical',
            ARRAY['warehouse_manager']::TEXT[],
            'm1-device-default',
            300::BIGINT,
            TRUE,
            '设备心跳超时离线：{{device_code}}',
            FALSE
        ),
        (
            'device_event_orphan',
            '设备孤儿事件',
            'business.device_event_orphan',
            '{"field":"orphan_minutes","op":"gte","value":1}',
            'warning',
            ARRAY['warehouse_manager']::TEXT[],
            'm1-device-default',
            300::BIGINT,
            TRUE,
            '设备事件无匹配指令任务：{{event_type}}/{{device_code}}',
            FALSE
        ),
        (
            'wcs_task_failed',
            '设备指令重试耗尽',
            'business.wcs_task_failed',
            '{"field":"retry_count","op":"gte","value":3}',
            'critical',
            ARRAY['warehouse_manager']::TEXT[],
            'm1-device-default',
            300::BIGINT,
            TRUE,
            '设备指令重试耗尽已失败：{{task_no}}',
            FALSE
        ),
        (
            'wcs_task_stalled',
            '设备停用致任务停滞',
            'business.wcs_task_stalled',
            '{"field":"stalled_minutes","op":"gte","value":5}',
            'warning',
            ARRAY['warehouse_manager']::TEXT[],
            'm1-device-default',
            300::BIGINT,
            TRUE,
            '设备停用，活跃指令停滞：{{task_no}}',
            FALSE
        ),
        (
            'ptl_qty_diff',
            '拍灯数量差异',
            'business.ptl_qty_diff',
            '{"field":"diff_ratio","op":"gte","value":0.1}',
            'warning',
            ARRAY['warehouse_manager']::TEXT[],
            'm1-device-default',
            300::BIGINT,
            TRUE,
            '拍灯数量与提示数量存在差异：{{task_no}}',
            FALSE
        ),
        (
            'agv_marker_inconsistent',
            'AGV 不可达标记不一致',
            'business.agv_marker_inconsistent',
            '{"field":"mismatch_count","op":"gte","value":1}',
            'critical',
            ARRAY['warehouse_manager']::TEXT[],
            'm1-device-default',
            300::BIGINT,
            TRUE,
            'AGV 不可达标记与活跃搬运任务不一致',
            FALSE
        )
      ) AS defaults(alert_code, name, event_type, condition_expression,
                    default_severity, recipient_roles, escalation_ref,
                    silence_period_seconds, is_disable_allowed, message_template,
                    is_gsp_forced)
    ON CONFLICT DO NOTHING;
END;
$$;

CREATE OR REPLACE FUNCTION seed_m1_device_alert_definitions_for_new_owner()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    PERFORM seed_m1_device_alert_definitions(NEW.id);
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_owners_seed_m1_device_alert_definitions ON auth_owners;
CREATE TRIGGER auth_owners_seed_m1_device_alert_definitions
AFTER INSERT ON auth_owners FOR EACH ROW
EXECUTE FUNCTION seed_m1_device_alert_definitions_for_new_owner();

SELECT seed_m1_device_alert_definitions(id) FROM auth_owners;

-- ============================================================
-- 9. GRANT（iot_event_logs 只 SELECT/INSERT；设备与指令无 DELETE）
-- ============================================================
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        GRANT SELECT, INSERT, UPDATE ON iot_devices TO wms_app;
        GRANT SELECT, INSERT, UPDATE ON wcs_tasks TO wms_app;
        GRANT SELECT, INSERT ON iot_event_logs TO wms_app;
    END IF;
END
$$;
