-- US-M3-005：库存盘点最小真实闭环。

CREATE TABLE IF NOT EXISTS inventory_counts (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL,
    count_type       TEXT NOT NULL CHECK (count_type IN ('cycle', 'full', 'blind', 'spot')),
    warehouse_id     UUID,
    zone_id          UUID,
    product_code     TEXT,
    status           TEXT NOT NULL DEFAULT 'in_progress'
                     CHECK (status IN ('in_progress', 'pending_approval', 'approved')),
    started_at       TIMESTAMPTZ NOT NULL,
    created_by       UUID NOT NULL,
    approved_by      UUID,
    approved_at      TIMESTAMPTZ,
    approval_source  TEXT,
    approval_id      TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, id)
);

CREATE INDEX IF NOT EXISTS inventory_counts_owner_status_idx
    ON inventory_counts (owner_id, status, started_at DESC);

CREATE TABLE IF NOT EXISTS inventory_count_lines (
    id                  UUID PRIMARY KEY,
    count_id            UUID NOT NULL,
    owner_id            UUID NOT NULL,
    inventory_batch_id  UUID,
    location_id         UUID NOT NULL,
    location_code       TEXT NOT NULL,
    product_code        TEXT NOT NULL,
    batch_no            TEXT NOT NULL,
    book_qty            BIGINT NOT NULL CHECK (book_qty >= 0),
    physical_qty        BIGINT CHECK (physical_qty >= 0),
    variance_qty        BIGINT,
    variance_type       TEXT CHECK (variance_type IS NULL OR variance_type IN ('gain', 'loss', 'none', 'MATCH', 'SURPLUS', 'SHORTAGE', 'match', 'surplus', 'shortage')),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (owner_id, count_id) REFERENCES inventory_counts(owner_id, id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id, inventory_batch_id) REFERENCES inventory_batches(owner_id, id)
);

CREATE INDEX IF NOT EXISTS inventory_count_lines_owner_batch_idx
    ON inventory_count_lines (owner_id, inventory_batch_id, count_id);

GRANT SELECT, INSERT, UPDATE ON inventory_counts TO wms_app;
GRANT SELECT, INSERT, UPDATE ON inventory_count_lines TO wms_app;

CREATE OR REPLACE FUNCTION prevent_inventory_allocation_during_count()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status = 'locked'
       AND EXISTS (
           SELECT 1
             FROM inventory_count_lines line
             JOIN inventory_counts count_sheet
               ON count_sheet.owner_id = line.owner_id
              AND count_sheet.id = line.count_id
            WHERE line.owner_id = NEW.owner_id
              AND line.inventory_batch_id = NEW.batch_id
              AND count_sheet.status IN ('in_progress', 'pending_approval')
       ) THEN
        RAISE EXCEPTION 'inventory batch is locked by an active inventory count';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS inventory_allocations_blocked_by_count ON inventory_allocations;
CREATE TRIGGER inventory_allocations_blocked_by_count
BEFORE INSERT ON inventory_allocations
FOR EACH ROW
EXECUTE FUNCTION prevent_inventory_allocation_during_count();

INSERT INTO auth_permissions (id, permission_code, permission_name)
VALUES
    (md5('auth_permission:m3.inventory_count.write')::uuid, 'm3.inventory_count.write', 'M3 库存盘点执行'),
    (md5('auth_permission:' || 'm3.inventory_count.approve')::uuid, 'm3.inventory_count.approve', 'M3 库存盘点差异审批')
ON CONFLICT DO NOTHING;

INSERT INTO auth_role_permissions (role_id, permission_id)
SELECT role.id, permission.id
  FROM auth_roles role
  JOIN auth_permissions permission
    ON (
        (lower(role.role_code) IN ('system_admin', 'warehouse_manager'))
        OR (lower(role.role_code) = 'custodian' AND permission.permission_code = 'm3.inventory_count.write')
    )
   AND permission.permission_code IN ('m3.inventory_count.write', 'm3.inventory_count.approve')
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION grant_m3_inventory_count_permissions()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT NEW.id, permission.id
      FROM auth_permissions permission
     WHERE permission.permission_code = 'm3.inventory_count.write'
       AND lower(NEW.role_code) IN ('system_admin', 'warehouse_manager', 'custodian')
    ON CONFLICT DO NOTHING;

    INSERT INTO auth_role_permissions (role_id, permission_id)
    SELECT NEW.id, permission.id
      FROM auth_permissions permission
     WHERE permission.permission_code = 'm3.inventory_count.approve'
       AND lower(NEW.role_code) IN ('system_admin', 'warehouse_manager')
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS auth_roles_grant_m3_inventory_count_permissions ON auth_roles;
CREATE TRIGGER auth_roles_grant_m3_inventory_count_permissions
AFTER INSERT ON auth_roles
FOR EACH ROW
EXECUTE FUNCTION grant_m3_inventory_count_permissions();
