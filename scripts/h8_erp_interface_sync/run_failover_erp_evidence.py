#!/usr/bin/env python3
"""本地主备降级 + ERP Mock 证据采集。

场景：
1) unit：HTTP 连续失败 → 接口表 fallback（无双写）
2) live-http：真实 ERP mock 接收成功
3) live-failover：ERP mock 拒绝 → 记录 fallback 决策（table 调用被模拟）

输出：docs/retros/h8-failover-runtime-evidence.json
"""

from __future__ import annotations

import json
import os
import sys
import threading
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from channel_failover import (  # noqa: E402
    map_channel_mode_to_transport,
    production_allows_simultaneous_dual_write,
    publish_with_failover,
)

EVIDENCE_PATH = ROOT / "docs" / "retros" / "h8-failover-runtime-evidence.json"


class _FailThenOkHandler(BaseHTTPRequestHandler):
    fail_remaining = 2
    received = 0

    def log_message(self, fmt: str, *args: object) -> None:
        return

    def do_POST(self) -> None:  # noqa: N802
        length = int(self.headers.get("Content-Length") or "0")
        _ = self.rfile.read(length) if length else b""
        if type(self).fail_remaining > 0:
            type(self).fail_remaining -= 1
            self.send_response(503)
            self.end_headers()
            self.wfile.write(b'{"error":"erp temporary unavailable"}')
            return
        type(self).received += 1
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(b'{"accepted":true}')

    def do_GET(self) -> None:  # noqa: N802
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"ok")


def _run_mock(port: int) -> ThreadingHTTPServer:
    server = ThreadingHTTPServer(("127.0.0.1", port), _FailThenOkHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


def scenario_unit_failover() -> dict:
    calls: list[str] = []

    def http() -> None:
        calls.append("http")
        raise RuntimeError("connection refused")

    def table() -> None:
        calls.append("table")

    result = publish_with_failover(
        transport="failover",
        publish_http=http,
        publish_table=table,
        http_max_attempts=2,
    )
    return {
        "name": "unit_http_fail_then_table",
        "ok": result.channel == "table_fallback" and calls == ["http", "http", "table"],
        "channel": result.channel,
        "calls": calls,
        "fallback_used": result.fallback_used,
        "idempotency_note": "if_out_message uses out:{table}:{id}; HTTP body carries same source_outbox_id",
    }


def scenario_live_http_then_recovery(port: int) -> dict:
    base = f"http://127.0.0.1:{port}"
    # first two fail, third succeeds
    _FailThenOkHandler.fail_remaining = 2
    _FailThenOkHandler.received = 0
    attempts = 0
    last_err = None
    for _ in range(4):
        attempts += 1
        try:
            req = urllib.request.Request(
                base + "/inbound-complete",
                data=json.dumps(
                    {
                        "event_type": "inbound_putaway_completed",
                        "source_outbox_id": "e2e-failover-1",
                        "payload": {"qty": 1},
                    }
                ).encode("utf-8"),
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=3) as resp:
                body = resp.read().decode("utf-8")
            return {
                "name": "live_erp_mock_recovery",
                "ok": True,
                "attempts": attempts,
                "response": body,
                "erp_accepted_count": _FailThenOkHandler.received,
            }
        except urllib.error.HTTPError as exc:
            last_err = f"HTTP {exc.code}"
            time.sleep(0.05)
        except Exception as exc:  # noqa: BLE001
            last_err = str(exc)
            time.sleep(0.05)
    return {
        "name": "live_erp_mock_recovery",
        "ok": False,
        "attempts": attempts,
        "error": last_err,
    }


def scenario_live_failover_to_table(port: int) -> dict:
    base = f"http://127.0.0.1:{port}"
    _FailThenOkHandler.fail_remaining = 99  # always fail REST
    table_payloads: list[str] = []

    def publish_http() -> None:
        req = urllib.request.Request(
            base + "/shipment-confirm",
            data=json.dumps(
                {
                    "event_type": "shipment_confirm",
                    "source_outbox_id": "e2e-failover-2",
                    "payload": {"shipment": "S1"},
                }
            ).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=3) as resp:
            _ = resp.read()

    def publish_table() -> None:
        # 模拟写入 if_out_message（幂等键 out:table:id）
        table_payloads.append("out:shipment_confirm_erp_feedback_outbox:e2e-failover-2")

    result = publish_with_failover(
        transport="failover",
        publish_http=publish_http,
        publish_table=publish_table,
        http_max_attempts=2,
    )
    return {
        "name": "live_rest_fail_table_fallback",
        "ok": result.channel == "table_fallback" and len(table_payloads) == 1,
        "channel": result.channel,
        "fallback_used": result.fallback_used,
        "table_idempotency_key": table_payloads[0] if table_payloads else None,
        "http_error": result.error,
        "not_dual_write": len(table_payloads) == 1 and result.fallback_used,
    }


def main() -> int:
    port = int(os.environ.get("H8_ERP_MOCK_PORT", "18191"))
    server = _run_mock(port)
    time.sleep(0.1)
    scenarios = [
        {
            "name": "channel_mode_mapping",
            "ok": map_channel_mode_to_transport("rest_primary_table_fallback")
            == "failover"
            and not production_allows_simultaneous_dual_write(
                "rest_primary_table_fallback"
            ),
            "transport": map_channel_mode_to_transport("rest_primary_table_fallback"),
        },
        scenario_unit_failover(),
        scenario_live_http_then_recovery(port),
        scenario_live_failover_to_table(port),
    ]
    server.shutdown()
    evidence = {
        "collected_at": datetime.now(timezone.utc).isoformat(),
        "story": "US-H8-001",
        "scope": "rest_primary_table_fallback runtime + local ERP mock",
        "environment": {
            "erp_mock": f"http://127.0.0.1:{port}",
            "note": "S4 real ERP host can replace mock; same Idempotency-Key on fallback",
        },
        "scenarios": scenarios,
        "ok": all(bool(s.get("ok")) for s in scenarios),
    }
    EVIDENCE_PATH.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE_PATH.write_text(
        json.dumps(evidence, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(evidence, ensure_ascii=False, indent=2))
    print(f"wrote {EVIDENCE_PATH}", file=sys.stderr)
    return 0 if evidence["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
