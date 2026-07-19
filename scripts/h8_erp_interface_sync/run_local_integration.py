#!/usr/bin/env python3
"""H8 本地联调：WMS outbox → 容器 ERP（A）/ 接口表（B）主备。

依赖：
- 容器 wms-h8-erp-vendor-a（18092）
- 容器 wms-mssql-erp-if（14333）
- DATABASE_URL / WMS_DB_URL 指向可写 PostgreSQL

输出：docs/retros/h8-local-integration-evidence.json
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
import uuid
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from channel_failover import publish_with_failover  # noqa: E402
from outbound_publish import (  # noqa: E402
    process_outbound_once,
    resolve_wms_db_url,
)
from sync_worker import Settings, sqlcmd_query  # noqa: E402

EVIDENCE = ROOT / "docs" / "retros" / "h8-local-integration-evidence.json"
VENDOR = os.environ.get("ERP_CALLBACK_BASE", "http://127.0.0.1:18092").rstrip("/")
OUTBOX = "receiving_putaway_erp_feedback_outbox"


def http_json(method: str, url: str, body: dict | None = None) -> tuple[int, dict]:
    data = None if body is None else json.dumps(body).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=data,
        method=method,
        headers={"Content-Type": "application/json", "Accept": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=8) as resp:
            raw = resp.read().decode("utf-8")
            return resp.status, (json.loads(raw) if raw else {})
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        try:
            parsed = json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            parsed = {"raw": raw}
        return exc.code, parsed
    except Exception as exc:  # noqa: BLE001
        return 0, {"error": str(exc)}


def psql(database_url: str, sql: str) -> str:
    proc = subprocess.run(
        [
            "psql",
            database_url,
            "-v",
            "ON_ERROR_STOP=1",
            "-t",
            "-A",
            "-c",
            sql,
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr or proc.stdout)
    return proc.stdout.strip()


def ensure_vendor() -> dict:
    code, body = http_json("GET", f"{VENDOR}/healthz")
    if code == 200 and body.get("status") == "ok":
        http_json("POST", f"{VENDOR}/_admin/fail-count", {"count": 0})
        return {"ok": True, "health": body}
    # try compose up
    proc = subprocess.run(
        [
            "docker",
            "compose",
            "-f",
            str(ROOT / "deploy" / "docker-compose.h8-erp-vendor.yml"),
            "up",
            "-d",
            "--build",
        ],
        cwd=str(ROOT / "deploy"),
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        return {"ok": False, "error": proc.stderr or proc.stdout}
    for _ in range(40):
        code, body = http_json("GET", f"{VENDOR}/healthz")
        if code == 200:
            http_json("POST", f"{VENDOR}/_admin/fail-count", {"count": 0})
            return {"ok": True, "health": body, "started": True}
        time.sleep(0.5)
    return {"ok": False, "error": "vendor timeout"}


def ensure_mssql(settings: Settings) -> dict:
    try:
        out = sqlcmd_query(settings, "SELECT DB_NAME()")
        # schema ensure via wait-and-init
        init = subprocess.run(
            ["bash", str(ROOT / "deploy" / "h8-erp-if" / "wait-and-init.sh")],
            capture_output=True,
            text=True,
            check=False,
        )
        return {
            "ok": init.returncode == 0,
            "db_name": out.strip().splitlines()[0] if out.strip() else None,
            "init_log": (init.stdout or "")[-400:],
            "init_err": (init.stderr or "")[-400:],
        }
    except Exception as exc:  # noqa: BLE001
        return {"ok": False, "error": str(exc)}


def ensure_outbox_table(database_url: str) -> None:
    # 联调用最小表（无 putaway FK），字段满足 claim_wms_outbox
    psql(
        database_url,
        f"""
CREATE TABLE IF NOT EXISTS {OUTBOX} (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL,
    receiving_order_id UUID,
    event_type TEXT NOT NULL DEFAULT 'inbound_putaway_completed',
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'failed', 'succeeded', 'dead')),
    attempt_count INT NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error TEXT,
    max_attempts INT NOT NULL DEFAULT 5,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
""",
    )


def seed_outbox(database_url: str, *, label: str) -> str:
    row_id = str(uuid.uuid4())
    owner = "00000000-0000-0000-0000-000000000001"
    rcv = str(uuid.uuid4())
    payload = json.dumps(
        {"label": label, "qty": 1, "product_code": "H8-LIAN"},
        ensure_ascii=False,
    ).replace("'", "''")
    psql(
        database_url,
        f"""
INSERT INTO {OUTBOX} (
  id, owner_id, receiving_order_id, event_type, payload, status, attempt_count, next_attempt_at
) VALUES (
  '{row_id}'::uuid,
  '{owner}'::uuid,
  '{rcv}'::uuid,
  'inbound_putaway_completed',
  '{payload}'::jsonb,
  'pending',
  0,
  now()
);
""",
    )
    return row_id


def outbox_status(database_url: str, row_id: str) -> dict:
    raw = psql(
        database_url,
        f"""
