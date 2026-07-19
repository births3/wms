-- US-H8-003：尝试结果允许 archived（归档不删除）
ALTER TABLE h8_erp_message_attempts
    DROP CONSTRAINT IF EXISTS h8_erp_message_attempts_result_check;

ALTER TABLE h8_erp_message_attempts
    ADD CONSTRAINT h8_erp_message_attempts_result_check
    CHECK (result IN ('succeeded', 'failed', 'dead', 'replayed', 'claimed', 'archived'));
