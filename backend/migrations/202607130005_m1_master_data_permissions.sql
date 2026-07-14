-- M1 master-data permission provisioning.

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:m1.master_data.write')::uuid,
    'm1.master_data.write',
    'M1 主数据维护'
)
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'm1.master_data.write'
 WHERE lower(role.role_code) = 'warehouse_manager'
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_m1_master_data_write_to_warehouse_manager()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF lower(NEW.role_code) = 'warehouse_manager' THEN
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT NEW.id, permission.id
          FROM auth_permissions permission
         WHERE permission.permission_code = 'm1.master_data.write'
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_m1_master_data_write
    ON auth_roles;
CREATE TRIGGER auth_roles_grant_m1_master_data_write
AFTER INSERT ON auth_roles
FOR EACH ROW
EXECUTE FUNCTION grant_m1_master_data_write_to_warehouse_manager();
