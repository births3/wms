-- US-H8-002 AC6：保存消息处理所绑定的不可变连接运行配置。

CREATE TABLE h8_erp_connector_versions (
    owner_id       UUID NOT NULL,
    connector_id   UUID NOT NULL,
    config_version BIGINT NOT NULL CHECK (config_version >= 1),
    runtime_config JSONB NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, connector_id, config_version),
    FOREIGN KEY (owner_id, connector_id)
        REFERENCES h8_erp_connectors(owner_id, id) ON DELETE CASCADE
);

CREATE FUNCTION capture_h8_erp_connector_version() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    snapshot JSONB;
BEGIN
    snapshot := jsonb_build_object(
        'id', NEW.id,
        'owner_id', NEW.owner_id,
        'connector_code', NEW.connector_code,
        'warehouse_ids', to_jsonb(NEW.warehouse_ids),
        'directions', to_jsonb(NEW.directions),
        'message_types', to_jsonb(NEW.message_types),
        'channel_mode', NEW.channel_mode,
        'api_base_url', NEW.api_base_url,
        'interface_db_host', NEW.interface_db_host,
        'interface_db_port', NEW.interface_db_port,
        'interface_db_name', NEW.interface_db_name,
        'interface_db_username', NEW.interface_db_username,
        'api_key_id', NEW.api_key_id,
        'bearer_secret_alias', NEW.bearer_secret_alias,
        'interface_db_password_alias', NEW.interface_db_password_alias,
        'config_version', NEW.config_version
    );

    IF TG_OP = 'UPDATE' AND NEW.config_version = OLD.config_version THEN
        IF snapshot IS DISTINCT FROM (
            SELECT runtime_config
              FROM h8_erp_connector_versions
             WHERE owner_id = OLD.owner_id
               AND connector_id = OLD.id
               AND config_version = OLD.config_version
        ) THEN
            RAISE EXCEPTION 'H8 runtime config changed without config_version increment';
        END IF;
        RETURN NEW;
    END IF;

    INSERT INTO h8_erp_connector_versions (
        owner_id, connector_id, config_version, runtime_config
    ) VALUES (
        NEW.owner_id, NEW.id, NEW.config_version, snapshot
    );
    RETURN NEW;
END;
$$;

INSERT INTO h8_erp_connector_versions (
    owner_id, connector_id, config_version, runtime_config
)
SELECT owner_id, id, config_version, jsonb_build_object(
    'id', id,
    'owner_id', owner_id,
    'connector_code', connector_code,
    'warehouse_ids', to_jsonb(warehouse_ids),
    'directions', to_jsonb(directions),
    'message_types', to_jsonb(message_types),
    'channel_mode', channel_mode,
    'api_base_url', api_base_url,
    'interface_db_host', interface_db_host,
    'interface_db_port', interface_db_port,
    'interface_db_name', interface_db_name,
    'interface_db_username', interface_db_username,
    'api_key_id', api_key_id,
    'bearer_secret_alias', bearer_secret_alias,
    'interface_db_password_alias', interface_db_password_alias,
    'config_version', config_version
)
FROM h8_erp_connectors;

CREATE TRIGGER h8_erp_connector_version_capture
AFTER INSERT OR UPDATE ON h8_erp_connectors
FOR EACH ROW EXECUTE FUNCTION capture_h8_erp_connector_version();

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        GRANT SELECT, INSERT ON h8_erp_connector_versions TO wms_app;
    END IF;
END $$;
