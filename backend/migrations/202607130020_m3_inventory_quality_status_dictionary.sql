-- M3-003 inventory quality statuses are maintained through the system dictionary.

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
VALUES (
    'inventory_quality_status',
    '库存质量状态',
    TRUE,
    'controlled',
    '{"required": [], "properties": {}}'::jsonb,
    'owner_override',
    '{"allowed_owner_params": []}'::jsonb,
    34,
    'M3-003 库存质量状态默认字典'
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
    params,
    source,
    created_at,
    updated_at
)
SELECT
    seed.id,
    'inventory_quality_status',
    seed.item_code,
    seed.item_name,
    TRUE,
    NULL,
    '{}'::jsonb,
    'global',
    now(),
    now()
FROM (
    VALUES
        ('10000000-0000-0000-0000-000000000046'::uuid, 'loss_deducted', '报损扣减'),
        ('10000000-0000-0000-0000-000000000047'::uuid, 'pending_destruction', '待销毁'),
        ('10000000-0000-0000-0000-000000000048'::uuid, 'qualified', '合格'),
        ('10000000-0000-0000-0000-000000000049'::uuid, 'quarantined', '隔离'),
        ('10000000-0000-0000-0000-000000000050'::uuid, 'unqualified', '不合格')
) AS seed(id, item_code, item_name)
WHERE NOT EXISTS (
    SELECT 1
      FROM system_dictionary_items existing
     WHERE existing.dict_code = 'inventory_quality_status'
       AND existing.item_code = seed.item_code
       AND existing.owner_id IS NULL
);
