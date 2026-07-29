-- US-H9-009 / ADR-0031：统一 H-FILE 元数据、H9 文件业务绑定与分类 PDF。

ALTER TABLE attachments
    ADD COLUMN bucket TEXT,
    ADD COLUMN content_hash TEXT,
    ADD COLUMN file_version INT,
    ADD COLUMN status TEXT,
    ADD COLUMN retention_policy TEXT,
    ADD COLUMN retain_until TIMESTAMPTZ,
    ADD COLUMN cache_expires_at TIMESTAMPTZ,
    ADD COLUMN created_by UUID,
    ADD COLUMN confirmed_at TIMESTAMPTZ,
    ADD CONSTRAINT attachments_h9_pdf_metadata_check CHECK (
        bucket IS NULL
        OR (
            length(btrim(bucket)) BETWEEN 1 AND 100
            AND length(btrim(storage_key)) BETWEEN 1 AND 500
            AND length(btrim(file_name)) BETWEEN 1 AND 255
            AND content_type = 'application/pdf'
            AND size_bytes BETWEEN 1 AND 52428800
            AND content_hash ~ '^[0-9a-f]{64}$'
            AND file_version > 0
            AND status IN ('pending', 'ready', 'failed')
            AND retention_policy IN ('gsp_5_year', 'short_cache')
            AND created_by IS NOT NULL
            AND (
                (retention_policy = 'gsp_5_year'
                    AND retain_until IS NOT NULL
                    AND cache_expires_at IS NULL)
                OR (retention_policy = 'short_cache'
                    AND retain_until IS NULL
                    AND cache_expires_at IS NOT NULL)
            )
            AND (
                (status = 'ready' AND confirmed_at IS NOT NULL)
                OR (status <> 'ready' AND confirmed_at IS NULL)
            )
        )
    ),
    ADD CONSTRAINT attachments_bucket_storage_key_key UNIQUE (bucket, storage_key);

CREATE INDEX IF NOT EXISTS attachments_entity_idx
    ON attachments (owner_id, module, entity_type, entity_id);

CREATE INDEX IF NOT EXISTS attachments_retention_idx
    ON attachments (status, retention_policy, retain_until, cache_expires_at);

-- H9 只保留发票/药检单覆盖关系；文件事实统一由 attachments 承载。
DROP TABLE h9_ingested_document_files;

CREATE TABLE h9_document_file_bindings (
    id             UUID PRIMARY KEY,
    owner_id       UUID NOT NULL,
    category_code  TEXT NOT NULL,
    attachment_id  UUID NOT NULL,
    invoice_no     TEXT,
    product_code   TEXT,
    batch_no       TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, id),
    UNIQUE (owner_id, category_code, attachment_id, invoice_no, product_code, batch_no),
    FOREIGN KEY (owner_id, attachment_id)
        REFERENCES attachments(owner_id, id)
        ON DELETE RESTRICT,
    CHECK (category_code IN ('invoice', 'drug_inspection_report')),
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

CREATE INDEX h9_document_file_bindings_invoice_idx
    ON h9_document_file_bindings (owner_id, category_code, invoice_no)
    WHERE category_code = 'invoice';

CREATE INDEX h9_document_file_bindings_batch_idx
    ON h9_document_file_bindings (owner_id, category_code, product_code, batch_no)
    WHERE category_code = 'drug_inspection_report';

CREATE TABLE h9_category_pdf_preparations (
    id                UUID PRIMARY KEY,
    owner_id          UUID NOT NULL,
    instance_id       UUID NOT NULL,
    idempotency_key   TEXT NOT NULL,
    request_hash      TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'processing',
    last_error        TEXT,
    created_by        UUID NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at      TIMESTAMPTZ,
    UNIQUE (owner_id, id),
    UNIQUE (owner_id, instance_id),
    UNIQUE (owner_id, idempotency_key),
    FOREIGN KEY (owner_id, instance_id)
        REFERENCES h9_print_suite_instances(owner_id, id)
        ON DELETE RESTRICT,
    CHECK (length(btrim(idempotency_key)) BETWEEN 1 AND 200),
    CHECK (request_hash ~ '^[0-9a-f]{64}$'),
    CHECK (status IN ('processing', 'completed', 'failed')),
    CHECK (
        (status = 'completed' AND completed_at IS NOT NULL AND last_error IS NULL)
        OR (status = 'failed' AND completed_at IS NULL AND last_error IS NOT NULL)
        OR (status = 'processing' AND completed_at IS NULL AND last_error IS NULL)
    )
);

