-- US-TE-001：任务类型配置后端最小切片。

CREATE TABLE IF NOT EXISTS task_types (
    id                UUID PRIMARY KEY,
    owner_id          UUID NOT NULL REFERENCES auth_owners(id) ON DELETE CASCADE,
    task_type_code    TEXT NOT NULL CHECK (task_type_code ~ '^[a-z0-9][a-z0-9_.-]{0,63}$'),
    task_type_name    TEXT NOT NULL CHECK (length(trim(task_type_name)) BETWEEN 1 AND 128),
    default_priority  INT NOT NULL CHECK (default_priority BETWEEN 0 AND 1000),
    estimated_minutes INT NOT NULL CHECK (estimated_minutes BETWEEN 1 AND 10080),
    mergeable         BOOLEAN NOT NULL,
    insertable        BOOLEAN NOT NULL,
    enabled           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    version           BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    UNIQUE (owner_id, task_type_code)
);

CREATE UNIQUE INDEX IF NOT EXISTS task_types_owner_code_lower_idx
    ON task_types (owner_id, lower(task_type_code));

CREATE INDEX IF NOT EXISTS task_types_owner_enabled_idx
    ON task_types (owner_id, enabled, task_type_code);

GRANT SELECT, INSERT, UPDATE ON task_types TO wms_app;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:mte.task_type.read')::uuid, 'mte.task_type.read', 'M-TE 任务类型查询'),
    (md5('auth_permission:mte.task_type.write')::uuid, 'mte.task_type.write', 'M-TE 任务类型配置')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN ('mte.task_type.read', 'mte.task_type.write')
 WHERE lower(role.role_code) IN ('system_admin', 'warehouse_manager')
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION seed_mte_task_type_permissions()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT NEW.id, permission.id
      FROM auth_permissions permission
     WHERE permission.permission_code IN ('mte.task_type.read', 'mte.task_type.write')
       AND lower(NEW.role_code) IN ('system_admin', 'warehouse_manager')
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_mte_task_type_permissions ON auth_roles;
CREATE TRIGGER auth_roles_grant_mte_task_type_permissions
AFTER INSERT ON auth_roles
FOR EACH ROW EXECUTE FUNCTION seed_mte_task_type_permissions();

CREATE OR REPLACE FUNCTION seed_mte_task_types_for_owner()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO task_types (
        id, owner_id, task_type_code, task_type_name, default_priority,
        estimated_minutes, mergeable, insertable, enabled
    )
    VALUES
        (md5(format('mte-task-type:%s:pick', NEW.id))::uuid, NEW.id, 'pick', '拣选', 100, 15, TRUE, TRUE, TRUE),
        (md5(format('mte-task-type:%s:putaway', NEW.id))::uuid, NEW.id, 'putaway', '上架', 80, 20, TRUE, TRUE, TRUE),
        (md5(format('mte-task-type:%s:replenish', NEW.id))::uuid, NEW.id, 'replenish', '补货', 90, 20, TRUE, TRUE, TRUE),
        (md5(format('mte-task-type:%s:relocation', NEW.id))::uuid, NEW.id, 'relocation', '移库', 60, 15, TRUE, FALSE, TRUE),
        (md5(format('mte-task-type:%s:inventory_count', NEW.id))::uuid, NEW.id, 'inventory_count', '盘点', 70, 30, FALSE, FALSE, TRUE),
        (md5(format('mte-task-type:%s:loading', NEW.id))::uuid, NEW.id, 'loading', '装车', 50, 10, TRUE, TRUE, TRUE),
        (md5(format('mte-task-type:%s:return_putaway', NEW.id))::uuid, NEW.id, 'return_putaway', '退货上架', 85, 20, TRUE, TRUE, TRUE)
    ON CONFLICT (owner_id, task_type_code) DO NOTHING;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_owners_seed_mte_task_types ON auth_owners;
CREATE TRIGGER auth_owners_seed_mte_task_types
AFTER INSERT ON auth_owners
FOR EACH ROW EXECUTE FUNCTION seed_mte_task_types_for_owner();

INSERT INTO task_types (
    id, owner_id, task_type_code, task_type_name, default_priority,
    estimated_minutes, mergeable, insertable, enabled
)
SELECT md5(format('mte-task-type:%s:%s', owner_row.id, preset.task_type_code))::uuid,
       owner_row.id,
       preset.task_type_code,
       preset.task_type_name,
       preset.default_priority,
       preset.estimated_minutes,
       preset.mergeable,
       preset.insertable,
       TRUE
  FROM auth_owners owner_row
 CROSS JOIN (
    VALUES
        ('pick', '拣选', 100, 15, TRUE, TRUE),
        ('putaway', '上架', 80, 20, TRUE, TRUE),
        ('replenish', '补货', 90, 20, TRUE, TRUE),
        ('relocation', '移库', 60, 15, TRUE, FALSE),
        ('inventory_count', '盘点', 70, 30, FALSE, FALSE),
        ('loading', '装车', 50, 10, TRUE, TRUE),
        ('return_putaway', '退货上架', 85, 20, TRUE, TRUE)
 ) AS preset(task_type_code, task_type_name, default_priority, estimated_minutes, mergeable, insertable)
ON CONFLICT (owner_id, task_type_code) DO NOTHING;
