#!/usr/bin/env python3
"""通道 A 本地 ERP 回调 Mock。

接收 H8 worker HTTP 出站投递，写入 JSONL 便于验收。

用法：
  python3 scripts/h8_erp_interface_sync/channel_a_callback_mock.py --port 18091
  export ERP_CALLBACK_BASE=http://127.0.0.1:18091
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

RECEIVED: list[dict] = []
_LOG_PATH = Path("/tmp/h8-channel-a-callback.jsonl")


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt: str, *args: object) -> None:
        sys.stderr.write("[erp-mock] " + (fmt % args) + "\n")

    def _read_json(self) -> dict:
        length = int(self.headers.get("Content-Length") or "0")
        raw = self.rfile.read(length) if length else b"{}"
        try:
            data = json.loads(raw.decode("utf-8") or "{}")
        except json.JSONDecodeError:
            data = {"raw": raw.decode("utf-8", errors="replace")}
        if not isinstance(data, dict):
            data = {"value": data}
        return data

    def _ok(self, body: dict) -> None:
        payload = json.dumps(body).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self) -> None:  # noqa: N802
        if self.path in ("/health", "/healthz", "/"):
            self._ok({"status": "ok", "received": len(RECEIVED)})
            return
        if self.path == "/_dump":
            self._ok({"events": RECEIVED[-100:]})
            return
        self.send_error(404)

    def do_POST(self) -> None:  # noqa: N802
        data = self._read_json()
        event = {
            "path": self.path,
            "received_at": datetime.now(timezone.utc).isoformat(),
            "body": data,
        }
        RECEIVED.append(event)
        with _LOG_PATH.open("a", encoding="utf-8") as fh:
            fh.write(json.dumps(event, ensure_ascii=False) + "\n")
        print(
            f"[erp-mock] {self.path} event={data.get('event_type')} "
            f"source={data.get('source_outbox_id')}",
            flush=True,
        )
        self._ok({"accepted": True, "path": self.path})


def main(argv: list[str] | None = None) -> int:
    global _LOG_PATH
    parser = argparse.ArgumentParser(description="H8 channel A ERP callback mock")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=18091)
    parser.add_argument("--log", default=str(_LOG_PATH))
    args = parser.parse_args(argv)
    _LOG_PATH = Path(args.log)
    server = ThreadingHTTPServer((args.host, args.port), Handler)
    print(
        f"[erp-mock] listening http://{args.host}:{args.port} log={_LOG_PATH}",
        flush=True,
    )
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("[erp-mock] stop", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
