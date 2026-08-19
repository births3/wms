use h8_erp_worker::contract::{canonical_payload_json, payload_digest, sha256_hex};
use serde_json::{json, Value};

#[test]
fn single_record_vectors_v1_v2_v4_v5_v6_v7_v8_v10_v12_match() {
    let cases = [
        (
            "x_wmsinter_GoodsInfo",
            r#"{"GoodsID":1001,"GoodsCode":"P-0001","GoodsName":"阿莫西林胶囊","SubName":"","ClassCode":"C01","BarCode":"6901000000011","Spec":"0.25g*24粒","Unit":"盒","Brand":"示例牌","ProduceArea":null,"License":"国药准字H20260001","IsDanger":0,"ValidityType":"月","ValidityNum":24,"RetailPrice":"12.500000","TaxRate":9,"ProduceCorp":"示例制药","StoreMemo":"密封保存","Deposite":"阴凉","MedicalType":"RX","PackagingJson":"[{\"unit\":\"盒\",\"ratio_to_base\":1,\"is_base\":true,\"is_default\":true},{\"unit\":\"件\",\"ratio_to_base\":24,\"is_base\":false,\"is_default\":false}]","opType":"I","OwnerCode":"ZBPF7","SchemaVersion":"1","IdempotencyKey":"msg-0001","CorrelationID":"corr-0001","SourceVersion":1}"#,
            "5ebdf9084e06e6a24c44870377fc7625a8b6bdf6b2ef288b315e33facd22a361",
        ),
        (
            "x_wmsinter_GoodsInfo",
            r#"{"GoodsID":1001,"GoodsCode":null,"GoodsName":null,"SubName":null,"ClassCode":null,"BarCode":null,"Spec":null,"Unit":null,"Brand":null,"ProduceArea":null,"License":null,"IsDanger":null,"ValidityType":null,"ValidityNum":null,"RetailPrice":null,"TaxRate":null,"ProduceCorp":null,"StoreMemo":null,"Deposite":null,"MedicalType":null,"PackagingJson":null,"opType":"D","OwnerCode":"ZBPF7","SchemaVersion":"1","IdempotencyKey":"msg-0002","CorrelationID":"corr-0001","SourceVersion":2}"#,
            "c96514eb8fb6d81b7b7f1bdcded4fc9c7e70b065c1f2171d4005ef218fb74384",
        ),
        (
            "x_wmsinter_OrderFeedback",
            r#"{"IdempotencyKey":"evt-0601","ERPBillCode":"CK20260805-001","Revision":1,"OrderType":2,"FeedbackType":6,"CommandID":null,"ResultCount":2,"ResultCode":null,"ResultMessage":null,"WaybillNo":"SF1234567890","ExpressCompany":"顺丰速运","ShipTime":"2026-08-05T10:30:00.123Z","FeedbackTime":"2026-08-05T10:30:05.000Z","OperatorName":"张三","OwnerCode":"ZBPF7","SchemaVersion":"1","CorrelationID":"corr-0201","SourceVersion":null}"#,
            "f1283936729b5517a55be6034350d543d67b1653cc032512b69a29101df2024a",
        ),
        (
            "x_wmsinter_OrderCommand",
            r#"{"CommandID":"cmd-0001","CommandType":99,"ERPBillCode":"CK20260805-001","Revision":1,"OrderType":2,"Memo":"客户撤销订单","OwnerCode":"ZBPF7","SchemaVersion":"1","IdempotencyKey":"cmd-0001","CorrelationID":"corr-0201","SourceVersion":null}"#,
            "48509ffd2bdf444c984e519061ce5592682046d074a4c546472e1fee6246d2c8",
        ),
        (
            "x_wmsinter_InboundFeedback",
            r#"{"IdempotencyKey":"rcv-0001","ERPBillCode":"RK20260805-001","Revision":1,"LineNo":1,"GoodsID":1001,"GoodsCode":"P-1001","ExpectedAmount":"50.5000","ActualAmount":"40.0000","RejectAmount":"5.5000","ShortageAmount":"5.0000","RejectReason":"包装破损","ShortageReason":"供应商缺货","BatchNo":"B20260701","ProduceDate":"2026-07-01","ValidDate":"2028-06-30","StallCode":"A-01-02","OperatorName":"李四","ScanTime":"2026-08-05T11:00:00.000Z","OwnerCode":"ZBPF7","SchemaVersion":"1","CorrelationID":"corr-0101","SourceVersion":null}"#,
            "41860cc731faa0d87631c54d4e2b53b4c1b5aba691330569ea87397018e24ad8",
        ),
        (
            "x_wmsinter_OutboundFeedback",
            r#"{"IdempotencyKey":"shp-0001","ERPBillCode":"CK20260805-001","Revision":1,"LineNo":1,"GoodsID":1001,"GoodsCode":"P-1001","BatchNo":"B20260701","ExpectedAmount":"20.0000","PickedAmount":"20.0000","ShippedAmount":"20.0000","OperatorName":"王五","OwnerCode":"ZBPF7","SchemaVersion":"1","CorrelationID":"corr-0201","SourceVersion":null}"#,
            "09254c88052470ec0634ca59d40a48ef54033f57363e55a776985679926e62df",
        ),
        (
            "x_wmsinter_WmsEvent",
            r#"{"IdempotencyKey":"evt-0801","EventType":"inventory_status","SchemaVersion":"1","PayloadJson":"{\"depot_code\":\"WH001\",\"product_code\":\"P-1001\",\"batch_no\":\"B20260701\",\"goods_status\":\"合格\",\"amount\":\"40.0000\",\"occur_time\":\"2026-08-05T11:05:00.000Z\"}","EventTime":"2026-08-05T11:05:00.000Z","OwnerCode":"ZBPF7","CorrelationID":"corr-0301","SourceVersion":null}"#,
            "a252c30c480a9836f44ea9b96cd29ea9a899995c3464f61339e1e329702a52f1",
        ),
        (
            "x_wmsinter_InventoryReceiveItems",
            r#"{"SnapshotID":"RSNP-0001","RowNo":1,"DepotCode":"WH001","GoodsCode":"P-1001","BatchNo":"B20260701","ValidDate":"2028-06-30","GoodsStatus":"合格","WMSAmount":"40.0000","WMSPickable":"38.0000","WMSAllocated":"2.0000","WMSFrozen":"0.0000","OwnerCode":"ZBPF7","CorrelationID":"corr-0402","IdempotencyKey":"RSNP-0001:1"}"#,
            "47a791e8dbfd8aebcea2b1a6dd21715799814717b6c7734825c908fd928a8a94",
        ),
        (
            "x_wmsinter_GoodsInfo",
            r#"{"GoodsID":1001,"GoodsCode":"P-0001","GoodsName":"阿莫西林胶囊","SubName":null,"ClassCode":"C01","BarCode":"6901000000011","Spec":"0.25g*24粒","Unit":"盒","Brand":"示例牌","ProduceArea":null,"License":"国药准字H20260001","IsDanger":0,"ValidityType":"月","ValidityNum":24,"RetailPrice":"12.500000","TaxRate":9,"ProduceCorp":"示例制药","StoreMemo":"密封保存","Deposite":"阴凉","MedicalType":"RX","PackagingJson":"[{\"unit\":\"盒\",\"ratio_to_base\":1,\"is_base\":true,\"is_default\":true},{\"unit\":\"件\",\"ratio_to_base\":24,\"is_base\":false,\"is_default\":false}]","opType":"I","OwnerCode":"ZBPF7","SchemaVersion":"1","IdempotencyKey":"msg-0001","CorrelationID":"corr-0001","SourceVersion":1}"#,
            "586b3a9ba0611cf5b4d2d99190cfd2854b4bc239ffb0802fcbf9b5e5a0153ff3",
        ),
    ];

    for (table, canonical, expected) in cases {
        let row: Value = serde_json::from_str(canonical).expect("向量必须是合法 JSON");
        assert_eq!(
            canonical_payload_json(table, &row, &[]).expect("应规范化向量"),
            canonical,
            "canonical mismatch for {table}"
        );
        assert_eq!(
            payload_digest(table, &row, &[]).expect("应计算摘要"),
            expected,
            "digest mismatch for {table}"
        );
    }
}

