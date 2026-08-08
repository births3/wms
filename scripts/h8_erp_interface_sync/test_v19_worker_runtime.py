"""v1.9 Worker 主表路由与发布单元预检。"""

from __future__ import annotations

import json
import unittest
import urllib.parse
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from unittest.mock import Mock, patch

import sync_worker
from test_h8_sync_worker import settings, v19_goods_row
from v19_contract import payload_digest
from worker_route import RouteBinding, WorkerHttpError


class TestV19WorkerRuntime(unittest.TestCase):
    def test_source_version_from_mssql_is_json_serializable(self) -> None:
        row = v19_goods_row()
        row["SourceVersion"] = Decimal("1")

        command = sync_worker.build_v19_inbound_canonical(
            "product_master",
            sync_worker._runtime_row("x_wmsinter_GoodsInfo", row),
            None,
        )

        self.assertEqual(command.fields["request_body"]["source_version"], 1)
        json.dumps(command.fields["request_body"])

    def test_handlers_only_route_v19_inbound_main_tables(self) -> None:
        self.assertEqual(
            {name: table for name, (table, _handler) in sync_worker.HANDLERS.items()},
            {
                "product_master": "x_wmsinter_GoodsInfo",
                "customer_master": "x_wmsinter_CustomerInfo",
                "supplier_master": "x_wmsinter_SupplierInfo",
                "asn": "x_wmsinter_InboundOrder",
                "outbound_order": "x_wmsinter_OutboundOrder",
                "order_cancel": "x_wmsinter_OrderCommand",
                "inventory_seed_snapshot": "x_wmsinter_InventoryPushHeader",
            },
        )

    def test_route_resolves_v19_warehouse_code(self) -> None:
        paths: list[str] = []

        def fake_http(_settings, _method, path, _body, _key):
            paths.append(path)
            return (
                200,
                {
                    "connector": {
                        "id": settings().connector_id,
                        "connector_code": "ERP",
                        "config_version": 1,
                        "channel_mode": "interface_table",
                    }
                },
                "",
            )

        sync_worker.resolve_inbound_route(
            settings(),
            "asn",
            {"idempotency_key": "msg-1", "warehouse_code": "WH001"},
            http_json_fn=fake_http,
        )

        self.assertEqual(
            urllib.parse.parse_qs(urllib.parse.urlsplit(paths[0]).query)["warehouse_code"],
            ["WH001"],
        )

    def test_process_uses_table_primary_key_and_marks_sync_success(self) -> None:
        row = {
            "seqid": 7,
            "GoodsID": 1001,
            "opType": "D",
            "OwnerCode": "ZBPF7",
            "SchemaVersion": "1",
            "IdempotencyKey": "msg-0002",
            "CorrelationID": "corr-0001",
            "SourceVersion": 2,
            "retry_count": 0,
            "inserttime": "2026-08-05T00:00:00.000Z",
        }
        row["PayloadDigest"] = payload_digest("x_wmsinter_GoodsInfo", row)
        binding = RouteBinding(
            connector_id=settings().connector_id,
            connector_code="ERP",
            config_version=1,
            channel="interface_table",
            message_type="product_master",
        )
        handler = Mock(return_value="product-1")

        with (
            patch.dict(
                sync_worker.HANDLERS,
                {"product_master": ("x_wmsinter_GoodsInfo", handler)},
                clear=True,
            ),
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "list_manual_replays", return_value=[]),
            patch.object(sync_worker, "claim_rows", return_value=[row]),
            patch.object(sync_worker, "resolve_inbound_route", return_value=binding),
            patch.object(sync_worker, "mark_row") as mark,
        ):
            self.assertEqual(
                sync_worker.process_once(settings(), ["product_master"], False), 1
            )

        mark.assert_called_once_with(
            settings(), "x_wmsinter_GoodsInfo", 7, "success"
        )

    def test_invalid_digest_is_dead_before_route_or_business_call(self) -> None:
        row = {
            "seqid": 8,
            "GoodsID": 1001,
            "opType": "D",
            "OwnerCode": "ZBPF7",
            "SchemaVersion": "1",
            "IdempotencyKey": "msg-bad",
            "CorrelationID": "corr-0001",
            "SourceVersion": 3,
            "PayloadDigest": "0" * 64,
            "retry_count": 0,
            "inserttime": "2026-08-05T00:00:00.000Z",
        }
        handler = Mock(return_value="must-not-run")

        with (
            patch.dict(
                sync_worker.HANDLERS,
                {"product_master": ("x_wmsinter_GoodsInfo", handler)},
                clear=True,
            ),
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "list_manual_replays", return_value=[]),
            patch.object(sync_worker, "claim_rows", return_value=[row]),
            patch.object(sync_worker, "resolve_inbound_route") as route,
            patch.object(sync_worker, "mark_row") as mark,
        ):
            self.assertEqual(
                sync_worker.process_once(settings(), ["product_master"], False), 1
            )

        route.assert_not_called()
        handler.assert_not_called()
        self.assertEqual(mark.call_args.args[3], "dead")
        self.assertEqual(mark.call_args.kwargs["error_code"], "INVALID_DATA")

    def test_order_not_ready_uses_independent_30_minute_timeout(self) -> None:
        now = datetime.now(timezone.utc)
        rows = []
        for command_id, inserted in (
            ("cmd-wait", now - timedelta(minutes=29)),
            ("cmd-timeout", now - timedelta(minutes=31)),
        ):
            row = {
                "CommandID": command_id,
                "CommandType": 99,
                "ERPBillCode": "ERP-NOT-READY",
                "Revision": 1,
                "OrderType": 2,
                "Memo": None,
                "OwnerCode": "ZBPF7",
                "SchemaVersion": "1",
                "IdempotencyKey": command_id,
                "CorrelationID": f"corr-{command_id}",
                "SourceVersion": None,
                "retry_count": 0,
                "inserttime": inserted,
            }
            row["PayloadDigest"] = payload_digest("x_wmsinter_OrderCommand", row)
            rows.append(row)
        binding = RouteBinding(
            connector_id=settings().connector_id,
            connector_code="ERP",
            config_version=1,
            channel="interface_table",
            message_type="order_cancel",
        )
        handler = Mock(
            side_effect=WorkerHttpError(
                425, "H8 order_cancel API", "not ready", "ORDER_NOT_READY"
            )
        )

        with (
            patch.dict(
                sync_worker.HANDLERS,
                {"order_cancel": ("x_wmsinter_OrderCommand", handler)},
                clear=True,
            ),
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "list_manual_replays", return_value=[]),
            patch.object(sync_worker, "claim_rows", return_value=rows),
            patch.object(sync_worker, "resolve_inbound_route", return_value=binding),
            patch.object(sync_worker, "mark_row") as mark,
        ):
            self.assertEqual(
                sync_worker.process_once(settings(), ["order_cancel"], False), 2
            )

        self.assertEqual([call.args[3] for call in mark.call_args_list], ["retry", "dead"])
        self.assertTrue(all(call.kwargs["retry_count"] == 0 for call in mark.call_args_list))
        self.assertTrue(
            all(call.kwargs["error_code"] == "ORDER_NOT_READY" for call in mark.call_args_list)
        )


if __name__ == "__main__":
    unittest.main()
