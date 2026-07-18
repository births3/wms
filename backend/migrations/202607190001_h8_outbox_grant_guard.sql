-- 若 wms_app 角色存在则补授权（无角色的纯开发库跳过，避免迁移失败）
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'wms_app') THEN
        GRANT SELECT, INSERT, UPDATE ON archive_revision_erp_feedback_outbox TO wms_app;
        GRANT SELECT, INSERT, UPDATE ON reconciliation_erp_feedback_outbox TO wms_app;
        GRANT SELECT, INSERT, UPDATE ON shipment_confirm_erp_feedback_outbox TO wms_app;
        GRANT SELECT, INSERT, UPDATE ON inventory_snapshot_erp_feedback_outbox TO wms_app;
    END IF;
END $$;
