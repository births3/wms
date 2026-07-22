"""H8 worker 纯逻辑单测（无 Docker / 无 DB）。"""

from __future__ import annotations

import json
import unittest
from unittest.mock import patch

from channel_failover import (
    map_channel_mode_to_transport,
    production_allows_simultaneous_dual_write,
    publish_with_failover,
)
from circuit_breaker import CircuitBreaker
from outbound_publish import (
    OUTBOX_SOURCES,
    catalog_covers_outbox_sources,
    outbox_message_types,
    OutboxRow,
    insert_if_out_sql,
    sql_escape_mssql,
)
from sync_worker import (
    HANDLERS,
    Settings,
    WorkerHttpError,
    is_retryable_worker_error,
    resolve_inbound_route,
    validate_row_schema_version,
)
from worker_route import (
    get_worker_claim_decision,
    post_worker_heartbeat,
    sanitize_worker_error,
)
from worker_mssql import claim_rows


def settings() -> Settings:
    return Settings(
        mssql_host="localhost",
        mssql_port="1433",
        mssql_user="test",
        mssql_password="test",
        mssql_database="test",
        mssql_container="test",
        api_base="http://wms.test",
        api_token="token",
        poll_interval=1,
        max_retry=5,
        batch_size=1,
        use_sqlcmd=True,
        connector_id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        worker_id="worker-test",
        worker_version="test-1",
        heartbeat_ttl_seconds=15,
    )


