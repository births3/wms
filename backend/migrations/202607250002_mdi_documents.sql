-- M-DI report versions, ASN reuse links, and upstream delivery document versions.

CREATE UNIQUE INDEX IF NOT EXISTS products_owner_id_uidx
    ON products (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS suppliers_owner_id_uidx
    ON suppliers (owner_id, id);

CREATE TABLE IF NOT EXISTS drug_inspection_reports (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id),
    product_id          UUID NOT NULL,
    batch_no            TEXT NOT NULL CHECK (length(btrim(batch_no)) BETWEEN 1 AND 128),
    current_version_id  UUID,
    created_by          UUID NOT NULL REFERENCES auth_users(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, product_id, batch_no),
    FOREIGN KEY (owner_id, product_id) REFERENCES products(owner_id, id)
);

CREATE TABLE IF NOT EXISTS drug_inspection_report_versions (
    id                    UUID PRIMARY KEY,
    report_id             UUID NOT NULL REFERENCES drug_inspection_reports(id),
    owner_id              UUID NOT NULL REFERENCES auth_owners(id),
    version_number        INT NOT NULL CHECK (version_number > 0),
    report_no             TEXT NOT NULL CHECK (length(btrim(report_no)) BETWEEN 1 AND 128),
    original_file_id      UUID NOT NULL REFERENCES attachments(id),
    original_file_hash    TEXT NOT NULL,
    source                TEXT NOT NULL CHECK (source IN ('manual_upload', 'upstream_platform')),
    processing_mode       TEXT NOT NULL CHECK (processing_mode IN (
        'none', 'color_enhance', 'black_white_enhance'
    )),
    qualified             BOOLEAN NOT NULL,
    status                TEXT NOT NULL CHECK (status IN (
        'draft', 'pending_confirmation', 'confirmed', 'superseded'
    )),
    replaces_version_id   UUID REFERENCES drug_inspection_report_versions(id),
    modification_reason   TEXT,
    uploaded_by           UUID NOT NULL REFERENCES auth_users(id),
    submitted_at          TIMESTAMPTZ,
    reviewed_by           UUID REFERENCES auth_users(id),
    reviewed_at           TIMESTAMPTZ,
    review_result         TEXT CHECK (review_result IN ('confirmed', 'rejected')),
    review_comment        TEXT,
    customer_copy_status  TEXT NOT NULL DEFAULT 'not_requested' CHECK (
        customer_copy_status IN ('not_requested', 'queued', 'processing', 'available', 'failed')
    ),
    customer_copy_file_id UUID REFERENCES attachments(id),
    customer_copy_hash    TEXT,
    -- 权威原件为 PDF 且含数字签名字典标记时为 true；供客户平台提示，不验证证书。
    digitally_signed_original BOOLEAN NOT NULL DEFAULT FALSE,
    stamp_version_id      UUID,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (report_id, version_number),
    CHECK (reviewed_by IS NULL OR reviewed_by <> uploaded_by)
);

ALTER TABLE drug_inspection_reports
    ADD CONSTRAINT drug_inspection_reports_current_version_fk
    FOREIGN KEY (current_version_id) REFERENCES drug_inspection_report_versions(id);

CREATE UNIQUE INDEX IF NOT EXISTS drug_inspection_report_versions_one_current_idx
    ON drug_inspection_report_versions (owner_id, report_id)
    WHERE status = 'confirmed';

CREATE UNIQUE INDEX IF NOT EXISTS drug_inspection_report_versions_one_open_idx
    ON drug_inspection_report_versions (owner_id, report_id)
    WHERE status IN ('draft', 'pending_confirmation');

CREATE TABLE IF NOT EXISTS drug_inspection_asn_links (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id),
    asn_id              UUID NOT NULL,
    batch_no            TEXT NOT NULL,
    report_id           UUID NOT NULL REFERENCES drug_inspection_reports(id),
    source_version_id   UUID NOT NULL REFERENCES drug_inspection_report_versions(id),
    source              TEXT NOT NULL CHECK (source IN ('uploaded', 'reused')),
    linked_by           UUID NOT NULL REFERENCES auth_users(id),
    linked_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, asn_id, batch_no),
    FOREIGN KEY (owner_id, asn_id) REFERENCES receiving_orders(owner_id, id)
);

