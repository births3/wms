-- 202608180002_phase1_container_quality_lock_reason_dictionaries.sql
-- Phase 1 Ticket 02: M1 系统字典挂载容器质量锁原因与管理端维护

INSERT INTO system_dictionary_categories (
    dict_code,
    dict_name,
    enabled,
    control_level,
    param_schema,
    scope_mode,
    override_policy,
    sort_order,
    remark
)
VALUES
    (
        'container_quarantine_reason',
        '容器隔离原因',
        TRUE,
        'controlled',
        '{"required": [], "properties": {}}'::jsonb,
        'owner_extensible',
        '{"allowed_owner_params": []}'::jsonb,
        37,
        'US-M1-004b 容器质量锁隔离原因字典'
    ),
    (
        'container_rejected_reason',
        '容器不合格原因',
        TRUE,
        'controlled',
        '{"required": [], "properties": {}}'::jsonb,
        'owner_extensible',
        '{"allowed_owner_params": []}'::jsonb,
        38,
        'US-M1-004b 容器质量锁不合格原因字典'
    )
ON CONFLICT (dict_code) DO UPDATE
SET dict_name = EXCLUDED.dict_name,
    enabled = EXCLUDED.enabled,
    control_level = EXCLUDED.control_level,
    param_schema = EXCLUDED.param_schema,
    scope_mode = EXCLUDED.scope_mode,
    override_policy = EXCLUDED.override_policy,
    sort_order = EXCLUDED.sort_order,
    remark = EXCLUDED.remark,
    updated_at = now();

INSERT INTO system_dictionary_items (
    id,
    dict_code,
    item_code,
    item_name,
    enabled,
    owner_id,
    sort_order,
    params,
    source,
    created_at,
    updated_at
)
SELECT
    seed.id,
    seed.dict_code,
    seed.item_code,
    seed.item_name,
    TRUE,
    NULL,
    seed.sort_order,
    '{}'::jsonb,
    'global',
    now(),
    now()
FROM (
    VALUES
        -- container_quarantine_reason 隔离原因预置项
        ('10000000-0000-0000-0000-000000000071'::uuid, 'container_quarantine_reason', 'temp_anomaly', '温控异常', 10),
        ('10000000-0000-0000-0000-000000000072'::uuid, 'container_quarantine_reason', 'damaged_pending_inspect', '包装破损待检', 20),
        ('10000000-0000-0000-0000-000000000073'::uuid, 'container_quarantine_reason', 'sales_return_pending', '销退待验', 30),
        ('10000000-0000-0000-0000-000000000074'::uuid, 'container_quarantine_reason', 'routine_sampling', '例行抽样', 40),
        -- container_rejected_reason 不合格原因预置项
        ('10000000-0000-0000-0000-000000000075'::uuid, 'container_rejected_reason', 'expired', '药品过期', 10),
        ('10000000-0000-0000-0000-000000000076'::uuid, 'container_rejected_reason', 'damaged_leakage', '破损泄漏', 20),
        ('10000000-0000-0000-0000-000000000077'::uuid, 'container_rejected_reason', 'inspection_failed', '检验不合格', 30),
        ('10000000-0000-0000-0000-000000000078'::uuid, 'container_rejected_reason', 'regulatory_recall', '药监召回', 40)
) AS seed(id, dict_code, item_code, item_name, sort_order)
WHERE NOT EXISTS (
    SELECT 1
      FROM system_dictionary_items existing
     WHERE existing.dict_code = seed.dict_code
       AND existing.item_code = seed.item_code
       AND existing.owner_id IS NULL
);
