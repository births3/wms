-- US-MPM-004 / US-H8-002 AC5: persistent parameter mapping rules and unmatched queue.

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:mpm.execute')::uuid,
    'mpm.execute',
    'M-PM 参数映射执行'
)
ON CONFLICT DO NOTHING;

CREATE TABLE parameter_mapping_dictionaries (
    id                   UUID PRIMARY KEY,
    owner_id             UUID REFERENCES auth_owners(id),
    dict_code            TEXT NOT NULL,
    dict_name            TEXT NOT NULL,
    target_values        JSONB NOT NULL,
    case_sensitive       BOOLEAN NOT NULL DEFAULT FALSE,
    normalize_whitespace BOOLEAN NOT NULL DEFAULT TRUE,
    default_strategy     TEXT NOT NULL DEFAULT 'mark_unmapped',
    fallback_value       TEXT,
    enabled              BOOLEAN NOT NULL DEFAULT TRUE,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (jsonb_typeof(target_values) = 'array'),
    CHECK (default_strategy IN ('none', 'fallback', 'mark_unmapped')),
    CHECK (
        (default_strategy = 'fallback' AND fallback_value IS NOT NULL)
        OR (default_strategy <> 'fallback' AND fallback_value IS NULL)
    )
);

CREATE UNIQUE INDEX parameter_mapping_dictionaries_scope_uidx
    ON parameter_mapping_dictionaries (
        dict_code,
        COALESCE(owner_id, '00000000-0000-0000-0000-000000000000'::uuid)
    );

CREATE TABLE parameter_mapping_rules (
    id                        UUID PRIMARY KEY,
    dictionary_id             UUID NOT NULL REFERENCES parameter_mapping_dictionaries(id),
    owner_id                  UUID REFERENCES auth_owners(id),
    source_system             TEXT NOT NULL,
    match_type                TEXT NOT NULL,
    source_pattern            TEXT NOT NULL,
    normalized_source_pattern TEXT NOT NULL,
    target_value              TEXT NOT NULL,
    priority                  INT NOT NULL DEFAULT 100,
    confidence                INT NOT NULL DEFAULT 100,
    effective_from            TIMESTAMPTZ,
    effective_to              TIMESTAMPTZ,
    enabled                   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (match_type IN ('exact', 'contains', 'wildcard', 'regex')),
    CHECK (confidence BETWEEN 0 AND 100),
    CHECK (effective_to IS NULL OR effective_from IS NULL OR effective_to > effective_from)
);

CREATE INDEX parameter_mapping_rules_lookup_idx
    ON parameter_mapping_rules (dictionary_id, source_system, enabled, priority);

CREATE TABLE parameter_mapping_queue (
    id                      UUID PRIMARY KEY,
    owner_id                UUID NOT NULL,
    dictionary_id           UUID NOT NULL REFERENCES parameter_mapping_dictionaries(id),
    source_system           TEXT NOT NULL,
    source_record_id        TEXT,
    source_value            TEXT NOT NULL,
    normalized_source_value TEXT NOT NULL,
    occurrence_count        BIGINT NOT NULL DEFAULT 1,
    status                  TEXT NOT NULL DEFAULT 'pending_mapping',
    first_seen_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    handled_by              UUID,
    CHECK (status IN ('pending_mapping', 'mapped', 'ignored')),
    UNIQUE (owner_id, dictionary_id, normalized_source_value)
);

INSERT INTO parameter_mapping_dictionaries (
    id, dict_code, dict_name, target_values
) VALUES (
    '20000000-0000-0000-0000-000000000001',
    'storage_condition',
    '储存条件',
    '["frozen", "cold", "cool", "normal"]'::jsonb
);

INSERT INTO parameter_mapping_rules (
    id, dictionary_id, source_system, match_type, source_pattern,
    normalized_source_pattern, target_value, priority, confidence
)
SELECT
    seed.id,
    '20000000-0000-0000-0000-000000000001'::uuid,
    '*',
    'exact',
    seed.source_pattern,
    lower(btrim(seed.source_pattern)),
    seed.target_value,
    10,
    100
FROM (
    VALUES
        ('20000000-0000-0000-0001-000000000001'::uuid, 'normal', 'normal'),
        ('20000000-0000-0000-0001-000000000002'::uuid, '常温', 'normal'),
        ('20000000-0000-0000-0001-000000000003'::uuid, '室温', 'normal'),
        ('20000000-0000-0000-0001-000000000004'::uuid, '常温保存', 'normal'),
        ('20000000-0000-0000-0001-000000000005'::uuid, 'cool', 'cool'),
        ('20000000-0000-0000-0001-000000000006'::uuid, '阴凉', 'cool'),
        ('20000000-0000-0000-0001-000000000007'::uuid, '阴凉处保存', 'cool'),
        ('20000000-0000-0000-0001-000000000008'::uuid, 'cold', 'cold'),
        ('20000000-0000-0000-0001-000000000009'::uuid, '冷藏', 'cold'),
        ('20000000-0000-0000-0001-000000000010'::uuid, '2-8℃', 'cold'),
        ('20000000-0000-0000-0001-000000000011'::uuid, '2-8℃避光保存', 'cold'),
        ('20000000-0000-0000-0001-000000000012'::uuid, 'frozen', 'frozen'),
        ('20000000-0000-0000-0001-000000000013'::uuid, '冷冻', 'frozen')
) AS seed(id, source_pattern, target_value);

-- M1-011 remains the target-value fact source; M-PM only maps ERP text to its codes.
INSERT INTO parameter_mapping_dictionaries (
    id, dict_code, dict_name, target_values
)
SELECT
    '20000000-0000-0000-0000-000000000002'::uuid,
    'document_type',
    '单据类型',
    jsonb_agg(to_jsonb(item_code) ORDER BY item_code)
