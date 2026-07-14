-- US-TE-002/003/005/008：任务组、任务创建、分派与执行主链。

CREATE TABLE IF NOT EXISTS task_groups (
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    task_group_code    TEXT NOT NULL CHECK (task_group_code ~ '^[a-z0-9][a-z0-9_.-]{0,63}$'),
    task_group_name    TEXT NOT NULL CHECK (length(trim(task_group_name)) BETWEEN 1 AND 128),
    warehouse_id       UUID NOT NULL,
    zone_ids           UUID[] NOT NULL DEFAULT '{}',
    task_type_codes    TEXT[] NOT NULL CHECK (cardinality(task_type_codes) > 0),
    enabled            BOOLEAN NOT NULL DEFAULT TRUE,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    version            BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (owner_id, task_group_code),
    FOREIGN KEY (owner_id, warehouse_id) REFERENCES warehouses(owner_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS task_group_memberships (
    task_group_id UUID NOT NULL REFERENCES task_groups(id) ON DELETE CASCADE,
    owner_id      UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    user_id       UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (task_group_id, user_id),
    FOREIGN KEY (user_id, owner_id)
        REFERENCES auth_user_owner_bindings(user_id, owner_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS task_group_memberships_owner_user_idx
    ON task_group_memberships (owner_id, user_id, task_group_id);

CREATE TABLE IF NOT EXISTS warehouse_tasks (
    id                      UUID PRIMARY KEY,
    owner_id                UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    task_no                 TEXT NOT NULL,
    task_type_code          TEXT NOT NULL,
    source_module           TEXT NOT NULL,
    source_doc_type         TEXT NOT NULL,
    source_doc_id           UUID,
    source_doc_no           TEXT NOT NULL,
    source_line_no          INT,
    source_task_key         TEXT NOT NULL,
    warehouse_id            UUID NOT NULL,
    task_group_code         TEXT NOT NULL,
    product_id              UUID,
    product_code            TEXT NOT NULL,
    batch_id                UUID,
    batch_no                TEXT,
    planned_qty             BIGINT NOT NULL CHECK (planned_qty > 0),
    actual_qty              BIGINT CHECK (actual_qty >= 0),
    source_location_id      UUID,
    source_location_code    TEXT,
    target_location_id      UUID,
    target_location_code    TEXT,
    priority                INT NOT NULL CHECK (priority BETWEEN 0 AND 1000),
    estimated_minutes       INT NOT NULL CHECK (estimated_minutes BETWEEN 1 AND 10080),
    assignee_user_id        UUID REFERENCES auth_users(id),
    status                  TEXT NOT NULL DEFAULT 'pending_assignment',
    exception_code          TEXT,
    exception_note          TEXT,
    assigned_at             TIMESTAMPTZ,
    dispatched_at           TIMESTAMPTZ,
    started_at              TIMESTAMPTZ,
    completed_at            TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                 BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (owner_id, task_no),
    UNIQUE (owner_id, source_task_key),
    FOREIGN KEY (owner_id, warehouse_id) REFERENCES warehouses(owner_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (owner_id, task_type_code) REFERENCES task_types(owner_id, task_type_code),
    FOREIGN KEY (owner_id, task_group_code) REFERENCES task_groups(owner_id, task_group_code),
    CHECK (status IN (
        'pending_release', 'pending_assignment', 'assigned', 'dispatched',
        'in_progress', 'completed', 'exception', 'cancelled'
    )),
    CHECK (exception_code IS NULL OR length(trim(exception_code)) BETWEEN 1 AND 64)
);

CREATE INDEX IF NOT EXISTS warehouse_tasks_owner_queue_idx
    ON warehouse_tasks (owner_id, status, priority DESC, created_at, id);
CREATE INDEX IF NOT EXISTS warehouse_tasks_owner_assignee_idx
    ON warehouse_tasks (owner_id, assignee_user_id, status, priority DESC, created_at);
CREATE INDEX IF NOT EXISTS warehouse_tasks_owner_source_idx
    ON warehouse_tasks (owner_id, source_module, source_doc_type, source_doc_id);
CREATE UNIQUE INDEX IF NOT EXISTS warehouse_tasks_source_identity_idx
    ON warehouse_tasks (
        owner_id,
        source_module,
        source_doc_type,
        COALESCE(source_doc_id, '00000000-0000-0000-0000-000000000000'::uuid),
        (CASE WHEN source_doc_id IS NULL THEN source_doc_no ELSE '' END),
        COALESCE(source_line_no, -1),
        task_type_code,
        COALESCE(batch_id, '00000000-0000-0000-0000-000000000000'::uuid)
    );

CREATE TABLE IF NOT EXISTS task_execution_events (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    task_id          UUID NOT NULL REFERENCES warehouse_tasks(id) ON DELETE RESTRICT,
    action           TEXT NOT NULL,
    from_status      TEXT,
    to_status        TEXT NOT NULL,
    actor_user_id    UUID NOT NULL,
    assignee_user_id UUID REFERENCES auth_users(id) ON DELETE RESTRICT,
    actual_qty       BIGINT,
    exception_code   TEXT,
    exception_note   TEXT,
    occurred_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS task_execution_events_owner_task_idx
    ON task_execution_events (owner_id, task_id, occurred_at, id);

GRANT SELECT, INSERT, UPDATE ON task_groups TO wms_app;
GRANT SELECT, INSERT, DELETE ON task_group_memberships TO wms_app;
GRANT SELECT, INSERT, UPDATE ON warehouse_tasks TO wms_app;
GRANT SELECT, INSERT ON task_execution_events TO wms_app;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:mte.task.read')::uuid, 'mte.task.read', 'M-TE 任务查询'),
    (md5('auth_permission:mte.task.read_all')::uuid, 'mte.task.read_all', 'M-TE 全仓任务查询'),
    (md5('auth_permission:mte.task.write')::uuid, 'mte.task.write', 'M-TE 任务创建'),
    (md5('auth_permission:mte.task.assign')::uuid, 'mte.task.assign', 'M-TE 任务分派下发'),
    (md5('auth_permission:mte.task.execute')::uuid, 'mte.task.execute', 'M-TE 任务执行'),
    (md5('auth_permission:mte.task_group.write')::uuid, 'mte.task_group.write', 'M-TE 任务组配置')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON (
        lower(role.role_code) IN ('system_admin', 'warehouse_manager')
        AND permission.permission_code IN (
            'mte.task.read', 'mte.task.read_all', 'mte.task.write', 'mte.task.assign',
            'mte.task.execute', 'mte.task_group.write'
        )
    ) OR (
        lower(role.role_code) = 'custodian'
        AND permission.permission_code IN ('mte.task.read', 'mte.task.execute')
    )
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION seed_mte_task_permissions_for_role()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT NEW.id, permission.id
      FROM auth_permissions permission
     WHERE (
            lower(NEW.role_code) IN ('system_admin', 'warehouse_manager')
            AND permission.permission_code IN (
                'mte.task.read', 'mte.task.read_all', 'mte.task.write', 'mte.task.assign',
                'mte.task.execute', 'mte.task_group.write'
            )
        ) OR (
            lower(NEW.role_code) = 'custodian'
            AND permission.permission_code IN ('mte.task.read', 'mte.task.execute')
        )
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_mte_task_permissions ON auth_roles;
CREATE TRIGGER auth_roles_grant_mte_task_permissions
AFTER INSERT ON auth_roles
FOR EACH ROW EXECUTE FUNCTION seed_mte_task_permissions_for_role();

INSERT INTO task_groups (
    id, owner_id, task_group_code, task_group_name, warehouse_id,
    zone_ids, task_type_codes, enabled
)
SELECT md5(format('mte-default-task-group:%s:%s', warehouse.owner_id, warehouse.id))::uuid,
       warehouse.owner_id,
       'default-' || replace(warehouse.id::text, '-', ''),
       warehouse.warehouse_name || '默认任务组',
       warehouse.id,
       '{}',
       ARRAY['inventory_count', 'loading', 'pick', 'putaway', 'relocation', 'replenish', 'return_putaway'],
       TRUE
  FROM warehouses warehouse
ON CONFLICT (owner_id, task_group_code) DO NOTHING;
