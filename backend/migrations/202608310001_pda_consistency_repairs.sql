-- PDA consistency repairs: canonical operation timestamps, trace persistence and tote bindings.

ALTER TABLE inventory_counts
    ADD COLUMN IF NOT EXISTS reason TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS outbound_pick_tasks_owner_id_uidx
    ON outbound_pick_tasks (owner_id, id);

CREATE UNIQUE INDEX IF NOT EXISTS lpn_containers_owner_id_uidx
    ON lpn_containers (owner_id, id);

CREATE TABLE IF NOT EXISTS outbound_pick_trace_codes (
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL,
    pick_task_id       UUID NOT NULL,
    outbound_order_id  UUID NOT NULL,
    line_no            INT NOT NULL CHECK (line_no > 0),
    trace_code         TEXT NOT NULL CHECK (btrim(trace_code) <> ''),
    scanned_by         UUID NOT NULL,
    operated_at        TIMESTAMPTZ NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, trace_code),
    FOREIGN KEY (owner_id, pick_task_id)
        REFERENCES outbound_pick_tasks(owner_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (owner_id, outbound_order_id)
        REFERENCES outbound_orders(owner_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS outbound_pick_trace_codes_task_idx
    ON outbound_pick_trace_codes (owner_id, pick_task_id, operated_at);

CREATE TABLE IF NOT EXISTS outbound_pick_tote_bindings (
    id                 UUID PRIMARY KEY,
    owner_id           UUID NOT NULL,
    outbound_order_id  UUID NOT NULL,
    tote_id            UUID NOT NULL,
    tote_code          TEXT NOT NULL CHECK (btrim(tote_code) <> ''),
    status             TEXT NOT NULL CHECK (status IN ('active', 'released')),
    bound_by           UUID NOT NULL,
    bound_at           TIMESTAMPTZ NOT NULL,
    released_at        TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (
        (status = 'active' AND released_at IS NULL)
        OR (status = 'released' AND released_at IS NOT NULL)
    ),
    FOREIGN KEY (owner_id, outbound_order_id)
        REFERENCES outbound_orders(owner_id, id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id, tote_id)
        REFERENCES lpn_containers(owner_id, id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS outbound_pick_tote_bindings_active_tote_uidx
    ON outbound_pick_tote_bindings (owner_id, tote_id)
    WHERE status = 'active';

CREATE UNIQUE INDEX IF NOT EXISTS outbound_pick_tote_bindings_active_order_uidx
    ON outbound_pick_tote_bindings (owner_id, outbound_order_id)
    WHERE status = 'active';

CREATE INDEX IF NOT EXISTS outbound_pick_tote_bindings_order_history_idx
    ON outbound_pick_tote_bindings (owner_id, outbound_order_id, bound_at DESC);

GRANT SELECT, INSERT ON outbound_pick_trace_codes TO wms_app;
GRANT SELECT, INSERT, UPDATE ON outbound_pick_tote_bindings TO wms_app;
