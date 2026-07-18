"""H8 出站：从 WMS PostgreSQL ERP outbox 投递到 MSSQL if_out_message。"""

from __future__ import annotations

import json
import os
import subprocess
from dataclasses import dataclass
from typing import Any, Callable

# table + 用作 external_ref 的列
OUTBOX_SOURCES: list[dict[str, str]] = [
    {
        "table": "receiving_putaway_erp_feedback_outbox",
        "ref_col": "receiving_order_id",
    },
    {
        "table": "inventory_status_erp_feedback_outbox",
        "ref_col": "batch_id",
    },
    {
        "table": "stock_adjustment_erp_feedback_outbox",
        "ref_col": "order_id",
    },
]


@dataclass
class OutboxRow:
    table: str
    id: str
    owner_id: str
    event_type: str
    payload: dict[str, Any]
    external_ref: str
    attempt_count: int


def psql_query(database_url: str, sql: str) -> str:
    """执行 SQL，返回对齐文本（-t -A -F|）。"""
    cmd = [
        "psql",
        database_url,
        "-v",
        "ON_ERROR_STOP=1",
        "-t",
        "-A",
        "-F",
        "|",
        "-c",
        sql,
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        raise RuntimeError(
            f"psql failed rc={proc.returncode}: {proc.stderr or proc.stdout}"
        )
    return proc.stdout


def sql_escape_pg(value: str) -> str:
    return value.replace("'", "''")


def sql_escape_mssql(value: str) -> str:
    return value.replace("'", "''")


def claim_wms_outbox(
    database_url: str, table: str, ref_col: str, batch_size: int
) -> list[OutboxRow]:
    """单语句认领 pending/failed outbox（FOR UPDATE SKIP LOCKED）。"""
    sql = f"""
WITH cte AS (
  SELECT id
    FROM {table}
   WHERE status IN ('pending', 'failed')
     AND next_attempt_at <= now()
   ORDER BY next_attempt_at ASC
   LIMIT {int(batch_size)}
   FOR UPDATE SKIP LOCKED
),
upd AS (
  UPDATE {table} o
     SET attempt_count = o.attempt_count + 1,
         updated_at = now()
    FROM cte
   WHERE o.id = cte.id
  RETURNING
    o.id::text AS id,
    o.owner_id::text AS owner_id,
    o.event_type AS event_type,
    o.payload::text AS payload,
    COALESCE(o.{ref_col}::text, '') AS external_ref,
    o.attempt_count::text AS attempt_count
)
SELECT id, owner_id, event_type, payload, external_ref, attempt_count FROM upd;
"""
    out = psql_query(database_url, sql)
    rows: list[OutboxRow] = []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        parts = line.split("|")
        if len(parts) < 6:
            continue
        try:
            payload = json.loads(parts[3]) if parts[3] else {}
        except json.JSONDecodeError:
            payload = {"raw": parts[3]}
        if not isinstance(payload, dict):
            payload = {"value": payload}
        rows.append(
            OutboxRow(
                table=table,
                id=parts[0],
                owner_id=parts[1],
                event_type=parts[2],
                payload=payload,
                external_ref=parts[4],
                attempt_count=int(parts[5] or "0"),
            )
        )
    return rows


def mark_wms_outbox(
    database_url: str,
    table: str,
    row_id: str,
    *,
    succeeded: bool,
    error: str | None = None,
) -> None:
    if succeeded:
        sql = f"""
UPDATE {table}
   SET status = 'succeeded',
       last_error = NULL,
       updated_at = now()
 WHERE id = '{sql_escape_pg(row_id)}'::uuid;
"""
    else:
        err = sql_escape_pg((error or "h8 publish failed")[:900])
        sql = f"""
UPDATE {table}
   SET status = 'failed',
       last_error = '{err}',
       next_attempt_at = now() + interval '5 minutes',
       updated_at = now()
 WHERE id = '{sql_escape_pg(row_id)}'::uuid;
"""
    psql_query(database_url, sql)


def insert_if_out_sql(row: OutboxRow) -> str:
    """生成插入 if_out_message 的 T-SQL（同源幂等）。"""
    idem = f"out:{row.table}:{row.id}"
    payload = sql_escape_mssql(json.dumps(row.payload, ensure_ascii=False))
    event = sql_escape_mssql(row.event_type)
    owner = sql_escape_mssql(row.owner_id)
    table = sql_escape_mssql(row.table)
    oid = sql_escape_mssql(row.id)
    ext = sql_escape_mssql(row.external_ref or "")
    idem_sql = sql_escape_mssql(idem)
    return f"""
SET NOCOUNT ON;
IF NOT EXISTS (
  SELECT 1 FROM dbo.if_out_message
   WHERE source_outbox_table = N'{table}' AND source_outbox_id = N'{oid}'
)
BEGIN
  INSERT INTO dbo.if_out_message (
    event_type, owner_id, source_outbox_table, source_outbox_id,
    external_ref, payload_json, sync_status, idempotency_key
  ) VALUES (
    N'{event}', '{owner}', N'{table}', N'{oid}',
    NULLIF(N'{ext}', N''), N'{payload}', N'pending', N'{idem_sql}'
  );
END
ELSE
BEGIN
  UPDATE dbo.if_out_message
     SET payload_json = N'{payload}',
         event_type = N'{event}',
         updated_at = SYSUTCDATETIME()
   WHERE source_outbox_table = N'{table}' AND source_outbox_id = N'{oid}'
     AND sync_status IN (N'pending', N'failed');
END
"""


def process_outbound_once(
    *,
    database_url: str,
    sqlcmd_exec: Callable[[str], str],
    batch_size: int,
    dry_run: bool,
) -> int:
    """投递所有已注册 outbox 源。"""
    processed = 0
    for src in OUTBOX_SOURCES:
        table = src["table"]
        ref_col = src["ref_col"]
        try:
            rows = claim_wms_outbox(database_url, table, ref_col, batch_size)
        except Exception as exc:  # noqa: BLE001
            print(f"[h8-out] skip claim {table}: {exc}", flush=True)
            continue
        for row in rows:
            processed += 1
            print(
                f"[h8-out] claim {table} id={row.id} event={row.event_type}",
                flush=True,
            )
            if dry_run:
                mark_wms_outbox(
                    database_url, table, row.id, succeeded=False, error="dry-run"
                )
                continue
            try:
                sqlcmd_exec(insert_if_out_sql(row))
                mark_wms_outbox(database_url, table, row.id, succeeded=True)
                print(f"[h8-out] published {table}/{row.id}", flush=True)
            except Exception as exc:  # noqa: BLE001
                mark_wms_outbox(
                    database_url, table, row.id, succeeded=False, error=str(exc)
                )
                print(f"[h8-out] error {table}/{row.id}: {exc}", flush=True)
    return processed


def resolve_wms_db_url() -> str | None:
    return os.environ.get("WMS_DB_URL") or os.environ.get("DATABASE_URL") or None
