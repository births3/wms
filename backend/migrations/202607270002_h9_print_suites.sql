-- US-H9-008 print suite versions, ordered print items, readiness sources and
-- frozen suite-instance snapshots.

-- AC4: extend the controlled M1 dictionary print_document_category with the
-- first external_file categories (drug inspection report and invoice).
INSERT INTO system_dictionary_items (
    id, dict_code, item_code, item_name, enabled, owner_id,
    params, source, created_at, updated_at
)
SELECT
    '10000000-0000-0000-0000-000000000062'::uuid,
    'print_document_category',
    'drug_inspection_report',
    '药检单',
    TRUE,
    NULL,
    '{"source_mode": "external_file"}'::jsonb,
    'global',
    now(),
    now()
WHERE NOT EXISTS (
    SELECT 1
      FROM system_dictionary_items
     WHERE dict_code = 'print_document_category'
       AND item_code = 'drug_inspection_report'
       AND owner_id IS NULL
);

INSERT INTO system_dictionary_items (
    id, dict_code, item_code, item_name, enabled, owner_id,
    params, source, created_at, updated_at
)
SELECT
    '10000000-0000-0000-0000-000000000063'::uuid,
    'print_document_category',
    'invoice',
    '发票',
    TRUE,
    NULL,
    '{"source_mode": "external_file"}'::jsonb,
    'global',
    now(),
    now()
WHERE NOT EXISTS (
    SELECT 1
      FROM system_dictionary_items
     WHERE dict_code = 'print_document_category'
       AND item_code = 'invoice'
       AND owner_id IS NULL
);

-- AC1/AC2: immutable print suite versions with four-level scope resolution.
CREATE TABLE IF NOT EXISTS h9_print_suite_versions (
    id                   UUID PRIMARY KEY,
    owner_id             UUID NOT NULL,
    version_no           INT NOT NULL,
    name                 TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'draft',
    warehouse_id         UUID NOT NULL,
    scope_type           TEXT NOT NULL,
    customer_id          UUID,
    delivery_address_id  UUID,
    route_code           TEXT,
    effective_from       TIMESTAMPTZ NOT NULL,
    effective_to         TIMESTAMPTZ,
    test_result          JSONB,
    tested_by            UUID,
    tested_at            TIMESTAMPTZ,
    published_by         UUID,
    published_at         TIMESTAMPTZ,
    disabled_by          UUID,
    disabled_at          TIMESTAMPTZ,
    created_by           UUID NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, id),
    UNIQUE (owner_id, version_no),
    CHECK (version_no > 0),
    CHECK (length(btrim(name)) BETWEEN 1 AND 100),
    CHECK (status IN ('draft', 'tested', 'published', 'disabled')),
    CHECK (scope_type IN ('delivery_address', 'customer', 'route', 'warehouse_default')),
    CHECK (
        (scope_type = 'delivery_address'
            AND customer_id IS NOT NULL
            AND delivery_address_id IS NOT NULL
            AND route_code IS NULL)
        OR (scope_type = 'customer'
            AND customer_id IS NOT NULL
            AND delivery_address_id IS NULL
            AND route_code IS NULL)
        OR (scope_type = 'route'
            AND customer_id IS NULL
            AND delivery_address_id IS NULL
            AND route_code IS NOT NULL)
        OR (scope_type = 'warehouse_default'
            AND customer_id IS NULL
            AND delivery_address_id IS NULL
            AND route_code IS NULL)
    ),
    CHECK (route_code IS NULL OR length(btrim(route_code)) BETWEEN 1 AND 64),
    CHECK (effective_to IS NULL OR effective_to > effective_from),
    CHECK (
        (status = 'draft'
            AND tested_by IS NULL AND tested_at IS NULL
            AND published_by IS NULL AND published_at IS NULL
            AND disabled_by IS NULL AND disabled_at IS NULL)
        OR (status = 'tested'
            AND tested_by IS NOT NULL AND tested_at IS NOT NULL
            AND test_result IS NOT NULL
            AND published_by IS NULL AND published_at IS NULL
            AND disabled_by IS NULL AND disabled_at IS NULL)
        OR (status = 'published'
            AND tested_by IS NOT NULL AND tested_at IS NOT NULL
            AND test_result IS NOT NULL
            AND published_by IS NOT NULL AND published_at IS NOT NULL
            AND disabled_by IS NULL AND disabled_at IS NULL)
        OR (status = 'disabled'
            AND disabled_by IS NOT NULL AND disabled_at IS NOT NULL)
    ),
    FOREIGN KEY (owner_id, delivery_address_id, customer_id)
        REFERENCES customer_addresses(owner_id, id, customer_id)
        ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS h9_print_suite_versions_resolution_idx
    ON h9_print_suite_versions (
        owner_id,
        warehouse_id,
        status,
        scope_type,
        effective_from,
        effective_to
    );