CREATE INDEX IF NOT EXISTS drug_inspection_asn_links_report_idx
    ON drug_inspection_asn_links (owner_id, report_id, linked_at DESC);

CREATE TABLE IF NOT EXISTS upstream_delivery_documents (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES auth_owners(id),
    supplier_id     UUID NOT NULL,
    created_by      UUID NOT NULL REFERENCES auth_users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (owner_id, supplier_id) REFERENCES suppliers(owner_id, id)
);

CREATE TABLE IF NOT EXISTS upstream_delivery_document_versions (
    id                   UUID PRIMARY KEY,
    document_id          UUID NOT NULL REFERENCES upstream_delivery_documents(id),
    owner_id             UUID NOT NULL REFERENCES auth_owners(id),
    version_number       INT NOT NULL CHECK (version_number > 0),
    modification_reason  TEXT,
    uploaded_by          UUID NOT NULL REFERENCES auth_users(id),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (document_id, version_number),
    CHECK (version_number = 1 OR length(btrim(modification_reason)) > 0)
);

CREATE TABLE IF NOT EXISTS upstream_delivery_document_files (
    version_id       UUID NOT NULL REFERENCES upstream_delivery_document_versions(id),
    attachment_id    UUID NOT NULL REFERENCES attachments(id),
    position         INT NOT NULL CHECK (position > 0),
    PRIMARY KEY (version_id, attachment_id),
    UNIQUE (version_id, position)
);

CREATE TABLE IF NOT EXISTS upstream_delivery_document_asn_links (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES auth_owners(id),
    version_id      UUID NOT NULL REFERENCES upstream_delivery_document_versions(id),
    asn_id          UUID NOT NULL,
    linked_by       UUID NOT NULL REFERENCES auth_users(id),
    linked_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (version_id, asn_id),
    FOREIGN KEY (owner_id, asn_id) REFERENCES receiving_orders(owner_id, id)
);

CREATE TABLE IF NOT EXISTS upstream_delivery_asn_current (
    owner_id        UUID NOT NULL REFERENCES auth_owners(id),
    asn_id          UUID NOT NULL,
    version_id      UUID NOT NULL REFERENCES upstream_delivery_document_versions(id),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, asn_id),
    FOREIGN KEY (owner_id, asn_id) REFERENCES receiving_orders(owner_id, id)
);

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:m-di.document.read')::uuid, 'm-di.document.read', '药检资料查看'),
    (md5('auth_permission:m-di.document.write')::uuid, 'm-di.document.write', '药检资料录入'),
    (md5('auth_permission:m-di.document.review')::uuid, 'm-di.document.review', '药检单审核')
ON CONFLICT (lower(permission_code)) DO UPDATE
SET permission_name = EXCLUDED.permission_name;

GRANT SELECT, INSERT, UPDATE ON drug_inspection_reports TO wms_app;
GRANT SELECT, INSERT, UPDATE ON drug_inspection_report_versions TO wms_app;
-- reuse_report 换绑走 ON CONFLICT DO UPDATE，必须授 UPDATE。
GRANT SELECT, INSERT, UPDATE ON drug_inspection_asn_links TO wms_app;
GRANT SELECT, INSERT ON upstream_delivery_documents TO wms_app;
GRANT SELECT, INSERT ON upstream_delivery_document_versions TO wms_app;
GRANT SELECT, INSERT ON upstream_delivery_document_files TO wms_app;
GRANT SELECT, INSERT ON upstream_delivery_document_asn_links TO wms_app;
GRANT SELECT, INSERT, UPDATE ON upstream_delivery_asn_current TO wms_app;

