"""US-H8-002 AC4：出站必须按消息解析唯一连接，不能取全局首条配置。"""

from __future__ import annotations

import json
import unittest
from types import SimpleNamespace
from unittest.mock import patch

import outbound_publish
import sync_worker
import worker_route
from exchange_lifecycle import run_outbound_pipeline
from outbound_publish import (
    OUTBOX_SOURCES,
    OutboxRow,
    claim_wms_outbox,
    mark_wms_outbox,
    process_outbound_once,
    requeue_wms_outbox,
)
from worker_route import (
    RouteBinding,
    WorkerHttpError,
    resolve_bearer_token,
    resolve_outbound_route,
)


CONNECTOR_ID = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
OWNER_ID = "11111111-1111-1111-1111-111111111111"
WAREHOUSE_ID = "22222222-2222-2222-2222-222222222222"


def settings(config_version: int = 3) -> SimpleNamespace:
    return SimpleNamespace(
        connector_id=CONNECTOR_ID,
        connector_config_version=config_version,
        api_base="http://wms.test",
        api_token="token",
    )


def binding(channel_mode: str = "rest") -> RouteBinding:
    return RouteBinding(
        connector_id=CONNECTOR_ID,
        connector_code="SELF-ERP",
        config_version=3,
        channel="interface_table" if channel_mode == "interface_table" else "rest",
        message_type="putaway_complete",
        owner_id=OWNER_ID,
        api_base_url="https://erp.test/h8",
        channel_mode=channel_mode,
        bearer_secret_alias="vault://erp/test",
    )


class TestOutboundRoute(unittest.TestCase):
    def test_retry_loads_frozen_outbound_binding_instead_of_current_route(self) -> None:
        calls: list[str] = []

        def fake_http(_settings, method, path, body, _idem):
            calls.append(path)
            self.assertEqual(method, "GET")
            self.assertIsNone(body)
            if "/versions/2" in path:
                return (
                    200,
                    {
                        "id": CONNECTOR_ID,
                        "owner_id": OWNER_ID,
                        "connector_code": "SELF-ERP",
                        "config_version": 2,
                        "channel_mode": "rest",
                        "api_base_url": "https://erp-v2.test/h8",
                        "bearer_secret_alias": "vault://erp/v2",
                        "directions": ["outbound"],
                        "message_types": ["putaway_complete"],
                        "warehouse_ids": [WAREHOUSE_ID],
                    },
                    "",
                )
            return (
                200,
                {
                    "data": [
                        {
                            "connector_id": CONNECTOR_ID,
                            "connector_code": "SELF-ERP",
                            "config_version": 2,
                            "direction": "outbound",
                            "message_type": "putaway_complete",
                            "schema_version": "1",
                            "channel": "rest",
                            "warehouse_id": WAREHOUSE_ID,
                        }
                    ]
                },
                "",
            )

        route = worker_route.resolve_existing_outbound_binding(
            settings(2),
            "putaway_complete",
            OWNER_ID,
            WAREHOUSE_ID,
            "receiving-1",
            "out:receiving_putaway_erp_feedback_outbox:row-1",
            http_json_fn=fake_http,
        )

        self.assertIsNotNone(route)
        self.assertEqual(route.config_version, 2)
        self.assertEqual(route.api_base_url, "https://erp-v2.test/h8")
        self.assertEqual(route.bearer_secret_alias, "vault://erp/v2")
        self.assertIn("direction=outbound", calls[0])
        self.assertIn("idempotency_key=out%3Areceiving_putaway", calls[0])
        self.assertIn(f"/{CONNECTOR_ID}/versions/2", calls[1])

    def test_rest_outbound_rejects_missing_secret_alias_even_with_global_token(
        self,
    ) -> None:
        with (
            patch.dict(
                "os.environ",
                {"ERP_API_TOKEN": "must-not-be-used"},
                clear=True,
            ),
            self.assertRaises(WorkerHttpError) as caught,
        ):
            resolve_bearer_token(None)

        self.assertEqual(caught.exception.status, 503)
        self.assertNotIn("must-not-be-used", str(caught.exception))

    def test_resolves_by_direction_type_and_warehouse(self) -> None:
        def fake_http(_settings, method, path, body, _idem):
            self.assertEqual(method, "GET")
            self.assertIsNone(body)
            self.assertIn("direction=outbound", path)
            self.assertIn("message_type=putaway_complete", path)
            self.assertIn(f"warehouse_id={WAREHOUSE_ID}", path)
            return (
                200,
                {
                    "connector": {
                        "id": CONNECTOR_ID,
                        "owner_id": OWNER_ID,
                        "connector_code": "SELF-ERP",
                        "config_version": 3,
                        "channel_mode": "rest",
                        "api_base_url": "https://erp.test/h8",
                        "bearer_secret_alias": "vault://erp/test",
                        "directions": ["outbound"],
                        "message_types": ["putaway_complete"],
                        "warehouse_ids": [WAREHOUSE_ID],
                    }
                },
                "",
            )

        route = resolve_outbound_route(
            settings(),
            "putaway_complete",
            OWNER_ID,
            WAREHOUSE_ID,
            "out:row-1",
            http_json_fn=fake_http,
        )

        self.assertEqual(route, binding())

    def test_rejects_route_from_another_owner(self) -> None:
        def fake_http(*_args):
            return (
                200,
                {
                    "connector": {
                        "id": CONNECTOR_ID,
                        "owner_id": "99999999-9999-9999-9999-999999999999",
                        "connector_code": "SELF-ERP",
                        "config_version": 3,
                        "channel_mode": "interface_table",
                        "directions": ["outbound"],
                        "message_types": ["putaway_complete"],
                        "warehouse_ids": [],
                    }
                },
                "",
            )

        with self.assertRaises(WorkerHttpError) as caught:
            resolve_outbound_route(
                settings(),
                "putaway_complete",
                OWNER_ID,
                None,
                "out:row-1",
                http_json_fn=fake_http,
            )
        self.assertEqual(caught.exception.status, 409)


