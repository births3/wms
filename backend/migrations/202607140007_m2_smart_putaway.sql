-- US-M2-005：智能上架查询与 PC Web 上架确认使用独立动作权限。

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:m2.putaway.write')::uuid,
    'm2.putaway.write',
    'M2 上架确认'
)
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'm2.putaway.write'
 WHERE role.role_code IN ('system_admin', 'warehouse_manager', 'custodian')
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION seed_m2_putaway_permission_for_new_owner()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT role.id, permission.id
      FROM auth_roles role
      JOIN auth_permissions permission
        ON permission.permission_code = 'm2.putaway.write'
     WHERE role.owner_id = NEW.id
       AND role.role_code IN ('system_admin', 'warehouse_manager', 'custodian')
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_owners_seed_m2_putaway_permission ON auth_owners;
CREATE TRIGGER auth_owners_seed_m2_putaway_permission
AFTER INSERT ON auth_owners
FOR EACH ROW EXECUTE FUNCTION seed_m2_putaway_permission_for_new_owner();
