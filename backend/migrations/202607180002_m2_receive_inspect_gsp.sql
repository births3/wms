-- M2-002/003：收货 GSP 现场信息已用 JSONB；验收核对字段落库。

ALTER TABLE receiving_inspections
    ADD COLUMN IF NOT EXISTS quality_checks JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS sampling_qty BIGINT NOT NULL DEFAULT 0
        CHECK (sampling_qty >= 0),
    ADD COLUMN IF NOT EXISTS approval_no TEXT;

COMMENT ON COLUMN receiving_inspections.quality_checks IS
    'GSP 验收核对：appearance/package/instruction/label 等。';
COMMENT ON COLUMN receiving_inspections.sampling_qty IS
    '抽验数量；0 表示未单独登记抽验。';
COMMENT ON COLUMN receiving_inspections.approval_no IS
    '验收时核对的批准文号（可与档案比对）。';
