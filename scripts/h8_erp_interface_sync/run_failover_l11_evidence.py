#!/usr/bin/env python3
"""US-H8-002 AC7：重复主备降级只保留一条真实 MSSQL 接口消息。"""

from __future__ import annotations

import argparse
import json
import os
import uuid
from datetime import datetime, timezone
from pathlib import Path
from types import SimpleNamespace
from typing import Any, Callable

from channel_failover import publish_with_failover
from outbound_publish import OUTBOX_SOURCES, OutboxRow, insert_if_out_sql
from worker_mssql import sqlcmd_query

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "docs" / "retros" / "h8-failover-l11-evidence.json"
OUTBOX_TABLE = "shipment_confirm_erp_feedback_outbox"


def settings_from_env() -> SimpleNamespace:
    return SimpleNamespace(
        mssql_host=os.environ.get("H8_MSSQL_HOST", "127.0.0.1"),
        mssql_port=os.environ.get("H8_MSSQL_PORT", "14333"),
        mssql_user=os.environ.get("H8_MSSQL_USER", "sa"),
        mssql_password=os.environ.get("H8_MSSQL_PASSWORD", "Wms_Erp_If_Dev_2026!"),
        mssql_database=os.environ.get("H8_MSSQL_DATABASE", "wms_erp_if"),
    )


def _count(output: str) -> int:
    for line in reversed(output.splitlines()):
        value = line.strip()
        if value.isdigit():
            return int(value)
    raise RuntimeError("MSSQL count result missing")


def collect(
    *,
    settings: Any | None = None,
    sqlcmd_fn: Callable[[Any, str], str] = sqlcmd_query,
    source_id: str | None = None,
    source: dict[str, str] | None = None,
) -> dict[str, Any]:
    settings = settings or settings_from_env()
    source_id = source_id or str(uuid.uuid4())
    source = source or next(
        item for item in OUTBOX_SOURCES if item["table"] == OUTBOX_TABLE
    )
    outbox_table = source["table"]
    message_type = source["message_type"]
    idempotency_key = f"out:{outbox_table}:{source_id}"
    row = OutboxRow(
        table=outbox_table,
        id=source_id,
        owner_id="00000000-0000-0000-0000-000000000001",
        event_type=message_type,
        payload={"business_id": source_id, "warehouse_id": None},
        external_ref=f"H8-L11-{source_id}",
        attempt_count=1,
        max_attempts=5,
        deadline_at=None,
        callback_path=source["callback_path"],
    )
    rest_attempts = 0
    table_attempts = 0
    channels: list[str] = []

    def fail_rest() -> None:
        nonlocal rest_attempts
        rest_attempts += 1
        raise RuntimeError("forced REST temporary unavailable")

    def write_table() -> None:
        nonlocal table_attempts
        table_attempts += 1
        sqlcmd_fn(settings, insert_if_out_sql(row))

    try:
        for _ in range(2):
            result = publish_with_failover(
                transport="failover",
                publish_http=fail_rest,
                publish_table=write_table,
                http_max_attempts=2,
            )
            channels.append(result.channel)
        interface_row_count = _count(
            sqlcmd_fn(
                settings,
                f"""
SET NOCOUNT ON;
SELECT COUNT(1)
  FROM dbo.if_out_message
 WHERE source_outbox_table = N'{outbox_table}'
   AND source_outbox_id = N'{source_id}'
   AND idempotency_key = N'{idempotency_key}';
""",
            )
        )
    finally:
        sqlcmd_fn(
            settings,
            f"""
DELETE FROM dbo.if_out_message
 WHERE source_outbox_table = N'{outbox_table}'
   AND source_outbox_id = N'{source_id}';
""",
        )

    return {
        "ok": (
            channels == ["table_fallback", "table_fallback"]
            and rest_attempts == 4
            and table_attempts == 2
            and interface_row_count == 1
        ),
        "message_type": message_type,
        "source_outbox_table": outbox_table,
        "source_outbox_id": source_id,
        "idempotency_key": idempotency_key,
        "channels": channels,
        "rest_attempts": rest_attempts,
        "table_attempts": table_attempts,
        "interface_row_count": interface_row_count,
        "cleanup": "deleted acceptance row",
    }


def collect_catalog(
    *,
    settings: Any | None = None,
    sqlcmd_fn: Callable[[Any, str], str] = sqlcmd_query,
) -> dict[str, Any]:
    settings = settings or settings_from_env()
    checks = [
        collect(settings=settings, sqlcmd_fn=sqlcmd_fn, source=source)
        for source in OUTBOX_SOURCES
    ]
    return {
        "ok": len(checks) == len(OUTBOX_SOURCES)
        and all(check["ok"] for check in checks),
        "message_checks": checks,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--record",
        action="store_true",
        help="明确写入 docs/retros 证据；默认只检查并输出",
    )
    args = parser.parse_args()
    try:
        check = collect_catalog()
        evidence = {
            "collected_at": datetime.now(timezone.utc).isoformat(),
            "story": "US-H8-002",
            "acceptance": "AC7 failover L11",
            "environment": "Docker MSSQL interface table",
            **check,
        }
    except Exception as exc:  # noqa: BLE001
        evidence = {
            "collected_at": datetime.now(timezone.utc).isoformat(),
            "story": "US-H8-002",
            "acceptance": "AC7 failover L11",
            "environment": "Docker MSSQL interface table",
            "ok": False,
            "error": str(exc)[:500],
        }
    print(json.dumps(evidence, ensure_ascii=False, indent=2))
    if args.record:
        EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
        EVIDENCE.write_text(
            json.dumps(evidence, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        print(f"wrote {EVIDENCE}")
    return 0 if evidence["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
