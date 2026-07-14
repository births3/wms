-- M3-002 召回标记的双人取消审批与恢复依据。
CREATE TABLE IF NOT EXISTS inventory_recall_actions (
    id                       UUID PRIMARY KEY,
    owner_id                 UUID NOT NULL,
    batch_id                 UUID NOT NULL,
    recall_approval_source   TEXT NOT NULL,
    recall_approval_id       TEXT NOT NULL,
    previous_quality_status  TEXT NOT NULL,
    marked_by                UUID NOT NULL,
    marked_at                TIMESTAMPTZ NOT NULL,
    canceled_by              UUID,
    canceled_at              TIMESTAMPTZ,
    cancel_approval_id       TEXT,
    cancel_reason            TEXT,
    FOREIGN KEY (owner_id, batch_id) REFERENCES inventory_batches(owner_id, id)
);

CREATE UNIQUE INDEX IF NOT EXISTS inventory_recall_actions_active_batch_idx
    ON inventory_recall_actions (owner_id, batch_id)
    WHERE canceled_at IS NULL;

CREATE INDEX IF NOT EXISTS inventory_recall_actions_owner_batch_idx
    ON inventory_recall_actions (owner_id, batch_id, marked_at DESC);

GRANT SELECT, INSERT, UPDATE ON inventory_recall_actions TO wms_app;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:m3.recall.cancel')::uuid, 'm3.recall.cancel', 'M3 取消召回'),
    (md5('auth_permission:m3.recall.approve')::uuid, 'm3.recall.approve', 'M3 召回取消质量审批')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN ('m3.recall.cancel', 'm3.recall.approve')
 WHERE role.role_code = 'system_admin'
ON CONFLICT DO NOTHING;
