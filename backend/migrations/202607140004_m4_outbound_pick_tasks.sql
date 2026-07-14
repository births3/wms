-- M4 波次规划：把已有库存锁定分配固化为按库位排序的拣选任务。

CREATE TABLE IF NOT EXISTS outbound_pick_tasks (
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL,
    wave_id            UUID NOT NULL,
    outbound_order_id  UUID NOT NULL,
    line_no            INT NOT NULL CHECK (line_no > 0),
    batch_id           UUID NOT NULL,
    product_code       TEXT NOT NULL,
    batch_no           TEXT NOT NULL,
    location_id        UUID NOT NULL,
    location_code      TEXT NOT NULL,
    planned_qty        BIGINT NOT NULL CHECK (planned_qty > 0),
    picked_qty         BIGINT NOT NULL DEFAULT 0 CHECK (picked_qty >= 0),
    status             TEXT NOT NULL DEFAULT 'pending_assignment'
                       CHECK (status IN ('pending_assignment', 'assigned', 'dispatched',
                                         'in_progress', 'completed', 'exception', 'cancelled')),
    route_sequence     INT NOT NULL CHECK (route_sequence > 0),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (picked_qty <= planned_qty),
    UNIQUE (owner_id, wave_id, outbound_order_id, line_no, batch_id),
    FOREIGN KEY (owner_id, wave_id)
        REFERENCES outbound_waves(owner_id, id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id, outbound_order_id)
        REFERENCES outbound_orders(owner_id, id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id, batch_id)
        REFERENCES inventory_batches(owner_id, id)
);

CREATE INDEX IF NOT EXISTS outbound_pick_tasks_owner_wave_route_idx
    ON outbound_pick_tasks (owner_id, wave_id, route_sequence);

GRANT SELECT, INSERT, UPDATE, DELETE ON outbound_pick_tasks TO wms_app;
