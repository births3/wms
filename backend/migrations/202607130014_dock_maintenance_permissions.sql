-- US-DOCK-001：只有系统管理员和仓库主管可维护月台。

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:dock.manage')::uuid,
    'dock.manage',
    '月台档案维护'
)
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'dock.manage'
 WHERE role.role_code IN ('system_admin', 'warehouse_manager')
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_dock_manage_to_warehouse_manager()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF lower(NEW.role_code) = 'warehouse_manager' THEN
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT NEW.id, permission.id
          FROM auth_permissions permission
         WHERE permission.permission_code = 'dock.manage'
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_dock_manage
    ON auth_roles;
CREATE TRIGGER auth_roles_grant_dock_manage
AFTER INSERT ON auth_roles
FOR EACH ROW
EXECUTE FUNCTION grant_dock_manage_to_warehouse_manager();

-- 预约模块后续复用此表；此处建立删除保护所需的最小关联约束。
CREATE TABLE IF NOT EXISTS dock_appointments (
    id          UUID PRIMARY KEY,
    dock_id     UUID NOT NULL REFERENCES warehouse_docks(id) ON DELETE RESTRICT,
    status      TEXT NOT NULL DEFAULT 'pending',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status IN ('pending', 'confirmed', 'arrived', 'completed', 'timed_out', 'cancelled'))
);

GRANT SELECT, INSERT, UPDATE, DELETE ON dock_appointments TO wms_app;
GRANT DELETE ON warehouse_docks TO wms_app;
