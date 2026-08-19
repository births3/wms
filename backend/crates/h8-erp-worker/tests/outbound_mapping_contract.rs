use h8_erp_worker::outbound::{build_published_unit, OutboxRow, PublishedUnit};
use serde_json::json;
use uuid::Uuid;

fn row(event_type: &str, payload: serde_json::Value) -> OutboxRow {
    OutboxRow {
        table: "shipment_confirm_erp_feedback_outbox",
        id: Uuid::parse_str("10000000-0000-0000-0000-000000000001").expect("uuid"),
        owner_id: Uuid::nil(),
        event_type: event_type.to_owned(),
        payload,
        external_ref: "external-1".to_owned(),
        attempt_count: 1,
        max_attempts: 5,
        created_at: "2026-08-05T10:30:00.000Z".to_owned(),
    }
}

#[test]
fn shipment_details_are_sorted_before_the_completion_barrier() {
    let unit = build_published_unit(
        &row(
            "shipment_confirm",
            json!({
                "erp_bill_code": "CK-1", "revision": 1, "line_count": 2,
                "correlation_id": "corr-1", "ship_time": "2026-08-05T10:30:00.000Z",
                "operator_name": "张三",
                "lines": [
                    {"line_no": 2, "goods_id": 2, "product_code": "P-2", "batch_no": "B-2", "expected_amount": "2.0000", "picked_amount": "2.0000", "shipped_amount": "2.0000"},
                    {"line_no": 1, "goods_id": 1, "product_code": "P-1", "batch_no": "B-1", "expected_amount": "1.0000", "picked_amount": "1.0000", "shipped_amount": "1.0000"}
                ]
            }),
        ),
        "ZBPF7",
    )
    .expect("shipment should map");
    let PublishedUnit::Transaction(records) = unit else {
        panic!("shipment must be one transaction")
    };
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].table, "x_wmsinter_OutboundFeedback");
    assert_eq!(records[0].row["LineNo"], 1);
    assert_eq!(records[1].row["LineNo"], 2);
    assert_eq!(records[2].table, "x_wmsinter_OrderFeedback");
    assert_eq!(records[2].row["FeedbackType"], 6);
    assert_eq!(records[2].row["ResultCount"], 2);
}

#[test]
fn inventory_snapshot_defaults_zero_balances_and_keeps_children_digestless() {
    let mut source = row(
        "inventory_snapshot",
        json!({
            "snapshot_id": "RSNP-1", "depot_code": "WH001",
            "receive_time": "2026-08-05T10:30:00.000Z", "correlation_id": "corr-2",
            "lines": [{
                "row_no": 1, "product_code": "P-1", "batch_no": "B-1",
                "goods_status": "合格", "wms_amount": "4.0000", "wms_pickable": "3.0000"
            }]
        }),
    );
    source.table = "inventory_snapshot_erp_feedback_outbox";
    let PublishedUnit::HeaderChildren { header, children } =
        build_published_unit(&source, "ZBPF7").expect("snapshot should map")
    else {
        panic!("snapshot must be header plus children")
    };
    assert_eq!(header.table, "x_wmsinter_InventoryReceiveHeader");
    assert_eq!(header.row["TotalCount"], 1);
    assert!(header.row["PayloadDigest"]
        .as_str()
        .is_some_and(|v| v.len() == 64));
    assert_eq!(children[0].row["WMSAllocated"], "0.0000");
    assert_eq!(children[0].row["WMSFrozen"], "0.0000");
    assert!(children[0].row.get("PayloadDigest").is_none());
}

#[test]
fn outbound_quantity_mismatch_is_rejected_before_database_write() {
    let error = build_published_unit(
        &row(
            "shipment_confirm",
            json!({
                "erp_bill_code": "CK-1", "revision": 1, "line_count": 1,
                "correlation_id": "corr-1", "ship_time": "2026-08-05T10:30:00.000Z",
                "lines": [{"line_no": 1, "goods_id": 1, "product_code": "P-1", "batch_no": "B-1", "expected_amount": "2.0000", "picked_amount": "1.0000", "shipped_amount": "1.0000"}]
            }),
        ),
        "ZBPF7",
    )
    .expect_err("partial shipment must fail");
    assert_eq!(error.code(), "INVALID_DATA");
}

#[test]
fn order_status_and_wms_events_use_outbox_id_as_stable_idempotency_key() {
    let status = build_published_unit(
        &row(
            "order_status",
            json!({
                "erp_bill_code": "CK-1", "revision": 1, "order_type": 2,
                "feedback_type": 9, "result_code": "OUTBOUND_SHORTAGE",
                "feedback_time": "2026-08-05T10:30:00.000Z", "correlation_id": "corr-1"
            }),
        ),
        "ZBPF7",
    )
    .expect("status should map");
    let PublishedUnit::Transaction(records) = status else {
        panic!("single record uses transactional publisher")
    };
    assert_eq!(
        records[0].row["IdempotencyKey"],
        records[0].row["CorrelationID"]
            .as_str()
            .map(|_| "10000000-0000-0000-0000-000000000001")
            .expect("correlation")
    );

    let event = build_published_unit(
        &row(
            "inventory_status_changed",
            json!({
                "depot_code": "WH001", "product_code": "P-1", "batch_no": "B-1",
                "to_status": "合格", "qty": "4.0000", "occur_time": "2026-08-05T10:30:00.000Z"
            }),
        ),
        "ZBPF7",
    )
    .expect("event should map");
    let PublishedUnit::Transaction(records) = event else {
        panic!("single record uses transactional publisher")
    };
    assert_eq!(records[0].table, "x_wmsinter_WmsEvent");
    assert_eq!(
        records[0].row["IdempotencyKey"],
        "10000000-0000-0000-0000-000000000001"
    );
}
