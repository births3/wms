-- T02: replenishment_tasks, strategy min-max CHECK, permissions, document_type.

ALTER TABLE replenishment_strategies
    DROP CONSTRAINT IF EXISTS replenishment_strategies_minmax_check;
ALTER TABLE replenishment_strategies
    ADD CONSTRAINT replenishment_strategies_minmax_check
    CHECK (min_safety_threshold >= 0 AND max_replenish_target > min_safety_threshold);

CREATE TABLE IF NOT EXISTS replenishment_tasks (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL,
    task_no VARCHAR(64) NOT NULL,
    trigger_mode VARCHAR(16) NOT NULL,
    priority VARCHAR(16) NOT NULL DEFAULT 'normal',
    strategy_id UUID REFERENCES replenishment_strategies(id),
    source_location_id UUID NOT NULL,
    source_batch_id UUID NOT NULL,
    source_lpn_id UUID,
    target_location_id UUID NOT NULL,
    product_id UUID NOT NULL,
    batch_no VARCHAR(64) NOT NULL,
    qty NUMERIC(19, 4) NOT NULL CHECK (qty > 0),
    picked_qty NUMERIC(19, 4) NOT NULL DEFAULT 0,
    done_qty NUMERIC(19, 4) NOT NULL DEFAULT 0,
    status VARCHAR(16) NOT NULL DEFAULT 'pending',
    operator_id UUID,
    wave_id UUID,
    outbound_order_id UUID,
    outbound_line_no INT,
    claimed_at TIMESTAMPTZ,
    last_progress_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,
    cancel_reason TEXT,
    return_reason VARCHAR(32),
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, task_no),
    CHECK (trigger_mode IN ('min_max', 'wave_gap', 'manual')),
    CHECK (priority IN ('normal', 'urgent')),
    CHECK (status IN ('pending', 'in_progress', 'suspended', 'done', 'cancelled')),
    CHECK (picked_qty >= 0 AND done_qty >= 0 AND picked_qty + done_qty <= qty),
    CHECK (return_reason IS NULL OR return_reason IN ('source_mismatch', 'target_blocked', 'other')),
    CHECK (outbound_line_no IS NULL OR outbound_line_no > 0)
);

CREATE INDEX IF NOT EXISTS replenishment_tasks_owner_status_target_idx
    ON replenishment_tasks (owner_id, status, target_location_id);
CREATE INDEX IF NOT EXISTS replenishment_tasks_owner_source_batch_idx
    ON replenishment_tasks (owner_id, source_batch_id);
CREATE INDEX IF NOT EXISTS replenishment_tasks_owner_priority_created_idx
    ON replenishment_tasks (owner_id, priority, created_at);

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON replenishment_tasks TO wms_app;
    END IF;
END
$$;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:' || 'm3.replenishment.manage')::uuid, 'm3.replenishment.manage', 'M3 补货策略与大盘'),
    (md5('auth_permission:' || 'm3.replenishment.execute')::uuid, 'm3.replenishment.execute', 'M3 补货领取执行')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON (
        (
            lower(role.role_code) IN ('system_admin', 'warehouse_manager')
            AND permission.permission_code = 'm3.replenishment.manage'
        )
        OR (
            lower(role.role_code) IN ('system_admin', 'warehouse_manager', 'custodian')
            AND permission.permission_code = 'm3.replenishment.execute'
        )
    )
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_m3_replenishment_permissions()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT NEW.id, permission.id
      FROM auth_permissions permission
     WHERE permission.permission_code = 'm3.replenishment.manage'
       AND lower(NEW.role_code) IN ('system_admin', 'warehouse_manager')
    ON CONFLICT DO NOTHING;

    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT NEW.id, permission.id
      FROM auth_permissions permission
     WHERE permission.permission_code = 'm3.replenishment.execute'
       AND lower(NEW.role_code) IN ('system_admin', 'warehouse_manager', 'custodian')
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_m3_replenishment_permissions ON auth_roles;
CREATE TRIGGER auth_roles_grant_m3_replenishment_permissions
AFTER INSERT ON auth_roles
FOR EACH ROW
EXECUTE FUNCTION grant_m3_replenishment_permissions();

UPDATE system_dictionary_categories
   SET param_schema = jsonb_set(
           param_schema,
           '{properties,workflow_template,enum}',
           '["purchase_inbound", "sales_return", "other_inbound", "purchase_return_outbound", "sales_outbound", "sample_outbound", "other_outbound", "stock_loss", "stock_surplus", "quality_liaison", "lpn_container", "replenishment_task"]'::jsonb
       ),
       updated_at = now()
 WHERE dict_code = 'document_type';

INSERT INTO system_dictionary_items (
    id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at
)
VALUES (
    '10000000-0000-0000-0000-00000000c001'::uuid,
    'document_type',
    'replenishment_task',
    '补货任务',
    TRUE,
    NULL,
    '{"direction":"internal","workflow_template":"replenishment_task","batch_policy":"none"}'::jsonb,
    'global',
    now(),
    now()
)
ON CONFLICT DO NOTHING;

INSERT INTO document_number_rules (
    id, owner_id, document_type, rule_code, rule_name, template,
    reset_policy, sequence_width, sequence_mode, enabled, created_at, updated_at
)
VALUES (
    '10000000-0000-0000-0000-00000000c101'::uuid,
    NULL,
    'replenishment_task',
    'GLOBAL-REPLENISH-TASK',
    '补货任务默认编号',
    'RT-{OWNER}-{YYYY}{MM}{DD}-{SEQ}',
    'daily',
    4,
    'no_gap',
    TRUE,
    now(),
    now()
)
ON CONFLICT DO NOTHING;
