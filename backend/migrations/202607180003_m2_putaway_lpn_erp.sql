-- US-M2-005：上架 LPN 落库 + ERP 反馈 outbox（本地闭环标记成功，外部投递仍待 S4）。

ALTER TABLE receiving_putaways
    ADD COLUMN IF NOT EXISTS lpn_code TEXT;

COMMENT ON COLUMN receiving_putaways.lpn_code IS
    '可选容器/托盘 LPN；整托上架时记录。';

CREATE TABLE IF NOT EXISTS receiving_putaway_erp_feedback_outbox (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL,
    putaway_id      UUID NOT NULL REFERENCES receiving_putaways(id) ON DELETE CASCADE,
    receiving_order_id UUID NOT NULL,
    batch_id        UUID,
    event_type      TEXT NOT NULL DEFAULT 'inbound_putaway_completed',
    payload         JSONB NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'failed', 'succeeded')),
    attempt_count   INT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS receiving_putaway_erp_outbox_owner_status_idx
    ON receiving_putaway_erp_feedback_outbox (owner_id, status, next_attempt_at);

GRANT SELECT, INSERT, UPDATE ON receiving_putaway_erp_feedback_outbox TO wms_app;
