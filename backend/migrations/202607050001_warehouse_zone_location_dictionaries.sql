-- Seed US-M1-004 warehouse zone and location attribute dictionaries.

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
SELECT
    seed.dict_code,
    seed.dict_name,
    TRUE,
    seed.control_level,
    seed.param_schema,
    'owner_override',
    '{"allowed_owner_params": []}'::jsonb,
    seed.sort_order,
    seed.remark
FROM (
    VALUES
        (
            'temperature_zone',
            '库区温区',
            'controlled',
            '{
                "properties": {
                    "min_celsius": {"type": "number"},
                    "max_celsius": {"type": "number"}
                }
            }'::jsonb,
            30,
            'US-M1-004 库区温区属性字典'
        ),
        (
            'quality_color',
            '库区色标',
            'controlled',
            '{
                "required": ["inventory_quality_status"],
                "properties": {
                    "inventory_quality_status": {
                        "type": "string",
                        "enum": ["qualified", "quarantine", "unqualified"]
                    }
                }
            }'::jsonb,
            31,
            'US-M1-004 库区色标属性字典'
        ),
        (
            'zone_type',
            '库区类型',
            'controlled',
            '{
                "required": ["allow_stock"],
                "properties": {
                    "allow_stock": {"type": "boolean"},
                    "quality_color": {
                        "type": "string",
                        "enum": ["qualified_green", "quarantine_yellow", "unqualified_red"]
                    }
                }
            }'::jsonb,
            32,
            'US-M1-004 库区类型属性字典'
        ),
        (
            'location_type',
            '库位类型',
            'controlled',
            '{
                "required": ["picking_mode"],
                "properties": {
                    "picking_mode": {
                        "type": "string",
                        "enum": ["none", "case", "piece"]
                    }
                }
            }'::jsonb,
            33,
            'US-M1-004 库位类型属性字典'
        )
) AS seed(dict_code, dict_name, control_level, param_schema, sort_order, remark)
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
    seed.dict_code,
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
            '10000000-0000-0000-0000-000000000031'::uuid,
            'temperature_zone',
            'normal',
            '常温',
            '{"min_celsius": 10, "max_celsius": 30}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000032'::uuid,
            'temperature_zone',
            'cool',
            '阴凉',
            '{"min_celsius": 0, "max_celsius": 20}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000033'::uuid,
            'temperature_zone',
            'cold',
            '冷藏',
            '{"min_celsius": 2, "max_celsius": 8}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000034'::uuid,
            'temperature_zone',
            'frozen',
            '冷冻',
            '{"max_celsius": -10}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000035'::uuid,
            'quality_color',
            'qualified_green',
            '合格绿',
            '{"inventory_quality_status": "qualified"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000036'::uuid,
            'quality_color',
            'quarantine_yellow',
            '待验黄',
            '{"inventory_quality_status": "quarantine"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000037'::uuid,
            'quality_color',
            'unqualified_red',
            '不合格红',
            '{"inventory_quality_status": "unqualified"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000038'::uuid,
            'zone_type',
            'storage',
            '存储区',
            '{"allow_stock": true}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000039'::uuid,
            'zone_type',
            'receiving',
            '待验区',
            '{"allow_stock": false}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000040'::uuid,
            'zone_type',
            'return',
            '退货区',
            '{"allow_stock": true}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000041'::uuid,
            'zone_type',
            'unqualified',
            '不合格区',
            '{"allow_stock": true, "quality_color": "unqualified_red"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000042'::uuid,
            'zone_type',
            'shipping',
            '发货暂存区',
            '{"allow_stock": false}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000043'::uuid,
            'location_type',
            'storage',
            '存储位',
            '{"picking_mode": "none"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000044'::uuid,
            'location_type',
            'case_pick',
            '箱拣位',
            '{"picking_mode": "case"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000045'::uuid,
            'location_type',
            'piece_pick',
            '零拣位',
            '{"picking_mode": "piece"}'::jsonb
        )
) AS seed(id, dict_code, item_code, item_name, params)
WHERE NOT EXISTS (
    SELECT 1
      FROM system_dictionary_items existing
     WHERE existing.dict_code = seed.dict_code
       AND existing.item_code = seed.item_code
       AND existing.owner_id IS NULL
);