class TestOutboundClaimScope(unittest.TestCase):
    def test_claim_sets_soft_lease_before_releasing_database_lock(self) -> None:
        sql_calls: list[str] = []
        with (
            patch.object(outbound_publish, "table_has_column", return_value=True),
            patch.object(
                outbound_publish,
                "psql_query",
                side_effect=lambda _url, sql: sql_calls.append(sql) or "",
            ),
        ):
            claim_wms_outbox(
                "postgres://wms",
                "receiving_putaway_erp_feedback_outbox",
                "receiving_order_id",
                10,
                callback_path="/inbound-complete",
            )

        claim_sql = sql_calls[-1]
        self.assertIn("next_attempt_at = now() + interval '5 minutes'", claim_sql)
        self.assertIn("attempt_count = o.attempt_count + 1", claim_sql)

    def test_completion_is_guarded_by_the_claimed_attempt(self) -> None:
        with patch.object(outbound_publish, "psql_query") as query:
            mark_wms_outbox(
                "postgres://wms",
                "receiving_putaway_erp_feedback_outbox",
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                succeeded=True,
                attempt_count=2,
            )

        sql = query.call_args.args[1]
        self.assertIn("attempt_count = 2", sql)

    def test_receipt_timeout_requeues_only_catalog_outbox_from_original_key(
        self,
    ) -> None:
        key = (
            "out:inventory_snapshot_erp_feedback_outbox:"
            "11111111-1111-1111-1111-111111111111"
        )
        with patch.object(
            outbound_publish,
            "psql_query",
            return_value="11111111-1111-1111-1111-111111111111\n",
        ) as query:
            requeue_wms_outbox(
                "postgres://wms",
                key,
            )

        sql = query.call_args.args[1]
        self.assertIn("inventory_snapshot_erp_feedback_outbox", sql)
        self.assertIn("status = 'failed'", sql)
        self.assertIn("status = 'succeeded'", sql)
        self.assertIn("last_error = 'business receipt timeout'", sql)
        self.assertIn("already_requeued", sql)
        with self.assertRaises(ValueError):
            requeue_wms_outbox(
                "postgres://wms",
                "out:unknown_table:11111111-1111-1111-1111-111111111111",
            )
        with (
            patch.object(outbound_publish, "psql_query", return_value=""),
            self.assertRaises(RuntimeError),
        ):
            requeue_wms_outbox("postgres://wms", key)

    def test_claim_sql_is_scoped_to_bound_connector_owner_and_message_type(
        self,
    ) -> None:
        sql_calls: list[str] = []

        def query(_database_url: str, sql: str) -> str:
            sql_calls.append(sql)
            return ""

        with (
            patch.object(outbound_publish, "table_has_column", return_value=True),
            patch.object(outbound_publish, "psql_query", side_effect=query),
        ):
            claim_wms_outbox(
                "postgres://wms",
                "receiving_putaway_erp_feedback_outbox",
                "receiving_order_id",
                10,
                callback_path="/inbound-complete",
                connector_id=CONNECTOR_ID,
                message_type="putaway_complete",
            )

        claim_sql = sql_calls[-1]
        self.assertIn("h8_erp_connectors", claim_sql)
        self.assertIn(CONNECTOR_ID, claim_sql)
        self.assertIn("putaway_complete", claim_sql)
        self.assertIn("unnest(warehouse_ids)", claim_sql)
        self.assertIn("o.payload ->> 'warehouse_id'", claim_sql)
        self.assertIn("FROM h8_erp_messages message", claim_sql)
        self.assertIn(
            "'out:receiving_putaway_erp_feedback_outbox:' || o.id::text",
            claim_sql,
        )
        self.assertIn("message.connector_id", claim_sql)

    def test_archive_fifth_failure_has_no_future_retry(self) -> None:
        with patch.object(outbound_publish, "psql_query") as query:
            mark_wms_outbox(
                "postgres://wms",
                "archive_revision_erp_feedback_outbox",
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                succeeded=False,
                error="timeout",
                special_retry="archive",
                attempt_count=5,
                max_attempts=5,
            )

        sql = query.call_args.args[1]
        self.assertIn("status = 'dead'", sql)
        self.assertIn("next_attempt_at = now()", sql)
        self.assertNotIn("next_attempt_at = now() + interval", sql)

    def test_reconciliation_fifth_failure_enters_dead_and_notifies_operations(
        self,
    ) -> None:
        source = next(
            item
            for item in outbound_publish.OUTBOX_SOURCES
            if item["message_type"] == "reconciliation_diff"
        )
        self.assertEqual(source.get("special_retry"), "bounded")

        with patch.object(outbound_publish, "psql_query") as query:
            mark_wms_outbox(
                "postgres://wms",
                "reconciliation_erp_feedback_outbox",
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                succeeded=False,
                error="timeout",
                special_retry="bounded",
                attempt_count=5,
                max_attempts=5,
            )

        sql = query.call_args.args[1]
        self.assertIn("status = 'dead'", sql)
        self.assertIn("rc.reconciliation.erp_feedback_dead", sql)
        self.assertIn("ON CONFLICT", sql)
        self.assertNotIn("next_attempt_at = now() + interval", sql)

    def test_reconciliation_claim_closes_exhausted_rows_left_by_worker_crash(
        self,
    ) -> None:
        sql_calls: list[str] = []
        with (
            patch.object(outbound_publish, "table_has_column", return_value=True),
            patch.object(
                outbound_publish,
                "psql_query",
                side_effect=lambda _url, sql: sql_calls.append(sql) or "",
            ),
        ):
            claim_wms_outbox(
                "postgres://wms",
                "reconciliation_erp_feedback_outbox",
                "recon_doc_no",
                10,
                callback_path="/reconciliation-diff",
                special_retry="bounded",
            )

        self.assertIn("status = 'dead'", sql_calls[0])
        self.assertIn("attempt_count >= max_attempts", sql_calls[0])
        self.assertIn("rc.reconciliation.erp_feedback_dead", sql_calls[0])
        self.assertIn("o.attempt_count < o.max_attempts", sql_calls[1])

    def test_archive_retry_waits_five_minutes_and_honors_24_hour_deadline(self) -> None:
        sql_calls: list[str] = []

        with (
            patch.object(outbound_publish, "table_has_column", return_value=True),
            patch.object(
                outbound_publish,
                "psql_query",
                side_effect=lambda _url, sql: sql_calls.append(sql) or "",
            ),
        ):
            claim_wms_outbox(
                "postgres://wms",
                "archive_revision_erp_feedback_outbox",
                "liaison_id",
                10,
                callback_path="/archive-revision",
                special_retry="archive",
            )
            mark_wms_outbox(
                "postgres://wms",
                "archive_revision_erp_feedback_outbox",
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                succeeded=False,
                error="timeout",
                special_retry="archive",
                attempt_count=1,
                max_attempts=5,
            )

        self.assertIn("deadline_at <= now()", sql_calls[0])
        self.assertIn("attempt_count >= max_attempts", sql_calls[0])
        self.assertIn("o.deadline_at > now()", sql_calls[1])
        self.assertIn("o.attempt_count < o.max_attempts", sql_calls[1])
        self.assertIn("next_attempt_at = now() + interval '5 minutes'", sql_calls[2])
        self.assertIn("status = 'failed'", sql_calls[2])


