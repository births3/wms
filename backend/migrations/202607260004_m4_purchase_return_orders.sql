-- M4 采购退货出库：purchase_return_orders 真实后端域。

CREATE TABLE IF NOT EXISTS purchase_return_orders (
    id                        UUID PRIMARY KEY,
    owner_id                  UUID NOT NULL,
    warehouse_id              UUID NOT NULL,
    return_no                 TEXT NOT NULL,
    document_type             TEXT NOT NULL DEFAULT 'purchase_return_outbound',
    source_purchase_order_no  TEXT NOT NULL,
    supplier_id               UUID,
    supplier_name             TEXT NOT NULL,
    reason                    TEXT NOT NULL,
    approval_source           TEXT NOT NULL DEFAULT 'purchase_return_approval',
    status                    TEXT NOT NULL,
    product_code              TEXT NOT NULL,
    qty                       BIGINT NOT NULL CHECK (qty > 0),
    reject_reason             TEXT,
    shipped_at                TIMESTAMPTZ,
    shipped_by                UUID,
    shipped_by_name           TEXT,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    version                   BIGINT NOT NULL DEFAULT 1,
    UNIQUE (owner_id, return_no)
);

CREATE INDEX IF NOT EXISTS purchase_return_orders_owner_status_idx
    ON purchase_return_orders (owner_id, status, updated_at DESC);

-- 对齐数据库设计规范：为多租户复合外键预留 (owner_id, id) 唯一索引。
CREATE UNIQUE INDEX IF NOT EXISTS purchase_return_orders_owner_id_uidx
    ON purchase_return_orders (owner_id, id);
