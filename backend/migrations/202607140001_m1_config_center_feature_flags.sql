CREATE TABLE IF NOT EXISTS config_center_feature_flags (
    owner_id    UUID NOT NULL,
    flag_key    TEXT NOT NULL,
    owner       TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    cleanup_by  TEXT NOT NULL,
    enabled     BOOLEAN NOT NULL,
    source      TEXT NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, flag_key)
);

CREATE INDEX IF NOT EXISTS config_center_feature_flags_owner_idx
    ON config_center_feature_flags (owner_id, flag_key);

GRANT SELECT, INSERT, UPDATE ON config_center_feature_flags TO wms_app;
