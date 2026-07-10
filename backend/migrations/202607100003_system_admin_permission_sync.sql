-- Keep built-in system_admin roles aligned regardless of tenant provisioning order.

CREATE OR REPLACE FUNCTION prevent_system_admin_role_code_change()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF lower(OLD.role_code) <> lower(NEW.role_code)
       AND (lower(OLD.role_code) = 'system_admin'
            OR lower(NEW.role_code) = 'system_admin') THEN
        RAISE EXCEPTION 'system_admin role_code is immutable'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_prevent_system_admin_role_code_change ON auth_roles;
CREATE TRIGGER auth_roles_prevent_system_admin_role_code_change
BEFORE UPDATE OF role_code ON auth_roles
FOR EACH ROW
EXECUTE FUNCTION prevent_system_admin_role_code_change();

CREATE OR REPLACE FUNCTION grant_permissions_to_new_system_admin_role()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF lower(NEW.role_code) = 'system_admin' THEN
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT NEW.id, permission.id
          FROM auth_permissions permission
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_system_admin_permissions ON auth_roles;
CREATE TRIGGER auth_roles_grant_system_admin_permissions
AFTER INSERT ON auth_roles
FOR EACH ROW
EXECUTE FUNCTION grant_permissions_to_new_system_admin_role();

CREATE OR REPLACE FUNCTION grant_new_permission_to_system_admin_roles()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT role.id, NEW.id
      FROM auth_roles role
     WHERE lower(role.role_code) = 'system_admin'
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_permissions_grant_to_system_admin ON auth_permissions;
CREATE TRIGGER auth_permissions_grant_to_system_admin
AFTER INSERT ON auth_permissions
FOR EACH ROW
EXECUTE FUNCTION grant_new_permission_to_system_admin_roles();

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
 CROSS JOIN auth_permissions permission
 WHERE lower(role.role_code) = 'system_admin'
ON CONFLICT DO NOTHING;
