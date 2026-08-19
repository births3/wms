-- 增量对齐：ERP 业务身份列 / 数量与金额 Decimal 化 / 出站回执 outbox 扩展。
-- 对应此前被就地改写的 15 个既有迁移（均已按 ADR-0045 还原为原状），
-- 本迁移以增量 ALTER 形式落地，在已应用库与全新库上均可重复执行。

-- ===== 1. ERP 业务身份列与部分唯一索引 =====

ALTER TABLE receiving_orders
    ADD COLUMN IF NOT EXISTS erp_bill_id          BIGINT,
    ADD COLUMN IF NOT EXISTS erp_bill_code        TEXT,
    ADD COLUMN IF NOT EXISTS erp_revision         INT,
    ADD COLUMN IF NOT EXISTS erp_line_no          INT,
    ADD COLUMN IF NOT EXISTS erp_correlation_id   TEXT,
    ADD COLUMN IF NOT EXISTS partner_type         TEXT,
    ADD COLUMN IF NOT EXISTS partner_id           UUID,
    ADD COLUMN IF NOT EXISTS partner_code         TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS receiving_orders_owner_erp_version_line_uidx
    ON receiving_orders (owner_id, erp_bill_code, erp_revision, erp_line_no)
    WHERE erp_bill_code IS NOT NULL;

ALTER TABLE outbound_orders
    ADD COLUMN IF NOT EXISTS erp_bill_id          BIGINT,
    ADD COLUMN IF NOT EXISTS erp_bill_code        TEXT,
    ADD COLUMN IF NOT EXISTS erp_revision         INT,
    ADD COLUMN IF NOT EXISTS erp_order_type       INT,
    ADD COLUMN IF NOT EXISTS send_mode            INT,
    ADD COLUMN IF NOT EXISTS erp_correlation_id   TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS outbound_orders_owner_erp_version_uidx
    ON outbound_orders (owner_id, erp_bill_code, erp_revision)
    WHERE erp_bill_code IS NOT NULL;

ALTER TABLE products
    ADD COLUMN IF NOT EXISTS erp_goods_id        BIGINT,
    ADD COLUMN IF NOT EXISTS erp_source_version  BIGINT;

CREATE UNIQUE INDEX IF NOT EXISTS products_owner_erp_goods_uidx
    ON products (owner_id, erp_goods_id)
    WHERE erp_goods_id IS NOT NULL;

ALTER TABLE suppliers
    ADD COLUMN IF NOT EXISTS erp_supplier_id    BIGINT,
    ADD COLUMN IF NOT EXISTS erp_source_version BIGINT,
    ADD COLUMN IF NOT EXISTS erp_payload        JSONB NOT NULL DEFAULT '{}'::JSONB;

CREATE UNIQUE INDEX IF NOT EXISTS suppliers_owner_erp_supplier_uidx
    ON suppliers (owner_id, erp_supplier_id)
    WHERE erp_supplier_id IS NOT NULL;

ALTER TABLE customers
    ADD COLUMN IF NOT EXISTS erp_client_id      BIGINT,
    ADD COLUMN IF NOT EXISTS erp_source_version BIGINT,
    ADD COLUMN IF NOT EXISTS erp_payload        JSONB NOT NULL DEFAULT '{}'::JSONB;

CREATE UNIQUE INDEX IF NOT EXISTS customers_owner_erp_client_uidx
    ON customers (owner_id, erp_client_id)
    WHERE erp_client_id IS NOT NULL;

ALTER TABLE customer_addresses
    ADD COLUMN IF NOT EXISTS erp_address_id  BIGINT,
    ADD COLUMN IF NOT EXISTS address_code    TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS customer_addresses_owner_address_code_uidx
    ON customer_addresses (owner_id, address_code)
    WHERE address_code IS NOT NULL;

-- ===== 2. 出站回执 outbox 扩展 =====

ALTER TABLE receiving_putaway_erp_feedback_outbox
    ALTER COLUMN putaway_id DROP NOT NULL,
    ADD COLUMN IF NOT EXISTS command_id TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS receiving_putaway_erp_outbox_owner_command_uidx
    ON receiving_putaway_erp_feedback_outbox (owner_id, command_id)
    WHERE command_id IS NOT NULL;

