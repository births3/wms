-- US-SA-001：报损单后端主链、M-VR 双人策略执行证据与 ERP 异步反馈 outbox。

INSERT INTO system_dictionary_items (
    id, dict_code, item_code, item_name, enabled, owner_id, params, source, created_at, updated_at
)
VALUES (
    '10000000-0000-0000-0000-000000000015'::uuid,
    'document_type',
    'stock_loss',
    '报损单',
    TRUE,
    NULL,
    '{"direction":"outbound","workflow_template":"stock_loss","batch_policy":"specified_batch"}'::jsonb,
    'global',
    now(),
    now()
)
ON CONFLICT DO NOTHING;

ALTER TABLE inventory_movements
    ADD COLUMN IF NOT EXISTS approval_source TEXT;

ALTER TABLE inventory_movements
    ADD COLUMN IF NOT EXISTS approval_id TEXT;

INSERT INTO document_number_rules (
    id, owner_id, document_type, rule_code, rule_name, template,
    reset_policy, sequence_width, sequence_mode, enabled, created_at, updated_at
)
VALUES (
    '10000000-0000-0000-0000-00000000a101'::uuid,
    NULL,
    'stock_loss',
    'GLOBAL-STOCK-LOSS',
    '报损单默认编号规则',
    'BS{YYYY}{MM}{DD}{SEQ}',
    'daily',
    6,
    'no_gap',
    TRUE,
    now(),
    now()
)
ON CONFLICT DO NOTHING;

CREATE TABLE IF NOT EXISTS stock_adjustment_orders (
    id                          UUID PRIMARY KEY,
    owner_id                    UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    warehouse_id                UUID NOT NULL,
    order_no                    TEXT NOT NULL,
    adjustment_type             TEXT NOT NULL CHECK (adjustment_type IN ('loss', 'surplus')),
    batch_id                    UUID NOT NULL REFERENCES inventory_batches(id) ON DELETE RESTRICT,
    product_code                TEXT NOT NULL,
    batch_no                    TEXT NOT NULL,
    quantity                    BIGINT NOT NULL CHECK (quantity > 0),
    reason_code                 TEXT NOT NULL CHECK (reason_code IN (
                                    'expired', 'damaged', 'quality_unqualified', 'inventory_loss',
                                    'destruction', 'recall_destruction', 'other'
                                )),
    recall_id                   TEXT,
    source                      TEXT NOT NULL CHECK (source IN ('erp', 'manual')),
    external_ref                TEXT,
    status                      TEXT NOT NULL CHECK (status IN (
                                    'pending_approval', 'pending_execution', 'in_progress',
                                    'completed', 'rejected', 'cancelled', 'exception_suspended'
                                )),
    requires_quality_approval   BOOLEAN NOT NULL DEFAULT FALSE,
    quality_liaison_id          TEXT,
    policy                      TEXT CHECK (policy IN ('single', 'dual_scan', 'dual_scan_with_approval')),
    source_rule_id              UUID REFERENCES dual_person_policy_rules(id) ON DELETE RESTRICT,
    first_operator_id           UUID REFERENCES auth_users(id) ON DELETE RESTRICT,
    second_operator_id          UUID REFERENCES auth_users(id) ON DELETE RESTRICT,
    approval_record_id          UUID REFERENCES h4_approval_records(id) ON DELETE RESTRICT,
    started_at                  TIMESTAMPTZ,
    completed_at                TIMESTAMPTZ,
    created_by                  UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                     BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (owner_id, order_no),
    FOREIGN KEY (owner_id, warehouse_id) REFERENCES warehouses(owner_id, id) ON DELETE RESTRICT,
    CHECK (source <> 'erp' OR external_ref IS NOT NULL),
    CHECK (reason_code <> 'recall_destruction' OR recall_id IS NOT NULL),
    CHECK (reason_code NOT IN ('destruction', 'recall_destruction') OR requires_quality_approval),
    CHECK (second_operator_id IS NULL OR second_operator_id <> first_operator_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS stock_adjustment_orders_erp_ref_uidx
    ON stock_adjustment_orders (owner_id, external_ref)
    WHERE source = 'erp';

CREATE INDEX IF NOT EXISTS stock_adjustment_orders_query_idx
    ON stock_adjustment_orders (owner_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS stock_adjustment_execution_records (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    order_id            UUID NOT NULL UNIQUE REFERENCES stock_adjustment_orders(id) ON DELETE RESTRICT,
    process_code        TEXT NOT NULL CHECK (process_code IN ('报损', '报溢', '销毁')),
    node_code           TEXT NOT NULL CHECK (node_code IN ('报损执行', '报溢执行', '销毁执行')),
    policy              TEXT NOT NULL CHECK (policy IN ('single', 'dual_scan', 'dual_scan_with_approval')),
    source_rule_id      UUID REFERENCES dual_person_policy_rules(id) ON DELETE RESTRICT,
    first_operator_id   UUID NOT NULL REFERENCES auth_users(id) ON DELETE RESTRICT,
    second_operator_id  UUID REFERENCES auth_users(id) ON DELETE RESTRICT,
    approval_record_id  UUID REFERENCES h4_approval_records(id) ON DELETE RESTRICT,
    quantity            BIGINT NOT NULL CHECK (quantity > 0),
    executed_at         TIMESTAMPTZ NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (second_operator_id IS NULL OR second_operator_id <> first_operator_id)
);

CREATE INDEX IF NOT EXISTS stock_adjustment_execution_owner_idx
    ON stock_adjustment_execution_records (owner_id, executed_at DESC);

CREATE TABLE IF NOT EXISTS stock_adjustment_erp_feedback_outbox (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    order_id         UUID NOT NULL UNIQUE REFERENCES stock_adjustment_orders(id) ON DELETE RESTRICT,
    event_type       TEXT NOT NULL CHECK (event_type IN ('stock_loss_completed', 'stock_surplus_completed')),
    payload          JSONB NOT NULL,
    status           TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sending', 'succeeded', 'failed')),
    attempt_count    INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at  TIMESTAMPTZ NOT NULL,
    last_error       TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS stock_adjustment_erp_feedback_pending_idx
    ON stock_adjustment_erp_feedback_outbox (status, next_attempt_at)
    WHERE status IN ('pending', 'failed');

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    ('00000000-0000-0000-0000-00000000a101', 'msa.stock-adjustment.read', '报损报溢查询'),
    ('00000000-0000-0000-0000-00000000a102', 'msa.stock-adjustment.write', '报损报溢创建'),
    ('00000000-0000-0000-0000-00000000a103', 'msa.stock-adjustment.execute', '报损报溢执行'),
    ('00000000-0000-0000-0000-00000000a104', 'msa.stock-adjustment.quality-approve', '报损报溢质量审批回写')
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE ON stock_adjustment_orders TO wms_app;
GRANT SELECT, INSERT ON stock_adjustment_execution_records TO wms_app;
GRANT SELECT, INSERT, UPDATE ON stock_adjustment_erp_feedback_outbox TO wms_app;
