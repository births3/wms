# @governance: skip-page-size 同一 Worker 协议回归夹具集中复用；后续按消息族机械拆分，不以删测试降规模。
"""H8 worker 纯逻辑单测（无 Docker / 无 DB）。"""

from __future__ import annotations

import json
import unittest
from types import SimpleNamespace
from unittest.mock import MagicMock, patch

import worker_mssql
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
    load_runtime_settings,
    resolve_inbound_route,
    validate_row_schema_version,
)
from inbound_canonical import CanonicalMappingError, build_inbound_canonical
from worker_route import (
    RouteBinding,
    claim_manual_replay,
    get_worker_claim_decision,
    list_manual_replays,
    post_worker_heartbeat,
    resolve_existing_inbound_binding,
    sanitize_worker_error,
)
from worker_mssql import claim_rows, requeue_replay_row
from v19_contract import payload_digest


def settings(config_version: int = 1) -> Settings:
    return Settings(
        mssql_host="localhost",
        mssql_port="1433",
        mssql_user="test",
        mssql_password="test",
        mssql_database="test",
        api_base="http://wms.test",
        api_token="token",
        poll_interval=1,
        max_retry=5,
        batch_size=1,
        connector_id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        connector_config_version=config_version,
        worker_id="worker-test",
        worker_version="test-1",
        heartbeat_ttl_seconds=15,
    )


def v19_goods_row(*, retry_count: int = 0) -> dict:
    row = {
        "seqid": 7,
        "GoodsID": 1001,
        "GoodsCode": "P-1",
        "GoodsName": "药品一",
        "Spec": "10mg*30片",
        "License": "国药准字H20260001",
        "ProduceCorp": "测试药业",
        "SpecialCategory": "普通药品",
        "Deposite": "阴凉",
        "PackagingJson": json.dumps(
            [
                {
                    "unit": "盒",
                    "ratio_to_base": 1,
                    "is_base": True,
                    "is_default": True,
                }
            ],
            ensure_ascii=False,
        ),
        "opType": "I",
        "OwnerCode": "ZBPF7",
        "SchemaVersion": "1",
        "IdempotencyKey": "idem-product-1",
        "CorrelationID": "corr-product-1",
        "SourceVersion": 1,
        "retry_count": retry_count,
        "inserttime": "2026-08-05T00:00:00.000Z",
    }
    row["PayloadDigest"] = payload_digest("x_wmsinter_GoodsInfo", row)
    return row


