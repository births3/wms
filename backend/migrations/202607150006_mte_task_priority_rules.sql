-- US-TE-004：结构化任务优先级权重、冷链/加急标记和主管手动加急。

CREATE TABLE task_priority_rules (
    id                        UUID PRIMARY KEY,
    owner_id                  UUID NOT NULL UNIQUE REFERENCES auth_owners(id) ON DELETE CASCADE,
    urgent_order_bonus        INT NOT NULL DEFAULT 20 CHECK (urgent_order_bonus BETWEEN 0 AND 1000),
    waiting_minutes_per_point INT NOT NULL DEFAULT 30 CHECK (waiting_minutes_per_point BETWEEN 1 AND 1440),
    cold_chain_bonus          INT NOT NULL DEFAULT 20 CHECK (cold_chain_bonus BETWEEN 0 AND 1000),
    manual_expedite_bonus     INT NOT NULL DEFAULT 50 CHECK (manual_expedite_bonus BETWEEN 0 AND 1000),
    created_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                   BIGINT NOT NULL DEFAULT 1 CHECK (version > 0)
);

INSERT INTO task_priority_rules (id, owner_id)
SELECT md5('mte_priority_rule:' || owner.id::text)::uuid, owner.id
  FROM auth_owners owner
ON CONFLICT (owner_id) DO NOTHING;

CREATE OR REPLACE FUNCTION seed_mte_task_priority_rule_for_owner()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO task_priority_rules (id, owner_id)
    VALUES (md5('mte_priority_rule:' || NEW.id::text)::uuid, NEW.id)
    ON CONFLICT (owner_id) DO NOTHING;
    RETURN NEW;
END;
$$;

CREATE TRIGGER auth_owners_seed_mte_task_priority_rule
AFTER INSERT ON auth_owners
FOR EACH ROW EXECUTE FUNCTION seed_mte_task_priority_rule_for_owner();

ALTER TABLE warehouse_tasks
    ADD COLUMN urgent_order BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN cold_chain BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN manually_expedited BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE warehouse_tasks task
   SET cold_chain = TRUE
  FROM products product
 WHERE product.owner_id = task.owner_id
   AND product.storage_condition IN ('frozen', 'cold', 'cool')
   AND (product.id = task.product_id OR (task.product_id IS NULL AND product.product_code = task.product_code));

GRANT SELECT, INSERT, UPDATE ON task_priority_rules TO wms_app;

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES (
    md5('auth_permission:mte.priority_rule.write')::uuid,
    'mte.priority_rule.write',
    'M-TE 任务优先级规则配置'
)
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
 CROSS JOIN auth_permissions permission
 WHERE lower(role.role_code) IN ('system_admin', 'warehouse_manager')
   AND permission.permission_code = 'mte.priority_rule.write'
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION seed_mte_priority_rule_permission_for_role()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF lower(NEW.role_code) IN ('system_admin', 'warehouse_manager') THEN
        INSERT INTO auth_role_permissions (role_id, permission_id)
        SELECT NEW.id, permission.id
          FROM auth_permissions permission
         WHERE permission.permission_code = 'mte.priority_rule.write'
        ON CONFLICT DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER auth_roles_seed_mte_priority_rule_permission
AFTER INSERT ON auth_roles
FOR EACH ROW EXECUTE FUNCTION seed_mte_priority_rule_permission_for_role();