class TestOutboundProcessRoute(unittest.TestCase):
    def setUp(self) -> None:
        aliases = patch.dict(
            "os.environ",
            {"WMS_H8_SECRET_ALIASES": '{"vault://erp/test":"erp-test-token"}'},
            clear=False,
        )
        aliases.start()
        self.addCleanup(aliases.stop)

    def test_rest_callback_carries_receipt_binding_idempotency_and_bearer(self) -> None:
        row = OutboxRow(
            table="receiving_putaway_erp_feedback_outbox",
            id="bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
            owner_id=OWNER_ID,
            event_type="putaway_complete",
            payload={"warehouse_id": WAREHOUSE_ID, "qty": 1},
            external_ref="receiving-1",
            attempt_count=1,
            max_attempts=5,
            deadline_at=None,
            callback_path="/inbound-complete",
        )
        lifecycle = SimpleNamespace(
            message_id="33333333-3333-3333-3333-333333333333",
            schema_version="1",
            correlation_id="corr-out-1",
            idempotency_key=f"out:{row.table}:{row.id}",
            connector_id=CONNECTOR_ID,
            config_version=3,
        )
        captured: dict[str, object] = {}

        class Response:
            status = 202

            def __enter__(self):
                return self

            def __exit__(self, *_args):
                return None

            def read(self) -> bytes:
                return b"{}"

        def urlopen(request, timeout):
            captured["request"] = request
            captured["timeout"] = timeout
            return Response()

        with patch.object(outbound_publish.urllib.request, "urlopen", side_effect=urlopen):
            outbound_publish.http_callback_publish(
                "https://erp.test/h8",
                row,
                lifecycle,
                "erp-secret",
            )

        request = captured["request"]
        body = json.loads(request.data)
        self.assertEqual(body["message_id"], lifecycle.message_id)
        self.assertEqual(body["schema_version"], "1")
        self.assertEqual(body["correlation_id"], "corr-out-1")
        self.assertEqual(body["idempotency_key"], lifecycle.idempotency_key)
        self.assertEqual(request.get_header("Idempotency-key"), lifecycle.idempotency_key)
        self.assertEqual(request.get_header("Authorization"), "Bearer erp-secret")

    def test_retry_process_uses_frozen_binding_without_current_route(self) -> None:
        row = OutboxRow(
            table="receiving_putaway_erp_feedback_outbox",
            id="bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
            owner_id=OWNER_ID,
            event_type="putaway_complete",
            payload={"warehouse_id": WAREHOUSE_ID, "qty": 1},
            external_ref="receiving-1",
            attempt_count=2,
            max_attempts=5,
            deadline_at=None,
            callback_path="/inbound-complete",
        )
        frozen = RouteBinding(
            connector_id=CONNECTOR_ID,
            connector_code="SELF-ERP",
            config_version=2,
            channel="interface_table",
            message_type="putaway_complete",
            owner_id=OWNER_ID,
            channel_mode="interface_table",
        )

        def lifecycle(_settings, _type, _ref, _idem, send, **kwargs):
            self.assertIs(kwargs["route_binding"], frozen)
            send(SimpleNamespace())
            return object()

        with (
            patch.object(
                outbound_publish,
                "OUTBOX_SOURCES",
                [
                    {
                        "table": row.table,
                        "ref_col": "receiving_order_id",
                        "callback_path": row.callback_path,
                        "message_type": "putaway_complete",
                    }
                ],
            ),
            patch.object(outbound_publish, "claim_wms_outbox", return_value=[row]),
            patch.object(
                outbound_publish,
                "resolve_existing_outbound_binding",
                return_value=frozen,
            ) as resolve_frozen,
            patch.object(outbound_publish, "resolve_outbound_route") as resolve_current,
            patch.object(outbound_publish, "mark_wms_outbox"),
            patch("exchange_lifecycle.run_outbound_pipeline", side_effect=lifecycle),
        ):
            processed = process_outbound_once(
                database_url="postgres://wms",
                sqlcmd_exec=lambda _sql: None,
                batch_size=10,
                dry_run=False,
                connector_id=CONNECTOR_ID,
                settings=settings(),
            )

        self.assertEqual(processed, 1)
        resolve_frozen.assert_called_once()
        resolve_current.assert_not_called()

    def test_each_catalog_message_has_rest_table_and_failover_paths(self) -> None:
        for source in OUTBOX_SOURCES:
            for channel_mode in (
                "rest",
                "interface_table",
                "rest_primary_table_fallback",
            ):
                with self.subTest(
                    message_type=source["message_type"], channel_mode=channel_mode
                ):
                    row = OutboxRow(
                        table=source["table"],
                        id="bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                        owner_id=OWNER_ID,
                        event_type=source["message_type"],
                        payload={"warehouse_id": WAREHOUSE_ID, "qty": 1},
                        external_ref=f"{source['message_type']}-1",
                        attempt_count=1,
                        max_attempts=5,
                        deadline_at=None,
                        callback_path=source["callback_path"],
                    )
                    route = RouteBinding(
                        connector_id=CONNECTOR_ID,
                        connector_code="SELF-ERP",
                        config_version=3,
                        channel=(
                            "interface_table"
                            if channel_mode == "interface_table"
                            else "rest"
                        ),
                        message_type=source["message_type"],
                        owner_id=OWNER_ID,
                        api_base_url="https://erp.test/h8",
                        channel_mode=channel_mode,
                        bearer_secret_alias="vault://erp/test",
                    )
                    table_sql: list[str] = []

                    def lifecycle(_settings, message_type, _ref, idem, send, **kwargs):
                        self.assertEqual(message_type, source["message_type"])
                        self.assertIs(kwargs["route_binding"], route)
                        send(SimpleNamespace(
                            message_id="message-1",
                            schema_version="1",
                            correlation_id="corr-1",
                            idempotency_key=idem,
                            connector_id=route.connector_id,
                            config_version=route.config_version,
                        ))
                        return object()

                    with (
                        patch.object(outbound_publish, "OUTBOX_SOURCES", [source]),
                        patch.object(
                            outbound_publish, "claim_wms_outbox", return_value=[row]
                        ) as claim,
                        patch.object(
                            outbound_publish,
                            "resolve_outbound_route",
                            return_value=route,
                        ),
                        patch.object(
                            outbound_publish, "http_callback_publish"
                        ) as callback,
                        patch.object(outbound_publish, "mark_wms_outbox") as mark,
                        patch(
                            "exchange_lifecycle.run_outbound_pipeline",
                            side_effect=lifecycle,
                        ),
                    ):
                        if channel_mode == "rest_primary_table_fallback":
                            callback.side_effect = RuntimeError("temporary unavailable")
                        processed = process_outbound_once(
                            database_url="postgres://wms",
                            sqlcmd_exec=table_sql.append,
                            batch_size=10,
                            dry_run=False,
                            connector_id=CONNECTOR_ID,
                            settings=settings(),
                            http_max_attempts=1,
                        )

                    self.assertEqual(processed, 1)
                    self.assertEqual(
                        claim.call_args.kwargs["message_type"],
                        source["message_type"],
                    )
                    mark.assert_called_once_with(
                        "postgres://wms",
                        source["table"],
                        row.id,
                        succeeded=True,
                        attempt_count=row.attempt_count,
                    )
                    if channel_mode == "rest":
                        self.assertEqual(callback.call_args.args[:2], ("https://erp.test/h8", row))
                        self.assertEqual(table_sql, [])
                    elif channel_mode == "interface_table":
                        callback.assert_not_called()
                        self.assertEqual(len(table_sql), 1)
                    else:
                        self.assertEqual(callback.call_args.args[:2], ("https://erp.test/h8", row))
                        self.assertEqual(len(table_sql), 1)
                    if table_sql:
                        self.assertIn(source["table"], table_sql[0])
                        self.assertIn(source["message_type"], table_sql[0])

    def test_lifecycle_persists_resolved_connector_version_and_channel(self) -> None:
        posts: list[dict] = []

        def fake_http(_settings, _method, _path, body, _idem):
            posts.append(body)
            return 200, {"id": "message-1"}, ""

        run_outbound_pipeline(
            settings(),
            "putaway_complete",
            "receiving-1",
            "out:row-1",
            lambda _life: None,
            http_json=fake_http,
            route_binding=binding("interface_table"),
            channel="interface_table",
        )

        self.assertTrue(posts)
        self.assertTrue(all(post["connector_id"] == CONNECTOR_ID for post in posts))
        self.assertTrue(all(post["connector_code"] == "SELF-ERP" for post in posts))
        self.assertTrue(all(post["config_version"] == 3 for post in posts))
        self.assertTrue(all(post["channel"] == "interface_table" for post in posts))

    def test_route_configuration_selects_transport_and_lifecycle_binding(self) -> None:
        row = OutboxRow(
            table="receiving_putaway_erp_feedback_outbox",
            id="bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
            owner_id=OWNER_ID,
            event_type="inbound_putaway_completed",
            payload={"warehouse_id": WAREHOUSE_ID, "qty": 1},
            external_ref="receiving-1",
            attempt_count=1,
            max_attempts=5,
            deadline_at=None,
            callback_path="/inbound-complete",
        )
        route = binding()

        def lifecycle(_settings, _type, _ref, idem, send, **kwargs):
            self.assertIs(kwargs["route_binding"], route)
            self.assertEqual(kwargs["channel"], "rest")
            send(SimpleNamespace(
                message_id="message-1",
                schema_version="1",
                correlation_id="corr-1",
                idempotency_key=idem,
                connector_id=route.connector_id,
                config_version=route.config_version,
            ))
            return object()

        with (
            patch.object(
                outbound_publish,
                "OUTBOX_SOURCES",
                [
                    {
                        "table": row.table,
                        "ref_col": "receiving_order_id",
                        "callback_path": row.callback_path,
                        "message_type": "putaway_complete",
                    }
                ],
            ),
            patch.object(
                outbound_publish, "claim_wms_outbox", return_value=[row]
            ) as claim,
            patch.object(
                outbound_publish, "resolve_outbound_route", return_value=route
            ),
            patch.object(outbound_publish, "http_callback_publish") as callback,
            patch.object(outbound_publish, "mark_wms_outbox"),
            patch("exchange_lifecycle.run_outbound_pipeline", side_effect=lifecycle),
        ):
            processed = process_outbound_once(
                database_url="postgres://wms",
                sqlcmd_exec=None,
                batch_size=10,
                dry_run=False,
                transport="table",
                callback_base="https://wrong-global.test",
                connector_id=CONNECTOR_ID,
                settings=settings(),
            )

        self.assertEqual(processed, 1)
        self.assertEqual(claim.call_args.kwargs["connector_id"], CONNECTOR_ID)
        self.assertEqual(callback.call_args.args[:2], ("https://erp.test/h8", row))

    def test_worker_main_enables_per_message_route_resolution(self) -> None:
        worker_settings = SimpleNamespace(
            connector_id=CONNECTOR_ID,
            api_base="http://wms.test",
            api_token="token",
            mssql_password="secret",
            batch_size=10,
        )
        with (
            patch.object(
                sync_worker.Settings, "from_env", return_value=worker_settings
            ),
            patch.object(
                sync_worker, "load_runtime_settings", return_value=worker_settings
            ),
            patch.object(
                sync_worker, "resolve_wms_db_url", return_value="postgres://wms"
            ),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "process_outbound_receipts", return_value=0),
            patch.object(
                sync_worker, "process_outbound_receipt_timeouts", return_value=0
            ),
            patch.object(
                sync_worker, "process_outbound_once", return_value=0
            ) as publish,
        ):
            self.assertEqual(
                sync_worker.main(["--once", "--direction", "out"]),
                0,
            )

        self.assertIs(publish.call_args.kwargs["settings"], worker_settings)
        self.assertIs(publish.call_args.kwargs["http_json_fn"], sync_worker.http_json)

    def test_paused_outbound_still_consumes_inflight_receipts(self) -> None:
        worker_settings = SimpleNamespace(
            connector_id=CONNECTOR_ID,
            api_base="http://wms.test",
            api_token="token",
            mssql_password="secret",
            batch_size=10,
        )
        with (
            patch.object(sync_worker.Settings, "from_env", return_value=worker_settings),
            patch.object(
                sync_worker, "load_runtime_settings", return_value=worker_settings
            ),
            patch.object(sync_worker, "resolve_wms_db_url", return_value="postgres://wms"),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "get_worker_claim_decision", return_value=False),
            patch.object(sync_worker, "process_outbound_receipts", return_value=1) as receipts,
            patch.object(sync_worker, "process_outbound_receipt_timeouts") as timeouts,
            patch.object(sync_worker, "process_outbound_once") as publish,
        ):
            self.assertEqual(sync_worker.main(["--once", "--direction", "out"]), 0)

        receipts.assert_called_once()
        timeouts.assert_not_called()
        publish.assert_not_called()

    def test_worker_requires_token_before_claiming_outbound(self) -> None:
        worker_settings = SimpleNamespace(
            connector_id=CONNECTOR_ID,
            api_base="http://wms.test",
            api_token=None,
            mssql_password="secret",
            batch_size=10,
        )
        with (
            patch.object(
                sync_worker.Settings, "from_env", return_value=worker_settings
            ),
            patch.object(
                sync_worker, "resolve_wms_db_url", return_value="postgres://wms"
            ),
            patch.object(sync_worker, "process_outbound_once") as publish,
        ):
            self.assertEqual(
                sync_worker.main(["--once", "--direction", "out"]),
                2,
            )
        publish.assert_not_called()


if __name__ == "__main__":
    unittest.main()
