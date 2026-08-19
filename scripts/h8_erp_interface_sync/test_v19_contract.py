"""ERP-WMS v1.9 规范摘要与接口表认领契约。"""

from __future__ import annotations

import json
import unittest
from unittest.mock import patch

from v19_contract import (
    ContractError,
    canonical_payload_json,
    payload_digest,
    validate_published_unit,
)
from worker_mssql import _plain_rows, claim_rows

from test_h8_sync_worker import settings


class TestV19PayloadDigest(unittest.TestCase):
    def test_mssql_rows_restore_gbk_varchar_without_touching_unicode(self) -> None:
        self.assertEqual(
            _plain_rows([{"Unit": "ºÐ", "GoodsName": "胃康灵胶囊", "GoodsCode": "P-1"}]),
            [{"Unit": "盒", "GoodsName": "胃康灵胶囊", "GoodsCode": "P-1"}],
        )

    def assert_vector(self, table: str, canonical: str, expected: str) -> None:
        row = json.loads(canonical)
        self.assertEqual(canonical_payload_json(table, row), canonical)
        self.assertEqual(payload_digest(table, row), expected)

    def test_goods_master_vector_v1(self) -> None:
        row = {
            "GoodsID": "1001",
            "GoodsCode": "P-0001",
            "GoodsName": "阿莫西林胶囊",
            "SubName": "",
            "ClassCode": "C01",
            "BarCode": "6901000000011",
            "Spec": "0.25g*24粒",
            "Unit": "盒",
            "Brand": "示例牌",
            "ProduceArea": None,
            "License": "国药准字H20260001",
            "IsDanger": "0",
            "ValidityType": "月",
            "ValidityNum": "24",
            "RetailPrice": "12.5",
            "TaxRate": "9",
            "ProduceCorp": "示例制药",
            "StoreMemo": "密封保存",
            "Deposite": "阴凉",
            "MedicalType": "RX",
            "PackagingJson": json.dumps(
                [
                    {
                        "unit": "盒",
                        "ratio_to_base": 1,
                        "is_base": True,
                        "is_default": True,
                    },
                    {
                        "unit": "件",
                        "ratio_to_base": 24,
                        "is_base": False,
                        "is_default": False,
                    },
                ],
                ensure_ascii=False,
            ),
            "opType": "I",
            "OwnerCode": "ZBPF7",
            "SchemaVersion": "1",
            "IdempotencyKey": "msg-0001",
            "CorrelationID": "corr-0001",
            "SourceVersion": "1",
        }

        self.assertEqual(
            payload_digest("x_wmsinter_GoodsInfo", row),
            "5ebdf9084e06e6a24c44870377fc7625a8b6bdf6b2ef288b315e33facd22a361",
        )
        self.assertIn('"RetailPrice":"12.500000"', canonical_payload_json("x_wmsinter_GoodsInfo", row))

    def test_goods_delete_vector_v2_preserves_null(self) -> None:
        row = {
            "GoodsID": 1001,
            "opType": "D",
            "OwnerCode": "ZBPF7",
            "SchemaVersion": "1",
            "IdempotencyKey": "msg-0002",
            "CorrelationID": "corr-0001",
            "SourceVersion": 2,
        }

        self.assertEqual(
            payload_digest("x_wmsinter_GoodsInfo", row),
            "c96514eb8fb6d81b7b7f1bdcded4fc9c7e70b065c1f2171d4005ef218fb74384",
        )

    def test_inbound_order_vectors_v3_and_v11_sort_children(self) -> None:
        header = {
            "ERPBillID": 9001,
            "ERPBillCode": "RK20260805-001",
            "Revision": 1,
            "OrderType": 1,
            "PartnerType": "supplier",
            "PartnerID": 2001,
            "PartnerCode": "S-001",
            "PartnerName": "示例供应商",
            "DepotID": 1,
            "DepotCode": "WH001",
            "DeptID": None,
            "BusiDate": "2026-08-05",
            "SumMoney": "1200",
            "NoteCode": "PO-20260805-01",
            "LineCount": 2,
            "OwnerCode": "ZBPF7",
            "SchemaVersion": "1",
            "IdempotencyKey": "msg-0101",
            "CorrelationID": "corr-0101",
            "SourceVersion": None,
        }
        items = [
            {
                "ERPBillID": 9001,
                "ERPBillCode": "RK20260805-001",
                "Revision": 1,
                "LineNo": line_no,
                "GoodsID": goods_id,
                "GoodsCode": f"P-{goods_id}",
                "GoodsName": None,
                "Amount": amount,
                "Price": "23.5",
                "Sums": sums,
                "BatchNo": "B20260701",
                "ProduceDate": "2026-07-01",
                "ValidDate": "2028-06-30",
                "Unit": "盒",
                "OwnerCode": "ZBPF7",
                "CorrelationID": "corr-0101",
                "IdempotencyKey": idem,
            }
            for line_no, goods_id, amount, sums, idem in (
                (2, 1002, "24", "564", "0944b05501770be534c1fdc05f87515eb09e1cf1ad6f4bd35ef85705ce3d77f6"),
                (1, 1001, "50.5", "1186.75", "6d5acae1510745fdf00e0323b5e82eca657d4dac891e2f21c14c3fc39143e085"),
            )
        ]

        self.assertEqual(
            payload_digest("x_wmsinter_InboundOrder", header, items),
            "a8df28e52f388b7a1dd4521a2007c1417bca436377c7daf8c1f149d9ffaf3f5a",
        )
        self.assertEqual(
            payload_digest("x_wmsinter_InboundOrder", header, list(reversed(items))),
            "a8df28e52f388b7a1dd4521a2007c1417bca436377c7daf8c1f149d9ffaf3f5a",
        )

    def test_single_record_vectors_v4_to_v8_and_v12(self) -> None:
        cases = (
            (
                "x_wmsinter_OrderFeedback",
                '{"IdempotencyKey":"evt-0601","ERPBillCode":"CK20260805-001","Revision":1,"OrderType":2,"FeedbackType":6,"CommandID":null,"ResultCount":2,"ResultCode":null,"ResultMessage":null,"WaybillNo":"SF1234567890","ExpressCompany":"顺丰速运","ShipTime":"2026-08-05T10:30:00.123Z","FeedbackTime":"2026-08-05T10:30:05.000Z","OperatorName":"张三","OwnerCode":"ZBPF7","SchemaVersion":"1","CorrelationID":"corr-0201","SourceVersion":null}',
                "f1283936729b5517a55be6034350d543d67b1653cc032512b69a29101df2024a",
            ),
            (
                "x_wmsinter_OrderCommand",
                '{"CommandID":"cmd-0001","CommandType":99,"ERPBillCode":"CK20260805-001","Revision":1,"OrderType":2,"Memo":"客户撤销订单","OwnerCode":"ZBPF7","SchemaVersion":"1","IdempotencyKey":"cmd-0001","CorrelationID":"corr-0201","SourceVersion":null}',
                "48509ffd2bdf444c984e519061ce5592682046d074a4c546472e1fee6246d2c8",
            ),
            (
                "x_wmsinter_InboundFeedback",
                '{"IdempotencyKey":"rcv-0001","ERPBillCode":"RK20260805-001","Revision":1,"LineNo":1,"GoodsID":1001,"GoodsCode":"P-1001","ExpectedAmount":"50.5000","ActualAmount":"40.0000","RejectAmount":"5.5000","ShortageAmount":"5.0000","RejectReason":"包装破损","ShortageReason":"供应商缺货","BatchNo":"B20260701","ProduceDate":"2026-07-01","ValidDate":"2028-06-30","StallCode":"A-01-02","OperatorName":"李四","ScanTime":"2026-08-05T11:00:00.000Z","OwnerCode":"ZBPF7","SchemaVersion":"1","CorrelationID":"corr-0101","SourceVersion":null}',
                "41860cc731faa0d87631c54d4e2b53b4c1b5aba691330569ea87397018e24ad8",
            ),
            (
                "x_wmsinter_OutboundFeedback",
                '{"IdempotencyKey":"shp-0001","ERPBillCode":"CK20260805-001","Revision":1,"LineNo":1,"GoodsID":1001,"GoodsCode":"P-1001","BatchNo":"B20260701","ExpectedAmount":"20.0000","PickedAmount":"20.0000","ShippedAmount":"20.0000","OperatorName":"王五","OwnerCode":"ZBPF7","SchemaVersion":"1","CorrelationID":"corr-0201","SourceVersion":null}',
                "09254c88052470ec0634ca59d40a48ef54033f57363e55a776985679926e62df",
            ),
            (
                "x_wmsinter_WmsEvent",
                '{"IdempotencyKey":"evt-0801","EventType":"inventory_status","SchemaVersion":"1","PayloadJson":"{\\"depot_code\\":\\"WH001\\",\\"product_code\\":\\"P-1001\\",\\"batch_no\\":\\"B20260701\\",\\"goods_status\\":\\"合格\\",\\"amount\\":\\"40.0000\\",\\"occur_time\\":\\"2026-08-05T11:05:00.000Z\\"}","EventTime":"2026-08-05T11:05:00.000Z","OwnerCode":"ZBPF7","CorrelationID":"corr-0301","SourceVersion":null}',
                "a252c30c480a9836f44ea9b96cd29ea9a899995c3464f61339e1e329702a52f1",
            ),
            (
                "x_wmsinter_GoodsInfo",
                '{"GoodsID":1001,"GoodsCode":"P-0001","GoodsName":"阿莫西林胶囊","SubName":null,"ClassCode":"C01","BarCode":"6901000000011","Spec":"0.25g*24粒","Unit":"盒","Brand":"示例牌","ProduceArea":null,"License":"国药准字H20260001","IsDanger":0,"ValidityType":"月","ValidityNum":24,"RetailPrice":"12.500000","TaxRate":9,"ProduceCorp":"示例制药","StoreMemo":"密封保存","Deposite":"阴凉","MedicalType":"RX","PackagingJson":"[{\\"unit\\":\\"盒\\",\\"ratio_to_base\\":1,\\"is_base\\":true,\\"is_default\\":true},{\\"unit\\":\\"件\\",\\"ratio_to_base\\":24,\\"is_base\\":false,\\"is_default\\":false}]","opType":"I","OwnerCode":"ZBPF7","SchemaVersion":"1","IdempotencyKey":"msg-0001","CorrelationID":"corr-0001","SourceVersion":1}',
                "586b3a9ba0611cf5b4d2d99190cfd2854b4bc239ffb0802fcbf9b5e5a0153ff3",
            ),
        )
        for table, canonical, expected in cases:
            with self.subTest(table=table, expected=expected):
                self.assert_vector(table, canonical, expected)

    def test_inventory_snapshot_vectors_v9_and_v10(self) -> None:
        header = {
            "SnapshotID": "SNP-20260805-0001",
            "DepotID": 1,
            "DepotCode": "WH001",
            "PushType": 1,
            "PushTime": "2026-08-05T00:00:00.000Z",
            "TotalCount": 1,
            "OwnerCode": "ZBPF7",
            "SchemaVersion": "1",
            "IdempotencyKey": "SNP-20260805-0001",
            "CorrelationID": "corr-0401",
            "SourceVersion": None,
        }
        item = {
            "SnapshotID": "SNP-20260805-0001",
            "RowNo": 1,
            "GoodsID": 1001,
            "GoodsCode": "P-1001",
            "BatchID": 3001,
            "BatchNo": "B20260701",
            "ValidDate": "2028-06-30",
            "StallCode": "A-01-02",
            "GoodsStatus": "合格",
            "RealAmount": "100",
            "CanSell": "95",
            "OwnerCode": "ZBPF7",
            "CorrelationID": "corr-0401",
            "IdempotencyKey": "SNP-20260805-0001:1",
        }
        self.assertEqual(
            payload_digest("x_wmsinter_InventoryPushHeader", header, [item]),
            "fc88721d689578a0285c68b44329f3f37e4d180e44dd5503b3a32c8d733159d3",
        )

        receive_item = {
            "SnapshotID": "RSNP-0001",
            "RowNo": 1,
            "DepotCode": "WH001",
            "GoodsCode": "P-1001",
            "BatchNo": "B20260701",
            "ValidDate": "2028-06-30",
            "GoodsStatus": "合格",
            "WMSAmount": "40",
            "WMSPickable": "38",
            "WMSAllocated": "2",
            "WMSFrozen": "0",
            "OwnerCode": "ZBPF7",
            "CorrelationID": "corr-0402",
            "IdempotencyKey": "RSNP-0001:1",
        }
        receive_header = {
            "SnapshotID": "RSNP-0001",
            "ReceiveTime": "2026-08-05T12:00:00.000Z",
            "TotalCount": 1,
            "OwnerCode": "ZBPF7",
            "SchemaVersion": "1",
            "IdempotencyKey": "RSNP-0001",
            "CorrelationID": "corr-0402",
            "SourceVersion": None,
        }
        self.assertEqual(
            payload_digest(
                "x_wmsinter_InventoryReceiveHeader", receive_header, [receive_item]
            ),
            "2b40eb3be1c68d58caf3cd52b19e49a398f6bed0c9f1dce337ce547917f99945",
        )

    def test_outbound_digest_covers_frozen_delivery_fields(self) -> None:
        row = {
            "ERPBillID": 1,
            "ERPBillCode": "CK-1",
            "Revision": 1,
            "OrderType": 1,
            "ClientID": 2,
            "DepotID": 3,
            "DepotCode": "WH001",
            "BusiDate": "2026-08-05",
            "RequiredShipAt": "2026-08-05T12:00:00.000Z",
            "ERPAddressID": 4,
            "AddressCode": "ADDR-1",
            "Address": "地址一",
            "LineCount": 0,
            "OwnerCode": "ZBPF7",
            "SchemaVersion": "1",
            "IdempotencyKey": "msg-out-1",
            "CorrelationID": "corr-out-1",
        }
        original = payload_digest("x_wmsinter_OutboundOrder", row, [])
        for field, changed in (
            ("RequiredShipAt", "2026-08-05T13:00:00.000Z"),
            ("ERPAddressID", 5),
            ("AddressCode", "ADDR-2"),
        ):
            with self.subTest(field=field):
                self.assertNotEqual(
                    payload_digest("x_wmsinter_OutboundOrder", row | {field: changed}, []),
                    original,
                )


