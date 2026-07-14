-- US-VR-006：在 M2 验收与 M4 出库复核落库双人策略执行证据。

ALTER TABLE receiving_inspection_signatures
    ADD COLUMN IF NOT EXISTS approval_record_id UUID REFERENCES h4_approval_records(id) ON DELETE RESTRICT;

ALTER TABLE receiving_inspection_signatures
    ADD CONSTRAINT receiving_inspection_signatures_strategy_rule_fk
    FOREIGN KEY (strategy_rule_id) REFERENCES dual_person_policy_rules(id) ON DELETE RESTRICT;

CREATE TABLE IF NOT EXISTS outbound_review_records (
    id                    UUID PRIMARY KEY,
    owner_id              UUID NOT NULL REFERENCES auth_owners(id) ON DELETE RESTRICT,
    outbound_order_id     UUID NOT NULL REFERENCES outbound_orders(id) ON DELETE RESTRICT,
    review_mode           TEXT NOT NULL,
    first_reviewer_id     UUID NOT NULL,
    second_reviewer_id    UUID,
    strategy_rule_id      UUID REFERENCES dual_person_policy_rules(id) ON DELETE RESTRICT,
    approval_record_id    UUID REFERENCES h4_approval_records(id) ON DELETE RESTRICT,
    reviewed_at           TIMESTAMPTZ NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (second_reviewer_id IS NULL OR second_reviewer_id <> first_reviewer_id)
);

CREATE INDEX IF NOT EXISTS outbound_review_records_owner_order_idx
    ON outbound_review_records (owner_id, outbound_order_id, reviewed_at DESC);

GRANT SELECT, INSERT ON outbound_review_records TO wms_app;
