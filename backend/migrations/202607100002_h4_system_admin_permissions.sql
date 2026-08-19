-- Grant the built-in system_admin role every H4 permission used by handlers.

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN (
        'h4.notify.read',
        'h4.notify.write',
        'h4.notify.send',
        'h4.approval.write'
    )
 WHERE role.role_code = 'system_admin'
ON CONFLICT DO NOTHING;