class TestInboundCorePipeline(unittest.TestCase):
    def test_http_json_uses_bound_api_key_only_for_inbound_business_calls(self) -> None:
        import sync_worker

        current = settings()
        current.api_key = "wms_connector_secret"
        response = MagicMock(status=200)
        response.read.return_value = b"{}"
        response.__enter__.return_value = response

        with patch.object(sync_worker.urllib.request, "urlopen", return_value=response) as urlopen:
            sync_worker.http_json(
                current,
                "POST",
                "/api/v1/integration/erp-messages/inbound/supplier_master",
                {},
                "idem-1",
            )
            inbound = urlopen.call_args.args[0]
            sync_worker.http_json(
                current,
                "GET",
                f"/api/v1/config/erp-connectors/{current.connector_id}",
                None,
                "idem-2",
            )
            bootstrap = urlopen.call_args.args[0]

        self.assertEqual(inbound.get_header("X-wms-api-key"), "wms_connector_secret")
        self.assertIsNone(inbound.get_header("Authorization"))
        self.assertEqual(bootstrap.get_header("Authorization"), "Bearer token")
        self.assertIsNone(bootstrap.get_header("X-wms-api-key"))

    def test_legacy_sql_executor_uses_in_process_tds_without_password_argv(
        self,
    ) -> None:
        with patch.object(worker_mssql, "mssql_execute") as execute:
            self.assertEqual(worker_mssql.sqlcmd_query(settings(), "SELECT 1"), "")

        execute.assert_called_once_with(settings(), "SELECT 1")

    def test_runtime_settings_are_frozen_from_connector_snapshot_and_secret_alias(
        self,
    ) -> None:
        calls: list[str] = []

        def fake_http(_settings, _method, path, _body, _key):
            calls.append(path)
            if path.endswith(settings().connector_id):
                return (
                    200,
                    {
                        "id": settings().connector_id,
                        "status": "active",
                        "config_version": 7,
                    },
                    "",
                )
            return (
                200,
                {
                    "id": settings().connector_id,
                    "owner_id": "owner-1",
                    "connector_code": "ERP-ONE",
                    "warehouse_ids": [],
                    "directions": ["inbound", "outbound"],
                    "message_types": ["asn", "shipment_confirm"],
                    "channel_mode": "interface_table",
                    "interface_db_host": "erp-sql.internal",
                    "interface_db_port": 1433,
                    "interface_db_name": "erp_if",
                    "interface_db_username": "h8_worker",
                    "interface_db_password_alias": "vault://h8/erp-if",
                    "config_version": 7,
                },
                "",
            )

        with patch.dict(
            "os.environ",
            {"WMS_H8_SECRET_ALIASES": '{"vault://h8/erp-if":"snapshot-secret"}'},
            clear=False,
        ):
            loaded = load_runtime_settings(settings(), http_json_fn=fake_http)

        self.assertEqual(loaded.mssql_host, "erp-sql.internal")
        self.assertEqual(loaded.mssql_port, "1433")
        self.assertEqual(loaded.mssql_user, "h8_worker")
        self.assertEqual(loaded.mssql_password, "snapshot-secret")
        self.assertEqual(loaded.mssql_database, "erp_if")
        self.assertEqual(loaded.connector_config_version, 7)
        self.assertEqual(
            calls,
            [
                f"/api/v1/config/erp-connectors/{settings().connector_id}",
                f"/api/v1/config/erp-connectors/{settings().connector_id}/versions/7",
            ],
        )

    def test_product_handler_posts_complete_shared_h8_rest_contract(self) -> None:
        import sync_worker

        row = sync_worker._runtime_row(
            "x_wmsinter_GoodsInfo", v19_goods_row()
        )
        command = sync_worker.build_v19_inbound_canonical(
            "product_master",
            row,
            RouteBinding(
                connector_id=settings().connector_id,
                connector_code="SELF-ERP",
                config_version=1,
                channel="interface_table",
                message_type="product_master",
            ),
        )
        calls: list[tuple[str, str, dict, str]] = []

        def fake_http(_settings, method, path, body, idempotency_key):
            calls.append((method, path, body, idempotency_key))
            return 200, {"wms_resource_id": "product-1", "status": "succeeded"}, ""

        with patch.object(sync_worker, "http_json", side_effect=fake_http):
            resource_id = HANDLERS["product_master"][1](settings(), command)

        self.assertEqual(resource_id, "product-1")
        self.assertEqual(
            calls[0][1],
            "/api/v1/integration/erp-messages/inbound/product_master",
        )
        self.assertEqual(calls[0][2]["special_drug_category"], "普通药品")
        self.assertEqual(calls[0][2]["packaging_levels"][0]["unit"], "盒")
        self.assertEqual(calls[0][3], "idem-product-1")

    def test_manual_replay_http_calls_are_scoped_and_claimed(self) -> None:
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
                                "direction": "inbound",
                                "message_type": "asn",
                                "channel": "interface_table",
                                "claimed_by": "replay:admin",
                                "idempotency_key": "idem-1",
                            }
                        ]
                    },
                    "",
                )
            return 200, {"id": "message-1", "claimed_by": "worker-test"}, ""

        messages = list_manual_replays(settings(), "asn", http_json_fn=fake_http)
        claim_manual_replay(settings(), str(messages[0]["id"]), http_json_fn=fake_http)

        self.assertIn(f"connector_id={settings().connector_id}", calls[0][1])
        self.assertIn("replay_requested=true", calls[0][1])
        self.assertEqual(calls[1][0], "POST")
        self.assertEqual(calls[1][2]["worker_id"], "worker-test")

    def test_manual_replay_requeues_then_claims_before_normal_processing(self) -> None:
        import sync_worker

        replay_message = {
            "id": "message-1",
            "idempotency_key": "idem-1",
        }
        row = v19_goods_row(retry_count=1)
        replay_message["idempotency_key"] = row["IdempotencyKey"]
        order: list[str] = []
        with (
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(
                sync_worker,
                "list_manual_replays",
                return_value=[replay_message],
            ) as list_replays,
            patch.object(
                sync_worker,
                "requeue_replay_row",
                side_effect=lambda *_args: order.append("requeue") or True,
            ),
            patch.object(
                sync_worker,
                "claim_manual_replay",
                side_effect=lambda *_args, **_kwargs: order.append("claim"),
            ),
            patch.object(
                sync_worker,
                "claim_rows",
                side_effect=lambda *_args: order.append("process") or [row],
            ),
            patch.object(
                sync_worker,
                "resolve_existing_inbound_binding",
                return_value=sync_worker.RouteBinding(
                    connector_id=settings().connector_id,
                    connector_code="SELF-ERP",
                    config_version=1,
                    channel="interface_table",
                    message_type="product_master",
                ),
            ),
            patch.object(
                sync_worker,
                "http_json",
                return_value=(200, {"wms_resource_id": "wms-1"}, ""),
            ),
            patch.object(sync_worker, "mark_row"),
        ):
            self.assertEqual(
                sync_worker.process_once(settings(), ["product_master"], False), 1
            )

        self.assertEqual(order, ["requeue", "claim", "process"])
        list_replays.assert_called_once_with(settings(), "product_master")

    def test_requeue_replay_row_restores_terminal_row_with_original_key(self) -> None:
        with patch(
            "worker_mssql.mssql_query",
            return_value=[{"IdempotencyKey": "idem-'1"}],
        ) as query:
            self.assertTrue(
                requeue_replay_row(
                    settings(), "x_wmsinter_InboundOrder", "idem-'1"
                )
            )
        sql = query.call_args.args[1]
        self.assertIn("handelflag = 0", sql)
        self.assertIn("handelflag IN (3, 4)", sql)
        self.assertEqual(query.call_args.args[2], ("ZBPF7", "idem-'1"))

    def test_missing_manual_replay_row_does_not_claim_message(self) -> None:
        import sync_worker

        with (
            patch.object(
                sync_worker,
                "list_manual_replays",
                return_value=[{"id": "message-1", "idempotency_key": "missing"}],
            ),
            patch.object(sync_worker, "requeue_replay_row", return_value=False),
            patch.object(sync_worker, "claim_manual_replay") as claim,
        ):
            sync_worker.prepare_manual_replays(
                settings(), "asn", "x_wmsinter_InboundOrder"
            )
        claim.assert_not_called()

    def test_retry_rejects_snapshot_that_does_not_match_original_binding(self) -> None:
        replies = iter(
            [
                (
                    200,
                    {
                        "data": [
                            {
                                "connector_id": settings().connector_id,
                                "connector_code": "SELF-ERP",
                                "config_version": 2,
                                "direction": "inbound",
                                "message_type": "asn",
                                "schema_version": "1",
                                "channel": "interface_table",
                            }
                        ]
                    },
                    "",
                ),
                (
                    200,
                    {
                        "id": settings().connector_id,
                        "connector_code": "OTHER-ERP",
                        "config_version": 2,
                        "warehouse_ids": [],
                        "directions": ["inbound"],
                        "message_types": ["asn"],
                        "channel_mode": "interface_table",
                    },
                    "",
                ),
            ]
        )

        with self.assertRaises(WorkerHttpError) as caught:
            resolve_existing_inbound_binding(
                settings(),
                "asn",
                {
                    "id": "row-1",
                    "external_ref": "ASN-1",
                    "idempotency_key": "idem-1",
                    "schema_version": "1",
                },
                http_json_fn=lambda *_args: next(replies),
            )
        self.assertEqual(caught.exception.status, 409)

    def test_retry_loads_original_binding_instead_of_resolving_current_route(
        self,
    ) -> None:
        import sync_worker

        calls: list[str] = []

        def fake_http(_settings, method, path, body, _idem):
            calls.append(path)
            if method == "POST":
                return 200, {"wms_resource_id": "wms-1"}, ""
            self.assertEqual(method, "GET")
            self.assertIsNone(body)
            if "/versions/2" in path:
                return (
                    200,
                    {
                        "id": settings().connector_id,
                        "connector_code": "SELF-ERP",
                        "config_version": 2,
                        "warehouse_ids": [],
                        "directions": ["inbound"],
                        "message_types": ["product_master"],
                        "channel_mode": "interface_table",
                    },
                    "",
                )
            return (
                200,
                {
                    "data": [
                        {
                            "connector_id": settings().connector_id,
                            "connector_code": "SELF-ERP",
                            "config_version": 2,
                            "direction": "inbound",
                            "message_type": "product_master",
                            "schema_version": "1",
                            "channel": "interface_table",
                        }
                    ]
                },
                "",
            )

        row = v19_goods_row(retry_count=1)
        with (
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "list_manual_replays", return_value=[]),
            patch.object(sync_worker, "claim_rows", return_value=[row]),
            patch.object(sync_worker, "http_json", side_effect=fake_http),
            patch.object(sync_worker, "resolve_inbound_route") as resolve_current,
            patch.object(
                sync_worker,
                "build_v19_inbound_canonical",
                wraps=sync_worker.build_v19_inbound_canonical,
            ) as build,
            patch.object(sync_worker, "mark_row"),
        ):
            self.assertEqual(
                sync_worker.process_once(settings(2), ["product_master"], False),
                1,
            )

        resolve_current.assert_not_called()
        binding = build.call_args.args[2]
        self.assertEqual(binding.config_version, 2)
        self.assertIn("direction=inbound", calls[0])
        self.assertIn("message_type=product_master", calls[0])
        self.assertIn("external_ref=P-1", calls[0])
        self.assertIn("idempotency_key=idem-product-1", calls[0])
        self.assertIn("created_from=1970-01-01T00%3A00%3A00Z", calls[0])
        self.assertIn(f"/{settings().connector_id}/versions/2", calls[1])

    def test_product_rejects_unmapped_storage_condition_before_business_api(
        self,
    ) -> None:
        row = {
            "id": "row-1",
            "owner_id": "owner-1",
            "external_doc_no": "ERP-1",
            "idempotency_key": "idem-product-1",
            "product_code": "P-1",
            "product_name": "药品一",
            "storage_condition": "ERP_UNKNOWN",
        }
        with self.assertRaises(CanonicalMappingError) as caught:
            build_inbound_canonical(
                "product_master",
                row,
                RouteBinding(
                    connector_id=settings().connector_id,
                    connector_code="SELF-ERP",
                    config_version=1,
                    channel="interface_table",
                    message_type="product_master",
                ),
            )
        self.assertEqual(caught.exception.status, 422)
        self.assertFalse(is_retryable_worker_error(caught.exception))

    def test_each_inbound_business_api_uses_shared_error_classification(self) -> None:
        for status, expected_retryable in ((503, True), (422, False)):
            for message_type, (_table, handler) in HANDLERS.items():
                with self.subTest(message_type=message_type, status=status):
                    command = SimpleNamespace(
                        message_type=message_type,
                        idempotency_key="idem-1",
                        fields={"request_body": {}},
                    )
                    with patch(
                        "sync_worker.http_json",
                        return_value=(status, None, '{"token":"response-secret"}'),
                    ):
                        with self.assertRaises(WorkerHttpError) as caught:
                            handler(settings(), command)
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
            settings(3),
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

    def test_inbound_route_rejects_config_version_not_frozen_by_this_worker(
        self,
    ) -> None:
        with self.assertRaises(WorkerHttpError) as caught:
            resolve_inbound_route(
                settings(2),
                "asn",
                {"idempotency_key": "idem-version-drift"},
                http_json_fn=lambda *_args: (
                    200,
                    {
                        "connector": {
                            "id": settings().connector_id,
                            "config_version": 3,
                            "channel_mode": "interface_table",
                            "connector_code": "SELF-ERP",
                        }
                    },
                    "",
                ),
            )
        self.assertEqual(caught.exception.status, 409)

    def test_inbound_route_rejects_another_worker_connector(self) -> None:
        def fake_http(*_args, **_kwargs):
            return (
                200,
                {
                    "connector": {
                        "id": "ffffffff-ffff-ffff-ffff-ffffffffffff",
                        "config_version": 1,
                        "channel_mode": "interface_table",
                        "connector_code": "OTHER-ERP",
                    }
                },
                "",
            )

        with self.assertRaises(WorkerHttpError):
            resolve_inbound_route(
                settings(),
                "asn",
                {"idempotency_key": "idem-other"},
                http_json_fn=fake_http,
            )

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

        row = v19_goods_row()
        with (
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "list_manual_replays", return_value=[]),
            patch.object(sync_worker, "claim_rows", return_value=[row]),
            patch.object(
                sync_worker,
                "resolve_inbound_route",
                side_effect=WorkerHttpError(422, "schema", "unsupported"),
            ),
            patch.object(sync_worker, "mark_row") as mark,
        ):
            self.assertEqual(
                sync_worker.process_once(settings(), ["product_master"], False), 1
            )
        self.assertEqual(mark.call_args.args[3], "dead")
        self.assertEqual(mark.call_args.kwargs["retry_count"], 1)

    def test_invalid_schema_version_enters_dead_without_route_lookup(self) -> None:
        import sync_worker

        row = v19_goods_row()
        row["SchemaVersion"] = "999"
        row["PayloadDigest"] = payload_digest("x_wmsinter_GoodsInfo", row)
        with (
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
        self.assertEqual(mark.call_args.args[3], "dead")
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

    def test_claim_rows_uses_v19_lease_protocol(self) -> None:
        claimed = [{"seqid": 1, "IdempotencyKey": "idem-1"}]
        with patch("worker_mssql.mssql_query", return_value=claimed) as query:
            rows = claim_rows(settings(), "x_wmsinter_GoodsInfo")
        sql = query.call_args.args[1]
        self.assertIn("handelflag = 0", sql)
        self.assertIn("handelflag = 3 AND next_retry_at <= SYSUTCDATETIME()", sql)
        self.assertIn("handelflag = 2 AND lease_until < SYSUTCDATETIME()", sql)
        self.assertIn("ORDER BY inserttime, seqid", sql)
        self.assertNotIn("FOR JSON PATH", sql)
        self.assertEqual(rows, claimed)

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
    def test_order_status_writes_v19_order_feedback_without_legacy_table(self) -> None:
        row = OutboxRow(
            table="receiving_putaway_erp_feedback_outbox",
            id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            owner_id="11111111-1111-1111-1111-111111111111",
            event_type="order_status",
            payload={
                "erp_bill_code": "RK-1",
                "revision": 1,
                "order_type": 1,
                "feedback_type": 100,
                "command_id": "cmd-1",
                "result_code": None,
                "result_message": "it's cancelled",
                "correlation_id": "corr-1",
                "feedback_time": "2026-08-05T12:00:00.000Z",
            },
            external_ref="rcv-1",
            attempt_count=1,
            max_attempts=5,
            deadline_at=None,
            callback_path="/inbound-complete",
        )
        sql = insert_if_out_sql(row, owner_code="ZBPF7")
        self.assertIn("x_wmsinter_OrderFeedback", sql)
        self.assertNotIn("if_out_message", sql)
        self.assertIn("it''s cancelled", sql)
        self.assertIn(row.id, sql)
        self.assertNotIn(f"out:{row.table}:{row.id}", sql)
        self.assertNotIn("UPDATE dbo.x_wmsinter_OrderFeedback", sql)

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
            payload={
                "depot_code": "WH001",
                "product_code": "P1",
                "batch_no": "B|1",
                "to_status": "合格",
                "qty": "1.0000",
                "occur_time": "2026-08-05T12:00:00.000Z",
            },
            external_ref="x",
            attempt_count=1,
            max_attempts=5,
            deadline_at=None,
            callback_path="/inventory-status",
        )
        sql = insert_if_out_sql(row, owner_code="ZBPF7")
        self.assertIn("x_wmsinter_WmsEvent", sql)
        self.assertNotIn("if_out_message", sql)
        self.assertIn("B|1", sql)
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
