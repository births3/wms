-- US-M3-002: 近效期预警默认值及货主覆盖。

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
    'inventory_policy',
    '库存管理参数',
    TRUE,
    'controlled',
    '{
        "required": ["warning_days"],
        "properties": {
            "warning_days": {"type": "integer", "minimum": 1, "maximum": 3650}
        }
    }'::jsonb,
    'owner_override',
    '{"allowed_owner_params": ["warning_days"]}'::jsonb,
    45,
    'US-M3-002 近效期预警阈值，货主级覆盖优先于全局'
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
VALUES (
    '10000000-0000-0000-0000-000000000145'::uuid,
    'inventory_policy',
    'expiry_warning_days',
    '近效期预警天数',
    TRUE,
    NULL,
    '{"warning_days": 180}'::jsonb,
    'global',
    now(),
    now()
)
ON CONFLICT (dict_code, item_code, COALESCE(owner_id, '00000000-0000-0000-0000-000000000000'::uuid))
DO NOTHING;
