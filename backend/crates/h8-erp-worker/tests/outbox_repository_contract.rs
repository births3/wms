use h8_erp_worker::outbox_repository::{claim_sql, outbox_sources};

#[test]
fn all_frozen_outbox_sources_are_claimed_with_skip_locked() {
    assert_eq!(outbox_sources().len(), 7);
    for source in outbox_sources() {
        let sql = claim_sql(source);
        assert!(sql.contains("FOR UPDATE SKIP LOCKED"));
        assert!(sql.contains("status IN ('pending', 'failed')"));
        assert!(sql.contains("attempt_count = source.attempt_count + 1"));
        assert!(sql.contains(source.table));
    }
}

#[test]
fn source_registry_is_the_only_dynamic_sql_surface() {
    let names = outbox_sources()
        .iter()
        .map(|source| source.table)
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "receiving_putaway_erp_feedback_outbox",
            "inventory_status_erp_feedback_outbox",
            "stock_adjustment_erp_feedback_outbox",
            "archive_revision_erp_feedback_outbox",
            "reconciliation_erp_feedback_outbox",
            "shipment_confirm_erp_feedback_outbox",
            "inventory_snapshot_erp_feedback_outbox",
        ]
    );
}
