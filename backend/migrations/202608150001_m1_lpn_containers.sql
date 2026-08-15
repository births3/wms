-- US-M1-004a 最小 LPN 容器主档；货主内码唯一。

CREATE TABLE IF NOT EXISTS lpn_containers (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL,
    lpn_code        VARCHAR(64) NOT NULL,
    container_type  TEXT NOT NULL,
    capacity_cm3    BIGINT,
    status          TEXT NOT NULL DEFAULT 'idle',
    location_id     UUID,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT lpn_containers_lpn_code_not_blank CHECK (btrim(lpn_code) <> ''),
    CONSTRAINT lpn_containers_container_type_valid CHECK (container_type IN (
        'pallet',
        'tote',
        'outbound_box',
        'insulated_box',
        'blind_label'
    )),
    CONSTRAINT lpn_containers_status_valid CHECK (status IN (
        'idle',
        'in_use',
        'in_transit',
        'recycling',
        'shipped'
    )),
    UNIQUE (owner_id, lpn_code)
);

GRANT SELECT, INSERT, UPDATE ON lpn_containers TO wms_app;
