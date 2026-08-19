-- M10-001 TMS 路径规划结果接收：仅保存外部规划结果，不在 WMS 重新计算路径。

CREATE TABLE IF NOT EXISTS tms_route_plans (
    id                   UUID PRIMARY KEY,
    owner_id             UUID NOT NULL,
    dispatch_result_id   TEXT NOT NULL,
    delivery_date        DATE NOT NULL,
    vehicle_no           TEXT NOT NULL,
    plate_no             TEXT NOT NULL,
    driver_user_id       UUID NOT NULL,
    status               TEXT NOT NULL DEFAULT 'received',
    planning_version     INT NOT NULL CHECK (planning_version > 0),
    payload_hash         TEXT NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, dispatch_result_id),
    UNIQUE (owner_id, id),
    CHECK (length(btrim(dispatch_result_id)) BETWEEN 1 AND 128),
    CHECK (length(btrim(vehicle_no)) > 0),
    CHECK (length(btrim(plate_no)) > 0),
    CHECK (status IN ('received', 'applied', 'superseded'))
);

CREATE INDEX IF NOT EXISTS tms_route_plans_owner_delivery_date_idx
    ON tms_route_plans (owner_id, delivery_date, created_at DESC);

CREATE TABLE IF NOT EXISTS tms_route_stops (
    id                    UUID PRIMARY KEY,
    owner_id              UUID NOT NULL,
    route_plan_id         UUID NOT NULL,
    store_id              UUID NOT NULL,
    stop_sequence         INT NOT NULL CHECK (stop_sequence > 0),
    estimated_arrival_at  TIMESTAMPTZ NOT NULL,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, id),
    UNIQUE (owner_id, route_plan_id, id),
    UNIQUE (owner_id, route_plan_id, stop_sequence),
    FOREIGN KEY (owner_id, route_plan_id)
        REFERENCES tms_route_plans(owner_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tms_route_orders (
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL,
    route_plan_id      UUID NOT NULL,
    route_stop_id      UUID NOT NULL,
    outbound_order_id  UUID NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, id),
    UNIQUE (owner_id, route_plan_id, outbound_order_id),
    FOREIGN KEY (owner_id, route_plan_id)
        REFERENCES tms_route_plans(owner_id, id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id, route_plan_id, route_stop_id)
        REFERENCES tms_route_stops(owner_id, route_plan_id, id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id, outbound_order_id)
        REFERENCES outbound_orders(owner_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS tms_route_orders_owner_order_idx
    ON tms_route_orders (owner_id, outbound_order_id);

GRANT SELECT, INSERT, UPDATE, DELETE ON tms_route_plans TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON tms_route_stops TO wms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON tms_route_orders TO wms_app;
