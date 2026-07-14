ALTER TABLE receiving_order_receipts
    ADD COLUMN IF NOT EXISTS receiving_details JSONB;

COMMENT ON COLUMN receiving_order_receipts.receiving_details IS
    '类型化收货现场信息：温控、车辆、承运商、时间及随货核对结果';
