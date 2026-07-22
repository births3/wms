"""US-H8-002 AC4：出站必须按消息解析唯一连接，不能取全局首条配置。"""

from __future__ import annotations

import unittest
from types import SimpleNamespace
from unittest.mock import patch

import outbound_publish
import sync_worker
from exchange_lifecycle import run_outbound_pipeline
from outbound_publish import OutboxRow, claim_wms_outbox, process_outbound_once
from worker_route import RouteBinding, WorkerHttpError, resolve_outbound_route


CONNECTOR_ID = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
OWNER_ID = "11111111-1111-1111-1111-111111111111"
WAREHOUSE_ID = "22222222-2222-2222-2222-222222222222"


def settings() -> SimpleNamespace:
    return SimpleNamespace(
        connector_id=CONNECTOR_ID,
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
    )


class TestOutboundRoute(unittest.TestCase):
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


class TestOutboundProcessRoute(unittest.TestCase):
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
            lambda: None,
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

        def lifecycle(_settings, _type, _ref, _idem, send, **kwargs):
            self.assertIs(kwargs["route_binding"], route)
            self.assertEqual(kwargs["channel"], "rest")
            send()
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
        callback.assert_called_once_with("https://erp.test/h8", row)

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
                sync_worker, "resolve_wms_db_url", return_value="postgres://wms"
            ),
            patch.object(sync_worker, "try_record_worker_heartbeat"),
            patch.object(sync_worker, "get_worker_claim_decision", return_value=True),
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
