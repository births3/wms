-- Seed US-M1-010 special drug category dictionary.

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
    'special_drug_category',
    '特殊药品分类',
    TRUE,
    'gsp_critical',
    '{
        "required": ["requires_dual_sign"],
        "properties": {
            "requires_dual_sign": {
                "type": "boolean"
            }
        }
    }'::jsonb,
    'owner_override',
    '{"allowed_owner_params": ["requires_dual_sign"]}'::jsonb,
    20,
    'US-M1-010 特殊药品分类预置字典'
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
    'special_drug_category',
    seed.item_code,
    seed.item_name,
    TRUE,
    NULL,
    seed.params,
    'global',
    now(),
    now()
FROM (
    VALUES
        (
            '10000000-0000-0000-0000-000000000021'::uuid,
            'none',
            '普通药品',
            '{"requires_dual_sign": true}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000022'::uuid,
            'narcotic',
            '麻醉药品',
            '{"requires_dual_sign": true}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000023'::uuid,
            'psychotropic_1',
            '第一类精神药品',
            '{"requires_dual_sign": true}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000024'::uuid,
            'psychotropic_2',
            '第二类精神药品',
            '{"requires_dual_sign": true}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000025'::uuid,
            'toxic_medical',
            '医疗用毒性药品',
            '{"requires_dual_sign": true}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000026'::uuid,
            'radioactive',
            '放射性药品',
            '{"requires_dual_sign": true}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000027'::uuid,
            'vaccine',
            '疫苗',
            '{"requires_dual_sign": true}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000028'::uuid,
            'blood_product',
            '血液制品',
            '{"requires_dual_sign": true}'::jsonb
        )
) AS seed(id, item_code, item_name, params)
WHERE NOT EXISTS (
    SELECT 1
      FROM system_dictionary_items existing
     WHERE existing.dict_code = 'special_drug_category'
       AND existing.item_code = seed.item_code
       AND existing.owner_id IS NULL
);
