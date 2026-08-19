-- US-H1-002 normalized role scope and inheritance delta.

ALTER TABLE auth_roles
    ADD COLUMN IF NOT EXISTS data_scope TEXT NOT NULL DEFAULT 'owner',
    ADD COLUMN IF NOT EXISTS parent_role_id UUID REFERENCES auth_roles(id) ON DELETE RESTRICT,
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD CONSTRAINT auth_roles_data_scope_check
        CHECK (data_scope IN ('self', 'warehouse', 'owner', 'all'));

CREATE INDEX IF NOT EXISTS auth_roles_parent_idx ON auth_roles(parent_role_id);

CREATE TABLE IF NOT EXISTS auth_role_permission_exclusions (
    role_id UUID NOT NULL REFERENCES auth_roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES auth_permissions(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (role_id, permission_id)
);

GRANT SELECT, INSERT, DELETE ON auth_role_permission_exclusions TO wms_app;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:h1.roles.manage')::uuid,
    'h1.roles.manage',
    'H1 角色权限维护'
)
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION seed_h1_default_roles(target_owner_id UUID)
RETURNS VOID
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO auth_roles (id, owner_id, role_code, role_name, data_scope)
    SELECT
        md5(target_owner_id::text || ':auth_role:' || role_code)::uuid,
        target_owner_id,
        role_code,
        role_name,
        data_scope
      FROM (VALUES
        ('system_admin', '系统管理员', 'all'),
        ('warehouse_manager', '仓库主管', 'warehouse'),
        ('receiving_clerk', '收货员', 'warehouse'),
        ('maintenance_clerk', '养护员', 'warehouse'),
        ('custodian', '保管员', 'warehouse'),
        ('owner_user', '货主', 'owner'),
        ('store_user', '门店用户', 'self'),
        ('driver', '司机', 'self')
      ) AS defaults(role_code, role_name, data_scope)
    ON CONFLICT DO NOTHING;

    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT role.id, permission.id
      FROM auth_roles role
      JOIN auth_permissions permission ON (
        role.role_code = 'system_admin'
        OR (role.role_code = 'warehouse_manager' AND permission.permission_code IN (
            'm1.master_data.read', 'm2.write', 'm3.read', 'm4.read',
            'h4.approval.write', 'h4.notify.read', 'h4.notify.send',
            'h5.express.read', 'h5.express.write',
            'h9.print_template.read', 'h9.print_template.print',
            'menu.inbound', 'menu.inbound.operation',
            'menu.inventory', 'menu.inventory.management',
            'menu.outbound', 'menu.outbound.operation'
        ))
        OR (role.role_code = 'receiving_clerk' AND permission.permission_code IN (
            'm1.master_data.read', 'm2.write',
            'h9.print_template.read', 'h9.print_template.print',
            'menu.inbound', 'menu.inbound.operation'
        ))
        OR (role.role_code = 'maintenance_clerk' AND permission.permission_code IN (
            'm1.master_data.read', 'm3.read',
            'menu.inventory', 'menu.inventory.management'
        ))
        OR (role.role_code = 'custodian' AND permission.permission_code IN (
            'm1.master_data.read', 'm3.read', 'm4.read',
            'h9.print_template.read', 'h9.print_template.print',
            'menu.inventory', 'menu.inventory.management',
            'menu.outbound', 'menu.outbound.operation'
        ))
        OR (role.role_code = 'owner_user' AND permission.permission_code IN (
            'm1.master_data.read', 'm3.read', 'm4.read',
            'h4.notify.read', 'h5.express.read', 'h9.print_template.read',
            'menu.inbound', 'menu.inventory', 'menu.outbound'
        ))
        OR (role.role_code = 'store_user' AND permission.permission_code IN (
            'm4.read', 'h5.express.read', 'menu.outbound'
        ))
        OR (role.role_code = 'driver' AND permission.permission_code IN (
            'm4.read', 'h5.express.read', 'menu.outbound'
        ))
      )
     WHERE role.owner_id = target_owner_id
    ON CONFLICT DO NOTHING;
END;
$$;

CREATE OR REPLACE FUNCTION seed_h1_default_roles_for_new_owner()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM seed_h1_default_roles(NEW.id);
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_owners_seed_h1_default_roles ON auth_owners;
CREATE TRIGGER auth_owners_seed_h1_default_roles
AFTER INSERT ON auth_owners
FOR EACH ROW EXECUTE FUNCTION seed_h1_default_roles_for_new_owner();

SELECT seed_h1_default_roles(id) FROM auth_owners;