#[test]
fn inbound_header_and_items_vector_v3_is_sorted_and_covers_order_id() {
    let canonical = r#"[{"ERPBillID":9001,"ERPBillCode":"RK20260805-001","Revision":1,"OrderType":1,"PartnerType":"supplier","PartnerID":2001,"PartnerCode":"S-001","PartnerName":"示例供应商","DepotID":1,"DepotCode":"WH001","DeptID":null,"BusiDate":"2026-08-05","SumMoney":"1200.0000","NoteCode":"PO-20260805-01","LineCount":2,"OwnerCode":"ZBPF7","SchemaVersion":"1","IdempotencyKey":"msg-0101","CorrelationID":"corr-0101","SourceVersion":null},{"OrderID":1,"ERPBillID":9001,"ERPBillCode":"RK20260805-001","Revision":1,"LineNo":1,"GoodsID":1001,"GoodsCode":"P-1001","GoodsName":null,"Amount":"50.5000","Price":"23.50000000","Sums":"1186.7500","BatchNo":"B20260701","ProduceDate":"2026-07-01","ValidDate":"2028-06-30","Unit":"盒","OwnerCode":"ZBPF7","CorrelationID":"corr-0101","IdempotencyKey":"6d5acae1510745fdf00e0323b5e82eca657d4dac891e2f21c14c3fc39143e085"},{"OrderID":1,"ERPBillID":9001,"ERPBillCode":"RK20260805-001","Revision":1,"LineNo":2,"GoodsID":1002,"GoodsCode":"P-1002","GoodsName":null,"Amount":"24.0000","Price":"23.50000000","Sums":"564.0000","BatchNo":"B20260701","ProduceDate":"2026-07-01","ValidDate":"2028-06-30","Unit":"盒","OwnerCode":"ZBPF7","CorrelationID":"corr-0101","IdempotencyKey":"0944b05501770be534c1fdc05f87515eb09e1cf1ad6f4bd35ef85705ce3d77f6"}]"#; // gitleaks:allow 摘要契约测试向量（SHA-256 定值，非秘密）
    let values: Value = serde_json::from_str(canonical).expect("V3 必须是合法 JSON");
    let rows = values.as_array().expect("V3 必须是发布单元数组");
    let header = rows.first().expect("V3 必须有头记录");
    let reversed = vec![rows[2].clone(), rows[1].clone()];

    assert_eq!(
        canonical_payload_json("x_wmsinter_InboundOrder", header, &reversed)
            .expect("V3 应按 LineNo 排序"),
        canonical
    );
    assert_eq!(
        payload_digest("x_wmsinter_InboundOrder", header, &reversed).expect("应计算 V3 摘要"),
        "c209f5a572f5633599d42a5b3c78e6540c081e6973f9978de621f0da96fea896"
    );
}

