-- m4 出库写权限：handler 要求 m4.write，历史种子仅有 m4.read。
-- INSERT 触发器 grant_new_permission_to_system_admin_roles 会自动授予各货主 system_admin。
INSERT INTO auth_permissions (id, permission_code, permission_name)
SELECT gen_random_uuid(), 'm4.write', 'm4.write'
WHERE NOT EXISTS (
    SELECT 1 FROM auth_permissions WHERE lower(permission_code) = lower('m4.write')
);
