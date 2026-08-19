-- US-M1-011 system dictionary first backend slice.

CREATE TABLE IF NOT EXISTS system_dictionary_categories (
    dict_code        TEXT PRIMARY KEY,
    dict_name        TEXT NOT NULL,
    enabled          BOOLEAN NOT NULL DEFAULT TRUE,
    control_level    TEXT NOT NULL,
    param_schema     JSONB NOT NULL DEFAULT '{}'::jsonb,
    scope_mode       TEXT NOT NULL,
    override_policy  JSONB NOT NULL DEFAULT '{}'::jsonb,
    sort_order       INT NOT NULL DEFAULT 0,
    remark           TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (control_level IN ('normal', 'controlled', 'gsp_critical')),
    CHECK (scope_mode IN ('global_only', 'owner_extensible', 'owner_override'))
);

CREATE TABLE IF NOT EXISTS system_dictionary_items (
    id               UUID PRIMARY KEY,
    dict_code        TEXT NOT NULL REFERENCES system_dictionary_categories(dict_code),
    item_code        TEXT NOT NULL,
    item_name        TEXT NOT NULL,
    enabled          BOOLEAN NOT NULL DEFAULT TRUE,
    owner_id         UUID,
    params           JSONB NOT NULL DEFAULT '{}'::jsonb,
    effective_from   TIMESTAMPTZ,
    effective_to     TIMESTAMPTZ,
    source           TEXT NOT NULL,
    disabled_reason  TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    version          BIGINT NOT NULL DEFAULT 1,
    CHECK (source IN ('global', 'owner')),
    CHECK (
        (owner_id IS NULL AND source = 'global')
        OR (owner_id IS NOT NULL AND source = 'owner')
    ),
    CHECK (effective_to IS NULL OR effective_from IS NULL OR effective_to > effective_from)
);

CREATE UNIQUE INDEX IF NOT EXISTS system_dictionary_items_scope_uidx
    ON system_dictionary_items (
        dict_code,
        item_code,
        COALESCE(owner_id, '00000000-0000-0000-0000-000000000000'::uuid)
    );

CREATE INDEX IF NOT EXISTS system_dictionary_items_owner_lookup_idx
    ON system_dictionary_items (dict_code, owner_id, item_code);

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
    'document_type',
    '单据类型',
    TRUE,
    'controlled',
    '{
        "required": ["direction", "workflow_template", "batch_policy"],
        "properties": {
            "direction": {
                "type": "string",
                "enum": ["inbound", "outbound"]
            },
            "workflow_template": {
                "type": "string",
                "enum": [
                    "purchase_inbound",
                    "sales_return",
                    "other_inbound",
                    "purchase_return_outbound",
                    "sales_outbound",
                    "sample_outbound",
                    "other_outbound"
                ]
            },
            "batch_policy": {
                "type": "string",
                "enum": ["standard_batch"]
            }
        }
    }'::jsonb,
    'owner_override',
    '{"allowed_owner_params": []}'::jsonb,
    10,
    'US-M1-011 首批系统预置单据类型'
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
    'document_type',
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
            '10000000-0000-0000-0000-000000000011'::uuid,
            'purchase_inbound',
            '采购入库',
            '{"direction": "inbound", "workflow_template": "purchase_inbound", "batch_policy": "standard_batch"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000012'::uuid,
            'sales_return',
            '销售退货入库',
            '{"direction": "inbound", "workflow_template": "sales_return", "batch_policy": "standard_batch"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000013'::uuid,
            'purchase_return_outbound',
            '采购退货出库',
            '{"direction": "outbound", "workflow_template": "purchase_return_outbound", "batch_policy": "standard_batch"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000014'::uuid,
            'sales_outbound',
            '销售出库',
            '{"direction": "outbound", "workflow_template": "sales_outbound", "batch_policy": "standard_batch"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000018'::uuid,
            'other_inbound',
            '其他入库',
            '{"direction": "inbound", "workflow_template": "other_inbound", "batch_policy": "standard_batch"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000019'::uuid,
            'sample_outbound',
            '样品出库',
            '{"direction": "outbound", "workflow_template": "sample_outbound", "batch_policy": "standard_batch"}'::jsonb
        ),
        (
            '10000000-0000-0000-0000-000000000020'::uuid,
            'other_outbound',
            '其他出库',
            '{"direction": "outbound", "workflow_template": "other_outbound", "batch_policy": "standard_batch"}'::jsonb
        )
) AS seed(id, item_code, item_name, params)
WHERE NOT EXISTS (
    SELECT 1
      FROM system_dictionary_items existing
     WHERE existing.dict_code = 'document_type'
       AND existing.item_code = seed.item_code
       AND existing.owner_id IS NULL
);

GRANT SELECT ON system_dictionary_categories TO wms_app;
GRANT SELECT, INSERT, UPDATE ON system_dictionary_items TO wms_app;
