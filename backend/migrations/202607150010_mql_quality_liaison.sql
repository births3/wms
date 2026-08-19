-- US-QL-001/002/003：质量联系单类型、创建、H4 审批与回写主链。

INSERT INTO system_dictionary_items (
    id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at
)
VALUES (
    '10000000-0000-0000-0000-000000000017'::uuid,
    'document_type',
    'quality_liaison',
    '质量联系单',
    TRUE,
    NULL,
    '{"direction":"internal","workflow_template":"quality_liaison","batch_policy":"optional"}'::jsonb,
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
    '10000000-0000-0000-0000-00000000a103'::uuid,
    NULL,
    'quality_liaison',
    'GLOBAL-QUALITY-LIAISON',
    '质量联系单默认编号规则',
    'QL{YYYY}{MM}{DD}{SEQ}',
    'daily',
    6,
    'no_gap',
    TRUE,
    now(),
    now()
)
ON CONFLICT DO NOTHING;

CREATE TABLE IF NOT EXISTS quality_liaison_types (
    id                      UUID PRIMARY KEY,
    owner_id                UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    type_code               TEXT NOT NULL,
    type_name               TEXT NOT NULL,
    approval_template_id    TEXT NOT NULL,
    approver_user_id        UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT,
    timeout_seconds         INT NOT NULL CHECK (timeout_seconds > 0),
    enabled                 BOOLEAN NOT NULL DEFAULT TRUE,
    created_by              UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                 BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (owner_id, type_code),
    CHECK (type_code ~ '^[a-z][a-z0-9_]{1,63}$')
);

CREATE TABLE IF NOT EXISTS quality_liaison_orders (
    id                      UUID PRIMARY KEY,
    owner_id                UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    liaison_no              TEXT NOT NULL,
    type_code               TEXT NOT NULL,
    related_document_type   TEXT NOT NULL,
    related_document_no     TEXT NOT NULL,
    problem_description     TEXT NOT NULL,
    disposition_suggestion  TEXT NOT NULL,
    trigger_source          TEXT NOT NULL,
    business_payload        JSONB NOT NULL DEFAULT '{}'::jsonb,
    status                  TEXT NOT NULL CHECK (status IN (
                                'pending_approval', 'approved', 'rejected',
                                'pending_erp_sync', 'landed', 'sync_failed', 'closed'
                            )),
    approval_record_id      UUID UNIQUE REFERENCES h4_approval_records(id) ON DELETE RESTRICT,
    approved_by             UUID REFERENCES auth_users(id) ON DELETE RESTRICT,
    approval_opinion        TEXT,
    approved_at             TIMESTAMPTZ,
    created_by              UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                 BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (owner_id, liaison_no),
    FOREIGN KEY (owner_id, type_code)
        REFERENCES quality_liaison_types(owner_id, type_code) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS quality_liaison_orders_query_idx
    ON quality_liaison_orders (owner_id, status, updated_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS stock_adjustment_orders_quality_liaison_uidx
    ON stock_adjustment_orders (owner_id, quality_liaison_id)
    WHERE quality_liaison_id IS NOT NULL;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    ('00000000-0000-0000-0000-00000000b101', 'mql.quality-liaison.read', '质量联系单查询'),
    ('00000000-0000-0000-0000-00000000b102', 'mql.quality-liaison.write', '质量联系单创建'),
    ('00000000-0000-0000-0000-00000000b103', 'mql.quality-liaison.config', '质量联系单类型配置'),
    ('00000000-0000-0000-0000-00000000b104', 'mql.quality-liaison.approve', '质量联系单审批回写')
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE ON quality_liaison_types TO wms_app;
GRANT SELECT, INSERT, UPDATE ON quality_liaison_orders TO wms_app;
