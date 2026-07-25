"""H8 出站：WMS PG ERP outbox → 通道 B(if_out_message) 或 通道 A(HTTP 回调)。"""

from __future__ import annotations

import json
import os
import subprocess
import urllib.error
import urllib.request
import uuid
from dataclasses import dataclass
from typing import Any, Callable

from outbound_receipts import (
    process_receipt_timeouts,
    process_table_receipts,
    requeue_outbox,
)
from worker_route import (
    WorkerHttpError,
    resolve_bearer_token,
    resolve_existing_outbound_binding,
    resolve_outbound_route,
    sanitize_worker_error,
)
from worker_mssql import list_acked_outbound, mark_outbound_receipt_recorded

# table + external_ref 列 + 可选档案补录专用
# message_type 与 US-H8-002 受控出站目录对齐
OUTBOX_SOURCES: list[dict[str, str]] = [
    {
        "table": "receiving_putaway_erp_feedback_outbox",
        "ref_col": "receiving_order_id",
        "callback_path": "/inbound-complete",
        "message_type": "putaway_complete",
    },
    {
        "table": "inventory_status_erp_feedback_outbox",
        "ref_col": "batch_id",
        "callback_path": "/inventory-status",
        "message_type": "inventory_status",
    },
    {
        "table": "stock_adjustment_erp_feedback_outbox",
        "ref_col": "order_id",
        "callback_path": "/stock-adjustment",
        "message_type": "stock_adjustment",
    },
    {
        "table": "archive_revision_erp_feedback_outbox",
        "ref_col": "liaison_id",
        "callback_path": "/archive-revision",
        "special_retry": "archive",
        "message_type": "archive_revision",
    },
    {
        "table": "reconciliation_erp_feedback_outbox",
        "ref_col": "recon_doc_no",
        "callback_path": "/reconciliation-diff",
        "special_retry": "bounded",
        "message_type": "reconciliation_diff",
    },
    {
        "table": "shipment_confirm_erp_feedback_outbox",
        "ref_col": "shipment_id",
        "callback_path": "/shipment-confirm",
        "message_type": "shipment_confirm",
    },
    {
        "table": "inventory_snapshot_erp_feedback_outbox",
        "ref_col": "snapshot_no",
        "callback_path": "/inventory-snapshot",
        "message_type": "inventory_snapshot",
    },
]

H8_OUTBOUND_CATALOG = {
    "putaway_complete",
    "inventory_status",
    "stock_adjustment",
    "archive_revision",
    "reconciliation_diff",
    "shipment_confirm",
    "inventory_snapshot",
}


def outbox_message_types() -> list[str]:
    return [src["message_type"] for src in OUTBOX_SOURCES if "message_type" in src]