CREATE OR REPLACE FUNCTION reject_h9_print_suite_content_rewrite()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status IN ('published', 'disabled')
       AND (
           NEW.owner_id IS DISTINCT FROM OLD.owner_id
           OR NEW.version_no IS DISTINCT FROM OLD.version_no
           OR NEW.name IS DISTINCT FROM OLD.name
           OR NEW.warehouse_id IS DISTINCT FROM OLD.warehouse_id
           OR NEW.scope_type IS DISTINCT FROM OLD.scope_type
           OR NEW.customer_id IS DISTINCT FROM OLD.customer_id
           OR NEW.delivery_address_id IS DISTINCT FROM OLD.delivery_address_id
           OR NEW.route_code IS DISTINCT FROM OLD.route_code
           OR NEW.effective_from IS DISTINCT FROM OLD.effective_from
           OR NEW.effective_to IS DISTINCT FROM OLD.effective_to
           OR NEW.test_result IS DISTINCT FROM OLD.test_result
           OR NEW.tested_by IS DISTINCT FROM OLD.tested_by
           OR NEW.tested_at IS DISTINCT FROM OLD.tested_at
           OR NEW.published_by IS DISTINCT FROM OLD.published_by
           OR NEW.published_at IS DISTINCT FROM OLD.published_at
           OR NEW.created_by IS DISTINCT FROM OLD.created_by
           OR NEW.created_at IS DISTINCT FROM OLD.created_at
       )
    THEN
        RAISE EXCEPTION 'published H9 print suite content is immutable';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS h9_print_suite_content_immutable
    ON h9_print_suite_versions;
CREATE TRIGGER h9_print_suite_content_immutable
BEFORE UPDATE ON h9_print_suite_versions
FOR EACH ROW
EXECUTE FUNCTION reject_h9_print_suite_content_rewrite();

-- AC3: ordered print items with per-item readiness and failure policies.
CREATE TABLE IF NOT EXISTS h9_print_suite_items (
    id                   UUID PRIMARY KEY,
    owner_id             UUID NOT NULL,
    suite_version_id     UUID NOT NULL,
    category_code        TEXT NOT NULL,
    copies               INT NOT NULL,
    sort_order           INT NOT NULL,
    output_slot          TEXT NOT NULL,
    required             BOOLEAN NOT NULL,
    ready_policy         TEXT NOT NULL,
    failure_policy       TEXT NOT NULL,
    source_mode          TEXT NOT NULL,
    template_version_id  UUID,
    external_file_ref    TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, suite_version_id, sort_order),
    FOREIGN KEY (owner_id, suite_version_id)
        REFERENCES h9_print_suite_versions(owner_id, id)
        ON DELETE RESTRICT,
    FOREIGN KEY (template_version_id)
        REFERENCES print_template_versions(id)
        ON DELETE RESTRICT,
    CHECK (length(btrim(category_code)) BETWEEN 1 AND 64),
    CHECK (copies BETWEEN 1 AND 20),
    CHECK (sort_order > 0),
    CHECK (length(btrim(output_slot)) BETWEEN 1 AND 64),
    CHECK (ready_policy IN ('wait_hold_instance', 'pause_agent_queue')),
    CHECK (failure_policy IN ('pause_suite', 'skip_and_continue')),
    -- ADR-0041: required items may never be skipped by a failure policy.
    CHECK (NOT required OR failure_policy = 'pause_suite'),
    CHECK (source_mode IN ('rendered', 'external_file')),
    CHECK (
        (source_mode = 'rendered'
            AND template_version_id IS NOT NULL
            AND external_file_ref IS NULL)
        OR (source_mode = 'external_file'
            AND template_version_id IS NULL
            AND external_file_ref IS NOT NULL)
    ),
    CHECK (external_file_ref IS NULL OR length(btrim(external_file_ref)) BETWEEN 1 AND 200)
);