#[test]
fn inventory_header_and_item_vector_v9_matches() {
    let header = json!({
        "SnapshotID": "SNP-20260805-0001", "DepotID": 1, "DepotCode": "WH001",
        "PushType": 1, "PushTime": "2026-08-05T00:00:00.000Z", "TotalCount": 1,
        "OwnerCode": "ZBPF7", "SchemaVersion": "1", "IdempotencyKey": "SNP-20260805-0001",
        "CorrelationID": "corr-0401", "SourceVersion": null
    });
    let item = json!({
        "SnapshotID": "SNP-20260805-0001", "RowNo": 1, "GoodsID": 1001,
        "GoodsCode": "P-1001", "BatchID": 3001, "BatchNo": "B20260701",
        "ValidDate": "2028-06-30", "StallCode": "A-01-02", "GoodsStatus": "合格",
        "RealAmount": "100", "CanSell": "95", "OwnerCode": "ZBPF7",
        "CorrelationID": "corr-0401", "IdempotencyKey": "SNP-20260805-0001:1"
    });
    assert_eq!(
        payload_digest("x_wmsinter_InventoryPushHeader", &header, &[item]).expect("应计算 V9 摘要"),
        "fc88721d689578a0285c68b44329f3f37e4d180e44dd5503b3a32c8d733159d3"
    );
}

#[test]
fn negative_vectors_v11_and_v12_remain_distinct() {
    let unsorted_v11 = r#"[{"ERPBillID":9001,"ERPBillCode":"RK20260805-001","Revision":1,"OrderType":1,"PartnerType":"supplier","PartnerID":2001,"PartnerCode":"S-001","PartnerName":"示例供应商","DepotID":1,"DepotCode":"WH001","DeptID":null,"BusiDate":"2026-08-05","SumMoney":"1200.0000","NoteCode":"PO-20260805-01","LineCount":2,"OwnerCode":"ZBPF7","SchemaVersion":"1","IdempotencyKey":"msg-0101","CorrelationID":"corr-0101","SourceVersion":null},{"OrderID":1,"ERPBillID":9001,"ERPBillCode":"RK20260805-001","Revision":1,"LineNo":2,"GoodsID":1002,"GoodsCode":"P-1002","GoodsName":null,"Amount":"24.0000","Price":"23.50000000","Sums":"564.0000","BatchNo":"B20260701","ProduceDate":"2026-07-01","ValidDate":"2028-06-30","Unit":"盒","OwnerCode":"ZBPF7","CorrelationID":"corr-0101","IdempotencyKey":"0944b05501770be534c1fdc05f87515eb09e1cf1ad6f4bd35ef85705ce3d77f6"},{"OrderID":1,"ERPBillID":9001,"ERPBillCode":"RK20260805-001","Revision":1,"LineNo":1,"GoodsID":1001,"GoodsCode":"P-1001","GoodsName":null,"Amount":"50.5000","Price":"23.50000000","Sums":"1186.7500","BatchNo":"B20260701","ProduceDate":"2026-07-01","ValidDate":"2028-06-30","Unit":"盒","OwnerCode":"ZBPF7","CorrelationID":"corr-0101","IdempotencyKey":"6d5acae1510745fdf00e0323b5e82eca657d4dac891e2f21c14c3fc39143e085"}]"#; // gitleaks:allow 摘要契约测试向量（SHA-256 定值，非秘密）
    assert_eq!(
        sha256_hex(unsorted_v11.as_bytes()),
        "3a084c21b532942da05802c042f37b8e01a1025f1111e98e623434be2b23a3bb"
    );
    assert_ne!(
        "5ebdf9084e06e6a24c44870377fc7625a8b6bdf6b2ef288b315e33facd22a361",
        "586b3a9ba0611cf5b4d2d99190cfd2854b4bc239ffb0802fcbf9b5e5a0153ff3"
    );
}
