import json
import os
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from unittest.mock import patch

from reconciliation_pull import (
    erp_get_json,
    pull_due_reconciliation_snapshots,
    pull_reconciliation_snapshot,
)
from sync_worker import Settings
from worker_route import WorkerHttpError


def claim_value(
    *,
    owner_id: str = "owner-1",
    window_key: str = "scheduled:owner-1:20260723T180000Z",
) -> dict[str, object]:
    return {
        "id": "10000000-0000-0000-0000-000000000001",
        "claim_token": "20000000-0000-0000-0000-000000000002",
        "owner_id": owner_id,
        "window_key": window_key,
        "worker_id": "worker-test",
        "attempt_no": 1,
        "lease_expires_at": "2026-07-23T18:02:00Z",
    }


def settings() -> Settings:
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
        batch_size=10,
        connector_id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        connector_config_version=2,
        worker_id="worker-test",
        worker_version="1",
        heartbeat_ttl_seconds=15,
    )


class ReconciliationPullTest(unittest.TestCase):
    def setUp(self) -> None:
        aliases = patch.dict(
            os.environ,
            {"WMS_H8_SECRET_ALIASES": '{"vault://erp/owner-1":"owner-token"}'},
            clear=False,
        )
        aliases.start()
        self.addCleanup(aliases.stop)

    def test_calls_real_self_built_erp_inventory_http_contract(self) -> None:
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                if self.headers.get("Authorization") != "Bearer erp-test-token":
                    self.send_error(401)
                    return
                if not self.path.startswith(
                    "/h8/inventory-snapshots?owner_id=owner-http"
                ):
                    self.send_error(404)
                    return
                body = json.dumps(
                    {
                        "snapshot_at": "2026-07-23T18:00:00Z",
                        "items": [
                            {
                                "product_code": "P-HTTP",
                                "batch_no": "B-HTTP",
                                "qty_on_hand": 5,
                            }
                        ],
                        "next_cursor": None,
                    }
                ).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, _format, *_args):
                return

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            result = erp_get_json(
                f"http://127.0.0.1:{server.server_port}/h8",
                "owner-http",
                None,
                "erp-test-token",
            )
        finally:
            server.shutdown()
            thread.join(timeout=2)
            server.server_close()
        self.assertEqual(result["items"][0]["qty_on_hand"], 5)

    def test_hsch_trigger_runs_every_due_owner_once(self) -> None:
        requests = []

        def http(_settings, method, path, body, key):
            requests.append((method, path, body, key))
            if path == "/api/v1/reconciliation/claims":
                return (
                    200,
                    {"claim": claim_value()},
                    "",
                )
            if path.endswith("/renew"):
                return 200, {"status": "active"}, ""
            if path.startswith("/api/v1/config/erp-connectors/route-resolve?"):
                return (
                    200,
                    {
                        "connector": {
                            "id": settings().connector_id,
                            "owner_id": "owner-1",
                            "connector_code": "SELF-ERP",
                            "config_version": 2,
                            "channel_mode": "rest",
                            "api_base_url": "https://erp.test/h8",
                            "bearer_secret_alias": "vault://erp/owner-1",
                            "directions": ["outbound"],
                            "message_types": ["inventory_snapshot"],
                            "warehouse_ids": [],
                        }
                    },
                    "",
                )
            return 200, {"id": "run-due-1"}, ""

        tokens = []
        with patch.dict(
            os.environ,
            {"WMS_H8_SECRET_ALIASES": '{"vault://erp/owner-1":"owner-token"}'},
            clear=False,
        ):
            run_ids = pull_due_reconciliation_snapshots(
                settings(),
                http_json_fn=http,
                erp_get_fn=lambda *_args: (
                    tokens.append(_args[3])
                    or {
                        "snapshot_at": "2026-07-23T18:00:00Z",
                        "items": [],
                        "next_cursor": None,
                    }
                ),
            )

        self.assertEqual(run_ids, ["run-due-1"])
        self.assertEqual(requests[0][0:2], ("POST", "/api/v1/reconciliation/claims"))
        self.assertEqual(requests[0][2]["worker_id"], "worker-test")
        self.assertEqual(requests[-1][2]["claim_id"], claim_value()["id"])
        self.assertEqual(
            requests[-1][2]["claim_token"], claim_value()["claim_token"]
        )
        self.assertEqual(requests[-1][2]["window_key"], "scheduled:owner-1:20260723T180000Z")
        self.assertEqual(tokens, ["owner-token"])

    def test_hsch_skips_when_another_worker_holds_the_claim(self) -> None:
        requests = []

        def http(_settings, method, path, body, key):
            requests.append((method, path, body, key))
            return 200, {"claim": None}, ""

        run_ids = pull_due_reconciliation_snapshots(
            settings(),
            http_json_fn=http,
            erp_get_fn=lambda *_args: self.fail("ERP must not be called without claim"),
        )

        self.assertEqual(run_ids, [])
        self.assertEqual(len(requests), 1)
        self.assertEqual(requests[0][1], "/api/v1/reconciliation/claims")

    def test_pulls_all_pages_and_posts_one_canonical_snapshot(self) -> None:
        posts = []
        pages = iter(
            [
                {
                    "snapshot_at": "2026-07-23T18:00:00Z",
                    "items": [
                        {"product_code": "P1", "batch_no": "B1", "qty_on_hand": 3}
                    ],
                    "next_cursor": "page-2",
                },
                {
                    "snapshot_at": "2026-07-23T18:00:00Z",
                    "items": [
                        {"product_code": "P2", "batch_no": "B2", "qty_on_hand": 4}
                    ],
                    "next_cursor": None,
                },
            ]
        )

        def http(_settings, method, path, body, key):
            if path.endswith("/renew"):
                return 200, {"status": "active"}, ""
            if path.endswith("/failed"):
                return 200, {"status": "failed"}, ""
            if method == "GET":
                return (
                    200,
                    {
                        "connector": {
                            "id": settings().connector_id,
                            "owner_id": "owner-1",
                            "connector_code": "SELF-ERP",
                            "config_version": 2,
                            "channel_mode": "rest",
                            "api_base_url": "https://erp.test/h8",
                            "bearer_secret_alias": "vault://erp/owner-1",
                            "directions": ["outbound"],
                            "message_types": ["inventory_snapshot"],
                            "warehouse_ids": [],
                        }
                    },
                    "",
                )
            posts.append((path, body, key))
            return 200, {"id": "run-1"}, ""

        run_id = pull_reconciliation_snapshot(
            settings(),
            "owner-1",
            "2026-07-23T18",
            claim={
                **claim_value(window_key="2026-07-23T18"),
                "owner_id": "owner-1",
            },
            http_json_fn=http,
            erp_get_fn=lambda *_args: next(pages),
        )

        self.assertEqual(run_id, "run-1")
        self.assertEqual(posts[0][0], "/api/v1/reconciliation/runs")
        self.assertEqual(len(posts[0][1]["items"]), 2)
        self.assertEqual(posts[0][2], "rc:owner-1:2026-07-23T18")

    def test_rejects_warehouse_scoped_inventory_snapshot_route(self) -> None:
        def http(_settings, method, _path, _body, _key):
            if method == "GET":
                return (
                    200,
                    {
                        "connector": {
                            "id": settings().connector_id,
                            "owner_id": "owner-1",
                            "connector_code": "WAREHOUSE-ERP",
                            "config_version": 2,
                            "channel_mode": "rest",
                            "api_base_url": "https://erp.test/h8",
                            "bearer_secret_alias": "vault://erp/owner-1",
                            "directions": ["outbound"],
                            "message_types": ["inventory_snapshot"],
                            "warehouse_ids": ["00000000-0000-0000-0000-000000000901"],
                        }
                    },
                    "",
                )
            return 200, {"id": "must-not-run"}, ""

        with self.assertRaisesRegex(WorkerHttpError, "owner-wide"):
            pull_reconciliation_snapshot(
                settings(),
                "owner-1",
                "2026-07-23T18",
                claim={
                    **claim_value(window_key="2026-07-23T18"),
                    "owner_id": "owner-1",
                },
                http_json_fn=http,
                erp_get_fn=lambda *_args: {
                    "snapshot_at": "2026-07-23T18:00:00Z",
                    "items": [],
                    "next_cursor": None,
                },
            )

    def test_pull_failure_is_reported_with_controlled_code(self) -> None:
        requests = []

        def http(_settings, method, path, body, key):
            requests.append((method, path, body, key))
            if path.startswith("/api/v1/config/erp-connectors/route-resolve?"):
                return (
                    200,
                    {
                        "connector": {
                            "id": settings().connector_id,
                            "owner_id": "owner-1",
                            "connector_code": "SELF-ERP",
                            "config_version": 2,
                            "channel_mode": "rest",
                            "api_base_url": "https://erp.test/h8",
                            "bearer_secret_alias": "vault://erp/owner-1",
                            "directions": ["outbound"],
                            "message_types": ["inventory_snapshot"],
                            "warehouse_ids": [],
                        }
                    },
                    "",
                )
            if path.endswith("/renew"):
                return 200, {"status": "active"}, ""
            if path.endswith("/failed"):
                return 200, {"status": "failed"}, ""
            return 500, None, "unexpected"

        with self.assertRaisesRegex(WorkerHttpError, "ERP inventory query"):
            pull_reconciliation_snapshot(
                settings(),
                "owner-1",
                "scheduled:owner-1:20260723T180000Z",
                claim=claim_value(),
                http_json_fn=http,
                erp_get_fn=lambda *_args: (_ for _ in ()).throw(
                    WorkerHttpError(503, "ERP inventory query", "unavailable")
                ),
            )
        failure = next(value for value in requests if value[1].endswith("/failed"))
        self.assertEqual(
            failure[2],
            {
                "claim_token": claim_value()["claim_token"],
                "stage": "pull",
                "error_code": "erp_pull_failed",
            },
        )

    def test_submit_failure_is_reported_separately(self) -> None:
        requests = []

        def http(_settings, method, path, body, key):
            requests.append((method, path, body, key))
            if path.startswith("/api/v1/config/erp-connectors/route-resolve?"):
                return (
                    200,
                    {
                        "connector": {
                            "id": settings().connector_id,
                            "owner_id": "owner-1",
                            "connector_code": "SELF-ERP",
                            "config_version": 2,
                            "channel_mode": "rest",
                            "api_base_url": "https://erp.test/h8",
                            "bearer_secret_alias": "vault://erp/owner-1",
                            "directions": ["outbound"],
                            "message_types": ["inventory_snapshot"],
                            "warehouse_ids": [],
                        }
                    },
                    "",
                )
            if path.endswith("/renew"):
                return 200, {"status": "active"}, ""
            if path.endswith("/failed"):
                return 200, {"status": "failed"}, ""
            if path == "/api/v1/reconciliation/runs":
                return 503, None, "submit unavailable"
            return 500, None, "unexpected"

        with self.assertRaisesRegex(WorkerHttpError, "WMS reconciliation"):
            pull_reconciliation_snapshot(
                settings(),
                "owner-1",
                "scheduled:owner-1:20260723T180000Z",
                claim=claim_value(),
                http_json_fn=http,
                erp_get_fn=lambda *_args: {
                    "snapshot_at": "2026-07-23T18:00:00Z",
                    "items": [],
                    "next_cursor": None,
                },
            )
        failure = next(value for value in requests if value[1].endswith("/failed"))
        self.assertEqual(failure[2]["stage"], "submit")
        self.assertEqual(failure[2]["error_code"], "snapshot_submit_failed")


if __name__ == "__main__":
    unittest.main()
