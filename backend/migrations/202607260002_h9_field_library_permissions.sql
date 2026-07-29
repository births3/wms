-- US-H9-002：字段库草稿维护和发布权限。

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:h9.print_template.read')::uuid, 'h9.print_template.read', 'H9 打印模板读取'),
    (md5('auth_permission:h9.print_template.write')::uuid, 'h9.print_template.write', 'H9 打印模板维护'),
    (md5('auth_permission:h9.print_template.publish')::uuid, 'h9.print_template.publish', 'H9 打印模板发布'),
    (md5('auth_permission:h9.print_template.print')::uuid, 'h9.print_template.print', 'H9 业务打印')
ON CONFLICT (lower(permission_code)) DO UPDATE
SET permission_name = EXCLUDED.permission_name;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission ON (
      lower(role.role_code) = 'system_admin'
      OR (
          lower(role.role_code) IN ('warehouse_manager', 'receiving_clerk', 'custodian')
          AND permission.permission_code IN ('h9.print_template.read', 'h9.print_template.print')
      )
      OR (
          lower(role.role_code) = 'owner_user'
          AND permission.permission_code = 'h9.print_template.read'
      )
  )
 WHERE permission.permission_code LIKE 'h9.print_template.%'
ON CONFLICT DO NOTHING;
