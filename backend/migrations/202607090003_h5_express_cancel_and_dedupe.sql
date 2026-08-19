-- H5 express: package-level waybill idempotency and cancel action.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conname = 'h5_express_waybills_owner_package_no_key'
           AND conrelid = 'h5_express_waybills'::regclass
    ) THEN
        ALTER TABLE h5_express_waybills
            ADD CONSTRAINT h5_express_waybills_owner_package_no_key
            UNIQUE (owner_id, package_no);
    END IF;
END $$;

INSERT INTO admin_menu_draft_button_permissions (
    id, menu_node_id, action_key, action_label, action_kind, enabled, sort_order
)
VALUES
    (md5('00000000-0000-0000-0000-000000130021:cancel')::uuid, '00000000-0000-0000-0000-000000130021', 'cancel', '取消', 'private', TRUE, 220)
ON CONFLICT DO NOTHING;

WITH version_row AS (
    SELECT id FROM admin_menu_versions WHERE version_no = 1
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
WHERE button.menu_node_id = '00000000-0000-0000-0000-000000130021'
  AND button.action_key = 'cancel'
ON CONFLICT DO NOTHING;