FROM system_dictionary_items
WHERE dict_code = 'document_type'
  AND owner_id IS NULL
  AND enabled;

INSERT INTO parameter_mapping_rules (
    id, dictionary_id, source_system, match_type, source_pattern,
    normalized_source_pattern, target_value, priority, confidence
)
SELECT
    md5('parameter_mapping_rule:document_type:' || source.kind || ':' || item.item_code)::uuid,
    '20000000-0000-0000-0000-000000000002'::uuid,
    '*',
    'exact',
    source.pattern,
    lower(btrim(source.pattern)),
    item.item_code,
    10,
    100
FROM system_dictionary_items AS item
CROSS JOIN LATERAL (
    VALUES ('code', item.item_code), ('name', item.item_name)
) AS source(kind, pattern)
WHERE item.dict_code = 'document_type'
  AND item.owner_id IS NULL
  AND item.enabled;

-- M1-010 remains the target-value fact source for controlled drug categories.
INSERT INTO parameter_mapping_dictionaries (
    id, dict_code, dict_name, target_values
)
SELECT
    '20000000-0000-0000-0000-000000000003'::uuid,
    'special_drug_category',
    '特殊药品分类',
    jsonb_agg(to_jsonb(item_code) ORDER BY item_code)
FROM system_dictionary_items
WHERE dict_code = 'special_drug_category'
  AND owner_id IS NULL
  AND enabled;

INSERT INTO parameter_mapping_rules (
    id, dictionary_id, source_system, match_type, source_pattern,
    normalized_source_pattern, target_value, priority, confidence
)
SELECT
    md5('parameter_mapping_rule:special_drug_category:' || source.kind || ':' || item.item_code)::uuid,
    '20000000-0000-0000-0000-000000000003'::uuid,
    '*',
    'exact',
    source.pattern,
    lower(btrim(source.pattern)),
    item.item_code,
    10,
    100
FROM system_dictionary_items AS item
CROSS JOIN LATERAL (
    VALUES ('code', item.item_code), ('name', item.item_name)
) AS source(kind, pattern)
WHERE item.dict_code = 'special_drug_category'
  AND item.owner_id IS NULL
  AND item.enabled;

-- US-MPM-001 preloads the story-defined dosage-form normalization.
INSERT INTO parameter_mapping_dictionaries (
    id, dict_code, dict_name, target_values
) VALUES (
    '20000000-0000-0000-0000-000000000004',
    'dosage_form',
    '剂型',
    '["片剂"]'::jsonb
);

INSERT INTO parameter_mapping_rules (
    id, dictionary_id, source_system, match_type, source_pattern,
    normalized_source_pattern, target_value, priority, confidence
)
SELECT
    md5('parameter_mapping_rule:dosage_form:' || source_pattern)::uuid,
    '20000000-0000-0000-0000-000000000004'::uuid,
    '*',
    'exact',
    source_pattern,
    lower(btrim(source_pattern)),
    '片剂',
    10,
    100
FROM unnest(ARRAY['片', '片剂', '普通片', '薄膜衣片']) AS source_pattern;

-- ERP may only operate public product states; pending_mapping remains internal.
INSERT INTO parameter_mapping_dictionaries (
    id, dict_code, dict_name, target_values
) VALUES (
    '20000000-0000-0000-0000-000000000005',
    'product_status',
    '商品状态',
    '["active", "disabled"]'::jsonb
);

INSERT INTO parameter_mapping_rules (
    id, dictionary_id, source_system, match_type, source_pattern,
    normalized_source_pattern, target_value, priority, confidence
)
SELECT
    md5('parameter_mapping_rule:product_status:' || source_pattern)::uuid,
    '20000000-0000-0000-0000-000000000005'::uuid,
    '*',
    'exact',
    source_pattern,
    lower(btrim(source_pattern)),
    target_value,
    10,
    100
FROM (
    VALUES
        ('active', 'active'),
        ('启用', 'active'),
        ('disabled', 'disabled'),
        ('停用', 'disabled')
) AS seed(source_pattern, target_value);

-- US-M1-001 packaging units use stable WMS codes; ERP spellings are source values.
INSERT INTO parameter_mapping_dictionaries (
    id, dict_code, dict_name, target_values
) VALUES (
    '20000000-0000-0000-0000-000000000006',
    'unit_pack',
    '包装单位',
    '["piece", "tablet", "board", "bottle", "bag", "box", "case", "pallet"]'::jsonb
);

INSERT INTO parameter_mapping_rules (
    id, dictionary_id, source_system, match_type, source_pattern,
    normalized_source_pattern, target_value, priority, confidence
)
SELECT
    md5('parameter_mapping_rule:unit_pack:' || source_pattern)::uuid,
    '20000000-0000-0000-0000-000000000006'::uuid,
    '*',
    'exact',
    source_pattern,
    lower(btrim(source_pattern)),
    target_value,
    10,
    100
FROM (
    VALUES
        ('piece', 'piece'), ('支', 'piece'), ('pcs', 'piece'),
        ('tablet', 'tablet'), ('片', 'tablet'),
        ('board', 'board'), ('板', 'board'),
        ('bottle', 'bottle'), ('瓶', 'bottle'),
        ('bag', 'bag'), ('袋', 'bag'),
        ('box', 'box'), ('盒', 'box'), ('小盒', 'box'),
        ('case', 'case'), ('件', 'case'), ('箱', 'case'),
        ('pallet', 'pallet'), ('托', 'pallet')
) AS seed(source_pattern, target_value);

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        GRANT SELECT ON parameter_mapping_dictionaries, parameter_mapping_rules TO wms_app;
        GRANT SELECT, INSERT, UPDATE ON parameter_mapping_queue TO wms_app;
    END IF;
END $$;
