-- US-VR-006：流程 × 节点双人策略矩阵及 M1 特殊药品字典默认规则同步。

CREATE TABLE IF NOT EXISTS dual_person_policy_rules (
    id                         UUID PRIMARY KEY,
    special_drug_category     TEXT NOT NULL,
    process_code               TEXT NOT NULL,
    node_code                  TEXT NOT NULL,
    owner_id                   UUID REFERENCES auth_owners(id) ON DELETE CASCADE,
    warehouse_id               UUID,
    policy                     TEXT NOT NULL CHECK (policy IN ('single', 'dual_scan', 'dual_scan_with_approval')),
    priority                   INT NOT NULL DEFAULT 100 CHECK (priority BETWEEN 0 AND 1000),
    enabled                    BOOLEAN NOT NULL DEFAULT TRUE,
    source_dictionary_item_id  UUID REFERENCES system_dictionary_items(id) ON DELETE CASCADE,
    confirmed_by_user_id       UUID REFERENCES auth_users(id) ON DELETE RESTRICT,
    created_at                 TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                 TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                    BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    CHECK (warehouse_id IS NULL OR owner_id IS NOT NULL),
    FOREIGN KEY (owner_id, warehouse_id)
        REFERENCES warehouses(owner_id, id) ON DELETE CASCADE,
    CHECK (
        (process_code = '入库' AND node_code IN ('收货', '验收', '上架')) OR
        (process_code = '出库' AND node_code IN ('拣货', '复核', '装箱', '发货交接')) OR
        (process_code = '报损' AND node_code = '报损执行') OR
        (process_code = '报溢' AND node_code = '报溢执行') OR
        (process_code = '销毁' AND node_code = '销毁执行') OR
        (process_code = '退货' AND node_code IN ('退货验收', '退货上架'))
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS dual_person_policy_rules_scope_idx
    ON dual_person_policy_rules (
        special_drug_category,
        process_code,
        node_code,
        COALESCE(owner_id, '00000000-0000-0000-0000-000000000000'::uuid),
        COALESCE(warehouse_id, '00000000-0000-0000-0000-000000000000'::uuid)
    );

CREATE INDEX IF NOT EXISTS dual_person_policy_rules_resolve_idx
    ON dual_person_policy_rules (
        special_drug_category,
        process_code,
        node_code,
        owner_id,
        warehouse_id,
        enabled,
        priority DESC
    );

GRANT SELECT, INSERT, UPDATE ON dual_person_policy_rules TO wms_app;

CREATE OR REPLACE FUNCTION sync_mvr_dual_person_rules_from_dictionary()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.dict_code <> 'special_drug_category' THEN
        RETURN NEW;
    END IF;

    UPDATE dual_person_policy_rules
       SET enabled = FALSE,
           updated_at = now(),
           version = version + 1
     WHERE source_dictionary_item_id = NEW.id
       AND warehouse_id IS NULL
       AND enabled;

    IF NEW.enabled AND jsonb_typeof(NEW.params -> 'requires_dual_person_matrix') = 'array' THEN
        INSERT INTO dual_person_policy_rules (
            id, special_drug_category, process_code, node_code, owner_id,
            warehouse_id, policy, priority, enabled, source_dictionary_item_id,
            created_at, updated_at
        )
        SELECT md5(format('mvr-dual:%s:%s:%s', NEW.id, cell ->> 'process', cell ->> 'node'))::uuid,
               NEW.item_code,
               cell ->> 'process',
               cell ->> 'node',
               NEW.owner_id,
               NULL,
               cell ->> 'policy',
               100,
               TRUE,
               NEW.id,
               now(),
               now()
          FROM jsonb_array_elements(NEW.params -> 'requires_dual_person_matrix') cell
        ON CONFLICT (id) DO UPDATE
           SET policy = EXCLUDED.policy,
               priority = EXCLUDED.priority,
               enabled = TRUE,
               source_dictionary_item_id = EXCLUDED.source_dictionary_item_id,
               updated_at = now(),
               version = dual_person_policy_rules.version + 1;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS system_dictionary_sync_mvr_dual_person_rules ON system_dictionary_items;
CREATE TRIGGER system_dictionary_sync_mvr_dual_person_rules
AFTER INSERT OR UPDATE OF params, enabled ON system_dictionary_items
FOR EACH ROW EXECUTE FUNCTION sync_mvr_dual_person_rules_from_dictionary();

INSERT INTO dual_person_policy_rules (
    id, special_drug_category, process_code, node_code, owner_id,
    warehouse_id, policy, priority, enabled, source_dictionary_item_id
)
SELECT md5(format('mvr-dual:%s:%s:%s', item.id, cell ->> 'process', cell ->> 'node'))::uuid,
       item.item_code,
       cell ->> 'process',
       cell ->> 'node',
       item.owner_id,
       NULL,
       cell ->> 'policy',
       100,
       TRUE,
       item.id
  FROM system_dictionary_items item
 CROSS JOIN LATERAL jsonb_array_elements(item.params -> 'requires_dual_person_matrix') cell
 WHERE item.dict_code = 'special_drug_category'
   AND item.enabled
ON CONFLICT DO NOTHING;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:mvr.dual_person.read')::uuid, 'mvr.dual_person.read', 'M-VR 双人策略查询'),
    (md5('auth_permission:mvr.dual_person.write')::uuid, 'mvr.dual_person.write', 'M-VR 双人策略配置'),
    (md5('auth_permission:mvr.dual_person.global.write')::uuid, 'mvr.dual_person.global.write', 'M-VR 全局双人策略配置')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code IN ('mvr.dual_person.read', 'mvr.dual_person.write')
 WHERE lower(role.role_code) IN ('system_admin', 'warehouse_manager')
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION seed_mvr_dual_person_permissions()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT NEW.id, permission.id
      FROM auth_permissions permission
     WHERE permission.permission_code IN (
               'mvr.dual_person.read',
               'mvr.dual_person.write',
               'mvr.dual_person.global.write'
           )
       AND (
           lower(NEW.role_code) = 'system_admin'
           OR (
               lower(NEW.role_code) = 'warehouse_manager'
               AND permission.permission_code <> 'mvr.dual_person.global.write'
           )
       )
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_mvr_dual_person_permissions ON auth_roles;
CREATE TRIGGER auth_roles_grant_mvr_dual_person_permissions
AFTER INSERT ON auth_roles
FOR EACH ROW EXECUTE FUNCTION seed_mvr_dual_person_permissions();

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON permission.permission_code = 'mvr.dual_person.global.write'
 WHERE lower(role.role_code) = 'system_admin'
ON CONFLICT DO NOTHING;
