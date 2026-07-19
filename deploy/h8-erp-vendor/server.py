#!/usr/bin/env python3
"""容器内「外部 ERP 厂商」模拟端（通道 A REST 回执）。

能力：
- POST 业务回调 path → 持久化回执 JSONL + 内存索引
- GET /healthz、/receipts、/receipts/{source_outbox_id}
- 环境变量 ERP_FAIL_COUNT：启动后前 N 次 POST 返回 503（用于主备降级联调）
- 不落真实业务单据到 WMS；仅模拟 ERP 侧接受/拒绝
"""

from __future__ import annotations

import json
import os
import threading
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

HOST = os.environ.get("ERP_VENDOR_HOST", "0.0.0.0")
PORT = int(os.environ.get("ERP_VENDOR_PORT", "8080"))
DATA_DIR = Path(os.environ.get("ERP_VENDOR_DATA", "/data"))
LOG_PATH = DATA_DIR / "receipts.jsonl"
FAIL_COUNT = int(os.environ.get("ERP_FAIL_COUNT", "0"))
VENDOR_CODE = os.environ.get("ERP_VENDOR_CODE", "container-erp-vendor-a")
# 可选：设置后 /_admin/* 需带 Header X-ERP-Admin-Token
ADMIN_TOKEN = os.environ.get("ERP_VENDOR_ADMIN_TOKEN", "").strip()

_lock = threading.Lock()
_fail_remaining = FAIL_COUNT
_receipts: list[dict[str, Any]] = []


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _load_existing() -> None:
    if not LOG_PATH.is_file():
        return
    with LOG_PATH.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(obj, dict):
                _receipts.append(obj)


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt: str, *args: object) -> None:
        print(f"[erp-vendor] {fmt % args}", flush=True)

    def _json(self, code: int, body: dict[str, Any]) -> None:
        raw = json.dumps(body, ensure_ascii=False).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(raw)))
        self.send_header("X-ERP-Vendor", VENDOR_CODE)
        self.end_headers()
        self.wfile.write(raw)

    def _read_json(self) -> dict[str, Any]:
        length = int(self.headers.get("Content-Length") or "0")
        raw = self.rfile.read(length) if length else b"{}"
        try:
            data = json.loads(raw.decode("utf-8") or "{}")
        except json.JSONDecodeError:
            data = {"raw": raw.decode("utf-8", errors="replace")}
        return data if isinstance(data, dict) else {"value": data}

    def do_GET(self) -> None:  # noqa: N802
        path = urlparse(self.path).path
        if path in ("/", "/health", "/healthz"):
            with _lock:
                n = len(_receipts)
                fail = _fail_remaining
            self._json(
                200,
                {
                    "status": "ok",
                    "vendor": VENDOR_CODE,
                    "receipt_count": n,
                    "fail_remaining": fail,
                },
            )
            return
        if path == "/receipts":
            with _lock:
                items = list(_receipts[-200:])
            self._json(200, {"vendor": VENDOR_CODE, "receipts": items})
            return
        if path.startswith("/receipts/"):
            rid = path[len("/receipts/") :].strip("/")
            with _lock:
                matches = [
                    r
                    for r in _receipts
                    if str(r.get("source_outbox_id") or "") == rid
                    or str((r.get("body") or {}).get("source_outbox_id") or "") == rid
                ]
            if not matches:
                self._json(404, {"error": "receipt not found", "source_outbox_id": rid})
                return
            self._json(200, {"vendor": VENDOR_CODE, "receipts": matches})
            return
        self._json(404, {"error": "not found", "path": path})

    def do_POST(self) -> None:  # noqa: N802
        global _fail_remaining
        path = urlparse(self.path).path
        body = self._read_json()
        # 运维探针：不经业务 fail 计数（生产镜像务必配置 ERP_VENDOR_ADMIN_TOKEN）
        if path == "/_admin/fail-count":
            if ADMIN_TOKEN:
                got = (self.headers.get("X-ERP-Admin-Token") or "").strip()
                if got != ADMIN_TOKEN:
                    self._json(401, {"error": "admin token required"})
                    return
            with _lock:
                n = int(body.get("count") or 0)
                _fail_remaining = max(0, n)
                left = _fail_remaining
            self._json(200, {"ok": True, "fail_remaining": left})
            return
        with _lock:
            if _fail_remaining > 0:
                _fail_remaining -= 1
                left = _fail_remaining
                self._json(
                    503,
                    {
                        "accepted": False,
                        "vendor": VENDOR_CODE,
                        "error": "erp temporary unavailable",
                        "fail_remaining": left,
                    },
                )
                return
            receipt = {
                "vendor": VENDOR_CODE,
                "path": path,
                "received_at": _now(),
                "source_outbox_id": body.get("source_outbox_id"),
                "event_type": body.get("event_type"),
                "owner_id": body.get("owner_id"),
                "idempotency_hint": f"out:{body.get('source_outbox_table')}:{body.get('source_outbox_id')}",
                "body": body,
                "status": "accepted",
            }
            _receipts.append(receipt)
            DATA_DIR.mkdir(parents=True, exist_ok=True)
            with LOG_PATH.open("a", encoding="utf-8") as fh:
                fh.write(json.dumps(receipt, ensure_ascii=False) + "\n")
        self._json(
            200,
            {
                "accepted": True,
                "vendor": VENDOR_CODE,
                "path": path,
                "source_outbox_id": body.get("source_outbox_id"),
                "received_at": receipt["received_at"],
            },
        )


def main() -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    _load_existing()
    server = ThreadingHTTPServer((HOST, PORT), Handler)
    print(
        f"[erp-vendor] {VENDOR_CODE} listening on {HOST}:{PORT} "
        f"data={DATA_DIR} fail_count={FAIL_COUNT}",
        flush=True,
    )
    server.serve_forever()


if __name__ == "__main__":
    main()
