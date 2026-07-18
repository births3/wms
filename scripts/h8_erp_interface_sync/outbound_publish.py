"""H8 出站：WMS PG ERP outbox → 通道 B(if_out_message) 或 通道 A(HTTP 回调)。"""

from __future__ import annotations

import json
import os
import subprocess
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any, Callable

# table + external_ref 列 + 可选档案补录专用
OUTBOX_SOURCES: list[dict[str, str]] = [
    {
        "table": "receiving_putaway_erp_feedback_outbox",
        "ref_col": "receiving_order_id",
        "callback_path": "/inbound-complete",
    },
    {
        "table": "inventory_status_erp_feedback_outbox",
        "ref_col": "batch_id",
        "callback_path": "/inventory-status",
    },
    {
        "table": "stock_adjustment_erp_feedback_outbox",
        "ref_col": "order_id",
        "callback_path": "/stock-adjustment",
    },
    {
        "table": "archive_revision_erp_feedback_outbox",
        "ref_col": "liaison_id",
        "callback_path": "/archive-revision",
        "special_retry": "archive",
    },
    {
        "table": "reconciliation_erp_feedback_outbox",
        "ref_col": "recon_doc_no",
        "callback_path": "/reconciliation-diff",
    },
    {
        "table": "shipment_confirm_erp_feedback_outbox",
        "ref_col": "shipment_id",
        "callback_path": "/shipment-confirm",
    },
    {
        "table": "inventory_snapshot_erp_feedback_outbox",
        "ref_col": "snapshot_no",
        "callback_path": "/inventory-snapshot",
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
    max_attempts: int
    deadline_at: str | None
    callback_path: str


def psql_query(database_url: str, sql: str) -> str:
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


def table_has_column(database_url: str, table: str, column: str) -> bool:
    sql = f"""
SELECT 1 FROM information_schema.columns
 WHERE table_schema = 'public' AND table_name = '{sql_escape_pg(table)}'
   AND column_name = '{sql_escape_pg(column)}'
 LIMIT 1;
"""
    return bool(psql_query(database_url, sql).strip())


def claim_wms_outbox(
    database_url: str,
    table: str,
    ref_col: str,
    batch_size: int,
    *,
    callback_path: str,
    special_retry: str | None = None,
) -> list[OutboxRow]:
    """认领 pending/failed；档案补录尊重 deadline_at 与 max_attempts。"""
    if not table_has_column(database_url, table, "id"):
        return []

    has_max = table_has_column(database_url, table, "max_attempts")
    has_deadline = table_has_column(database_url, table, "deadline_at")
    has_ref = table_has_column(database_url, table, ref_col)

    ref_expr = (
        f"COALESCE(o.{ref_col}::text, '')"
        if has_ref
        else "''"
    )
    max_expr = "o.max_attempts::text" if has_max else "5"
    deadline_expr = (
        "COALESCE(o.deadline_at::text, '')" if has_deadline else "''"
    )

    extra_where = ""
    if special_retry == "archive" and has_deadline:
        extra_where += " AND o.deadline_at > now()"
    if has_max:
        extra_where += " AND o.attempt_count < o.max_attempts"

    # dead 标记：档案过 deadline 或超 max
    if special_retry == "archive" and has_deadline:
        psql_query(
            database_url,
            f"""
UPDATE {table}
   SET status = 'dead',
       last_error = COALESCE(last_error, 'archive_revision deadline exceeded'),
       updated_at = now()
 WHERE status IN ('pending', 'failed')
   AND (deadline_at <= now() OR attempt_count >= max_attempts);
""",
        )

    sql = f"""
WITH cte AS (
  SELECT id
    FROM {table} o
   WHERE status IN ('pending', 'failed')
     AND next_attempt_at <= now()
     {extra_where}
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
    {ref_expr} AS external_ref,
    o.attempt_count::text AS attempt_count,
    {max_expr} AS max_attempts,
    {deadline_expr} AS deadline_at
)
SELECT id, owner_id, event_type, payload, external_ref, attempt_count,
       max_attempts, deadline_at FROM upd;
"""
    out = psql_query(database_url, sql)
    rows: list[OutboxRow] = []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        parts = line.split("|")
        if len(parts) < 8:
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
                max_attempts=int(parts[6] or "5"),
                deadline_at=parts[7] or None,
                callback_path=callback_path,
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
    special_retry: str | None = None,
    attempt_count: int = 0,
    max_attempts: int = 5,
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
        # 档案补录：5 分钟退避；普通：5 分钟
        interval = "5 minutes"
        dead = ""
        if special_retry == "archive" and attempt_count >= max_attempts:
            dead = ", status = 'dead'"
        else:
            dead = ", status = 'failed'"
        sql = f"""
UPDATE {table}
   SET last_error = '{err}',
       next_attempt_at = now() + interval '{interval}',
       updated_at = now()
       {dead}
 WHERE id = '{sql_escape_pg(row_id)}'::uuid;
"""
    psql_query(database_url, sql)


def insert_if_out_sql(row: OutboxRow) -> str:
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


def http_callback_publish(base_url: str, row: OutboxRow) -> None:
    """通道 A：POST {base}{callback_path}。"""
    url = base_url.rstrip("/") + row.callback_path
    body = {
        "event_type": row.event_type,
        "owner_id": row.owner_id,
        "source_outbox_table": row.table,
        "source_outbox_id": row.id,
        "external_ref": row.external_ref or None,
        "payload": row.payload,
    }
    data = json.dumps(body).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=data,
        headers={"Content-Type": "application/json", "Accept": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            if resp.status >= 300:
                raw = resp.read().decode("utf-8", errors="replace")
                raise RuntimeError(f"callback HTTP {resp.status}: {raw[:300]}")
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"callback HTTP {exc.code}: {raw[:300]}") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"callback unreachable: {exc.reason}") from exc


def process_outbound_once(
    *,
    database_url: str,
    sqlcmd_exec: Callable[[str], str] | None,
    batch_size: int,
    dry_run: bool,
    transport: str = "table",
    callback_base: str | None = None,
) -> int:
    """
    transport:
      table — 通道 B，写入 MSSQL if_out_message（需 sqlcmd_exec）
      http  — 通道 A，POST ERP_CALLBACK_BASE
      both  — 先 table 再 http（双写联调）
    """
    processed = 0
    for src in OUTBOX_SOURCES:
        table = src["table"]
        ref_col = src["ref_col"]
        callback_path = src["callback_path"]
        special = src.get("special_retry")
        try:
            rows = claim_wms_outbox(
                database_url,
                table,
                ref_col,
                batch_size,
                callback_path=callback_path,
                special_retry=special,
            )
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
                    database_url,
                    table,
                    row.id,
                    succeeded=False,
                    error="dry-run",
                    special_retry=special,
                    attempt_count=row.attempt_count,
                    max_attempts=row.max_attempts,
                )
                continue
            try:
                if transport in ("table", "both"):
                    if sqlcmd_exec is None:
                        raise RuntimeError("sqlcmd_exec required for table transport")
                    sqlcmd_exec(insert_if_out_sql(row))
                if transport in ("http", "both"):
                    if not callback_base:
                        raise RuntimeError(
                            "ERP_CALLBACK_BASE required for http transport"
                        )
                    http_callback_publish(callback_base, row)
                mark_wms_outbox(database_url, table, row.id, succeeded=True)
                print(
                    f"[h8-out] published {table}/{row.id} via {transport}",
                    flush=True,
                )
            except Exception as exc:  # noqa: BLE001
                mark_wms_outbox(
                    database_url,
                    table,
                    row.id,
                    succeeded=False,
                    error=str(exc),
                    special_retry=special,
                    attempt_count=row.attempt_count,
                    max_attempts=row.max_attempts,
                )
                print(f"[h8-out] error {table}/{row.id}: {exc}", flush=True)
    return processed


def resolve_wms_db_url() -> str | None:
    return os.environ.get("WMS_DB_URL") or os.environ.get("DATABASE_URL") or None


def resolve_outbound_transport() -> str:
    return os.environ.get("H8_OUTBOUND_TRANSPORT", "table").strip().lower()


def resolve_callback_base() -> str | None:
    return os.environ.get("ERP_CALLBACK_BASE") or os.environ.get(
        "H8_ERP_CALLBACK_BASE"
    )