CREATE OR REPLACE FUNCTION reject_h9_print_suite_item_rewrite()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    parent_status TEXT;
BEGIN
    SELECT status INTO parent_status
      FROM h9_print_suite_versions
     WHERE owner_id = OLD.owner_id AND id = OLD.suite_version_id;
    IF parent_status IN ('published', 'disabled') THEN
        RAISE EXCEPTION 'published H9 print suite items are immutable';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS h9_print_suite_items_immutable
    ON h9_print_suite_items;
CREATE TRIGGER h9_print_suite_items_immutable
BEFORE UPDATE OR DELETE ON h9_print_suite_items
FOR EACH ROW
EXECUTE FUNCTION reject_h9_print_suite_item_rewrite();

-- AC5/AC6: stable ingested authoritative file references (H-FILE placeholder
-- registry; H-FILE ingestion itself is out of scope for US-H9-008).
CREATE TABLE IF NOT EXISTS h9_ingested_document_files (
    id             UUID PRIMARY KEY,
    owner_id       UUID NOT NULL,
    category_code  TEXT NOT NULL,
    file_ref       TEXT NOT NULL,
    file_version   INT NOT NULL DEFAULT 1,
    content_hash   TEXT NOT NULL,
    status         TEXT NOT NULL DEFAULT 'valid',
    invoice_no     TEXT,
    product_code   TEXT,
    batch_no       TEXT,
    ingested_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, file_ref, file_version),
    CHECK (category_code IN ('invoice', 'drug_inspection_report')),
    CHECK (status IN ('valid', 'revoked')),
    CHECK (file_version > 0),
    CHECK (length(btrim(file_ref)) BETWEEN 1 AND 200),
    CHECK (length(btrim(content_hash)) BETWEEN 1 AND 128),
    CHECK (
        (category_code = 'invoice'
            AND invoice_no IS NOT NULL
            AND product_code IS NULL
            AND batch_no IS NULL)
        OR (category_code = 'drug_inspection_report'
            AND invoice_no IS NULL
            AND product_code IS NOT NULL
            AND batch_no IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS h9_ingested_document_files_invoice_idx
    ON h9_ingested_document_files (owner_id, category_code, invoice_no)
    WHERE category_code = 'invoice';

CREATE INDEX IF NOT EXISTS h9_ingested_document_files_batch_idx
    ON h9_ingested_document_files (owner_id, category_code, product_code, batch_no)
    WHERE category_code = 'drug_inspection_report';

-- FK 前置：h9_delivery_note_groups 仅有 6 列复合 UNIQUE 与 id 主键，
-- (owner_id, id) 组合外键需要独立唯一索引支撑。
CREATE UNIQUE INDEX IF NOT EXISTS h9_delivery_note_groups_owner_id_id_key
    ON h9_delivery_note_groups (owner_id, id);

-- AC7/AC8: suite instances freeze suite version, rule version, source
-- documents and per-item policies.
CREATE TABLE IF NOT EXISTS h9_print_suite_instances (
    id                           UUID PRIMARY KEY,
    owner_id                     UUID NOT NULL,
    group_id                     UUID NOT NULL,
    suite_version_id             UUID NOT NULL,
    suite_version_no             INT NOT NULL,
    suite_snapshot               JSONB NOT NULL,
    aggregation_rule_version_id  UUID,
    aggregation_rule_version_no  INT,
    source_documents             JSONB NOT NULL,
    status                       TEXT NOT NULL DEFAULT 'waiting_documents',
    hold_scope                   TEXT,
    created_by                   UUID NOT NULL,
    created_at                   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, id),
    UNIQUE (owner_id, group_id),
    FOREIGN KEY (owner_id, group_id)
        REFERENCES h9_delivery_note_groups(owner_id, id)
        ON DELETE RESTRICT,
    FOREIGN KEY (owner_id, suite_version_id)
        REFERENCES h9_print_suite_versions(owner_id, id)
        ON DELETE RESTRICT,
    CHECK (jsonb_typeof(suite_snapshot) = 'object'),
    CHECK (jsonb_typeof(source_documents) = 'array'),
    CHECK (status IN ('waiting_documents', 'queued', 'cancelled')),
    CHECK (hold_scope IS NULL OR hold_scope IN ('instance', 'agent_queue')),
    CHECK (
        (status = 'waiting_documents' AND hold_scope IS NOT NULL)
        OR (status <> 'waiting_documents' AND hold_scope IS NULL)
    )
);

CREATE OR REPLACE FUNCTION reject_h9_print_suite_instance_rewrite()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.owner_id IS DISTINCT FROM OLD.owner_id
       OR NEW.group_id IS DISTINCT FROM OLD.group_id
       OR NEW.suite_version_id IS DISTINCT FROM OLD.suite_version_id
       OR NEW.suite_version_no IS DISTINCT FROM OLD.suite_version_no
       OR NEW.suite_snapshot IS DISTINCT FROM OLD.suite_snapshot
       OR NEW.aggregation_rule_version_id IS DISTINCT FROM OLD.aggregation_rule_version_id
       OR NEW.aggregation_rule_version_no IS DISTINCT FROM OLD.aggregation_rule_version_no
       OR NEW.source_documents IS DISTINCT FROM OLD.source_documents
       OR NEW.created_by IS DISTINCT FROM OLD.created_by
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
    THEN
        RAISE EXCEPTION 'H9 print suite instance snapshot is immutable';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS h9_print_suite_instances_immutable
    ON h9_print_suite_instances;
CREATE TRIGGER h9_print_suite_instances_immutable
BEFORE UPDATE ON h9_print_suite_instances
FOR EACH ROW
EXECUTE FUNCTION reject_h9_print_suite_instance_rewrite();

CREATE TABLE IF NOT EXISTS h9_print_suite_instance_items (
    id                   UUID PRIMARY KEY,
    owner_id             UUID NOT NULL,
    instance_id          UUID NOT NULL,
    category_code        TEXT NOT NULL,
    copies               INT NOT NULL,
    sort_order           INT NOT NULL,
    output_slot          TEXT NOT NULL,
    required             BOOLEAN NOT NULL,
    ready_policy         TEXT NOT NULL,
    failure_policy       TEXT NOT NULL,
    source_mode          TEXT NOT NULL,
    template_version_id  UUID,
    external_file_ref    TEXT,
    file_bindings        JSONB NOT NULL DEFAULT '[]'::jsonb,
    ready                BOOLEAN NOT NULL DEFAULT FALSE,
    missing              JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, instance_id, sort_order),
    FOREIGN KEY (owner_id, instance_id)
        REFERENCES h9_print_suite_instances(owner_id, id)
        ON DELETE RESTRICT,
    FOREIGN KEY (template_version_id)
        REFERENCES print_template_versions(id)
        ON DELETE RESTRICT,
    CHECK (length(btrim(category_code)) BETWEEN 1 AND 64),
    CHECK (copies BETWEEN 1 AND 20),
    CHECK (sort_order > 0),
    CHECK (length(btrim(output_slot)) BETWEEN 1 AND 64),
    CHECK (ready_policy IN ('wait_hold_instance', 'pause_agent_queue')),
    CHECK (failure_policy IN ('pause_suite', 'skip_and_continue')),
    CHECK (NOT required OR failure_policy = 'pause_suite'),
    CHECK (source_mode IN ('rendered', 'external_file')),
    CHECK (
        (source_mode = 'rendered'
            AND template_version_id IS NOT NULL
            AND external_file_ref IS NULL)
        OR (source_mode = 'external_file'
            AND template_version_id IS NULL
            AND external_file_ref IS NOT NULL)
    ),
    CHECK (jsonb_typeof(file_bindings) = 'array'),
    CHECK (jsonb_typeof(missing) = 'array'),
    CHECK (source_mode = 'external_file' OR file_bindings = '[]'::jsonb)
);

