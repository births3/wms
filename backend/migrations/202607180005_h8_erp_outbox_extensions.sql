-- H8 补全：档案补录 / 对账差异 / 发货确认 / 库存快照 ERP outbox
-- 由 scripts/h8_erp_interface_sync 出站投递到 if_out_message 或通道 A HTTP 回调。

-- 档案补录：专用重试语义（5 次 / 5 分钟 / 24h 截止）由 worker 读取 deadline_at、max_attempts
CREATE TABLE IF NOT EXISTS archive_revision_erp_feedback_outbox (
    id                UUID PRIMARY KEY,
    owner_id          UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    liaison_id        UUID,
    asn_id            UUID,
    receipt_record_id UUID,
    product_code      TEXT NOT NULL,
    field_name        TEXT NOT NULL,
    event_type        TEXT NOT NULL DEFAULT 'archive_revision'
        CHECK (event_type = 'archive_revision'),
    payload           JSONB NOT NULL,
    status            TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'failed', 'succeeded', 'dead')),
    attempt_count     INT NOT NULL DEFAULT 0
        CHECK (attempt_count BETWEEN 0 AND 5),
    max_attempts      INT NOT NULL DEFAULT 5 CHECK (max_attempts = 5),
    next_attempt_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    deadline_at       TIMESTAMPTZ NOT NULL DEFAULT (now() + interval '24 hours'),
    last_error        TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (deadline_at = created_at + interval '24 hours')
);

CREATE INDEX IF NOT EXISTS archive_revision_erp_outbox_poll_idx
    ON archive_revision_erp_feedback_outbox (status, next_attempt_at)
    WHERE status IN ('pending', 'failed');

-- 对账差异反馈（M-RC → ERP）
CREATE TABLE IF NOT EXISTS reconciliation_erp_feedback_outbox (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    recon_doc_no    TEXT,
    event_type      TEXT NOT NULL DEFAULT 'reconciliation_diff'
        CHECK (event_type = 'reconciliation_diff'),
    payload         JSONB NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'failed', 'succeeded', 'dead')),
    attempt_count   INT NOT NULL DEFAULT 0 CHECK (attempt_count BETWEEN 0 AND 5),
    max_attempts    INT NOT NULL DEFAULT 5 CHECK (max_attempts = 5),
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS reconciliation_erp_outbox_poll_idx
    ON reconciliation_erp_feedback_outbox (status, next_attempt_at)
    WHERE status IN ('pending', 'failed');

-- 出库发货确认（M4 → ERP）
CREATE TABLE IF NOT EXISTS shipment_confirm_erp_feedback_outbox (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    shipment_id     UUID,
    outbound_order_id UUID,
    event_type      TEXT NOT NULL DEFAULT 'shipment_confirm'
        CHECK (event_type = 'shipment_confirm'),
    payload         JSONB NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'failed', 'succeeded')),
    attempt_count   INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS shipment_confirm_erp_outbox_poll_idx
    ON shipment_confirm_erp_feedback_outbox (status, next_attempt_at)
    WHERE status IN ('pending', 'failed');

-- 库存快照同步（M3 → ERP）
CREATE TABLE IF NOT EXISTS inventory_snapshot_erp_feedback_outbox (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    snapshot_no     TEXT,
    event_type      TEXT NOT NULL DEFAULT 'inventory_snapshot'
        CHECK (event_type = 'inventory_snapshot'),
    payload         JSONB NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'failed', 'succeeded')),
    attempt_count   INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS inventory_snapshot_erp_outbox_poll_idx
    ON inventory_snapshot_erp_feedback_outbox (status, next_attempt_at)
    WHERE status IN ('pending', 'failed');

GRANT SELECT, INSERT, UPDATE ON archive_revision_erp_feedback_outbox TO wms_app;
GRANT SELECT, INSERT, UPDATE ON reconciliation_erp_feedback_outbox TO wms_app;
GRANT SELECT, INSERT, UPDATE ON shipment_confirm_erp_feedback_outbox TO wms_app;
GRANT SELECT, INSERT, UPDATE ON inventory_snapshot_erp_feedback_outbox TO wms_app;
