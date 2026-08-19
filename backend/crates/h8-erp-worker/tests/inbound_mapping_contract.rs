use h8_erp_worker::inbound::{inbound_contract, request_body};
use serde_json::json;

#[test]
fn all_v19_inbound_main_tables_are_routed() {
    let expected = [
        ("x_wmsinter_GoodsInfo", "product_master"),
        ("x_wmsinter_CustomerInfo", "customer_master"),
        ("x_wmsinter_SupplierInfo", "supplier_master"),
        ("x_wmsinter_InboundOrder", "asn"),
        ("x_wmsinter_OutboundOrder", "outbound_order"),
        ("x_wmsinter_OrderCommand", "order_cancel"),
        ("x_wmsinter_InventoryPushHeader", "inventory_seed_snapshot"),
    ];
    for (table, message_type) in expected {
        assert_eq!(
            inbound_contract(table).map(|item| item.message_type),
            Some(message_type)
        );
    }
}

#[test]
fn order_and_snapshot_quantities_stay_decimal_strings() {
    let envelope = json!({
        "SchemaVersion": "1", "CorrelationID": "corr-1", "PayloadDigest": "a".repeat(64),
        "SourceVersion": null, "inserttime": "2026-08-05T00:00:00.000Z"
    });
    let mut inbound = json!({
        "ERPBillID": 9, "ERPBillCode": "RK-1", "Revision": 1, "OrderType": 1,
        "PartnerType": "supplier", "PartnerCode": "S-1", "DepotCode": "WH001",
        "BusiDate": "2026-08-05", "NoteCode": null
    });
    inbound
        .as_object_mut()
        .expect("object")
        .extend(envelope.as_object().expect("object").clone());
    let body = request_body(
        "asn",
        &inbound,
        &[json!({
            "LineNo": 1, "GoodsCode": "P-1", "Amount": "50.5000", "BatchNo": "B-1",
            "ProduceDate": null, "ValidDate": "2028-06-30"
        })],
    )
    .expect("asn body");
    assert_eq!(body["lines"][0]["expected_qty"], "50.5000");

    let mut snapshot = json!({
        "SnapshotID": "SNP-1", "DepotCode": "WH001", "PushType": 1,
        "PushTime": "2026-08-05T00:00:00.000Z"
    });
    snapshot
        .as_object_mut()
        .expect("object")
        .extend(envelope.as_object().expect("object").clone());
    let body = request_body(
        "inventory_seed_snapshot",
        &snapshot,
        &[json!({
            "RowNo": 1, "GoodsCode": "P-1", "BatchNo": "B-1", "ValidDate": null,
            "StallCode": "A-1", "GoodsStatus": "合格", "RealAmount": "1.2500"
        })],
    )
    .expect("snapshot body");
    assert_eq!(body["items"][0]["quantity"], "1.2500");
}

#[test]
fn product_packaging_gets_stable_sort_order() {
    let row = json!({
        "SchemaVersion": "1", "CorrelationID": "corr-1", "PayloadDigest": "a".repeat(64),
        "SourceVersion": 1, "inserttime": "2026-08-05T00:00:00.000Z",
        "GoodsID": 1001, "opType": "I", "GoodsCode": "P-1", "GoodsName": "药品",
        "License": null, "Spec": "10mg", "ProduceCorp": "药厂", "SpecialCategory": null,
        "Deposite": "阴凉", "PackagingJson": "[{\"unit\":\"盒\",\"ratio_to_base\":1,\"is_base\":true,\"is_default\":true}]"
    });
    let body = request_body("product_master", &row, &[]).expect("product body");
    assert_eq!(body["packaging_levels"][0]["sort_order"], 1);
    assert_eq!(body["source_version"], 1);
}