CREATE OR REPLACE FUNCTION seed_mdi_default_role_permissions(target_owner_id UUID)
RETURNS VOID
LANGUAGE sql
AS $$
    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT role.id, permission.id
      FROM auth_roles AS role
      JOIN auth_permissions AS permission
        ON role.role_code = 'system_admin'
        OR (
            role.role_code = 'warehouse_manager'
            AND permission.permission_code IN (
                'm-di.document.read',
                'm-di.document.write',
                'm-di.document.review'
            )
        )
        OR (
            role.role_code = 'receiving_clerk'
            AND permission.permission_code IN (
                'm-di.document.read',
                'm-di.document.write'
            )
        )
     WHERE role.owner_id = target_owner_id
       AND permission.permission_code IN (
           'm-di.document.read',
           'm-di.document.write',
           'm-di.document.review'
       )
    ON CONFLICT DO NOTHING;
$$;

CREATE OR REPLACE FUNCTION seed_mdi_roles_for_new_owner()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM seed_mdi_default_role_permissions(NEW.id);
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_owners_seed_mdi_roles ON auth_owners;
CREATE TRIGGER auth_owners_seed_mdi_roles
AFTER INSERT ON auth_owners
FOR EACH ROW EXECUTE FUNCTION seed_mdi_roles_for_new_owner();

SELECT seed_mdi_default_role_permissions(id) FROM auth_owners;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key, permission_key, sort_order, enabled
)
VALUES
    (
        '00000000-0000-0000-0000-000000130090',
        '00000000-0000-0000-0000-000000120018',
        3,
        'inbound.document_entry',
        'inbound/documents/entry',
        '入库资料录入',
        'm2-inbound-documents',
        'ClipboardList',
        'm-di.document.read',
        15,
        TRUE
    ),
    (
        '00000000-0000-0000-0000-000000130091',
        '00000000-0000-0000-0000-000000120018',
        3,
        'inbound.drug_inspection_review',
        'inbound/documents/drug_inspection_review',
        '药检单审核',
        'm-di-review',
        'ShieldCheck',
        'm-di.document.review',
        17,
        TRUE
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
    'standard',
    TRUE,
    action.sort_order
FROM admin_menu_draft_nodes AS node
CROSS JOIN (
    VALUES
        ('query', '查询', 10),
        ('refresh', '刷新', 20),
        ('upload', '上传', 30),
        ('reuse', '复用', 40),
        ('review', '审核', 50),
        ('detail', '详情', 60)
) AS action(key, label, sort_order)
WHERE node.id IN (
    '00000000-0000-0000-0000-000000130090',
    '00000000-0000-0000-0000-000000130091'
)
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
)
INSERT INTO admin_menu_version_nodes (
    id, version_id, source_node_id, parent_source_id, level, code, path, title,
    view_id, icon_key, permission_key, sort_order, enabled, created_at, updated_at
)
SELECT
    md5((SELECT id::text FROM version_row) || ':' || node.id::text)::uuid,
    (SELECT id FROM version_row),
    node.id,
    node.parent_id,
    node.level,
    node.code,
    node.path,
    node.title,
    node.view_id,
    node.icon_key,
    node.permission_key,
    node.sort_order,
    node.enabled,
    node.created_at,
    node.updated_at
FROM admin_menu_draft_nodes AS node
WHERE node.id IN (
    '00000000-0000-0000-0000-000000130090',
    '00000000-0000-0000-0000-000000130091'
)
  AND EXISTS (SELECT 1 FROM version_row)
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
)
INSERT INTO admin_menu_version_button_permissions (
    id, version_id, menu_source_node_id, action_key, action_label,
    action_kind, enabled, sort_order
)
SELECT
    md5((SELECT id::text FROM version_row) || ':' || permission.id::text)::uuid,
    (SELECT id FROM version_row),
    permission.menu_node_id,
    permission.action_key,
    permission.action_label,
    permission.action_kind,
    permission.enabled,
    permission.sort_order
FROM admin_menu_draft_button_permissions AS permission
WHERE permission.menu_node_id IN (
    '00000000-0000-0000-0000-000000130090',
    '00000000-0000-0000-0000-000000130091'
)
  AND EXISTS (SELECT 1 FROM version_row)
ON CONFLICT DO NOTHING;