SELECT json_build_object(
  'id', id::text,
  'status', status,
  'attempt_count', attempt_count,
  'last_error', last_error
)::text
FROM {OUTBOX} WHERE id = '{row_id}'::uuid;
""",
    )
    return json.loads(raw) if raw else {}


def mssql_if_out_count(settings: Settings, source_id: str) -> int:
    sql = f"""
SET NOCOUNT ON;
SELECT COUNT(1) FROM dbo.if_out_message
 WHERE source_outbox_id = N'{source_id}';
"""
    out = sqlcmd_query(settings, sql).strip()
    # last numeric line
    for line in reversed(out.splitlines()):
        line = line.strip()
        if line.isdigit():
            return int(line)
    return 0


def main() -> int:
    database_url = resolve_wms_db_url()
    if not database_url:
        print("DATABASE_URL or WMS_DB_URL required", file=sys.stderr)
        return 2

    settings = Settings.from_env()
    steps: list[dict] = []

    v = ensure_vendor()
    steps.append({"step": "vendor_up", **v})
    if not v.get("ok"):
        return _write(steps, False)

    m = ensure_mssql(settings)
    steps.append({"step": "mssql_up", **m})
    if not m.get("ok"):
        return _write(steps, False)

    ensure_outbox_table(database_url)
    steps.append({"step": "outbox_table", "ok": True, "table": OUTBOX})

    # --- A: REST → 容器 ERP ---
    http_json("POST", f"{VENDOR}/_admin/fail-count", {"count": 0})
    row_http = seed_outbox(database_url, label="lian-http")
    n = process_outbound_once(
        database_url=database_url,
        sqlcmd_exec=lambda sql: sqlcmd_query(settings, sql),
        batch_size=10,
        dry_run=False,
        transport="http",
        callback_base=VENDOR,
    )
    st = outbox_status(database_url, row_http)
    rcode, rbody = http_json("GET", f"{VENDOR}/receipts/{row_http}")
    steps.append(
        {
            "step": "channel_a_http",
            "ok": bool(
                n >= 1
                and st.get("status") == "succeeded"
                and rcode == 200
                and len(rbody.get("receipts") or []) >= 1
            ),
            "processed": n,
            "outbox": st,
            "vendor_receipts": rbody,
            "source_outbox_id": row_http,
        }
    )

    # --- failover: ERP 503 → 接口表 ---
    http_json("POST", f"{VENDOR}/_admin/fail-count", {"count": 5})
    row_fb = seed_outbox(database_url, label="lian-failover")
    n2 = process_outbound_once(
        database_url=database_url,
        sqlcmd_exec=lambda sql: sqlcmd_query(settings, sql),
        batch_size=10,
        dry_run=False,
        transport="failover",
        callback_base=VENDOR,
        http_max_attempts=2,
    )
    st2 = outbox_status(database_url, row_fb)
    cnt = mssql_if_out_count(settings, row_fb)
    http_json("POST", f"{VENDOR}/_admin/fail-count", {"count": 0})
    steps.append(
        {
            "step": "channel_failover_to_table",
            "ok": bool(n2 >= 1 and st2.get("status") == "succeeded" and cnt >= 1),
            "processed": n2,
            "outbox": st2,
            "if_out_message_count": cnt,
            "source_outbox_id": row_fb,
            "note": "REST 失败后写入 dbo.if_out_message，幂等键 out:table:id",
        }
    )

    # --- pure table path ---
    row_t = seed_outbox(database_url, label="lian-table")
    n3 = process_outbound_once(
        database_url=database_url,
        sqlcmd_exec=lambda sql: sqlcmd_query(settings, sql),
        batch_size=10,
        dry_run=False,
        transport="table",
        callback_base=None,
    )
    st3 = outbox_status(database_url, row_t)
    cnt3 = mssql_if_out_count(settings, row_t)
    steps.append(
        {
            "step": "channel_b_table",
            "ok": bool(n3 >= 1 and st3.get("status") == "succeeded" and cnt3 >= 1),
            "processed": n3,
            "outbox": st3,
            "if_out_message_count": cnt3,
            "source_outbox_id": row_t,
        }
    )

    ok = all(bool(s.get("ok")) for s in steps)
    # 证据不落明文连接串
    safe_db = database_url.split("@")[-1] if "@" in database_url else "(local)"
    return _write(steps, ok, database_host=safe_db, vendor=VENDOR)


def _write(steps: list[dict], ok: bool, **extra) -> int:
    evidence = {
        "collected_at": datetime.now(timezone.utc).isoformat(),
        "story": "US-H8-001",
        "scope": "local integration: outbox + container ERP vendor + MSSQL interface table",
        "ok": ok,
        "steps": steps,
        **extra,
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(
        json.dumps(evidence, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(evidence, ensure_ascii=False, indent=2))
    print(f"wrote {EVIDENCE}", file=sys.stderr)
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