class TestV19ClaimProtocol(unittest.TestCase):
    def test_goods_claim_uses_handelflag_lease_and_stable_order(self) -> None:
        with patch("worker_mssql.mssql_query", return_value=[]) as query:
            self.assertEqual(claim_rows(settings(), "x_wmsinter_GoodsInfo"), [])

        sql = query.call_args.args[1]
        self.assertIn("FROM dbo.x_wmsinter_GoodsInfo", sql)
        self.assertIn("handelflag = 0", sql)
        self.assertIn("handelflag = 3 AND next_retry_at <= SYSUTCDATETIME()", sql)
        self.assertIn("handelflag = 2 AND lease_until < SYSUTCDATETIME()", sql)
        self.assertIn("ORDER BY inserttime, seqid", sql)
        self.assertIn("SET handelflag = 2", sql)
        self.assertIn("UPDATE source", sql)
        self.assertNotIn("UPDATE claimable", sql)
        self.assertNotIn("sync_status", sql)

    def test_rejects_line_count_mismatch_before_digest(self) -> None:
        with self.assertRaisesRegex(ContractError, "LineCount=2, actual=0") as raised:
            validate_published_unit(
                "x_wmsinter_InboundOrder",
                {"LineCount": 2, "PayloadDigest": "0" * 64, "_items": []},
            )
        self.assertEqual(raised.exception.code, "LINE_COUNT_MISMATCH")

    def test_rejects_digest_mismatch(self) -> None:
        row = {
            "GoodsID": 1001,
            "opType": "D",
            "OwnerCode": "ZBPF7",
            "SchemaVersion": "1",
            "IdempotencyKey": "msg-0002",
            "CorrelationID": "corr-0001",
            "SourceVersion": 2,
            "PayloadDigest": "0" * 64,
        }
        with self.assertRaisesRegex(ContractError, "PayloadDigest mismatch") as raised:
            validate_published_unit("x_wmsinter_GoodsInfo", row)
        self.assertEqual(raised.exception.code, "INVALID_DATA")


if __name__ == "__main__":
    unittest.main()
