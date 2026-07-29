-- US-DI-004：入库验收按商品类别校验当前有效药检单，并固化规则版本与结果。

CREATE TABLE drug_inspection_requirement_rules (
    id                    UUID PRIMARY KEY,
    owner_id              UUID NOT NULL REFERENCES auth_owners(id),
    special_drug_category TEXT NOT NULL,
    missing_behavior      TEXT NOT NULL CHECK (missing_behavior IN ('warning', 'block')),
    enabled               BOOLEAN NOT NULL DEFAULT TRUE,
    version               BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    updated_by            UUID NOT NULL REFERENCES auth_users(id),
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, special_drug_category)
);

CREATE TABLE drug_inspection_acceptance_validations (
    id                    UUID PRIMARY KEY,
    owner_id              UUID NOT NULL REFERENCES auth_owners(id),
    receiving_order_id    UUID NOT NULL,
    batch_no              TEXT NOT NULL,
    product_id            UUID NOT NULL,
    rule_id               UUID,
    rule_version          BIGINT,
    result                TEXT NOT NULL CHECK (
        result IN ('not_required', 'valid', 'missing_warning', 'missing_blocked', 'unqualified_blocked')
    ),
    report_version_id     UUID REFERENCES drug_inspection_report_versions(id),
    idempotency_key       TEXT NOT NULL,
    detail                JSONB NOT NULL,
    validated_at          TIMESTAMPTZ NOT NULL,
    UNIQUE (owner_id, receiving_order_id, batch_no, idempotency_key),
    FOREIGN KEY (owner_id, receiving_order_id)
        REFERENCES receiving_orders(owner_id, id),
    FOREIGN KEY (owner_id, product_id)
        REFERENCES products(owner_id, id),
    FOREIGN KEY (rule_id)
        REFERENCES drug_inspection_requirement_rules(id)
);

CREATE INDEX drug_inspection_acceptance_validation_query_idx
    ON drug_inspection_acceptance_validations (
        owner_id, receiving_order_id, validated_at DESC
    );

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:m-di.requirement-rule.manage')::uuid,
     'm-di.requirement-rule.manage', '药检单验收规则维护')
ON CONFLICT DO NOTHING;

GRANT SELECT, INSERT, UPDATE ON drug_inspection_requirement_rules TO wms_app;
GRANT SELECT, INSERT ON drug_inspection_acceptance_validations TO wms_app;

CREATE OR REPLACE FUNCTION seed_mdi_acceptance_default_role_permissions(target_owner_id UUID)
RETURNS VOID
LANGUAGE sql
AS $$
    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT role.id, permission.id
      FROM auth_roles role
      JOIN auth_permissions permission
        ON permission.permission_code = 'm-di.requirement-rule.manage'
     WHERE role.owner_id = target_owner_id
       AND role.role_code IN ('system_admin', 'warehouse_manager')
    ON CONFLICT DO NOTHING;
$$;

CREATE OR REPLACE FUNCTION seed_mdi_roles_for_new_owner()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM seed_mdi_default_role_permissions(NEW.id);
    PERFORM seed_mdi_copy_default_role_permissions(NEW.id);
    PERFORM seed_mdi_acceptance_default_role_permissions(NEW.id);
    PERFORM seed_mdi_portal_subscription(NEW.id);
    RETURN NEW;
END;
$$;

SELECT seed_mdi_acceptance_default_role_permissions(id) FROM auth_owners;