def catalog_covers_outbox_sources() -> bool:
    types = set(outbox_message_types())
    return types == H8_OUTBOUND_CATALOG


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
    connector_id: str | None = None,
    message_type: str | None = None,
) -> list[OutboxRow]:
    """认领 pending/failed；档案补录尊重 deadline_at 与 max_attempts。"""
    if not table_has_column(database_url, table, "id"):
        return []

    has_max = table_has_column(database_url, table, "max_attempts")
    has_deadline = table_has_column(database_url, table, "deadline_at")
    has_ref = table_has_column(database_url, table, ref_col)

    ref_expr = f"COALESCE(o.{ref_col}::text, '')" if has_ref else "''"
    max_expr = "o.max_attempts" if has_max else "5"
    deadline_expr = (
        "to_char(o.deadline_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')"
        if has_deadline
        else "NULL"
    )

    extra_where = ""
    if special_retry == "archive" and has_deadline:
        extra_where += " AND o.deadline_at > now()"
    if has_max:
        extra_where += " AND o.attempt_count < o.max_attempts"
    if connector_id:
        if not message_type:
            raise ValueError("message_type required with connector_id")
        connector_sql = sql_escape_pg(connector_id)
        message_type_sql = sql_escape_pg(message_type)
        idempotency_expr = (
            f"'out:{sql_escape_pg(table)}:' || o.id::text"
        )
        extra_where += f"""
 AND (
   EXISTS (
     SELECT 1
       FROM h8_erp_messages message
      WHERE message.owner_id = o.owner_id
        AND message.direction = 'outbound'
        AND message.message_type = '{message_type_sql}'
        AND message.idempotency_key = {idempotency_expr}
        AND message.connector_id = '{connector_sql}'::uuid
   )
   OR (
     NOT EXISTS (
       SELECT 1
         FROM h8_erp_messages message
        WHERE message.owner_id = o.owner_id
          AND message.idempotency_key = {idempotency_expr}
     )
     AND o.owner_id = (
       SELECT owner_id
         FROM h8_erp_connectors
        WHERE id = '{connector_sql}'::uuid
          AND status = 'active'
          AND 'outbound' = ANY(directions)
          AND '{message_type_sql}' = ANY(message_types)
          AND (
            cardinality(warehouse_ids) = 0
            OR EXISTS (
              SELECT 1
                FROM unnest(warehouse_ids) AS route_warehouse_id
               WHERE route_warehouse_id::text = o.payload ->> 'warehouse_id'
            )
          )
     )
   )
 )"""

    # Worker 在认领后崩溃时，下一轮必须先收口已耗尽的有界重试行。
    if special_retry == "bounded" and has_max:
        psql_query(
            database_url,
            f"""
WITH exhausted AS (
  UPDATE {table}
     SET status = 'dead',
         last_error = COALESCE(last_error, 'outbound retry exhausted'),
         next_attempt_at = now(),
         updated_at = now()
   WHERE status IN ('pending', 'failed')
     AND attempt_count >= max_attempts
   RETURNING id, owner_id
)
INSERT INTO h4_notification_records
  (id, owner_id, event_type, dedupe_key, recipient, channel, content,
   content_summary, status, failure_reason, created_at, updated_at)
SELECT gen_random_uuid(), owner_id, 'rc.reconciliation.erp_feedback_dead',
       id::text, 'warehouse_manager', 'wechat',
       '库存对账反馈 ERP 重试耗尽，请检查连接与差异处理状态',
       '库存对账反馈 ERP 重试耗尽，请检查连接与差异处理状态',
       'retrying', 'awaiting_wechat_delivery', now(), now()
  FROM exhausted
ON CONFLICT (owner_id, event_type, recipient, dedupe_key) DO NOTHING;
""",
        )

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

    # 用 json_build_object 单行 JSON，避免 payload 含 | 时字段切割错误
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
         next_attempt_at = now() + interval '5 minutes',
         updated_at = now()
    FROM cte
   WHERE o.id = cte.id
  RETURNING
    o.id::text AS id,
    o.owner_id::text AS owner_id,
    o.event_type AS event_type,
    o.payload AS payload,
    {ref_expr} AS external_ref,
    o.attempt_count AS attempt_count,
    ({max_expr})::int AS max_attempts,
    {deadline_expr} AS deadline_at
)
SELECT json_build_object(
  'id', id,
  'owner_id', owner_id,
  'event_type', event_type,
  'payload', payload,
  'external_ref', external_ref,
  'attempt_count', attempt_count,
  'max_attempts', max_attempts,
  'deadline_at', deadline_at
)::text
FROM upd;
"""
    out = psql_query(database_url, sql)
    rows: list[OutboxRow] = []
    for line in out.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(obj, dict):
            continue
        payload = obj.get("payload") or {}
        if isinstance(payload, str):
            try:
                payload = json.loads(payload)
            except json.JSONDecodeError:
                payload = {"raw": payload}
        if not isinstance(payload, dict):
            payload = {"value": payload}
        rows.append(
            OutboxRow(
                table=table,
                id=str(obj.get("id") or ""),
                owner_id=str(obj.get("owner_id") or ""),
                event_type=str(obj.get("event_type") or ""),
                payload=payload,
                external_ref=str(obj.get("external_ref") or ""),
                attempt_count=int(obj.get("attempt_count") or 0),
                max_attempts=int(obj.get("max_attempts") or 5),
                deadline_at=(
                    str(obj["deadline_at"]) if obj.get("deadline_at") else None
                ),
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
    attempt_count: int,
    error: str | None = None,
    special_retry: str | None = None,
    max_attempts: int = 5,
    release_dry_run: bool = False,
) -> None:
    if release_dry_run:
        # 认领已 +1 attempt，dry-run 回退并保持 pending
        sql = f"""
