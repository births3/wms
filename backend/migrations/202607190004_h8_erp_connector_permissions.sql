-- US-H8-001：ERP 连接专用权限（不再复用 m1.config.*）
-- h8.erp_connector.read  → 系统管理员 + 仓库主管
-- h8.erp_connector.write → 仅系统管理员（system_admin 由 auth_permissions 触发器自动授予）

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (
        md5('auth_permission:h8.erp_connector.read')::uuid,
        'h8.erp_connector.read',
        'H8 ERP 连接只读'
    ),
    (
        md5('auth_permission:h8.erp_connector.write')::uuid,
        'h8.erp_connector.write',
        'H8 ERP 连接维护'
    )
ON CONFLICT DO NOTHING;

-- 仓库主管只读
INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'h8.erp_connector.read'
 WHERE lower(role.role_code) = 'warehouse_manager'
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_h8_erp_connector_read_to_warehouse_manager()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF lower(NEW.role_code) = 'warehouse_manager' THEN
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT NEW.id, permission.id
          FROM auth_permissions permission
         WHERE permission.permission_code = 'h8.erp_connector.read'
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_h8_erp_connector_read ON auth_roles;
CREATE TRIGGER auth_roles_grant_h8_erp_connector_read
AFTER INSERT ON auth_roles
FOR EACH ROW
EXECUTE FUNCTION grant_h8_erp_connector_read_to_warehouse_manager();

-- 菜单入口权限改为 H8 专用读权限
UPDATE admin_menu_draft_nodes
   SET permission_key = 'h8.erp_connector.read',
       updated_at = NOW()
 WHERE view_id = 'h8-erp-connectors';

UPDATE admin_menu_version_nodes
   SET permission_key = 'h8.erp_connector.read',
       updated_at = NOW()
 WHERE view_id = 'h8-erp-connectors';
