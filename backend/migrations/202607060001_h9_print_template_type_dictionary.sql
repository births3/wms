-- US-H9-001 print template type dictionary presets.

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
    'print_template_type',
    '打印模板类型',
    TRUE,
    'controlled',
    '{
        "required": [
            "field_library_code",
            "business_module",
            "business_direction",
            "paper_type",
            "default_scope"
        ],
        "properties": {
            "field_library_code": {"type": "string"},
            "business_module": {
                "type": "string",
                "enum": ["M1", "M2", "M3", "M4", "H5"]
            },
            "business_direction": {
                "type": "string",
                "enum": ["inbound", "outbound", "label"]
            },
            "paper_type": {
                "type": "string",
                "enum": ["a4", "a5", "label"]
            },
            "default_scope": {
                "type": "string",
                "enum": ["global", "owner"]
            }
        }
    }'::jsonb,
    'owner_override',
    '{"allowed_owner_params": ["field_library_code", "paper_type", "default_scope"]}'::jsonb,
    40,
    'US-H9-001 首批打印模板类型'
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
    'print_template_type',
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
            '10000000-0000-0000-0000-000000000051'::uuid,
            'asn',
            'ASN 单',
            '{"field_library_code": "m2_asn", "business_module": "M2", "business_direction": "inbound", "paper_type": "a4", "default_scope": "global"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000052'::uuid,
            'acceptance_record',
            '验收记录单',
            '{"field_library_code": "m2_acceptance_record", "business_module": "M2", "business_direction": "inbound", "paper_type": "a4", "default_scope": "global"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000053'::uuid,
            'delivery_note',
            '随货同行单',
            '{"field_library_code": "m4_delivery_note", "business_module": "M4", "business_direction": "outbound", "paper_type": "a4", "default_scope": "global"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000054'::uuid,
            'location_label',
            '库位标签',
            '{"field_library_code": "m1_location_label", "business_module": "M1", "business_direction": "label", "paper_type": "label", "default_scope": "global"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000055'::uuid,
            'lpn_label',
            'LPN 标签',
            '{"field_library_code": "m3_lpn_label", "business_module": "M3", "business_direction": "label", "paper_type": "label", "default_scope": "global"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000056'::uuid,
            'product_label',
            '商品标签',
            '{"field_library_code": "m1_product_label", "business_module": "M1", "business_direction": "label", "paper_type": "label", "default_scope": "global"}'::jsonb
        )
) AS seed(id, item_code, item_name, params)
WHERE NOT EXISTS (
    SELECT 1
      FROM system_dictionary_items existing
     WHERE existing.dict_code = 'print_template_type'
       AND existing.item_code = seed.item_code
       AND existing.owner_id IS NULL
);