UPDATE {table}
   SET status = 'pending',
       attempt_count = GREATEST(attempt_count - 1, 0),
       last_error = NULL,
       next_attempt_at = now(),
       updated_at = now()
 WHERE id = '{sql_escape_pg(row_id)}'::uuid
   AND attempt_count = {int(attempt_count)};
"""
        psql_query(database_url, sql)
        return
    if succeeded:
        sql = f"""
UPDATE {table}
   SET status = 'succeeded',
       last_error = NULL,
       updated_at = now()
 WHERE id = '{sql_escape_pg(row_id)}'::uuid
   AND attempt_count = {int(attempt_count)};
"""
    else:
        err = sql_escape_pg((error or "h8 publish failed")[:900])
        exhausted = special_retry in {"archive", "bounded"} and attempt_count >= max_attempts
        if exhausted:
            if special_retry == "bounded":
                sql = f"""
WITH exhausted AS (
  UPDATE {table}
     SET status = 'dead',
         last_error = '{err}',
         next_attempt_at = now(),
         updated_at = now()
   WHERE id = '{sql_escape_pg(row_id)}'::uuid
     AND attempt_count = {int(attempt_count)}
   RETURNING id, owner_id
)
INSERT INTO h4_notification_records
  (id, owner_id, event_type, dedupe_key, recipient, channel, content,
   content_summary, status, failure_reason, created_at, updated_at)
SELECT gen_random_uuid(), owner_id, 'rc.reconciliation.erp_feedback_dead',
       id::text, 'warehouse_manager', 'wechat',
       '库存对账反馈 ERP 重试耗尽，请检查连接与差异处理状态',
       '库存对账反馈 ERP 重试耗尽，请检查连接与差异处理状态',
       'retrying', 'awaiting_wechat_delivery', now(), now()
  FROM exhausted
ON CONFLICT (owner_id, event_type, recipient, dedupe_key) DO NOTHING;
"""
                psql_query(database_url, sql)
                return
            status_sql = "status = 'dead'"
            next_attempt_sql = "now()"
        else:
            status_sql = "status = 'failed'"
            next_attempt_sql = "now() + interval '5 minutes'"
        sql = f"""
UPDATE {table}
   SET last_error = '{err}',
       next_attempt_at = {next_attempt_sql},
       updated_at = now(),
       {status_sql}
 WHERE id = '{sql_escape_pg(row_id)}'::uuid
   AND attempt_count = {int(attempt_count)};
