-- US-DI-003：M4 稳定客户地址快照与 M-DI 客户平台 H2 outbox 订阅。

-- 分三步收紧：先可空加列，再回填存量单据，最后 SET NOT NULL，
-- 保证在已有出库单数据的库上迁移可执行。
ALTER TABLE outbound_orders
    ADD COLUMN delivery_address_id UUID,
    ADD COLUMN delivery_address_snapshot JSONB
        CHECK (jsonb_typeof(delivery_address_snapshot) = 'object');

-- 存量单据优先回填客户默认地址（无默认取最早创建的地址）。
UPDATE outbound_orders AS o
   SET delivery_address_id = a.id,
       delivery_address_snapshot = jsonb_build_object(
           'province', a.province,
           'city', a.city,
           'district', a.district,
           'detail_address', a.detail_address,
           'contact_name', a.contact_name,
           'contact_phone', a.contact_phone,
           'backfilled', TRUE
       )
  FROM (
       SELECT DISTINCT ON (owner_id, customer_id)
              owner_id, customer_id, id,
              province, city, district, detail_address, contact_name, contact_phone
         FROM customer_addresses
        ORDER BY owner_id, customer_id, is_default DESC, created_at ASC
       ) AS a
 WHERE o.delivery_address_id IS NULL
   AND a.owner_id = o.owner_id
   AND a.customer_id = o.customer_id;

-- 客户未登记任何地址的存量单据写入占位快照；
-- 占位地址 id 不指向真实地址，客户平台投影会自动跳过此类单据。
UPDATE outbound_orders
   SET delivery_address_id = md5(id::text || ':mdi-backfill-address')::uuid,
       delivery_address_snapshot = jsonb_build_object(
           'detail_address', '历史单据未登记地址',
           'backfilled', TRUE
       )
 WHERE delivery_address_id IS NULL;

ALTER TABLE outbound_orders
    ALTER COLUMN delivery_address_id SET NOT NULL,
    ALTER COLUMN delivery_address_snapshot SET NOT NULL;

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
