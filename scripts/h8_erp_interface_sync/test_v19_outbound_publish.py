"""v1.9 WMS→ERP 直写接口表回归。"""

from __future__ import annotations

import re
import unittest

from outbound_publish import OutboxRow, insert_if_out_sql


def row(event_type: str, payload: dict) -> OutboxRow:
    return OutboxRow(
        table="source_outbox",
        id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        owner_id="11111111-1111-1111-1111-111111111111",
        event_type=event_type,
        payload=payload,
        external_ref="ERP-1",
        attempt_count=1,
        max_attempts=5,
        deadline_at=None,
        callback_path="/unused",
    )


class TestV19OutboundPublish(unittest.TestCase):
    def test_inbound_putaway_writes_detail_table(self) -> None:
        sql = insert_if_out_sql(
            row(
                "inbound_putaway_completed",
                {
                    "erp_bill_code": "RK-1",
                    "revision": 1,
                    "line_no": 1,
                    "goods_id": 1001,
                    "product_code": "P1",
                    "expected_amount": "10.0000",
                    "actual_amount": "10.0000",
                    "reject_amount": "0.0000",
                    "shortage_amount": "0.0000",
                    "batch_no": "B1",
                    "production_date": "2026-01-01",
                    "expiry_date": "2028-01-01",
                    "location_code": "A-01",
                    "operator_name": "张三",
                    "scan_time": "2026-08-05T12:00:00.000Z",
                    "correlation_id": "corr-1",
                },
            ),
            owner_code="ZBPF7",
        )
        self.assertIn("x_wmsinter_InboundFeedback", sql)
        self.assertIn("[LineNo]", sql)
        self.assertNotIn("if_out_message", sql)

    def test_shipment_writes_details_before_completion_barrier(self) -> None:
        sql = insert_if_out_sql(
            row(
                "shipment_confirm",
                {
                    "erp_bill_code": "CK-1",
                    "revision": 1,
                    "order_type": 2,
                    "line_count": 2,
                    "waybill_no": "SF1",
                    "express_company": "顺丰",
                    "ship_time": "2026-08-05T12:00:00.000Z",
                    "operator_name": "李四",
                    "correlation_id": "corr-2",
                    "lines": [
                        {
                            "line_no": 1,
                            "goods_id": 1001,
                            "product_code": "P1",
                            "batch_no": "B1",
                            "expected_amount": "5.0000",
                            "picked_amount": "5.0000",
                            "shipped_amount": "5.0000",
                        },
                        {
                            "line_no": 2,
                            "goods_id": 1002,
                            "product_code": "P2",
                            "batch_no": "B2",
                            "expected_amount": "3.0000",
                            "picked_amount": "3.0000",
                            "shipped_amount": "3.0000",
                        },
                    ],
                },
            ),
            owner_code="ZBPF7",
        )
        detail = sql.index("x_wmsinter_OutboundFeedback")
        barrier = sql.index("x_wmsinter_OrderFeedback")
        self.assertLess(detail, barrier)
        self.assertIn("FeedbackType", sql[barrier:])
        self.assertIn("BEGIN TRANSACTION", sql)
        self.assertIn("COMMIT TRANSACTION", sql)
        declared = re.findall(r"DECLARE (@\w+)", sql)
        self.assertEqual(len(declared), len(set(declared)))

    def test_inventory_snapshot_writes_header_and_items(self) -> None:
        sql = insert_if_out_sql(
            row(
                "inventory_snapshot",
                {
                    "snapshot_id": "RSNP-1",
                    "receive_time": "2026-08-05T12:00:00.000Z",
                    "correlation_id": "corr-3",
                    "lines": [
                        {
                            "row_no": 1,
                            "depot_code": "WH001",
                            "product_code": "P1",
                            "batch_no": "B1",
                            "valid_date": "2028-01-01",
                            "goods_status": "合格",
                            "wms_amount": "10.0000",
                            "wms_pickable": "8.0000",
                            "wms_allocated": "2.0000",
                            "wms_frozen": "0.0000",
                        }
                    ],
                },
            ),
            owner_code="ZBPF7",
        )
        self.assertIn("x_wmsinter_InventoryReceiveHeader", sql)
        self.assertIn("x_wmsinter_InventoryReceiveItems", sql)
        self.assertIn("[RowNo]", sql)
        self.assertIn("RSNP-1:1", sql)
        self.assertIn("BEGIN TRANSACTION", sql)
        self.assertIn("ROLLBACK TRANSACTION", sql)


if __name__ == "__main__":
    unittest.main()