"""
    psql_query(database_url, sql)


def requeue_wms_outbox(database_url: str, idempotency_key: str) -> None:
    requeue_outbox(
        database_url,
        idempotency_key,
        allowed_tables={source["table"] for source in OUTBOX_SOURCES},
        query_fn=psql_query,
    )


def process_outbound_receipt_timeouts(
    settings: Any,
    database_url: str,
    *,
    http_json_fn: Callable[..., tuple[int, Any, str]],
) -> int:
    return process_receipt_timeouts(
        settings,
        database_url,
        http_json_fn=http_json_fn,
        requeue_fn=requeue_wms_outbox,
    )


def process_outbound_receipts(
    settings: Any,
    *,
    http_json_fn: Callable[..., tuple[int, Any, str]],
) -> int:
    return process_table_receipts(
        settings,
        http_json_fn=http_json_fn,
        list_acked_fn=list_acked_outbound,
        mark_recorded_fn=mark_outbound_receipt_recorded,
    )


def insert_if_out_sql(row: OutboxRow) -> str:
    idem = f"out:{row.table}:{row.id}"
    # 单行 JSON，避免 T-SQL 字面量换行
    payload_raw = json.dumps(row.payload, ensure_ascii=False, separators=(",", ":"))
    payload = sql_escape_mssql(payload_raw)
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
    external_ref, schema_version, payload_json, sync_status, idempotency_key
  ) VALUES (
    N'{event}', '{owner}', N'{table}', N'{oid}',
    NULLIF(N'{ext}', N''), N'1', N'{payload}', N'pending', N'{idem_sql}'
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


def http_callback_publish(
    base_url: str,
    row: OutboxRow,
    lifecycle: Any,
    bearer_token: str | None,
) -> None:
    """通道 A：POST {base}{callback_path}。"""
    if not lifecycle.message_id:
        raise RuntimeError("H8 message binding required before ERP callback")
    url = base_url.rstrip("/") + row.callback_path
    body = {
        "message_id": lifecycle.message_id,
        "schema_version": lifecycle.schema_version,
        "correlation_id": lifecycle.correlation_id,
        "idempotency_key": lifecycle.idempotency_key,
        "connector_id": lifecycle.connector_id,
        "config_version": lifecycle.config_version,
        "event_type": row.event_type,
        "owner_id": row.owner_id,
        "source_outbox_table": row.table,
        "source_outbox_id": row.id,
        "external_ref": row.external_ref or None,
        "payload": row.payload,
    }
    data = json.dumps(body).encode("utf-8")
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
        "Idempotency-Key": lifecycle.idempotency_key,
    }
    if bearer_token:
        headers["Authorization"] = f"Bearer {bearer_token}"
    req = urllib.request.Request(
        url,
        data=data,
        headers=headers,
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
    http_max_attempts: int | None = None,
    connector_id: str | None = None,
    settings: Any | None = None,
    http_json_fn: Callable[..., tuple[int, dict[str, Any] | None, str]] | None = None,
) -> int:
    """
    传入 settings 时生产通道由逐消息 route-resolve 决定；以下参数只供本地证据脚本：
      table    — 通道 B，写入 MSSQL if_out_message（需 sqlcmd_exec）
      http     — 通道 A，POST callback_base
      both     — 先 table 再 http（本地双写联调）
      failover — REST 主用失败后转接口表（rest_primary_table_fallback）
    """
    try:
        from channel_failover import publish_with_failover
    except ImportError:  # 允许从仓库根以 package 路径运行
        from scripts.h8_erp_interface_sync.channel_failover import (  # type: ignore
            publish_with_failover,
        )

    attempts = http_max_attempts
    if attempts is None:
        attempts = int(os.environ.get("H8_HTTP_MAX_ATTEMPTS", "2"))
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
                connector_id=connector_id,
                message_type=src.get("message_type"),
            )
        except Exception as exc:  # noqa: BLE001
            summary = sanitize_worker_error(
                str(exc),
                (
                    database_url,
                    os.environ.get("WMS_API_TOKEN"),
                    getattr(settings, "mssql_password", None),
                ),
            )
            print(f"[h8-out] skip claim {table}: {summary}", flush=True)
            continue
        for row in rows:
            processed += 1
            print(
                f"[h8-out] claim {table} id={row.id} event={row.event_type}",
                flush=True,
            )
            msg_type = src.get("message_type") or "putaway_complete"
            idem = f"out:{table}:{row.id}"
            route_binding = None
            active_transport = transport
            active_callback_base = callback_base
            lifecycle_channel = "interface_table" if transport == "table" else "rest"
            life_settings = type(
                "LifeSettings",
                (),
                {
                    "api_base": (
                        settings.api_base
                        if settings is not None
                        else os.environ.get("WMS_API_BASE", "http://127.0.0.1:8080")
                    ),
                    "api_token": (
                        settings.api_token
                        if settings is not None
                        else os.environ.get("WMS_API_TOKEN")
                    ),
                },
            )()
            if dry_run:
                try:
                    from exchange_lifecycle import run_outbound_pipeline

                    run_outbound_pipeline(
                        life_settings,
                        msg_type,
                        str(row.external_ref or row.id),
                        idem,
                        lambda _lifecycle: None,
                        connector_id=connector_id,
                        route_binding=route_binding,
                        channel=lifecycle_channel,
                        payload=row.payload,
                        dry_run=True,
                    )
                except Exception as life_exc:  # noqa: BLE001
                    print(f"[h8-out] lifecycle dry-run warn: {life_exc}", flush=True)
                mark_wms_outbox(
                    database_url,
                    table,
                    row.id,
                    succeeded=False,
                    attempt_count=row.attempt_count,
                    release_dry_run=True,
                )
                print(f"[h8-out] dry-run release {table}/{row.id}", flush=True)
                continue
            try:
                if settings is not None:
                    route_http = http_json_fn
                    if route_http is None:
                        from sync_worker import http_json as route_http

                    warehouse_id = (
                        str(row.payload.get("warehouse_id") or "").strip() or None
                    )
                    route_binding = (
                        resolve_existing_outbound_binding(
                            settings,
                            msg_type,
                            row.owner_id,
                            warehouse_id,
                            str(row.external_ref or row.id),
                            idem,
                            http_json_fn=route_http,
                        )
                        if row.attempt_count > 1
                        else None
                    )
                    if route_binding is None:
                        route_binding = resolve_outbound_route(
                            settings,
                            msg_type,
                            row.owner_id,
                            warehouse_id,
                            idem,
                            http_json_fn=route_http,
                        )
                    from channel_failover import map_channel_mode_to_transport

                    active_transport = map_channel_mode_to_transport(
                        route_binding.channel_mode or route_binding.channel
                    )
                    active_callback_base = route_binding.api_base_url
                    lifecycle_channel = route_binding.channel

                # 默认参数绑定当前 row，避免循环闭包误用末行
                def _http(lifecycle: Any, active: OutboxRow = row) -> None:
                    if not active_callback_base:
                        raise RuntimeError("connector api_base_url required for REST")
                    http_callback_publish(
                        active_callback_base,
                        active,
                        lifecycle,
                        resolve_bearer_token(
                            route_binding.bearer_secret_alias if route_binding else None
                        ),
                    )

                def _table(_lifecycle: Any, active: OutboxRow = row) -> None:
                    if sqlcmd_exec is None:
                        raise RuntimeError("sqlcmd_exec required for table transport")
                    sqlcmd_exec(insert_if_out_sql(active))

                from exchange_lifecycle import run_outbound_pipeline

                def _send(lifecycle: Any) -> None:
                    publish_with_failover(
                        transport=active_transport,
                        publish_http=lambda: _http(lifecycle),
                        publish_table=lambda: _table(lifecycle),
                        http_max_attempts=attempts,
                    )

                # US-H8-002 AC11：技术发送后等待 ERP 业务回执，不伪造 receipt
                run_outbound_pipeline(
                    life_settings,
                    msg_type,
                    str(getattr(row, "external_ref", None) or row.id),
                    idem,
                    _send,
                    connector_id=connector_id,
                    route_binding=route_binding,
                    channel=lifecycle_channel,
                    payload=row.payload,
                    dry_run=False,
                )
                mark_wms_outbox(
                    database_url,
                    table,
                    row.id,
                    succeeded=True,
                    attempt_count=row.attempt_count,
                )
                print(
                    f"[h8-out] published {table}/{row.id} "
                    f"(transport={active_transport})",
                    flush=True,
                )
            except Exception as exc:  # noqa: BLE001
                summary = sanitize_worker_error(
                    str(exc),
                    (
                        database_url,
                        os.environ.get("WMS_API_TOKEN"),
                        getattr(settings, "mssql_password", None),
                    ),
                )
                mark_wms_outbox(
                    database_url,
                    table,
                    row.id,
                    succeeded=False,
                    error=summary,
                    special_retry=special,
                    attempt_count=row.attempt_count,
                    max_attempts=row.max_attempts,
                )
                print(f"[h8-out] error {table}/{row.id}: {summary}", flush=True)
    return processed


def resolve_wms_db_url() -> str | None:
    return os.environ.get("WMS_DB_URL") or os.environ.get("DATABASE_URL") or None
