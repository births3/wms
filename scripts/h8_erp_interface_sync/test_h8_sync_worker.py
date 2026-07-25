# @governance: skip-page-size 同一 Worker 协议回归夹具集中复用；后续按消息族机械拆分，不以删测试降规模。
"""H8 worker 纯逻辑单测（无 Docker / 无 DB）。"""

from __future__ import annotations

import json
import unittest
from unittest.mock import patch

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


class TestInboundCorePipeline(unittest.TestCase):
    def test_sqlcmd_json_output_uses_unbounded_text_without_header_conflict(
        self,
    ) -> None:
        completed = type(
            "Completed",
            (),
            {"returncode": 0, "stdout": "[]", "stderr": ""},
        )()
        with patch.object(worker_mssql.subprocess, "run", return_value=completed) as run:
            self.assertEqual(worker_mssql.sqlcmd_query(settings(), "SELECT 1"), "[]")

        command = run.call_args.args[0]
        self.assertEqual(command[0], "sqlcmd")
        self.assertEqual(command[command.index("-S") + 1], "tcp:localhost,1433")
        self.assertEqual(command[command.index("-y") + 1], "0")
        self.assertEqual(command[command.index("-w") + 1], "65535")
        self.assertNotIn("-h", command)

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

    def test_each_interface_table_message_runs_shared_canonical_pipeline(self) -> None:
        import sync_worker

        row = {
            "id": "row-1",
            "owner_id": "owner-1",
            "warehouse_id": "warehouse-1",
            "external_doc_no": "ERP-1",
            "external_ref": "ERP-1",
            "receipt_no": "R-1",
            "document_type": "purchase_inbound",
            "supplier_id": "supplier-1",
            "customer_id": "customer-1",
            "expected_arrival_at": "2026-07-23T00:00:00Z",
            "product_id": "product-1",
            "product_code": "P-1",
            "product_name": "药品一",
            "spec": "10mg*30片",
            "special_drug_category": "普通药品",
            "storage_condition": "normal",
            "packaging_json": json.dumps(
                [
                    {
                        "unit": "盒",
                        "ratio_to_base": 1,
                        "is_base": True,
                        "is_default": True,
                        "sort_order": 1,
                    }
                ],
                ensure_ascii=False,
            ),
            "expected_qty": "1",
            "planned_qty": "1",
            "batch_no": "B-1",
            "required_ship_at": "2026-07-23T00:00:00Z",
            "field_name": "spec",
            "new_value": "10mg",
            "schema_version": "1",
            "idempotency_key": "idem-1",
            "retry_count": "0",
            "created_at": "2026-07-23T00:00:00Z",
        }
        expected_paths = {
            "asn": "/api/v1/inbound/receiving-orders",
            "outbound_order": "/api/v1/outbound/orders",
            "product_master": (
                "/api/v1/integration/erp-messages/inbound/product_master"
            ),
            "return_order": "/api/v1/inbound/receiving-orders",
            "product_change": (
                "/api/v1/integration/erp-messages/inbound/product_change"
            ),
        }

        for message_type, (table, _handler) in HANDLERS.items():
            with self.subTest(message_type=message_type):
                binding = RouteBinding(
                    connector_id=settings().connector_id,
                    connector_code="SELF-ERP",
                    config_version=1,
                    channel="interface_table",
                    message_type=message_type,
                )
                api_paths: list[str] = []

                def business_http(_settings, _method, path, _body, _key):
                    api_paths.append(path)
                    if message_type in ("product_master", "product_change"):
                        return 200, {"wms_resource_id": "resource-1"}, ""
                    return 201, {"id": "resource-1"}, ""

                def pipeline(
                    worker_settings,
                    actual_type,
                    actual_row,
                    handler,
                    converter,
                    **kwargs,
                ):
                    command = converter(
                        actual_type, actual_row, kwargs["route_binding"]
                    )
                    return handler(worker_settings, command), object()

                with (
                    patch.object(
                        sync_worker, "get_worker_claim_decision", return_value=True
                    ),
                    patch.object(sync_worker, "try_record_worker_heartbeat"),
                    patch.object(sync_worker, "list_manual_replays", return_value=[]),
                    patch.object(
                        sync_worker, "claim_rows", return_value=[row]
                    ) as claim,
                    patch.object(
                        sync_worker, "resolve_inbound_route", return_value=binding
                    ),
                    patch.object(
                        sync_worker,
                        "build_inbound_canonical_with_mpm",
                        side_effect=lambda _settings, kind, source, route: (
                            build_inbound_canonical(kind, source, route)
                        ),
                    ),
                    patch.object(
                        sync_worker, "run_inbound_pipeline", side_effect=pipeline
                    ),
                    patch.object(sync_worker, "http_json", side_effect=business_http),
                    patch.object(sync_worker, "mark_row") as mark,
                ):
                    self.assertEqual(
                        sync_worker.process_once(settings(), [message_type], False),
                        1,
                    )

                claim.assert_called_once_with(settings(), table)
                self.assertEqual(api_paths, [expected_paths[message_type]])
                mark.assert_called_once_with(
                    settings(), table, row["id"], "success", wms_id="resource-1"
                )

    def test_product_handler_posts_complete_shared_h8_rest_contract(self) -> None:
        import sync_worker

        command = build_inbound_canonical(
            "product_master",
            {
                "id": "row-product-1",
                "owner_id": "owner-1",
                "external_doc_no": "ERP-PM-1",
                "idempotency_key": "idem-product-1",
                "schema_version": "1",
                "created_at": "2026-07-23T00:00:00",
                "product_code": "P-1",
                "product_name": "药品一",
                "spec": "10mg*30片",
                "dosage_form": "薄膜衣片",
                "manufacturer": "测试药业",
                "special_drug_category": "普通药品",
                "storage_condition": "2-8℃避光保存",
                "udi_code": "06912345678901",
                "electronic_regulatory_code": "REG-001",
                "length_mm": "120",
                "width_mm": "80",
                "height_mm": "50",
                "volume_cm3": "",
                "weight_g": "350.5",
                "packaging_json": json.dumps(
                    [
                        {
                            "unit": "盒",
                            "ratio_to_base": 1,
                            "is_base": True,
                            "is_default": True,
                            "sort_order": 1,
                        }
                    ],
                    ensure_ascii=False,
                ),
            },
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
        row = {
            "id": "row-1",
            "external_doc_no": "ASN-1",
            "idempotency_key": "idem-1",
            "schema_version": "1",
            "retry_count": "1",
        }
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
                    message_type="asn",
                ),
            ),
            patch.object(
                sync_worker,
                "run_inbound_pipeline",
                return_value=("wms-1", object()),
            ) as pipeline,
            patch.object(sync_worker, "mark_row"),
        ):
            self.assertEqual(sync_worker.process_once(settings(), ["asn"], False), 1)

        self.assertEqual(order, ["requeue", "claim", "process"])
        list_replays.assert_called_once_with(settings(), "asn")
        self.assertTrue(callable(pipeline.call_args.args[4]))

    def test_requeue_replay_row_restores_terminal_row_with_original_key(self) -> None:
        with patch("worker_mssql.sqlcmd_query", return_value="ready") as query:
            self.assertTrue(requeue_replay_row(settings(), "if_in_asn", "idem-'1"))
        sql = query.call_args.args[1]
        self.assertIn("sync_status = N'pending'", sql)
        self.assertIn("retry_count < 1", sql)
        self.assertIn("idem-''1", sql)
        self.assertIn("N'failed', N'dead', N'success'", sql)

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
            sync_worker.prepare_manual_replays(settings(), "asn", "if_in_asn")
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
                        "message_types": ["asn"],
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
                            "message_type": "asn",
                            "schema_version": "1",
                            "channel": "interface_table",
                        }
                    ]
                },
                "",
            )

        row = {
            "id": "row-1",
            "external_doc_no": "ASN-1",
            "external_ref": "ERP ASN/1",
            "idempotency_key": "idem-1",
            "schema_version": "1",
            "retry_count": "1",
        }
        with (
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "list_manual_replays", return_value=[]),
            patch.object(sync_worker, "claim_rows", return_value=[row]),
            patch.object(sync_worker, "http_json", side_effect=fake_http),
            patch.object(sync_worker, "resolve_inbound_route") as resolve_current,
            patch.object(
                sync_worker,
                "run_inbound_pipeline",
                return_value=("wms-1", object()),
            ) as pipeline,
            patch.object(sync_worker, "mark_row"),
        ):
            self.assertEqual(sync_worker.process_once(settings(2), ["asn"], False), 1)

        resolve_current.assert_not_called()
        binding = pipeline.call_args.kwargs["route_binding"]
        self.assertEqual(binding.config_version, 2)
        self.assertIn("direction=inbound", calls[0])
        self.assertIn("message_type=asn", calls[0])
        self.assertIn("external_ref=ERP+ASN%2F1", calls[0])
        self.assertIn("idempotency_key=idem-1", calls[0])
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
        row = {
            "id": "row-1",
            "owner_id": "owner-1",
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
            "spec": "10mg*30片",
            "special_drug_category": "普通药品",
            "storage_condition": "normal",
            "packaging_json": json.dumps(
                [
                    {
                        "unit": "盒",
                        "ratio_to_base": 1,
                        "is_base": True,
                        "is_default": True,
                        "sort_order": 1,
                    }
                ],
                ensure_ascii=False,
            ),
            "expected_qty": "1",
            "planned_qty": "1",
            "batch_no": "B-1",
            "required_ship_at": "2026-07-23T00:00:00Z",
            "field_name": "spec",
            "new_value": "10mg",
        }
        binding = RouteBinding(
            connector_id=settings().connector_id,
            connector_code="SELF-ERP",
            config_version=1,
            channel="interface_table",
            message_type="asn",
        )
        for status, expected_retryable in ((503, True), (422, False)):
            for message_type, (_table, handler) in HANDLERS.items():
                with self.subTest(message_type=message_type, status=status):
                    command = build_inbound_canonical(message_type, row, binding)
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

    def test_archive_product_change_closes_quality_liaison_after_m1_update(
        self,
    ) -> None:
        row = {
            "id": "row-archive-1",
            "owner_id": "owner-1",
            "external_doc_no": "ARCHIVE-1",
            "idempotency_key": "idem-archive-1",
            "product_id": "11111111-1111-1111-1111-111111111111",
            "product_code": "P-ARCHIVE-001",
            "field_name": "approval_number",
            "new_value": "NEW-001",
            "liaison_id": "22222222-2222-2222-2222-222222222222",
            "asn_id": "33333333-3333-3333-3333-333333333333",
        }
        command = build_inbound_canonical(
            "product_change",
            row,
            RouteBinding(
                connector_id=settings().connector_id,
                connector_code="SELF-ERP",
                config_version=1,
                channel="interface_table",
                message_type="product_change",
            ),
        )
        calls: list[tuple[str, str, dict | None, str]] = []

        def fake_http(_settings, method, path, body, idempotency_key):
            calls.append((method, path, body, idempotency_key))
            return 200, {"wms_resource_id": row["product_id"]}, ""

        with patch("sync_worker.http_json", side_effect=fake_http):
            resource_id = HANDLERS["product_change"][1](settings(), command)

        self.assertEqual(resource_id, row["product_id"])
        self.assertEqual(
            calls[0][1],
            "/api/v1/integration/erp-messages/inbound/product_change",
        )
        self.assertEqual(
            calls[0][2],
            {
                "schema_version": "1",
                "external_ref": "ARCHIVE-1",
                "correlation_id": "row-archive-1",
                "occurred_at": "",
                "product_id": row["product_id"],
                "product_code": "P-ARCHIVE-001",
                "field_name": "approval_number",
                "new_value": "NEW-001",
                "liaison_id": row["liaison_id"],
                "asn_id": row["asn_id"],
            },
        )
        self.assertEqual(calls[0][3], "idem-archive-1")

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
            patch.object(sync_worker, "list_manual_replays", return_value=[]),
            patch.object(sync_worker, "record_preflight_failure") as preflight_audit,
            patch.object(sync_worker, "mark_terminal_inbound_message") as mark_dead,
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
        mark_dead.assert_called_once()

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
            patch.object(sync_worker, "list_manual_replays", return_value=[]),
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
        raw = json.dumps([{
            "id": "row-1",
            "external_doc_no": "ASN|1",
            "owner_id": "owner-1",
            "warehouse_id": "warehouse-1",
            "supplier_id": "supplier-1",
            "product_code": "P-1",
            "expected_qty": "2",
            "expected_arrival_at": "2026-07-22T10:00:00",
            "document_type": "purchase_inbound",
            "external_ref": "ERP-1",
            "receipt_no": "R-1",
            "schema_version": "1",
            "idempotency_key": "idem-1",
            "retry_count": "0",
            "created_at": "2026-07-22T09:59:00",
        }])
        with patch("worker_mssql.sqlcmd_query", return_value=raw) as query:
            rows = claim_rows(settings(), "if_in_asn")
        sql = query.call_args.args[1]
        self.assertIn("DATEADD(MILLISECOND", sql)
        self.assertIn("retry_count = 1 THEN 1000", sql)
        self.assertIn("retry_count = 4 THEN 8000", sql)
        self.assertIn("ELSE 16000", sql)
        self.assertIn("UNICODE(LEFT(idempotency_key, 1))", sql)
        self.assertIn("% 4001", sql)
        self.assertIn("last_error IS NULL", sql)
        self.assertIn("FOR JSON PATH", sql)
        self.assertEqual(rows[0]["external_doc_no"], "ASN|1")
        self.assertEqual(rows[0]["schema_version"], "1")
        self.assertEqual(rows[0]["created_at"], "2026-07-22T09:59:00")

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