class TestInboundCorePipeline(unittest.TestCase):
    def test_product_rejects_unmapped_storage_condition_before_business_api(
        self,
    ) -> None:
        handler = HANDLERS["product_master"][1]
        row = {
            "idempotency_key": "idem-product-1",
            "product_code": "P-1",
            "product_name": "药品一",
            "storage_condition": "ERP_UNKNOWN",
        }
        with patch(
            "sync_worker.http_json",
            return_value=(201, {"id": "product-1"}, ""),
        ) as business_api:
            with self.assertRaises(WorkerHttpError) as caught:
                handler(settings(), row)
        self.assertEqual(caught.exception.status, 422)
        self.assertFalse(is_retryable_worker_error(caught.exception))
        business_api.assert_not_called()

    def test_each_inbound_business_api_uses_shared_error_classification(self) -> None:
        row = {
            "idempotency_key": "idem-1",
            "external_doc_no": "ERP-1",
            "external_ref": "ERP-1",
            "receipt_no": "R-1",
            "document_type": "purchase_inbound",
            "supplier_id": "supplier-1",
            "customer_id": "customer-1",
            "warehouse_id": "warehouse-1",
            "expected_arrival_at": "2026-07-23T00:00:00Z",
            "product_id": "product-1",
            "product_code": "P-1",
            "product_name": "药品一",
            "expected_qty": "1",
            "planned_qty": "1",
            "batch_no": "B-1",
            "required_ship_at": "2026-07-23T00:00:00Z",
            "field_name": "spec",
            "new_value": "10mg",
        }
        for status, expected_retryable in ((503, True), (422, False)):
            for message_type, (_table, handler) in HANDLERS.items():
                with self.subTest(message_type=message_type, status=status):
                    with patch(
                        "sync_worker.http_json",
                        return_value=(status, None, '{"token":"response-secret"}'),
                    ):
                        with self.assertRaises(WorkerHttpError) as caught:
                            handler(settings(), row)
                    self.assertEqual(
                        is_retryable_worker_error(caught.exception),
                        expected_retryable,
                    )
                    self.assertNotIn("response-secret", str(caught.exception))

    def test_worker_error_summary_redacts_credentials(self) -> None:
        summary = sanitize_worker_error(
            'Authorization: Bearer top-secret password="db-secret" token=api-secret',
            secrets=("top-secret", "db-secret", "api-secret"),
        )
        self.assertNotIn("top-secret", summary)
        self.assertNotIn("db-secret", summary)
        self.assertNotIn("api-secret", summary)
        self.assertIn("***", summary)

        unknown = sanitize_worker_error(
            '{"token":"response-secret","password":"response-password"}'
        )
        self.assertNotIn("response-secret", unknown)
        self.assertNotIn("response-password", unknown)

    def test_route_binding_is_resolved_before_business_api(self) -> None:
        calls: list[str] = []

        def fake_http(_settings, method, path, body, idem):
            calls.append(path)
            self.assertEqual(method, "GET")
            self.assertIsNone(body)
            self.assertEqual(idem, "route-idem-1")
            return (
                200,
                {
                    "connector": {
                        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                        "config_version": 3,
                        "channel_mode": "rest_primary_table_fallback",
                        "connector_code": "SELF-ERP",
                    }
                },
                "",
            )

        binding = resolve_inbound_route(
            settings(),
            "asn",
            {
                "warehouse_id": "11111111-1111-1111-1111-111111111111",
                "idempotency_key": "idem-1",
            },
            http_json_fn=fake_http,
        )
        self.assertIn("direction=inbound", calls[0])
        self.assertIn("message_type=asn", calls[0])
        self.assertEqual(binding.connector_code, "SELF-ERP")
        self.assertEqual(binding.config_version, 3)
        self.assertEqual(binding.channel, "interface_table")

    def test_only_transient_http_errors_are_retryable(self) -> None:
        self.assertTrue(is_retryable_worker_error(WorkerHttpError(0, "route", "down")))
        self.assertTrue(
            is_retryable_worker_error(WorkerHttpError(503, "route", "down"))
        )
        self.assertFalse(
            is_retryable_worker_error(WorkerHttpError(422, "schema", "bad"))
        )
        self.assertFalse(is_retryable_worker_error(ValueError("bad mapping")))

    def test_schema_version_is_strict(self) -> None:
        self.assertEqual(validate_row_schema_version({"schema_version": "1"}), "1")
        with self.assertRaises(WorkerHttpError):
            validate_row_schema_version({"schema_version": "2"})

    def test_non_retryable_route_error_enters_dead_immediately(self) -> None:
        import sync_worker

        row = {
            "id": "row-1",
            "external_doc_no": "ASN-1",
            "idempotency_key": "idem-1",
            "schema_version": "1",
            "retry_count": "0",
        }
        with (
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "record_preflight_failure") as preflight_audit,
            patch.object(sync_worker, "claim_rows", return_value=[row]),
            patch.object(
                sync_worker,
                "resolve_inbound_route",
                side_effect=WorkerHttpError(422, "schema", "unsupported"),
            ),
            patch.object(sync_worker, "mark_row") as mark,
        ):
            self.assertEqual(sync_worker.process_once(settings(), ["asn"], False), 1)
        self.assertEqual(mark.call_args.args[3], "dead")
        self.assertEqual(mark.call_args.kwargs["retry_count"], 1)
        preflight_audit.assert_called_once()

    def test_preflight_audit_failure_releases_claim_for_retry(self) -> None:
        import sync_worker

        row = {
            "id": "row-1",
            "external_doc_no": "ASN-1",
            "idempotency_key": "idem-1",
            "schema_version": "999",
            "retry_count": "0",
        }
        with (
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(
                sync_worker,
                "record_preflight_failure",
                side_effect=WorkerHttpError(503, "audit", "unavailable"),
            ),
            patch.object(sync_worker, "claim_rows", return_value=[row]),
            patch.object(sync_worker, "mark_row") as mark,
        ):
            self.assertEqual(sync_worker.process_once(settings(), ["asn"], False), 1)
        self.assertEqual(mark.call_args.args[3], "pending")
        self.assertEqual(mark.call_args.kwargs["retry_count"], 1)

    def test_paused_connector_does_not_claim_mssql_rows(self) -> None:
        import sync_worker

        with (
            patch.object(sync_worker, "get_worker_claim_decision", return_value=False),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "claim_rows") as claim,
        ):
            self.assertEqual(sync_worker.process_once(settings(), ["asn"], False), 0)
        claim.assert_not_called()

    def test_claim_rows_parses_interface_table_output(self) -> None:
        raw = "|".join(
            [
                "row-1",
                "ASN-1",
                "owner-1",
                "warehouse-1",
                "supplier-1",
                "P-1",
                "2",
                "2026-07-22T10:00:00",
                "purchase_inbound",
                "ERP-1",
                "R-1",
                "1",
                "idem-1",
                "0",
            ]
        )
        with patch("worker_mssql.sqlcmd_query", return_value=raw):
            rows = claim_rows(settings(), "if_in_asn")
        self.assertEqual(rows[0]["external_doc_no"], "ASN-1")
        self.assertEqual(rows[0]["schema_version"], "1")

    def test_worker_runtime_calls_include_binding_and_claim_count(self) -> None:
        calls: list[tuple[str, str, dict | None]] = []

        def fake_http(_settings, method, path, body, _idem):
            calls.append((method, path, body))
            if method == "GET":
                return 200, {"allowed": False, "reason": "维护"}, ""
            return 200, {"health": "healthy"}, ""

        self.assertFalse(
            get_worker_claim_decision(
                settings(),
                "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "inbound",
                http_json_fn=fake_http,
            )
        )
        post_worker_heartbeat(
            settings(), ["inbound", "outbound"], 2, http_json_fn=fake_http
        )
        self.assertIn("connector_id=aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", calls[0][1])
        self.assertEqual(calls[1][2]["worker_id"], "worker-test")
        self.assertEqual(calls[1][2]["current_claims"], 2)
        self.assertEqual(calls[1][2]["directions"], ["inbound", "outbound"])


