use h8_erp_worker::receipts::{interface_receipt_table, parse_outbox_identity};
use uuid::Uuid;

#[test]
fn receipt_lookup_uses_only_v19_main_records() {
    assert_eq!(
        interface_receipt_table("putaway_complete"),
        Some("x_wmsinter_InboundFeedback")
    );
    assert_eq!(
        interface_receipt_table("shipment_confirm"),
        Some("x_wmsinter_OrderFeedback")
    );
    assert_eq!(
        interface_receipt_table("inventory_snapshot"),
        Some("x_wmsinter_InventoryReceiveHeader")
    );
    assert_eq!(
        interface_receipt_table("inventory_status"),
        Some("x_wmsinter_WmsEvent")
    );
    assert_eq!(interface_receipt_table("unknown"), None);
}

#[test]
fn timeout_requeue_identity_accepts_only_registered_outbox_keys() {
    let id = Uuid::nil();
    assert_eq!(
        parse_outbox_identity(&format!("out:shipment_confirm_erp_feedback_outbox:{id}")),
        Some(("shipment_confirm_erp_feedback_outbox", id))
    );
    assert!(parse_outbox_identity(&format!("out:users:{id}")).is_none());
    assert!(parse_outbox_identity("bad").is_none());
}