ALTER TABLE shipment_confirm_erp_feedback_outbox
    ADD COLUMN IF NOT EXISTS command_id TEXT;

ALTER TABLE shipment_confirm_erp_feedback_outbox
    DROP CONSTRAINT IF EXISTS shipment_confirm_erp_feedback_outbox_event_type_check,
    ADD CONSTRAINT shipment_confirm_erp_feedback_outbox_event_type_check
        CHECK (event_type IN ('shipment_confirm', 'order_status'));

CREATE UNIQUE INDEX IF NOT EXISTS shipment_confirm_erp_outbox_owner_command_uidx
    ON shipment_confirm_erp_feedback_outbox (owner_id, command_id)
    WHERE command_id IS NOT NULL;

-- ===== 3. 数量 / 金额列 BIGINT → NUMERIC =====

ALTER TABLE receiving_order_lines
    ALTER COLUMN expected_qty TYPE NUMERIC(19,4);

ALTER TABLE receiving_order_receipts
    ALTER COLUMN actual_qty   TYPE NUMERIC(19,4),
    ALTER COLUMN shortage_qty TYPE NUMERIC(19,4),
    ALTER COLUMN rejected_qty TYPE NUMERIC(19,4);

ALTER TABLE receiving_inspections
    ALTER COLUMN accepted_qty TYPE NUMERIC(19,4),
    ALTER COLUMN rejected_qty TYPE NUMERIC(19,4),
    ALTER COLUMN sampling_qty  TYPE NUMERIC(19,4);

ALTER TABLE receiving_putaways
    ALTER COLUMN qty TYPE NUMERIC(19,4);

ALTER TABLE inventory_batches
    ALTER COLUMN qty_on_hand TYPE NUMERIC(19,4),
    ALTER COLUMN qty_locked  TYPE NUMERIC(19,4);

ALTER TABLE inventory_movements
    ALTER COLUMN qty_delta TYPE NUMERIC(19,4);

ALTER TABLE billing_rules
    ALTER COLUMN unit_price_cents TYPE NUMERIC(19,8);

ALTER TABLE outbound_order_lines
    ALTER COLUMN planned_qty    TYPE NUMERIC(19,4),
    ALTER COLUMN picked_qty     TYPE NUMERIC(19,4),
    ALTER COLUMN reviewed_qty   TYPE NUMERIC(19,4),
    ALTER COLUMN shipped_qty    TYPE NUMERIC(19,4),
    ALTER COLUMN short_pick_qty TYPE NUMERIC(19,4);

ALTER TABLE retail_replenishment_suggestions
    ALTER COLUMN min_qty         TYPE NUMERIC(19,4),
    ALTER COLUMN max_qty         TYPE NUMERIC(19,4),
    ALTER COLUMN current_qty     TYPE NUMERIC(19,4),
    ALTER COLUMN in_transit_qty  TYPE NUMERIC(19,4),
    ALTER COLUMN daily_sales_avg TYPE NUMERIC(19,4),
    ALTER COLUMN suggested_qty   TYPE NUMERIC(19,4);

ALTER TABLE crossdock_plans
    ALTER COLUMN qty TYPE NUMERIC(19,4);

ALTER TABLE billing_charge_calculations
    ALTER COLUMN quantity      TYPE NUMERIC(19,4),
    ALTER COLUMN amount_cents  TYPE NUMERIC(19,4);

ALTER TABLE billing_statements
    ALTER COLUMN total_amount_cents TYPE NUMERIC(19,4);

ALTER TABLE inventory_allocations
    ALTER COLUMN allocated_qty TYPE NUMERIC(19,4);

ALTER TABLE outbound_pick_tasks
    ALTER COLUMN planned_qty TYPE NUMERIC(19,4),
    ALTER COLUMN picked_qty  TYPE NUMERIC(19,4);

