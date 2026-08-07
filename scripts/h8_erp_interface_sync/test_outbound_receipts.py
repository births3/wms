"""接口表业务回执 → H8 acked 的最小回归。"""

from __future__ import annotations

import unittest
from types import SimpleNamespace
from unittest.mock import patch

import sync_worker
import outbound_publish
import worker_mssql
from exchange_lifecycle import run_outbound_pipeline


def settings() -> SimpleNamespace:
    return SimpleNamespace(
        batch_size=10,
        connector_id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        api_token="token",
        mssql_password="secret",
    )


class TestOutboundReceipts(unittest.TestCase):
    def test_lists_only_acked_interface_rows(self) -> None:
        key = "11111111-1111-1111-1111-111111111111"
        with patch.object(
            worker_mssql,
            "mssql_query",
            side_effect=[[{"row_id": 7, "IdempotencyKey": key}], [], [], [], []],
        ) as query:
            rows = worker_mssql.list_acked_outbound(settings(), [key])

        self.assertEqual(rows[0]["id"], "x_wmsinter_OrderFeedback:7")
        self.assertEqual(rows[0]["idempotency_key"], key)
        self.assertIn("handelflag = 5", query.call_args_list[0].args[1])
        self.assertNotIn("FOR JSON PATH", query.call_args_list[0].args[1])

    def test_records_business_receipt_then_marks_interface_row_consumed(self) -> None:
        row = {
            "id": "11111111-1111-1111-1111-111111111111",
            "idempotency_key": "out:inventory_snapshot_erp_feedback_outbox:source-1",
            "erp_ack_ref": "erp-ack-1",
        }
        calls: list[tuple[str, str, dict | None]] = []

        def fake_http(_settings, method, path, body, _idem):
            calls.append((method, path, body))
            if method == "GET":
                return (
                    200,
                    {
                        "data": [
                            {
                                "id": "message-1",
                                "connector_id": settings().connector_id,
                                "connector_code": "SELF-ERP",
                                "config_version": 2,
                                "direction": "outbound",
                                "message_type": "inventory_snapshot",
                                "schema_version": "1",
                                "external_ref": "COUNT-1",
                                "idempotency_key": row["idempotency_key"],
                                "correlation_id": "corr-1",
                                "channel": "interface_table",
                                "sync_status": "awaiting_receipt",
                                "warehouse_id": None,
                            }
                        ]
                    },
                    "",
                )
            return 200, {"id": "message-1", "sync_status": "acked"}, ""

        with (
            patch.object(outbound_publish, "list_acked_outbound", return_value=[row]) as listed,
            patch.object(outbound_publish, "mark_outbound_receipt_recorded") as mark,
        ):
            processed = sync_worker.process_outbound_receipts(
                settings(),
                http_json_fn=fake_http,
            )

        self.assertEqual(processed, 1)
        self.assertEqual(calls[1][2]["stage"], "receipt")
        self.assertEqual(calls[1][2]["result"], "ok")
        self.assertEqual(calls[1][2]["message_id"], "message-1")
        listed.assert_called_once_with(settings(), [row["idempotency_key"]])
        mark.assert_called_once_with(settings(), row["id"])

    def test_table_receipt_follows_next_cursor_until_acked_row_is_found(self) -> None:
        acked_row = {
            "id": "x_wmsinter_OrderFeedback:7",
            "idempotency_key": (
                "out:inventory_snapshot_erp_feedback_outbox:"
                "33333333-3333-3333-3333-333333333333"
            ),
        }
        get_paths: list[str] = []

        def fake_http(_settings, method, path, body, _idem):
            if method == "GET":
                get_paths.append(path)
                if "cursor=" not in path:
                    return (
                        200,
                        {
                            "data": [
                                {
                                    "id": "message-page-1",
                                    "connector_id": settings().connector_id,
                                    "connector_code": "SELF-ERP",
                                    "config_version": 2,
                                    "direction": "outbound",
                                    "message_type": "inventory_snapshot",
                                    "schema_version": "1",
                                    "external_ref": "COUNT-1",
                                    "idempotency_key": (
                                        "out:inventory_snapshot_erp_feedback_outbox:"
                                        "11111111-1111-1111-1111-111111111111"
                                    ),
                                    "correlation_id": "corr-1",
                                    "channel": "interface_table",
                                    "sync_status": "awaiting_receipt",
                                    "warehouse_id": None,
                                }
                            ],
                            "page": {"next_cursor": "page-2"},
                        },
                        "",
                    )
                return (
                    200,
                    {
                        "data": [
                            {
                                "id": "message-page-2",
                                "connector_id": settings().connector_id,
                                "connector_code": "SELF-ERP",
                                "config_version": 2,
                                "direction": "outbound",
                                "message_type": "inventory_snapshot",
                                "schema_version": "1",
                                "external_ref": "COUNT-2",
                                "idempotency_key": acked_row["idempotency_key"],
                                "correlation_id": "corr-2",
                                "channel": "interface_table",
                                "sync_status": "awaiting_receipt",
                                "warehouse_id": None,
                            }
                        ],
                        "page": {"next_cursor": None},
                    },
                    "",
                )
            self.assertEqual(body["message_id"], "message-page-2")
            return 200, {"id": "message-page-2", "sync_status": "acked"}, ""

        with (
            patch.object(
                outbound_publish, "list_acked_outbound", return_value=[acked_row]
            ) as listed,
            patch.object(outbound_publish, "mark_outbound_receipt_recorded") as mark,
        ):
            processed = sync_worker.process_outbound_receipts(
                settings(),
                http_json_fn=fake_http,
            )

        self.assertEqual(processed, 1)
        self.assertEqual(len(get_paths), 2)
        self.assertIn("cursor=page-2", get_paths[1])
        listed.assert_called_once_with(
            settings(),
            [
                "out:inventory_snapshot_erp_feedback_outbox:"
                "11111111-1111-1111-1111-111111111111",
                "out:inventory_snapshot_erp_feedback_outbox:"
                "33333333-3333-3333-3333-333333333333",
            ],
        )
        mark.assert_called_once_with(settings(), acked_row["id"])

    def test_due_receipt_timeout_requeues_original_outbox_until_server_returns_dead(
        self,
    ) -> None:
        messages = [
            {
                "id": "message-retry",
                "connector_id": settings().connector_id,
                "connector_code": "SELF-ERP",
                "config_version": 2,
                "direction": "outbound",
                "message_type": "inventory_snapshot",
                "schema_version": "1",
                "external_ref": "COUNT-1",
                "idempotency_key": "out:inventory_snapshot_erp_feedback_outbox:11111111-1111-1111-1111-111111111111",
                "correlation_id": "corr-1",
                "channel": "rest",
                "sync_status": "awaiting_receipt",
                "retry_count": 0,
                "next_retry_at": "2020-01-01T00:00:00Z",
                "warehouse_id": None,
            },
            {
                "id": "message-dead",
                "connector_id": settings().connector_id,
                "connector_code": "SELF-ERP",
                "config_version": 2,
                "direction": "outbound",
                "message_type": "inventory_snapshot",
                "schema_version": "1",
                "external_ref": "COUNT-2",
                "idempotency_key": "out:inventory_snapshot_erp_feedback_outbox:22222222-2222-2222-2222-222222222222",
                "correlation_id": "corr-2",
                "channel": "rest",
                "sync_status": "awaiting_receipt",
                "retry_count": 4,
                "next_retry_at": "2020-01-01T00:00:00Z",
                "warehouse_id": None,
            },
            {
                "id": "message-not-due",
                "connector_id": settings().connector_id,
                "sync_status": "awaiting_receipt",
                "next_retry_at": "2999-01-01T00:00:00Z",
            },
        ]
        posts: list[dict] = []
        sequence: list[str] = []

        def fake_http(_settings, method, _path, body, _idem):
            if method == "GET":
                return 200, {"data": messages}, ""
            sequence.append("lifecycle")
            posts.append(body)
            return (
                200,
                {
                    "id": body["message_id"],
                    "sync_status": (
                        "dead" if body["message_id"] == "message-dead" else "processing"
                    ),
                },
                "",
            )

        with (
            patch.object(
                outbound_publish,
                "requeue_wms_outbox",
                side_effect=lambda *_args: sequence.append("requeue"),
            ) as requeue,
        ):
            processed = sync_worker.process_outbound_receipt_timeouts(
                settings(),
                "postgres://wms",
                http_json_fn=fake_http,
            )

        self.assertEqual(processed, 2)
        self.assertEqual([post["stage"] for post in posts], ["final_failure"] * 2)
        self.assertTrue(
            all(post["result"] == "business receipt timeout" for post in posts)
        )
        requeue.assert_called_once_with(
            "postgres://wms",
            messages[0]["idempotency_key"],
        )
        self.assertEqual(sequence[:2], ["requeue", "lifecycle"])

    def test_receipt_timeout_follows_next_cursor_until_due_message_is_found(
        self,
    ) -> None:
        due_message = {
            "id": "message-due-on-page-2",
            "connector_id": settings().connector_id,
            "connector_code": "SELF-ERP",
            "config_version": 2,
            "direction": "outbound",
            "message_type": "inventory_snapshot",
            "schema_version": "1",
            "external_ref": "COUNT-DUE",
            "idempotency_key": (
                "out:inventory_snapshot_erp_feedback_outbox:"
                "33333333-3333-3333-3333-333333333333"
            ),
            "correlation_id": "corr-due",
            "channel": "rest",
            "sync_status": "awaiting_receipt",
            "retry_count": 0,
            "next_retry_at": "2020-01-01T00:00:00Z",
            "warehouse_id": None,
        }
        get_paths: list[str] = []

        def fake_http(_settings, method, path, body, _idem):
            if method == "GET":
                get_paths.append(path)
                if "cursor=" not in path:
                    return (
                        200,
                        {
                            "data": [
                                {
                                    "id": "message-newer-not-due",
                                    "next_retry_at": "2999-01-01T00:00:00Z",
                                }
                            ],
                            "page": {"next_cursor": "page-2"},
                        },
                        "",
                    )
                return 200, {"data": [due_message], "page": {"next_cursor": None}}, ""
            self.assertEqual(body["message_id"], due_message["id"])
            return 200, {"sync_status": "processing"}, ""

        with patch.object(outbound_publish, "requeue_wms_outbox") as requeue:
            processed = sync_worker.process_outbound_receipt_timeouts(
                settings(),
                "postgres://wms",
                http_json_fn=fake_http,
            )

        self.assertEqual(processed, 1)
        self.assertEqual(len(get_paths), 2)
        self.assertIn("cursor=page-2", get_paths[1])
        requeue.assert_called_once_with(
            "postgres://wms",
            due_message["idempotency_key"],
        )

    def test_receipt_timeout_lifecycle_failure_keeps_durable_outbox_retry(self) -> None:
        message = {
            "id": "message-retry",
            "connector_id": settings().connector_id,
            "connector_code": "SELF-ERP",
            "config_version": 2,
            "direction": "outbound",
            "message_type": "inventory_snapshot",
            "schema_version": "1",
            "external_ref": "COUNT-1",
            "idempotency_key": (
                "out:inventory_snapshot_erp_feedback_outbox:"
                "11111111-1111-1111-1111-111111111111"
            ),
            "correlation_id": "corr-1",
            "channel": "rest",
            "sync_status": "awaiting_receipt",
            "retry_count": 0,
            "next_retry_at": "2020-01-01T00:00:00Z",
            "warehouse_id": None,
        }

        def fake_http(_settings, method, _path, _body, _idem):
            if method == "GET":
                return 200, {"data": [message]}, ""
            return 503, None, "temporarily unavailable"

        with patch.object(outbound_publish, "requeue_wms_outbox") as requeue:
            processed = sync_worker.process_outbound_receipt_timeouts(
                settings(),
                "postgres://wms",
                http_json_fn=fake_http,
            )

        self.assertEqual(processed, 0)
        requeue.assert_called_once_with(
            "postgres://wms",
            message["idempotency_key"],
        )

    def test_timeout_recovery_reuses_requeued_outbox_then_returns_to_awaiting(self) -> None:
        message = {
            "id": "message-retry",
            "connector_id": settings().connector_id,
            "connector_code": "SELF-ERP",
            "config_version": 2,
            "direction": "outbound",
            "message_type": "inventory_snapshot",
            "schema_version": "1",
            "external_ref": "COUNT-1",
            "idempotency_key": (
                "out:inventory_snapshot_erp_feedback_outbox:"
                "11111111-1111-1111-1111-111111111111"
            ),
            "correlation_id": "corr-1",
            "channel": "rest",
            "sync_status": "awaiting_receipt",
            "retry_count": 0,
            "next_retry_at": "2020-01-01T00:00:00Z",
            "warehouse_id": None,
        }
        server = {"status": "awaiting_receipt", "timeout_posts": 0}
        outbox = {"status": "succeeded", "last_error": None}
        lifecycle_stages: list[str] = []

        def fake_http(_settings, method, _path, body, _idem):
            if method == "GET":
                return 200, {"data": [message]}, ""
            stage = body["stage"]
            lifecycle_stages.append(stage)
            if stage == "final_failure":
                server["timeout_posts"] += 1
                if server["timeout_posts"] == 1:
                    return 503, None, "temporarily unavailable"
                server["status"] = "processing"
            elif stage == "receive":
                self.assertEqual(server["status"], "processing")
            elif stage == "send" and body["result"] == "ok":
                server["status"] = "awaiting_receipt"
            return (
                200,
                {"id": message["id"], "sync_status": server["status"]},
                "",
            )

        def idempotent_requeue(_database_url, _idempotency_key):
            if outbox["status"] == "succeeded":
                outbox.update(
                    status="failed",
                    last_error="business receipt timeout",
                )
                return
            self.assertEqual(outbox["status"], "failed")
            self.assertEqual(outbox["last_error"], "business receipt timeout")

        with patch.object(
            outbound_publish,
            "requeue_wms_outbox",
            side_effect=idempotent_requeue,
        ) as requeue:
            first = sync_worker.process_outbound_receipt_timeouts(
                settings(),
                "postgres://wms",
                http_json_fn=fake_http,
            )
            second = sync_worker.process_outbound_receipt_timeouts(
                settings(),
                "postgres://wms",
                http_json_fn=fake_http,
            )

        self.assertEqual((first, second), (0, 1))
        self.assertEqual(requeue.call_count, 2)
        self.assertEqual(server["status"], "processing")

        life = run_outbound_pipeline(
            settings(),
            message["message_type"],
            message["external_ref"],
            message["idempotency_key"],
            lambda _life: None,
            http_json=fake_http,
            connector_id=message["connector_id"],
            payload={},
        )

        self.assertEqual(
            life.stages_emitted,
            [
                ("receive", "ok"),
                ("convert", "ok"),
                ("send", "started"),
                ("send", "ok"),
            ],
        )
        self.assertEqual(
            lifecycle_stages,
            [
                "final_failure",
                "final_failure",
                "receive",
                "convert",
                "send",
                "send",
            ],
        )
        self.assertEqual(server["status"], "awaiting_receipt")


if __name__ == "__main__":
    unittest.main()
