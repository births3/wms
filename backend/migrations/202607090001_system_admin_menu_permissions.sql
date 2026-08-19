-- Keep the built-in system_admin role aligned with published admin menu permission keys.

WITH menu_permissions AS (
    SELECT DISTINCT permission_key AS permission_code
      FROM admin_menu_version_nodes
     WHERE permission_key IS NOT NULL
       AND trim(permission_key) <> ''
    UNION
    SELECT permission_code
      FROM (VALUES
        ('h1.menu.write'),
        ('h1.menu.publish')
      ) AS extra(permission_code)
),
inserted_permissions AS (
    INSERT INTO auth_permissions (id, permission_code, permission_name)
    SELECT
        md5('auth_permission:' || permission_code)::uuid,
        permission_code,
        permission_code
      FROM menu_permissions
    ON CONFLICT DO NOTHING
    RETURNING id
)
INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN (SELECT permission_code FROM menu_permissions)
 WHERE role.role_code = 'system_admin'
ON CONFLICT DO NOTHING;
