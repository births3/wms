-- US-H8-004：接口表探查专用只读账号，和 Worker 传输账号/版本独立。
ALTER TABLE h8_erp_connectors
    ADD COLUMN IF NOT EXISTS interface_probe_db_username TEXT,
    ADD COLUMN IF NOT EXISTS interface_probe_db_password_alias TEXT,
    ADD COLUMN IF NOT EXISTS interface_probe_config_version BIGINT NOT NULL DEFAULT 1;

ALTER TABLE h8_erp_connectors
    DROP CONSTRAINT IF EXISTS ck_h8_erp_probe_config_version_positive;

ALTER TABLE h8_erp_connectors
    ADD CONSTRAINT ck_h8_erp_probe_config_version_positive
    CHECK (interface_probe_config_version >= 1);

COMMENT ON COLUMN h8_erp_connectors.interface_probe_db_username IS
    'H8-004 MSSQL SELECT-only probe account; never reuse Worker account';
COMMENT ON COLUMN h8_erp_connectors.interface_probe_db_password_alias IS
    'ADR-0013 secret alias for H8-004 probe account password';
