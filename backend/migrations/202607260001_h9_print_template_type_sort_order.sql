-- US-H9-001：系统字典项排序与维护权限。

ALTER TABLE system_dictionary_items
    ADD COLUMN sort_order INT NOT NULL DEFAULT 0,
    ADD CONSTRAINT system_dictionary_items_sort_order_check CHECK (sort_order >= 0);

UPDATE system_dictionary_items
   SET sort_order = CASE item_code
       WHEN 'asn' THEN 10
       WHEN 'acceptance_record' THEN 20
       WHEN 'delivery_note' THEN 30
       WHEN 'location_label' THEN 40
       WHEN 'lpn_label' THEN 50
       WHEN 'product_label' THEN 60
       ELSE sort_order
   END
 WHERE dict_code = 'print_template_type'
   AND owner_id IS NULL;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:m1.system_dictionary.read')::uuid, 'm1.system_dictionary.read', 'M1 系统字典读取'),
    (md5('auth_permission:m1.system_dictionary.write')::uuid, 'm1.system_dictionary.write', 'M1 系统字典维护'),
    (md5('auth_permission:m1.system_dictionary.global.write')::uuid, 'm1.system_dictionary.global.write', 'M1 系统字典全局维护'),
    (md5('auth_permission:menu.master_data')::uuid, 'menu.master_data', '基础档案菜单'),
    (md5('auth_permission:menu.master_data.config')::uuid, 'menu.master_data.config', '系统配置菜单')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN (
        'm1.system_dictionary.read',
        'menu.master_data',
        'menu.master_data.config'
    )
 WHERE lower(role.role_code) = 'warehouse_manager'
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_system_dictionary_read_to_warehouse_manager()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF lower(NEW.role_code) = 'warehouse_manager' THEN
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT NEW.id, permission.id
          FROM auth_permissions permission
         WHERE permission.permission_code IN (
             'm1.system_dictionary.read',
             'menu.master_data',
             'menu.master_data.config'
         )
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER auth_roles_grant_system_dictionary_read
AFTER INSERT ON auth_roles
FOR EACH ROW
EXECUTE FUNCTION grant_system_dictionary_read_to_warehouse_manager();