ALTER TABLE inventory_count_lines
    ALTER COLUMN book_qty     TYPE NUMERIC(19,4),
    ALTER COLUMN physical_qty TYPE NUMERIC(19,4),
    ALTER COLUMN variance_qty TYPE NUMERIC(19,4);

ALTER TABLE warehouse_tasks
    ALTER COLUMN planned_qty TYPE NUMERIC(19,4),
    ALTER COLUMN actual_qty  TYPE NUMERIC(19,4);

ALTER TABLE task_execution_events
    ALTER COLUMN actual_qty TYPE NUMERIC(19,4);

ALTER TABLE stock_adjustment_orders
    ALTER COLUMN quantity TYPE NUMERIC(19,4);

ALTER TABLE stock_adjustment_execution_records
    ALTER COLUMN quantity TYPE NUMERIC(19,4);

ALTER TABLE inventory_relocations
    ALTER COLUMN qty TYPE NUMERIC(19,4);

ALTER TABLE inventory_abc_classifications
    ALTER COLUMN outbound_qty TYPE NUMERIC(19,4);

ALTER TABLE reconciliation_items
    ALTER COLUMN wms_qty        TYPE NUMERIC(19,4),
    ALTER COLUMN erp_qty        TYPE NUMERIC(19,4),
    ALTER COLUMN difference_qty TYPE NUMERIC(19,4);

ALTER TABLE reconciliation_item_adjustments
    ALTER COLUMN quantity TYPE NUMERIC(19,4);

ALTER TABLE purchase_return_orders
    ALTER COLUMN qty TYPE NUMERIC(19,4);

-- ===== 4. 新表：库存快照暂存 / ERP 取消命令 =====

CREATE TABLE IF NOT EXISTS erp_inventory_snapshot_staging (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL,
    warehouse_id    UUID NOT NULL,
    snapshot_id     TEXT NOT NULL,
    push_type       INT NOT NULL CHECK (push_type IN (1, 2)),
    push_time       TIMESTAMPTZ NOT NULL,
    payload_digest  TEXT NOT NULL CHECK (length(payload_digest) = 64),
    status          TEXT NOT NULL CHECK (status IN ('pending_approval', 'reconciliation_only')),
    summary         JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, snapshot_id),
    FOREIGN KEY (owner_id, warehouse_id) REFERENCES warehouses(owner_id, id)
);

CREATE TABLE IF NOT EXISTS erp_inventory_snapshot_staging_items (
    id                   UUID PRIMARY KEY,
    snapshot_staging_id  UUID NOT NULL REFERENCES erp_inventory_snapshot_staging(id) ON DELETE CASCADE,
    owner_id             UUID NOT NULL,
    row_no               INT NOT NULL CHECK (row_no > 0),
    product_code         TEXT NOT NULL,
    batch_no             TEXT NOT NULL,
    expiry_date          DATE,
    location_code        TEXT,
    goods_status         TEXT,
    quantity             NUMERIC(19,4) NOT NULL CHECK (quantity >= 0),
    quarantined          BOOLEAN NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (snapshot_staging_id, row_no)
);

CREATE INDEX IF NOT EXISTS erp_inventory_snapshot_staging_owner_status_idx
    ON erp_inventory_snapshot_staging (owner_id, status, created_at DESC);

CREATE TABLE IF NOT EXISTS erp_order_cancel_commands (
    owner_id        UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    command_id      TEXT NOT NULL,
    erp_bill_code   TEXT NOT NULL,
    revision        INT NOT NULL CHECK (revision > 0),
    order_type      INT NOT NULL CHECK (order_type IN (1, 2)),
    correlation_id  TEXT NOT NULL,
    memo            TEXT,
    status          TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'completed', 'rejected')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at     TIMESTAMPTZ,
    PRIMARY KEY (owner_id, command_id)
);

CREATE INDEX IF NOT EXISTS erp_order_cancel_pending_idx
    ON erp_order_cancel_commands
       (owner_id, order_type, erp_bill_code, revision)
    WHERE status = 'pending';

-- ===== 5. 授权 =====

GRANT SELECT, INSERT, UPDATE, DELETE ON erp_order_cancel_commands TO wms_app;
