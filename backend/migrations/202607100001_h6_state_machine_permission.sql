-- US-H6-001 状态机定义读取权限，并授予内置系统管理员。
INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:h6.state_machine.read')::uuid,
    'h6.state_machine.read',
    'H6 状态机读取'
)
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'h6.state_machine.read'
 WHERE role.role_code = 'system_admin'
ON CONFLICT DO NOTHING;