class TestInsertIfOutSql(unittest.TestCase):
    def test_idempotent_insert_contains_source_unique(self) -> None:
        row = OutboxRow(
            table="receiving_putaway_erp_feedback_outbox",
            id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            owner_id="11111111-1111-1111-1111-111111111111",
            event_type="inbound_putaway_completed",
            payload={"qty": 1, "note": "it's ok"},
            external_ref="rcv-1",
            attempt_count=1,
            max_attempts=5,
            deadline_at=None,
            callback_path="/inbound-complete",
        )
        sql = insert_if_out_sql(row)
        self.assertIn("if_out_message", sql)
        self.assertIn("source_outbox_table", sql)
        self.assertIn("inbound_putaway_completed", sql)
        self.assertIn("it''s ok", sql)  # MSSQL escape
        self.assertIn("out:receiving_putaway_erp_feedback_outbox:", sql)

    def test_escape_mssql(self) -> None:
        self.assertEqual(sql_escape_mssql("a'b"), "a''b")

    def test_outbox_sources_include_archive_and_recon(self) -> None:
        tables = {s["table"] for s in OUTBOX_SOURCES}
        self.assertIn("archive_revision_erp_feedback_outbox", tables)
        self.assertIn("reconciliation_erp_feedback_outbox", tables)
        self.assertIn("shipment_confirm_erp_feedback_outbox", tables)


class TestPayloadJson(unittest.TestCase):
    def test_roundtrip_payload(self) -> None:
        payload = {"product_code": "P1", "qty": 10, "note": "a|b"}
        raw = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
        self.assertEqual(json.loads(raw)["note"], "a|b")
        self.assertNotIn("\n", raw)

    def test_insert_sql_escapes_pipe_payload(self) -> None:
        row = OutboxRow(
            table="inventory_status_erp_feedback_outbox",
            id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            owner_id="11111111-1111-1111-1111-111111111111",
            event_type="inventory_status_changed",
            payload={"reason": "a|b", "qty": 1},
            external_ref="x",
            attempt_count=1,
            max_attempts=5,
            deadline_at=None,
            callback_path="/inventory-status",
        )
        sql = insert_if_out_sql(row)
        self.assertIn("a|b", sql)
        self.assertNotIn(
            "\n  INSERT", sql.split("IF NOT EXISTS")[0]
        )  # payload line compact


