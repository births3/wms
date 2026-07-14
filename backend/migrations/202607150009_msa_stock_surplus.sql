-- US-SA-002：报溢单类型、原因和 M-CG 编号规则。

INSERT INTO system_dictionary_items (
    id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at
)
VALUES (
    '10000000-0000-0000-0000-000000000016'::uuid,
    'document_type',
    'stock_surplus',
    '报溢单',
    TRUE,
    NULL,
    '{"direction":"inbound","workflow_template":"stock_surplus","batch_policy":"specified_batch"}'::jsonb,
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
    '10000000-0000-0000-0000-00000000a102'::uuid,
    NULL,
    'stock_surplus',
    'GLOBAL-STOCK-SURPLUS',
    '报溢单默认编号规则',
    'BY{YYYY}{MM}{DD}{SEQ}',
    'daily',
    6,
    'no_gap',
    TRUE,
    now(),
    now()
)
ON CONFLICT DO NOTHING;

ALTER TABLE stock_adjustment_orders
    DROP CONSTRAINT IF EXISTS stock_adjustment_orders_reason_code_check;

ALTER TABLE stock_adjustment_orders
    ADD CONSTRAINT stock_adjustment_orders_reason_code_check
    CHECK (
        (adjustment_type = 'loss' AND reason_code IN (
            'expired', 'damaged', 'quality_unqualified', 'inventory_loss',
            'destruction', 'recall_destruction', 'other'
        ))
        OR
        (adjustment_type = 'surplus' AND reason_code IN (
            'inventory_surplus', 'return_inbound',
            'system_difference_correction', 'other'
        ))
    );
