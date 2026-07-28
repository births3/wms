-- US-DI-002：货主透明 PNG 图章双人发布、不可变版本与客户 PDF 副本任务。

CREATE TABLE IF NOT EXISTS drug_inspection_stamp_versions (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id),
    version_number      INT NOT NULL CHECK (version_number > 0),
    png_attachment_id   UUID NOT NULL REFERENCES attachments(id),
    relative_x          DOUBLE PRECISION NOT NULL CHECK (relative_x BETWEEN 0 AND 1),
    relative_y          DOUBLE PRECISION NOT NULL CHECK (relative_y BETWEEN 0 AND 1),
    relative_width      DOUBLE PRECISION NOT NULL CHECK (
        relative_width > 0 AND relative_width <= 1
    ),
    status              TEXT NOT NULL CHECK (
        status IN ('draft', 'pending_review', 'published', 'superseded')
    ),
    configured_by       UUID NOT NULL REFERENCES auth_users(id),
    submitted_at        TIMESTAMPTZ,
    reviewed_by         UUID REFERENCES auth_users(id),
    reviewed_at         TIMESTAMPTZ,
    review_comment      TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, version_number),
    CHECK (reviewed_by IS NULL OR reviewed_by <> configured_by)
);

CREATE UNIQUE INDEX IF NOT EXISTS drug_inspection_stamp_one_published_idx
    ON drug_inspection_stamp_versions (owner_id)
    WHERE status = 'published';

CREATE UNIQUE INDEX IF NOT EXISTS drug_inspection_stamp_one_open_idx
    ON drug_inspection_stamp_versions (owner_id)
    WHERE status IN ('draft', 'pending_review');

ALTER TABLE drug_inspection_report_versions
    ADD CONSTRAINT drug_inspection_report_versions_stamp_version_fk
    FOREIGN KEY (stamp_version_id) REFERENCES drug_inspection_stamp_versions(id);

CREATE TABLE IF NOT EXISTS drug_inspection_customer_copy_jobs (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id),
    report_version_id   UUID NOT NULL REFERENCES drug_inspection_report_versions(id),
    status              TEXT NOT NULL CHECK (
        status IN ('queued', 'processing', 'succeeded', 'failed', 'oversize_review')
    ),
    attempt_count       INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    processing_rule     TEXT NOT NULL DEFAULT 'mdi-image-v1',
    oversize_reason     TEXT,
    oversize_approved_by UUID REFERENCES auth_users(id),
    candidate_file_id   UUID REFERENCES attachments(id),
    candidate_hash      TEXT,
    candidate_size      BIGINT CHECK (candidate_size IS NULL OR candidate_size > 0),
    last_error          TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at          TIMESTAMPTZ,
    finished_at         TIMESTAMPTZ,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS drug_inspection_customer_copy_one_active_idx
    ON drug_inspection_customer_copy_jobs (owner_id, report_version_id)
    WHERE status IN ('queued', 'processing', 'oversize_review');

CREATE INDEX IF NOT EXISTS drug_inspection_customer_copy_jobs_poll_idx
    ON drug_inspection_customer_copy_jobs (status, created_at)
    WHERE status IN ('queued', 'failed');

CREATE TABLE IF NOT EXISTS drug_inspection_processing_rule_versions (
    id                  UUID PRIMARY KEY,
    owner_id            UUID NOT NULL REFERENCES auth_owners(id),
    version_number      INT NOT NULL CHECK (version_number > 0),
    rule_code           TEXT NOT NULL,
    apply_scope         TEXT NOT NULL CHECK (
        apply_scope IN ('future_only', 'reprocess_current')
    ),
    reprocess_job_count INT NOT NULL DEFAULT 0 CHECK (reprocess_job_count >= 0),
    published_by        UUID NOT NULL REFERENCES auth_users(id),
    published_at        TIMESTAMPTZ NOT NULL,
    UNIQUE (owner_id, version_number)
);

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (
        md5('auth_permission:m-di.stamp.manage')::uuid,
        'm-di.stamp.manage',
        '药检图章配置'
    ),
    (
        md5('auth_permission:m-di.stamp.review')::uuid,
        'm-di.stamp.review',
        '药检图章发布审核'
    )
ON CONFLICT (lower(permission_code)) DO UPDATE
SET permission_name = EXCLUDED.permission_name;

GRANT SELECT, INSERT, UPDATE ON drug_inspection_stamp_versions TO wms_app;
GRANT SELECT, INSERT, UPDATE ON drug_inspection_customer_copy_jobs TO wms_app;
-- publish_processing_rule 发布后需要回写 reprocess_job_count，必须授 UPDATE。
GRANT SELECT, INSERT, UPDATE ON drug_inspection_processing_rule_versions TO wms_app;

INSERT INTO admin_menu_draft_nodes (
    id, parent_id, level, code, path, title, view_id, icon_key,
    permission_key, sort_order, enabled
)
VALUES (
    '00000000-0000-0000-0000-000000130092',
    '00000000-0000-0000-0000-000000120018',
    3,
    'inbound.drug_inspection_stamp',
    'inbound/documents/drug_inspection_stamp',
    '药检图章配置',
    'm-di-stamp',
    'Stamp',
    'm-di.stamp.manage',
    85,
    TRUE
)
ON CONFLICT DO NOTHING;

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
SELECT
    md5('00000000-0000-0000-0000-000000130092:' || action.key)::uuid,
    '00000000-0000-0000-0000-000000130092',
    action.key,
    action.label,
    'standard',
    TRUE,
    action.sort_order
FROM (
    VALUES
        ('query', '查询', 10),
        ('upload', '上传图章', 20),
        ('submit', '提交审核', 30),
        ('review', '审核发布', 40),
        ('history', '版本记录', 50)
) AS action(key, label, sort_order)
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
WHERE node.id = '00000000-0000-0000-0000-000000130092'
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
WHERE permission.menu_node_id = '00000000-0000-0000-0000-000000130092'
  AND EXISTS (SELECT 1 FROM version_row)
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION seed_mdi_copy_default_role_permissions(target_owner_id UUID)
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
                'm-di.stamp.manage',
                'm-di.stamp.review'
            )
        )
     WHERE role.owner_id = target_owner_id
       AND permission.permission_code IN (
           'm-di.stamp.manage',
           'm-di.stamp.review'
       )
    ON CONFLICT DO NOTHING;
$$;

SELECT seed_mdi_copy_default_role_permissions(id) FROM auth_owners;
