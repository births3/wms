"""H8 入站终态必须先与 WMS 消息状态对齐。"""

from __future__ import annotations

import unittest
from unittest.mock import patch

import sync_worker
from worker_route import WorkerHttpError, mark_inbound_message_dead


def settings() -> sync_worker.Settings:
    return sync_worker.Settings(
        mssql_host="localhost",
        mssql_port="1433",
        mssql_user="test",
        mssql_password="test",
        mssql_database="test",
        api_base="http://wms.test",
        api_token="token",
        poll_interval=1,
        max_retry=1,
        batch_size=1,
        connector_id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        connector_config_version=1,
        worker_id="worker-test",
        worker_version="test-1",
        heartbeat_ttl_seconds=15,
    )


ROW = {
    "id": "row-1",
    "external_doc_no": "ASN-1",
    "idempotency_key": "idem-1",
    "schema_version": "1",
    "retry_count": "0",
}


class TestInboundTerminalState(unittest.TestCase):
    def test_already_dead_message_does_not_repeat_dead_request(self) -> None:
        calls: list[str] = []

        def fake_http(_settings, method, path, _body, _idem):
            calls.append(method)
            return (
                200,
                {
                    "data": [
                        {
                            "id": "message-1",
                            "direction": "inbound",
                            "message_type": "asn",
                            "schema_version": "1",
                            "channel": "interface_table",
                            "sync_status": "dead",
                        }
                    ]
                },
                "",
            )

        mark_inbound_message_dead(
            settings(), "asn", ROW, "same failure", http_json_fn=fake_http
        )

        self.assertEqual(calls, ["GET"])

    def test_dead_api_uses_exact_existing_message(self) -> None:
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
                                "direction": "inbound",
                                "message_type": "asn",
                                "schema_version": "1",
                                "channel": "interface_table",
                            }
                        ]
                    },
                    "",
                )
            return 200, {"id": "message-1", "sync_status": "dead"}, ""

        mark_inbound_message_dead(
            settings(),
            "asn",
            ROW,
            "supplier missing",
            http_json_fn=fake_http,
        )

        self.assertIn("idempotency_key=idem-1", calls[0][1])
        self.assertEqual(
            calls[1],
            (
                "POST",
                "/api/v1/integration/erp-messages/message-1/dead",
                {"error_summary": "supplier missing"},
            ),
        )

    def test_marks_h8_dead_before_mssql_dead(self) -> None:
        order: list[str] = []
        with (
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "list_manual_replays", return_value=[]),
            patch.object(sync_worker, "record_preflight_failure"),
            patch.object(sync_worker, "claim_rows", return_value=[ROW]),
            patch.object(
                sync_worker,
                "resolve_inbound_route",
                side_effect=WorkerHttpError(422, "schema", "unsupported"),
            ),
            patch.object(
                sync_worker,
                "mark_terminal_inbound_message",
                side_effect=lambda *_args: order.append("h8"),
            ),
            patch.object(
                sync_worker,
                "mark_row",
                side_effect=lambda *_args, **_kwargs: order.append("mssql"),
            ),
        ):
            self.assertEqual(sync_worker.process_once(settings(), ["asn"], False), 1)
        self.assertEqual(order, ["h8", "mssql"])

    def test_h8_dead_failure_releases_mssql_for_retry(self) -> None:
        with (
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "list_manual_replays", return_value=[]),
            patch.object(sync_worker, "record_preflight_failure"),
            patch.object(sync_worker, "claim_rows", return_value=[ROW]),
            patch.object(
                sync_worker,
                "resolve_inbound_route",
                side_effect=WorkerHttpError(422, "schema", "unsupported"),
            ),
            patch.object(
                sync_worker,
                "mark_terminal_inbound_message",
                side_effect=WorkerHttpError(503, "mark dead", "unavailable"),
            ),
            patch.object(sync_worker, "mark_row") as mark_row,
        ):
            self.assertEqual(sync_worker.process_once(settings(), ["asn"], False), 1)
        self.assertEqual(mark_row.call_args.args[3], "pending")


if __name__ == "__main__":
    unittest.main()
