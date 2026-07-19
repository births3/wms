#!/usr/bin/env python3
"""对容器化 ERP 厂商做 S4 风格证据采集。

前置：
  cd deploy && docker compose -f docker-compose.h8-erp-vendor.yml up -d --build

场景：
  1) 健康检查与厂商标识
  2) REST 出站成功回执
  3) fail-count 后主备：HTTP 503 → table fallback（幂等键）
  4) 查询厂商 /receipts/{id} 回执

输出：docs/retros/h8-container-erp-s4-evidence.json
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from channel_failover import publish_with_failover  # noqa: E402

EVIDENCE = ROOT / "docs" / "retros" / "h8-container-erp-s4-evidence.json"
COMPOSE = ROOT / "deploy" / "docker-compose.h8-erp-vendor.yml"
BASE = os.environ.get("ERP_CALLBACK_BASE", "http://127.0.0.1:18092").rstrip("/")
# 避免误用本机旧 channel_a mock 端口


def http_json(method: str, url: str, body: dict | None = None, timeout: float = 5.0) -> tuple[int, dict]:
    data = None if body is None else json.dumps(body).encode("utf-8")
    headers = {"Content-Type": "application/json", "Accept": "application/json"}
    admin = os.environ.get("ERP_VENDOR_ADMIN_TOKEN", "").strip()
    if admin and "/_admin/" in url:
        headers["X-ERP-Admin-Token"] = admin
    req = urllib.request.Request(
        url,
        data=data,
        method=method,
        headers=headers,
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read().decode("utf-8")
            return resp.status, json.loads(raw) if raw else {}
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        try:
            parsed = json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            parsed = {"raw": raw}
        return exc.code, parsed
    except (urllib.error.URLError, ConnectionError, TimeoutError, OSError) as exc:
        return 0, {"error": str(exc)}


def ensure_vendor_up() -> dict:
    # 健康检查；失败则尝试 compose up
    code, body = http_json("GET", f"{BASE}/healthz")
    if code == 200 and body.get("status") == "ok":
        return {"ok": True, "health": body, "started": False}
    cmd = [
        "docker",
        "compose",
        "-f",
        str(COMPOSE),
        "up",
        "-d",
        "--build",
    ]
    proc = subprocess.run(cmd, cwd=str(ROOT / "deploy"), capture_output=True, text=True)
    if proc.returncode != 0:
        return {
            "ok": False,
            "error": proc.stderr or proc.stdout,
            "cmd": " ".join(cmd),
        }
    for _ in range(30):
        code, body = http_json("GET", f"{BASE}/healthz")
        if code == 200 and body.get("status") == "ok":
            return {"ok": True, "health": body, "started": True}
        time.sleep(1)
    return {"ok": False, "error": "vendor health timeout", "last": body}


def scenario_rest_receipt() -> dict:
    source_id = f"s4-rest-{int(time.time())}"
    payload = {
        "event_type": "inbound_putaway_completed",
        "owner_id": "00000000-0000-0000-0000-000000000001",
        "source_outbox_table": "receiving_putaway_erp_feedback_outbox",
        "source_outbox_id": source_id,
        "external_ref": "RCV-S4-1",
        "payload": {"qty": 2, "product_code": "P-S4"},
    }
    code, body = http_json("POST", f"{BASE}/inbound-complete", payload)
    if code != 200 or not body.get("accepted"):
        return {"name": "rest_receipt", "ok": False, "status": code, "body": body}
    # 厂商回执查询
    rcode, rbody = http_json("GET", f"{BASE}/receipts/{source_id}")
    ok = bool(
        rcode == 200
        and isinstance(rbody.get("receipts"), list)
        and len(rbody["receipts"]) >= 1
        and rbody.get("vendor")
    )
    return {
        "name": "rest_receipt",
        "ok": ok,
        "source_outbox_id": source_id,
        "post": body,
        "receipt_query": rbody,
        "vendor": rbody.get("vendor"),
    }


def scenario_failover_against_vendor() -> dict:
    """对厂商容器注入 fail-count，再走 table fallback（模拟 if_out 写入）。"""
    # 确保厂商在线
    code, health = http_json("GET", f"{BASE}/healthz")
    if code != 200:
        boot = ensure_vendor_up()
        if not boot.get("ok"):
            return {"name": "failover_vendor", "ok": False, "error": "vendor down", "boot": boot}

    # 动态注入：随后 3 次业务 POST 返回 503（无需重建镜像）
    acode, abody = http_json("POST", f"{BASE}/_admin/fail-count", {"count": 3})
    if acode != 200 or int(abody.get("fail_remaining") or 0) < 1:
        return {
            "name": "failover_vendor",
            "ok": False,
            "error": "fail-count admin inject failed",
            "admin": abody,
        }

    source_id = f"s4-fb-{int(time.time())}"
    table_keys: list[str] = []

    def publish_http() -> None:
        payload = {
            "event_type": "shipment_confirm",
            "owner_id": "00000000-0000-0000-0000-000000000001",
            "source_outbox_table": "shipment_confirm_erp_feedback_outbox",
            "source_outbox_id": source_id,
            "payload": {"shipment": "S-S4"},
        }
        code, body = http_json("POST", f"{BASE}/shipment-confirm", payload)
        if code == 0 or code >= 300:
            raise RuntimeError(f"HTTP {code}: {body}")

    def publish_table() -> None:
        table_keys.append(
            f"out:shipment_confirm_erp_feedback_outbox:{source_id}"
        )

    result = publish_with_failover(
        transport="failover",
        publish_http=publish_http,
        publish_table=publish_table,
        http_max_attempts=2,
    )
    # 清零 fail 计数
    http_json("POST", f"{BASE}/_admin/fail-count", {"count": 0})
    return {
        "name": "failover_vendor",
        "ok": bool(
            result.channel == "table_fallback"
            and result.fallback_used
            and len(table_keys) == 1
        ),
        "channel": result.channel,
        "fallback_used": result.fallback_used,
        "table_idempotency_key": table_keys[0] if table_keys else None,
        "http_error": result.error,
        "not_dual_write": len(table_keys) == 1,
        "source_outbox_id": source_id,
    }


def main() -> int:
    scenarios: list[dict] = []
    boot = ensure_vendor_up()
    scenarios.append({"name": "vendor_up", **boot})
    if not boot.get("ok"):
        evidence = {
            "collected_at": datetime.now(timezone.utc).isoformat(),
            "story": "US-H8-001",
            "scope": "container external ERP vendor S4-style",
            "erp_callback_base": BASE,
            "scenarios": scenarios,
            "ok": False,
        }
        EVIDENCE.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(evidence, ensure_ascii=False, indent=2))
        return 1

    # 清空残留 fail 注入，保证首轮回执成功
    http_json("POST", f"{BASE}/_admin/fail-count", {"count": 0})
    scenarios.append(scenario_rest_receipt())
    scenarios.append(scenario_failover_against_vendor())
    # 恢复后再次成功回执
    http_json("POST", f"{BASE}/_admin/fail-count", {"count": 0})
    scenarios.append(scenario_rest_receipt())

    evidence = {
        "collected_at": datetime.now(timezone.utc).isoformat(),
        "story": "US-H8-001",
        "scope": "container external ERP vendor S4-style",
        "erp_callback_base": BASE,
        "compose": str(COMPOSE.relative_to(ROOT)),
        "vendor_image": "wms-h8-erp-vendor:local",
        "scenarios": scenarios,
        "ok": all(bool(s.get("ok")) for s in scenarios),
        "note": (
            "本证据使用容器化 ERP 厂商模拟端，具备独立进程/网络端口/持久化回执；"
            "可替换为真实厂商 URL（ERP_CALLBACK_BASE）而不改 worker 契约。"
        ),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(evidence, ensure_ascii=False, indent=2))
    print(f"wrote {EVIDENCE}", file=sys.stderr)
    return 0 if evidence["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