CREATE OR REPLACE FUNCTION reject_h9_print_suite_instance_item_rewrite()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.owner_id IS DISTINCT FROM OLD.owner_id
       OR NEW.instance_id IS DISTINCT FROM OLD.instance_id
       OR NEW.category_code IS DISTINCT FROM OLD.category_code
       OR NEW.copies IS DISTINCT FROM OLD.copies
       OR NEW.sort_order IS DISTINCT FROM OLD.sort_order
       OR NEW.output_slot IS DISTINCT FROM OLD.output_slot
       OR NEW.required IS DISTINCT FROM OLD.required
       OR NEW.ready_policy IS DISTINCT FROM OLD.ready_policy
       OR NEW.failure_policy IS DISTINCT FROM OLD.failure_policy
       OR NEW.source_mode IS DISTINCT FROM OLD.source_mode
       OR NEW.template_version_id IS DISTINCT FROM OLD.template_version_id
       OR NEW.external_file_ref IS DISTINCT FROM OLD.external_file_ref
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
    THEN
        RAISE EXCEPTION 'H9 print suite instance item policies are frozen';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS h9_print_suite_instance_items_frozen
    ON h9_print_suite_instance_items;
CREATE TRIGGER h9_print_suite_instance_items_frozen
BEFORE UPDATE ON h9_print_suite_instance_items
FOR EACH ROW
EXECUTE FUNCTION reject_h9_print_suite_instance_item_rewrite();

