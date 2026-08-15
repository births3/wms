-- 容器类型混装策略 + M-CG 五种 LPN 单据类型与默认规则。

CREATE TABLE IF NOT EXISTS lpn_container_type_policies (
    owner_id         UUID NOT NULL,
    container_type   TEXT NOT NULL,
    allow_mix_batch  BOOLEAN NOT NULL DEFAULT FALSE,
    allow_mix_sku    BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, container_type),
    CHECK (container_type IN (
        'pallet',
        'tote',
        'outbound_box',
        'insulated_box',
        'blind_label'
    ))
);

GRANT SELECT, INSERT, UPDATE ON lpn_container_type_policies TO wms_app;

INSERT INTO system_dictionary_items (
    id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at
)
VALUES
    ('10000000-0000-0000-0000-00000000b001'::uuid, 'document_type', 'lpn_pallet', '容器LPN-托盘', TRUE, NULL, '{"direction":"internal","workflow_template":"lpn_container","batch_policy":"none"}'::jsonb, 'global', now(), now()),
    ('10000000-0000-0000-0000-00000000b002'::uuid, 'document_type', 'lpn_tote', '容器LPN-周转箱', TRUE, NULL, '{"direction":"internal","workflow_template":"lpn_container","batch_policy":"none"}'::jsonb, 'global', now(), now()),
    ('10000000-0000-0000-0000-00000000b003'::uuid, 'document_type', 'lpn_outbound_box', '容器LPN-出库箱', TRUE, NULL, '{"direction":"internal","workflow_template":"lpn_container","batch_policy":"none"}'::jsonb, 'global', now(), now()),
    ('10000000-0000-0000-0000-00000000b004'::uuid, 'document_type', 'lpn_insulated_box', '容器LPN-保温箱', TRUE, NULL, '{"direction":"internal","workflow_template":"lpn_container","batch_policy":"none"}'::jsonb, 'global', now(), now()),
    ('10000000-0000-0000-0000-00000000b005'::uuid, 'document_type', 'lpn_blind_label', '容器LPN-盲标签', TRUE, NULL, '{"direction":"internal","workflow_template":"lpn_container","batch_policy":"none"}'::jsonb, 'global', now(), now())
ON CONFLICT DO NOTHING;

INSERT INTO document_number_rules (
    id, owner_id, document_type, rule_code, rule_name, template,
    reset_policy, sequence_width, sequence_mode, enabled, created_at, updated_at
)
VALUES
    ('10000000-0000-0000-0000-00000000b101'::uuid, NULL, 'lpn_pallet', 'GLOBAL-LPN-PALLET', '托盘LPN默认编号', 'LPN-PL-{OWNER}-{YYYY}{MM}{DD}-{SEQ}', 'daily', 4, 'no_gap', TRUE, now(), now()),
    ('10000000-0000-0000-0000-00000000b102'::uuid, NULL, 'lpn_tote', 'GLOBAL-LPN-TOTE', '周转箱LPN默认编号', 'LPN-TT-{OWNER}-{YYYY}{MM}{DD}-{SEQ}', 'daily', 4, 'no_gap', TRUE, now(), now()),
    ('10000000-0000-0000-0000-00000000b103'::uuid, NULL, 'lpn_outbound_box', 'GLOBAL-LPN-OUTBOX', '出库箱LPN默认编号', 'LPN-OB-{OWNER}-{YYYY}{MM}{DD}-{SEQ}', 'daily', 4, 'no_gap', TRUE, now(), now()),
    ('10000000-0000-0000-0000-00000000b104'::uuid, NULL, 'lpn_insulated_box', 'GLOBAL-LPN-INSUL', '保温箱LPN默认编号', 'LPN-IB-{OWNER}-{YYYY}{MM}{DD}-{SEQ}', 'daily', 4, 'no_gap', TRUE, now(), now()),
    ('10000000-0000-0000-0000-00000000b105'::uuid, NULL, 'lpn_blind_label', 'GLOBAL-LPN-BLIND', '盲标签LPN默认编号', 'LPN-BL-{OWNER}-{YYYY}{MM}{DD}-{SEQ}', 'daily', 4, 'no_gap', TRUE, now(), now())
ON CONFLICT DO NOTHING;