CREATE TABLE h9_category_pdf_outputs (
    id                    UUID PRIMARY KEY,
    owner_id              UUID NOT NULL,
    preparation_id        UUID NOT NULL,
    instance_id           UUID NOT NULL,
    instance_item_id      UUID NOT NULL,
    category_code         TEXT NOT NULL,
    source_mode           TEXT NOT NULL,
    source_data_version   TEXT,
    source_file_bindings  JSONB NOT NULL DEFAULT '[]'::jsonb,
    template_version_id   UUID,
    attachment_id         UUID,
    content_hash          TEXT,
    processing_status     TEXT NOT NULL DEFAULT 'pending',
    failure_reason        TEXT,
    retention_policy      TEXT NOT NULL,
    cache_expires_at      TIMESTAMPTZ,
    attempt_count         INT NOT NULL DEFAULT 0,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at          TIMESTAMPTZ,
    UNIQUE (owner_id, id),
    UNIQUE (owner_id, instance_item_id),
    FOREIGN KEY (owner_id, preparation_id)
        REFERENCES h9_category_pdf_preparations(owner_id, id)
        ON DELETE RESTRICT,
    FOREIGN KEY (owner_id, instance_id)
        REFERENCES h9_print_suite_instances(owner_id, id)
        ON DELETE RESTRICT,
    FOREIGN KEY (owner_id, attachment_id)
        REFERENCES attachments(owner_id, id)
        ON DELETE RESTRICT,
    FOREIGN KEY (template_version_id)
        REFERENCES print_template_versions(id)
        ON DELETE RESTRICT,
    CHECK (source_mode IN ('rendered', 'external_file')),
    CHECK (jsonb_typeof(source_file_bindings) = 'array'),
    CHECK (processing_status IN ('pending', 'processing', 'ready', 'failed')),
    CHECK (retention_policy IN ('gsp_5_year', 'short_cache')),
    CHECK (attempt_count >= 0),
    CHECK (
        (source_mode = 'rendered'
            AND source_data_version IS NOT NULL
            AND source_file_bindings = '[]'::jsonb
            AND template_version_id IS NOT NULL
            AND retention_policy = 'gsp_5_year'
            AND cache_expires_at IS NULL)
        OR (source_mode = 'external_file'
            AND source_data_version IS NULL
            AND jsonb_array_length(source_file_bindings) > 0
            AND template_version_id IS NULL
            AND retention_policy = 'short_cache'
            AND cache_expires_at IS NOT NULL)
    ),
    CHECK (
        (processing_status = 'ready'
            AND attachment_id IS NOT NULL
            AND content_hash ~ '^[0-9a-f]{64}$'
            AND failure_reason IS NULL
            AND processed_at IS NOT NULL)
        OR (processing_status = 'failed'
            AND attachment_id IS NULL
            AND content_hash IS NULL
            AND failure_reason IS NOT NULL
            AND processed_at IS NOT NULL)
        OR (processing_status IN ('pending', 'processing')
            AND attachment_id IS NULL
            AND content_hash IS NULL
            AND failure_reason IS NULL
            AND processed_at IS NULL)
    )
);

CREATE INDEX h9_category_pdf_outputs_instance_idx
    ON h9_category_pdf_outputs (owner_id, instance_id, processing_status, category_code);

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:h9.print_pdf.read')::uuid,
        'h9.print_pdf.read', 'H9 分类 PDF 查看'),
    (md5('auth_permission:h9.print_pdf.prepare')::uuid,
        'h9.print_pdf.prepare', 'H9 分类 PDF 生成与重试'),
    (md5('auth_permission:h9.print_pdf.download')::uuid,
        'h9.print_pdf.download', 'H9 分类 PDF 下载'),
    (md5('auth_permission:h9.print_pdf.emergency_print')::uuid,
        'h9.print_pdf.emergency_print', 'H9 分类 PDF 应急打印')
ON CONFLICT (lower(permission_code)) DO UPDATE
SET permission_name = EXCLUDED.permission_name;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code LIKE 'h9.print_pdf.%'
 WHERE lower(role.role_code) IN ('system_admin', 'warehouse_manager')
    OR (
        lower(role.role_code) IN ('custodian', 'owner_user')
        AND permission.permission_code = 'h9.print_pdf.read'
    )
ON CONFLICT DO NOTHING;

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
        ('prepare_category_pdf', '生成分类 PDF', 75),
        ('download_category_pdf', '下载分类 PDF', 76),
        ('emergency_print_category_pdf', '应急打印分类 PDF', 77)
) AS action(key, label, sort_order)
WHERE node.id = '00000000-0000-0000-0000-000000130061'
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions ORDER BY version_no DESC LIMIT 1
)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label,
    action_kind, enabled, sort_order
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
  AND button.action_key IN (
      'prepare_category_pdf',
      'download_category_pdf',
      'emergency_print_category_pdf'
  )
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE ON attachments TO wms_app;
GRANT SELECT, INSERT ON h9_document_file_bindings TO wms_app;
GRANT SELECT, INSERT, UPDATE ON h9_category_pdf_preparations TO wms_app;
GRANT SELECT, INSERT, UPDATE ON h9_category_pdf_outputs TO wms_app;
