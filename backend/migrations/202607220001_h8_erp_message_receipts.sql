-- US-H8-002/003：契约版本与技术接收后等待业务回执状态。

ALTER TABLE h8_erp_messages
    ADD COLUMN IF NOT EXISTS schema_version TEXT NOT NULL DEFAULT '1';

ALTER TABLE h8_erp_messages
    DROP CONSTRAINT IF EXISTS h8_erp_messages_sync_status_check;

ALTER TABLE h8_erp_messages
    ADD CONSTRAINT h8_erp_messages_sync_status_check
    CHECK (sync_status IN (
        'pending', 'processing', 'succeeded', 'awaiting_receipt',
        'failed', 'dead', 'acked'
    ));
