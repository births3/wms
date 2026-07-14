-- US-DOCK-001 月台档案；月台按物理仓库归属，不重复存货主字段。

CREATE TABLE IF NOT EXISTS warehouse_docks (
    id                      UUID PRIMARY KEY,
    warehouse_id            UUID NOT NULL REFERENCES warehouses(id) ON DELETE RESTRICT,
    dock_code               VARCHAR(32) NOT NULL,
    dock_type               TEXT NOT NULL,
    temperature_zone        TEXT NOT NULL,
    status                  TEXT NOT NULL DEFAULT 'active',
    maintenance_recovery_at TIMESTAMPTZ,
    location_description    TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (btrim(dock_code) <> ''),
    CHECK (dock_type IN ('receiving', 'shipping', 'both')),
    CHECK (temperature_zone IN ('normal', 'cold', 'frozen', 'cold_chain')),
    CHECK (status IN ('active', 'disabled', 'maintenance')),
    CHECK (
        (status = 'maintenance' AND maintenance_recovery_at IS NOT NULL)
        OR (status IN ('active', 'disabled') AND maintenance_recovery_at IS NULL)
    ),
    UNIQUE (warehouse_id, dock_code)
);

GRANT SELECT, INSERT, UPDATE ON warehouse_docks TO wms_app;