-- Menu button permissions for the print-suite actions on the existing
-- delivery-note aggregation menu node.
INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5(node.id::text || ':' || action.key)::uuid,
    node.id,
    action.key,
    action.label,
    'private',
    TRUE,
    action.sort_order
FROM admin_menu_draft_nodes node
CROSS JOIN (
    VALUES
        ('create_suite', '新建组套版本', 71),
        ('test_suite', '测试组套', 72),
        ('publish_suite', '发布组套', 73),
        ('disable_suite', '停用组套', 74)
) AS action(key, label, sort_order)
WHERE node.id = '00000000-0000-0000-0000-000000130061'
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions ORDER BY version_no DESC LIMIT 1
)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5((SELECT id::text FROM version_row) || ':' || button.id::text)::uuid,
    (SELECT id FROM version_row),
    button.menu_node_id,
    button.action_key,
    button.action_label,
    button.action_kind,
    button.enabled,
    button.sort_order
FROM admin_menu_draft_button_permissions button
WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130061'
  AND button.action_key IN ('create_suite', 'test_suite', 'publish_suite', 'disable_suite')
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE ON h9_print_suite_versions TO wms_app;
GRANT SELECT, INSERT ON h9_print_suite_items TO wms_app;
GRANT SELECT, INSERT, UPDATE ON h9_ingested_document_files TO wms_app;
GRANT SELECT, INSERT, UPDATE ON h9_print_suite_instances TO wms_app;
GRANT SELECT, INSERT, UPDATE ON h9_print_suite_instance_items TO wms_app;
