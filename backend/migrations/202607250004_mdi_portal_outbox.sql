-- US-DI-003：M4 稳定客户地址快照与 M-DI 客户平台 H2 outbox 订阅。

CREATE INDEX outbound_orders_owner_delivery_address_idx
    ON outbound_orders (owner_id, delivery_address_id, updated_at DESC);

CREATE OR REPLACE FUNCTION seed_mdi_portal_subscription(target_owner_id UUID)
RETURNS VOID
LANGUAGE sql
AS $$
    INSERT INTO event_bus_subscription (
        id, owner_id, subscriber_key, event_pattern, active
    )
    VALUES (
        md5(target_owner_id::text || ':mdi-customer-portal')::uuid,
        target_owner_id,
        'mdi-customer-portal',
        'portal.*',
        TRUE
    )
    ON CONFLICT (owner_id, subscriber_key)
    DO UPDATE SET
        event_pattern = EXCLUDED.event_pattern,
        active = TRUE,
        updated_at = now();
$$;

CREATE OR REPLACE FUNCTION seed_mdi_roles_for_new_owner()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM seed_mdi_default_role_permissions(NEW.id);
    PERFORM seed_mdi_copy_default_role_permissions(NEW.id);
    PERFORM seed_mdi_portal_subscription(NEW.id);
    RETURN NEW;
END;
$$;

SELECT seed_mdi_copy_default_role_permissions(id) FROM auth_owners;
SELECT seed_mdi_portal_subscription(id) FROM auth_owners;

GRANT SELECT, UPDATE (delivery_address_id, delivery_address_snapshot)
    ON outbound_orders TO wms_app;
