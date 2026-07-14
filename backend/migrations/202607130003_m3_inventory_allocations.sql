CREATE TABLE IF NOT EXISTS inventory_allocations (
    id                UUID PRIMARY KEY,
    owner_id          UUID NOT NULL,
    outbound_order_id UUID NOT NULL,
    line_no           INT NOT NULL CHECK (line_no > 0),
    batch_id          UUID NOT NULL,
    allocated_qty     BIGINT NOT NULL CHECK (allocated_qty > 0),
    status            TEXT NOT NULL CHECK (status IN ('locked', 'consumed')),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    consumed_at       TIMESTAMPTZ,
    CONSTRAINT inventory_allocations_owner_order_fk
        FOREIGN KEY (owner_id, outbound_order_id)
        REFERENCES outbound_orders(owner_id, id) ON DELETE CASCADE,
    CONSTRAINT inventory_allocations_owner_batch_fk
        FOREIGN KEY (owner_id, batch_id)
        REFERENCES inventory_batches(owner_id, id),
    CONSTRAINT inventory_allocations_order_line_batch_uq
        UNIQUE (owner_id, outbound_order_id, line_no, batch_id)
);

CREATE INDEX IF NOT EXISTS inventory_allocations_owner_order_status_idx
    ON inventory_allocations (owner_id, outbound_order_id, line_no, status);

GRANT SELECT, INSERT, UPDATE, DELETE ON inventory_allocations TO wms_app;