class TestChannelFailover(unittest.TestCase):
    def test_map_modes(self) -> None:
        self.assertEqual(map_channel_mode_to_transport("rest"), "http")
        self.assertEqual(map_channel_mode_to_transport("interface_table"), "table")
        self.assertEqual(
            map_channel_mode_to_transport("rest_primary_table_fallback"),
            "failover",
        )
        self.assertFalse(
            production_allows_simultaneous_dual_write("rest_primary_table_fallback")
        )

    def test_failover_uses_table_after_http_fail(self) -> None:
        calls: list[str] = []

        def http() -> None:
            calls.append("http")
            raise RuntimeError("erp down")

        def table() -> None:
            calls.append("table")

        result = publish_with_failover(
            transport="failover",
            publish_http=http,
            publish_table=table,
            http_max_attempts=2,
        )
        self.assertEqual(result.channel, "table_fallback")
        self.assertTrue(result.fallback_used)
        self.assertEqual(result.attempts_http, 2)
        self.assertEqual(calls, ["http", "http", "table"])

    def test_failover_http_success_skips_table(self) -> None:
        calls: list[str] = []

        def http() -> None:
            calls.append("http")

        def table() -> None:
            calls.append("table")

        result = publish_with_failover(
            transport="failover",
            publish_http=http,
            publish_table=table,
            http_max_attempts=2,
        )
        self.assertEqual(result.channel, "http")
        self.assertFalse(result.fallback_used)
        self.assertEqual(calls, ["http"])

    def test_not_dual_write_on_failover_success_paths(self) -> None:
        """成功走 HTTP 时不得再写 table（非双写）。"""
        table_calls = 0

        def http() -> None:
            return None

        def table() -> None:
            nonlocal table_calls
            table_calls += 1

        publish_with_failover(
            transport="failover",
            publish_http=http,
            publish_table=table,
        )
        self.assertEqual(table_calls, 0)

    def test_circuit_open_skips_http_then_half_open_recovers(self) -> None:
        circuit = CircuitBreaker(failure_threshold=2, half_open_after_failures=2)
        http_calls = 0
        table_calls = 0

        def http_fail() -> None:
            nonlocal http_calls
            http_calls += 1
            raise RuntimeError("down")

        def table_ok() -> None:
            nonlocal table_calls
            table_calls += 1

        # 两次失败 → open
        publish_with_failover(
            transport="failover",
            publish_http=http_fail,
            publish_table=table_ok,
            http_max_attempts=1,
            circuit=circuit,
        )
        publish_with_failover(
            transport="failover",
            publish_http=http_fail,
            publish_table=table_ok,
            http_max_attempts=1,
            circuit=circuit,
        )
        self.assertEqual(circuit.state, "open")
        http_before_open = http_calls

        # open 首次调用：跳过 HTTP
        publish_with_failover(
            transport="failover",
            publish_http=http_fail,
            publish_table=table_ok,
            http_max_attempts=1,
            circuit=circuit,
        )
        self.assertEqual(http_calls, http_before_open)
        self.assertEqual(circuit.state, "open")

        # 第二次 open 窗口：进入 half_open 并允许一次 HTTP 探测
        def http_ok() -> None:
            nonlocal http_calls
            http_calls += 1

        result = publish_with_failover(
            transport="failover",
            publish_http=http_ok,
            publish_table=table_ok,
            http_max_attempts=1,
            circuit=circuit,
        )
        self.assertEqual(result.channel, "http")
        self.assertEqual(circuit.state, "closed")
        self.assertGreater(http_calls, http_before_open)


class TestOutboundCatalog(unittest.TestCase):
    def test_outbox_message_types_match_h8_002_catalog(self) -> None:
        self.assertTrue(catalog_covers_outbox_sources())
        self.assertEqual(len(outbox_message_types()), 7)
        self.assertEqual(len(OUTBOX_SOURCES), 7)


if __name__ == "__main__":
    unittest.main()
