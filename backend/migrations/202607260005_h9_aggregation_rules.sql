-- US-H9-007 controlled equality dimensions and immutable aggregation-rule versions.

ALTER TABLE outbound_orders
    ADD COLUMN IF NOT EXISTS invoice_no TEXT,
    ADD COLUMN IF NOT EXISTS transport_mode_code TEXT,
    ADD COLUMN IF NOT EXISTS department_code TEXT,
    ADD COLUMN IF NOT EXISTS sales_group_code TEXT,
    ADD COLUMN IF NOT EXISTS order_group_no TEXT,
    ADD COLUMN IF NOT EXISTS business_type_code TEXT;

CREATE TABLE IF NOT EXISTS h9_aggregation_field_catalog (
    field_code     TEXT PRIMARY KEY,
    display_name   TEXT NOT NULL,
    value_type     TEXT NOT NULL,
    enabled        BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order     INT NOT NULL,
    CHECK (value_type IN ('string')),
    CHECK (length(btrim(field_code)) BETWEEN 1 AND 64),
    CHECK (length(btrim(display_name)) BETWEEN 1 AND 100)
);

INSERT INTO h9_aggregation_field_catalog (
    field_code, display_name, value_type, enabled, sort_order
)
VALUES
    ('document_type', '单据类型', 'string', TRUE, 10),
    ('erp_order_no', 'ERP 订单号', 'string', TRUE, 20),
    ('invoice_no', '发票号', 'string', TRUE, 30),
    ('transport_mode_code', '运输方式', 'string', TRUE, 40),
    ('department_code', '业务部门', 'string', TRUE, 50),
    ('sales_group_code', '销售组', 'string', TRUE, 60),
    ('order_group_no', '订单组号', 'string', TRUE, 70),
    ('business_type_code', '业务类型', 'string', TRUE, 80)
ON CONFLICT (field_code) DO UPDATE
SET display_name = EXCLUDED.display_name,
    value_type = EXCLUDED.value_type,
    enabled = EXCLUDED.enabled,
    sort_order = EXCLUDED.sort_order;

CREATE TABLE IF NOT EXISTS h9_aggregation_rule_versions (
    id             UUID PRIMARY KEY,
    owner_id       UUID NOT NULL,
    version_no     INT NOT NULL,
    name           TEXT NOT NULL,
    status         TEXT NOT NULL DEFAULT 'draft',
    dimensions     JSONB NOT NULL,
    test_result    JSONB,
    tested_by      UUID,
    tested_at      TIMESTAMPTZ,
    published_by   UUID,
    published_at   TIMESTAMPTZ,
    disabled_by    UUID,
    disabled_at    TIMESTAMPTZ,
    created_by     UUID NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, id),
    UNIQUE (owner_id, version_no),
    CHECK (version_no > 0),
    CHECK (length(btrim(name)) BETWEEN 1 AND 100),
    CHECK (status IN ('draft', 'tested', 'published', 'disabled')),
    CHECK (jsonb_typeof(dimensions) = 'array' AND jsonb_array_length(dimensions) > 0),
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
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS h9_aggregation_rule_one_published_uidx
    ON h9_aggregation_rule_versions (owner_id)
    WHERE status = 'published';

CREATE OR REPLACE FUNCTION reject_h9_aggregation_rule_content_rewrite()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status IN ('published', 'disabled')
       AND (
           NEW.owner_id IS DISTINCT FROM OLD.owner_id
           OR NEW.version_no IS DISTINCT FROM OLD.version_no
           OR NEW.name IS DISTINCT FROM OLD.name
           OR NEW.dimensions IS DISTINCT FROM OLD.dimensions
           OR NEW.test_result IS DISTINCT FROM OLD.test_result
           OR NEW.tested_by IS DISTINCT FROM OLD.tested_by
           OR NEW.tested_at IS DISTINCT FROM OLD.tested_at
           OR NEW.published_by IS DISTINCT FROM OLD.published_by
           OR NEW.published_at IS DISTINCT FROM OLD.published_at
           OR NEW.created_by IS DISTINCT FROM OLD.created_by
           OR NEW.created_at IS DISTINCT FROM OLD.created_at
       )
    THEN
        RAISE EXCEPTION 'published H9 aggregation rule content is immutable';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS h9_aggregation_rule_content_immutable
    ON h9_aggregation_rule_versions;
CREATE TRIGGER h9_aggregation_rule_content_immutable
BEFORE UPDATE ON h9_aggregation_rule_versions
FOR EACH ROW
EXECUTE FUNCTION reject_h9_aggregation_rule_content_rewrite();

ALTER TABLE h9_delivery_note_groups
    ADD COLUMN IF NOT EXISTS aggregation_rule_version_id UUID,
    ADD COLUMN IF NOT EXISTS aggregation_rule_version_no INT,
    ADD COLUMN IF NOT EXISTS aggregation_rule_snapshot JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS aggregation_group_key JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD FOREIGN KEY (owner_id, aggregation_rule_version_id)
        REFERENCES h9_aggregation_rule_versions(owner_id, id)
        ON DELETE RESTRICT,
    ADD CHECK (
        (aggregation_rule_version_id IS NULL
            AND aggregation_rule_version_no IS NULL
            AND aggregation_rule_snapshot = '{}'::jsonb
            AND aggregation_group_key = '{}'::jsonb)
        OR
        (aggregation_rule_version_id IS NOT NULL
            AND aggregation_rule_version_no IS NOT NULL
            AND jsonb_typeof(aggregation_rule_snapshot) = 'object'
            AND jsonb_typeof(aggregation_group_key) = 'object')
    );

DROP INDEX IF EXISTS h9_delivery_note_groups_scheduled_once_uidx;
CREATE UNIQUE INDEX h9_delivery_note_groups_scheduled_once_uidx
    ON h9_delivery_note_groups (
        owner_id,
        warehouse_id,
        delivery_address_id,
        cutoff_plan_id,
        scheduled_cutoff_at,
        aggregation_group_key
    )
    WHERE cutoff_mode = 'scheduled';

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
        ('create_rule', '新建规则版本', 65),
        ('test_rule', '测试规则', 66),
        ('publish_rule', '发布规则', 67),
        ('disable_rule', '停用规则', 68)
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
  AND button.action_key IN ('create_rule', 'test_rule', 'publish_rule', 'disable_rule')
ON CONFLICT DO NOTHING;

GRANT SELECT ON h9_aggregation_field_catalog TO wms_app;
GRANT SELECT, INSERT, UPDATE ON h9_aggregation_rule_versions TO wms_app;
