"""US-H8-002 AC11：Worker 真实路径必须 emit 交换 lifecycle 阶段。"""

from __future__ import annotations

import unittest
from dataclasses import dataclass
from pathlib import Path
from types import SimpleNamespace
from typing import Any

from exchange_lifecycle import (
    H8_EXCHANGE_AUDIT_STAGES,
    is_exchange_audit_stage,
    record_preflight_failure,
    run_inbound_pipeline,
    run_outbound_pipeline,
)
from worker_route import WorkerHttpError


@dataclass
class FakeSettings:
    api_base: str = "http://127.0.0.1:9"
    api_token: str | None = "tok"


class TestExchangeLifecycle(unittest.TestCase):
    def test_lifecycle_audit_failure_blocks_business_handler(self) -> None:
        handler_called = False

        def failed_audit(*_args: Any) -> tuple[int, dict[str, Any] | None, str]:
            return 500, None, "audit unavailable"

        def handler(_settings: Any, _row: dict[str, str]) -> str:
            nonlocal handler_called
            handler_called = True
            return "unexpected"

        with self.assertRaises(WorkerHttpError):
            run_inbound_pipeline(
                FakeSettings(),
                "asn",
                {"id": "1", "external_doc_no": "D", "idempotency_key": "k"},
                handler,
                lambda _message_type, row, _binding: row,
                http_json=failed_audit,
            )
        self.assertFalse(handler_called)

    def test_stages_match_domain_catalog(self) -> None:
        self.assertEqual(
            list(H8_EXCHANGE_AUDIT_STAGES),
            [
                "receive",
                "convert",
                "business_api",
                "send",
                "receipt",
                "final_failure",
            ],
        )
        self.assertTrue(is_exchange_audit_stage("business_api"))
        self.assertFalse(is_exchange_audit_stage("free_text"))

    def test_inbound_pipeline_emits_stages_and_posts_lifecycle_api(self) -> None:
        posts: list[dict[str, Any]] = []

        def fake_http(
            settings: Any,
            method: str,
            path: str,
            body: dict[str, Any] | None,
            idem: str,
        ) -> tuple[int, dict[str, Any] | None, str]:
            posts.append({"method": method, "path": path, "body": body, "idem": idem})
            self.assertEqual(method, "POST")
            self.assertEqual(path, "/api/v1/integration/erp-messages/lifecycle")
            assert body is not None
            return 200, {"id": "00000000-0000-0000-0000-0000000000aa"}, "{}"

        def handler(_settings: Any, command: dict[str, str]) -> str:
            self.assertEqual(command, {"canonical_ref": "ERP-1"})
            return "wms-res-1"

        def convert(
            _message_type: str, raw: dict[str, str], _binding: Any
        ) -> dict[str, str]:
            return {"canonical_ref": raw["external_ref"]}

        row = {
            "id": "1",
            "external_doc_no": "DOC-1",
            "external_ref": "ERP-1",
            "idempotency_key": "idem-asn-1",
            "schema_version": "1",
        }
        wms_id, life = run_inbound_pipeline(
            FakeSettings(),
            "asn",
            row,
            handler,
            convert,
            http_json=fake_http,
            route_binding=SimpleNamespace(
                connector_id="connector-1",
                connector_code="SELF-ERP",
                config_version=3,
            ),
            dry_run=False,
        )
        self.assertEqual(wms_id, "wms-res-1")
        stages = [s for s, _ in life.stages_emitted]
        self.assertEqual(
            stages,
            ["receive", "convert", "business_api", "business_api", "receipt"],
        )
        # 每个 stage 都真实 POST 到 lifecycle 端点（非旁路假造）
        self.assertEqual(len(posts), 5)
        self.assertEqual(posts[0]["body"]["stage"], "receive")
        self.assertEqual(posts[0]["body"]["connector_id"], "connector-1")
        self.assertEqual(posts[0]["body"]["config_version"], 3)
        self.assertEqual(posts[0]["body"]["schema_version"], "1")
        self.assertEqual(posts[0]["body"]["payload"], row)
        self.assertTrue(all("payload" not in post["body"] for post in posts[1:]))
        self.assertEqual(posts[2]["body"]["stage"], "business_api")
        self.assertEqual(posts[2]["body"]["result"], "started")
        self.assertEqual(posts[3]["body"]["result"], "ok")
        self.assertEqual(posts[-1]["body"]["stage"], "receipt")
        self.assertEqual(life.message_id, "00000000-0000-0000-0000-0000000000aa")

    def test_inbound_failure_emits_final_failure(self) -> None:
        posts: list[dict[str, Any]] = []

        def fake_http(
            settings: Any,
            method: str,
            path: str,
            body: dict[str, Any] | None,
            idem: str,
        ) -> tuple[int, dict[str, Any] | None, str]:
            assert body is not None
            posts.append(body)
            return 200, {"id": "m1"}, "{}"

        def boom(_s: Any, _r: dict[str, str]) -> str:
            raise RuntimeError("api down")

        with self.assertRaises(RuntimeError):
            run_inbound_pipeline(
                FakeSettings(),
                "asn",
                {"id": "1", "external_doc_no": "D", "idempotency_key": "k"},
                boom,
                lambda _message_type, row, _binding: row,
                http_json=fake_http,
            )
        self.assertIn("final_failure", [post["stage"] for post in posts])
        self.assertEqual(posts[-1]["stage"], "final_failure")

    def test_preflight_failure_records_receive_and_final_failure_with_payload(
        self,
    ) -> None:
        posts: list[dict[str, Any]] = []

        def fake_http(
            settings: Any,
            method: str,
            path: str,
            body: dict[str, Any] | None,
            idem: str,
        ) -> tuple[int, dict[str, Any] | None, str]:
            assert body is not None
            posts.append(body)
            return 200, {"id": "preflight-message"}, "{}"

        row = {
            "id": "7",
            "external_ref": "ERP-BAD-SCHEMA-1",
            "idempotency_key": "idem-bad-schema-1",
            "schema_version": "999",
        }
        record_preflight_failure(
            FakeSettings(),
            "asn",
            row,
            "connector-1",
            http_json=fake_http,
        )

        self.assertEqual(
            [post["stage"] for post in posts],
            ["receive", "final_failure"],
        )
        self.assertEqual(posts[0]["payload"], row)
        self.assertNotIn("payload", posts[1])
        self.assertEqual(posts[1]["schema_version"], "999")

    def test_outbound_pipeline_emits_send_stages(self) -> None:
        posts: list[dict[str, Any]] = []

        def fake_http(
            settings: Any,
            method: str,
            path: str,
            body: dict[str, Any] | None,
            idem: str,
        ) -> tuple[int, dict[str, Any] | None, str]:
            assert body is not None
            posts.append(body)
            return 200, {"id": "out-1"}, "{}"

        def send() -> None:
            return None

        life = run_outbound_pipeline(
            FakeSettings(),
            "putaway_complete",
            "ref-1",
            "idem-out-1",
            send,
            http_json=fake_http,
            connector_id="connector-1",
            payload={"qty": 1},
        )
        self.assertEqual(
            [s for s, _ in life.stages_emitted],
            ["receive", "convert", "send", "send", "receipt"],
        )
        self.assertEqual(
            [post["stage"] for post in posts],
            ["receive", "convert", "send", "send", "receipt"],
        )
        self.assertEqual(posts[0]["payload"], {"qty": 1})
        self.assertEqual(posts[0]["connector_id"], "connector-1")
        self.assertTrue(all("payload" not in post for post in posts[1:]))

    def test_process_once_path_uses_inbound_pipeline(self) -> None:
        """process_once 真实调用 run_inbound_pipeline（非旁路）。"""
        import sync_worker

        # 确认 worker 模块绑定了 pipeline
        self.assertTrue(hasattr(sync_worker, "run_inbound_pipeline"))
        with open(sync_worker.__file__, encoding="utf-8") as fh:
            src = fh.read()
        self.assertIn("run_inbound_pipeline", src)
        self.assertIn("US-H8-002 AC11", src)
        # 出站真实路径同样绑定 lifecycle
        with open(
            Path(sync_worker.__file__).resolve().parent / "outbound_publish.py",
            encoding="utf-8",
        ) as fh:
            out_src = fh.read()
        self.assertIn("run_outbound_pipeline", out_src)
        self.assertIn("US-H8-002 AC11", out_src)


if __name__ == "__main__":
    unittest.main()
