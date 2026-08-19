# @governance: skip-page-size 出站认领、回执与同事务发布构成单一崩溃恢复状态机，需在同一模块审计。
"""H8 出站：WMS PG ERP outbox → 通道 B(if_out_message) 或 通道 A(HTTP 回调)。"""

from __future__ import annotations

import json
import os
import subprocess
import urllib.error
import urllib.request
import uuid
from dataclasses import dataclass
from decimal import Decimal
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
from v19_contract import payload_digest

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


def effective_message_type(source: dict[str, str], row: "OutboxRow") -> str:
    return "order_status" if row.event_type == "order_status" else source["message_type"]


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
    created_at: str | None = None


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
    if table == "receiving_putaway_erp_feedback_outbox":
        extra_where += """
 AND (
   o.event_type <> 'order_status'
   OR o.payload ->> 'feedback_type' <> '2'
   OR NOT EXISTS (
     SELECT 1
       FROM receiving_putaway_erp_feedback_outbox detail
      WHERE detail.owner_id = o.owner_id
        AND detail.event_type = 'inbound_putaway_completed'
        AND detail.payload ->> 'erp_bill_code' = o.payload ->> 'erp_bill_code'
        AND detail.payload ->> 'revision' = o.payload ->> 'revision'
        AND detail.status <> 'succeeded'
   )
 )"""
    if connector_id:
        if not message_type:
            raise ValueError("message_type required with connector_id")
        connector_sql = sql_escape_pg(connector_id)
        message_type_sql = sql_escape_pg(message_type)
        message_type_expr = (
            "CASE WHEN o.event_type = 'order_status' THEN 'order_status' "
            f"ELSE '{message_type_sql}' END"
        )
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
        AND message.message_type = ({message_type_expr})
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
          AND ({message_type_expr}) = ANY(message_types)
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
    o.payload || jsonb_build_object(
      'depot_code', COALESCE(
        o.payload ->> 'depot_code',
        (SELECT warehouse_code
           FROM warehouses
          WHERE id::text = o.payload ->> 'warehouse_id'
            AND owner_id = o.owner_id)
      )
    ) AS payload,
    {ref_expr} AS external_ref,
    o.attempt_count AS attempt_count,
    ({max_expr})::int AS max_attempts,
    {deadline_expr} AS deadline_at,
    to_char(o.created_at AT TIME ZONE 'UTC',
            'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS created_at
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
  ,'created_at', created_at
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
                created_at=(str(obj["created_at"]) if obj.get("created_at") else None),
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


def _required(payload: dict[str, Any], field: str) -> Any:
    value = payload.get(field)
    if value is None or value == "":
        raise ValueError(f"v1.9 outbound payload missing {field}")
    return value


def _sql_text(value: Any, *, unicode: bool = False) -> str:
    if value is None:
        return "NULL"
    prefix = "N" if unicode else ""
    return f"{prefix}'{sql_escape_mssql(str(value))}'"


def _sql_datetime(value: Any) -> str:
    if value is None:
        return "NULL"
    raw = str(value).replace("Z", "+00:00")
    from datetime import datetime, timezone

    parsed = datetime.fromisoformat(raw)
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    utc = parsed.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:23]
    return f"CONVERT(datetime2(3), '{utc}', 126)"


def _decimal4(value: Any) -> str:
    return f"{Decimal(str(value)):.4f}"


def _event_time(row: OutboxRow, field: str) -> Any:
    return row.payload.get(field) or row.created_at or _required(row.payload, field)


def _single_record_insert_sql(
    table: str,
    owner_code: str,
    idempotency_key: str,
    digest: str,
    columns: tuple[str, ...],
    values: tuple[str, ...],
) -> str:
    suffix = f"d{digest[:12]}"
    existing_digest = f"@ExistingDigest_{suffix}"
    error_message = f"@ErrorMessage_{suffix}"
    return f"""
SET NOCOUNT ON;
DECLARE {existing_digest} char(64);
SELECT {existing_digest} = PayloadDigest
  FROM dbo.{table}
 WHERE OwnerCode = {_sql_text(owner_code)}
   AND IdempotencyKey = {_sql_text(idempotency_key)};
IF {existing_digest} IS NOT NULL
BEGIN
  IF {existing_digest} <> '{digest}' RAISERROR('IDEMPOTENCY_CONFLICT', 16, 1);
END
ELSE
BEGIN
  BEGIN TRY
    INSERT INTO dbo.{table} (
      {', '.join(f'[{column}]' for column in columns)}, [handelflag], [retry_count], [inserttime]
    ) VALUES (
      {', '.join(values)}, 0, 0, SYSUTCDATETIME()
    );
  END TRY
  BEGIN CATCH
    IF ERROR_NUMBER() NOT IN (2601, 2627)
    BEGIN
      DECLARE {error_message} nvarchar(4000);
      SET {error_message} = ERROR_MESSAGE();
      RAISERROR({error_message}, 16, 1);
    END
    SELECT {existing_digest} = PayloadDigest
      FROM dbo.{table}
     WHERE OwnerCode = {_sql_text(owner_code)}
       AND IdempotencyKey = {_sql_text(idempotency_key)};
    IF {existing_digest} IS NULL RAISERROR('BUSINESS_KEY_CONFLICT', 16, 1);
    IF {existing_digest} <> '{digest}' RAISERROR('IDEMPOTENCY_CONFLICT', 16, 1);
  END CATCH
END
"""


def _transaction_sql(statements: str, suffix: str) -> str:
    error_message = f"@TxnError_{suffix}"
    return f"""
SET XACT_ABORT ON;
BEGIN TRANSACTION;
BEGIN TRY
{statements}
  COMMIT TRANSACTION;
END TRY
BEGIN CATCH
  DECLARE {error_message} nvarchar(4000);
  SET {error_message} = ERROR_MESSAGE();
  IF XACT_STATE() <> 0 ROLLBACK TRANSACTION;
  RAISERROR({error_message}, 16, 1);
END CATCH
"""


def _wms_event_sql(row: OutboxRow, owner_code: str) -> str:
    payload = row.payload
    if row.event_type in ("inventory_status", "inventory_status_changed"):
        event_type = "inventory_status"
        event_time = _event_time(row, "occur_time")
        event_payload = {
            "depot_code": _required(payload, "depot_code"),
            "product_code": _required(payload, "product_code"),
            "batch_no": _required(payload, "batch_no"),
            "goods_status": _required(payload, "to_status"),
            "amount": _decimal4(_required(payload, "qty")),
            "occur_time": event_time,
        }
    elif row.event_type in ("stock_adjustment", "stock_loss_completed", "stock_surplus_completed"):
        event_type = "stock_adjustment"
        event_time = _event_time(row, "completed_at")
        event_payload = {
            "depot_code": _required(payload, "depot_code"),
            "product_code": _required(payload, "product_code"),
            "batch_no": _required(payload, "batch_no"),
            "adjust_type": "损" if "loss" in row.event_type else "溢",
            "amount": _decimal4(_required(payload, "quantity")),
            "reason": str(_required(payload, "reason")),
            "adjust_time": event_time,
        }
    elif row.event_type == "archive_revision":
        event_type = row.event_type
        event_time = _event_time(row, "submitted_at")
        event_payload = {
            "liaison_id": _required(payload, "liaison_id"),
            "asn_id": _required(payload, "asn_id"),
            "receipt_record_id": _required(payload, "receipt_record_id"),
            "product_code": _required(payload, "product_code"),
            "field_name": _required(payload, "field_name"),
            "current_value": payload.get("current_value"),
            "new_value": payload.get("new_value"),
            "photo_urls": _required(payload, "photo_urls"),
            "operator_id": _required(payload, "operator_id"),
            "submitted_at": event_time,
        }
    elif row.event_type == "reconciliation_diff":
        event_type = row.event_type
        event_time = _event_time(row, "diff_at")
        event_payload = {
            "depot_code": _required(payload, "depot_code"),
            "product_code": _required(payload, "product_code"),
            "batch_no": _required(payload, "batch_no"),
            "erp_amount": _decimal4(_required(payload, "erp_qty")),
            "wms_amount": _decimal4(_required(payload, "wms_qty")),
            "diff_amount": _decimal4(_required(payload, "difference_qty")),
            "diff_at": event_time,
        }
    else:
        raise ValueError(f"unsupported v1.9 outbound event: {row.event_type}")
    correlation_id = str(payload.get("correlation_id") or row.id)
    canonical = {
        "IdempotencyKey": row.id,
        "EventType": event_type,
        "SchemaVersion": "1",
        "PayloadJson": event_payload,
        "EventTime": event_time,
        "OwnerCode": owner_code,
        "CorrelationID": correlation_id,
        "SourceVersion": None,
    }
    digest = payload_digest("x_wmsinter_WmsEvent", canonical)
    payload_json = json.dumps(event_payload, ensure_ascii=False, separators=(",", ":"))
    columns = (
        "IdempotencyKey", "EventType", "SchemaVersion", "PayloadJson",
        "EventTime", "OwnerCode", "PayloadDigest", "CorrelationID", "SourceVersion",
    )
    values = (
        _sql_text(row.id), _sql_text(event_type), _sql_text("1"),
        _sql_text(payload_json, unicode=True), _sql_datetime(event_time),
        _sql_text(owner_code), _sql_text(digest), _sql_text(correlation_id), "NULL",
    )
    return _single_record_insert_sql(
        "x_wmsinter_WmsEvent", owner_code, row.id, digest, columns, values
    )


def _inbound_feedback_sql(row: OutboxRow, owner_code: str) -> str:
    payload = row.payload
    actual = Decimal(str(_required(payload, "actual_amount")))
    rejected = Decimal(str(payload.get("reject_amount") or 0))
    shortage = Decimal(str(payload.get("shortage_amount") or 0))
    if min(actual, rejected, shortage) < 0:
        raise ValueError("v1.9 inbound feedback quantity must be non-negative")
    if actual > 0:
        _required(payload, "batch_no")
        _required(payload, "location_code")
    if rejected > 0:
        _required(payload, "reject_reason")
    if shortage > 0:
        _required(payload, "shortage_reason")
    correlation_id = str(_required(payload, "correlation_id"))
    canonical = {
        "IdempotencyKey": row.id,
        "ERPBillCode": _required(payload, "erp_bill_code"),
        "Revision": int(_required(payload, "revision")),
        "LineNo": int(_required(payload, "line_no")),
        "GoodsID": int(_required(payload, "goods_id")),
        "GoodsCode": _required(payload, "product_code"),
        "ExpectedAmount": _required(payload, "expected_amount"),
        "ActualAmount": actual,
        "RejectAmount": rejected,
        "ShortageAmount": shortage,
        "RejectReason": payload.get("reject_reason"),
        "ShortageReason": payload.get("shortage_reason"),
        "BatchNo": payload.get("batch_no"),
        "ProduceDate": payload.get("production_date"),
        "ValidDate": payload.get("expiry_date"),
        "StallCode": payload.get("location_code"),
        "OperatorName": payload.get("operator_name"),
        "ScanTime": payload.get("scan_time"),
        "OwnerCode": owner_code,
        "SchemaVersion": "1",
        "CorrelationID": correlation_id,
        "SourceVersion": None,
    }
    digest = payload_digest("x_wmsinter_InboundFeedback", canonical)
    columns = (
        "IdempotencyKey", "ERPBillCode", "Revision", "LineNo", "GoodsID",
        "GoodsCode", "ExpectedAmount", "ActualAmount", "RejectAmount",
        "ShortageAmount", "RejectReason", "ShortageReason", "BatchNo",
        "ProduceDate", "ValidDate", "StallCode", "OperatorName", "ScanTime",
        "OwnerCode", "SchemaVersion", "PayloadDigest", "CorrelationID", "SourceVersion",
    )
    values = (
        _sql_text(row.id), _sql_text(canonical["ERPBillCode"]), str(canonical["Revision"]),
        str(canonical["LineNo"]), str(canonical["GoodsID"]),
        _sql_text(canonical["GoodsCode"]), _sql_text(_decimal4(canonical["ExpectedAmount"])),
        _sql_text(_decimal4(actual)), _sql_text(_decimal4(rejected)),
        _sql_text(_decimal4(shortage)), _sql_text(canonical["RejectReason"], unicode=True),
        _sql_text(canonical["ShortageReason"], unicode=True), _sql_text(canonical["BatchNo"]),
        _sql_text(canonical["ProduceDate"]), _sql_text(canonical["ValidDate"]),
        _sql_text(canonical["StallCode"]), _sql_text(canonical["OperatorName"], unicode=True),
        _sql_datetime(canonical["ScanTime"]), _sql_text(owner_code), _sql_text("1"),
        _sql_text(digest), _sql_text(correlation_id), "NULL",
    )
    return _single_record_insert_sql(
        "x_wmsinter_InboundFeedback", owner_code, row.id, digest, columns, values
    )


def _outbound_feedback_record(
    row: OutboxRow,
    owner_code: str,
    correlation_id: str,
    line: dict[str, Any],
) -> tuple[str, str]:
    line_no = int(_required(line, "line_no"))
    idem = str(line.get("idempotency_key") or f"{row.id}:{line_no}")
    expected = Decimal(str(_required(line, "expected_amount")))
    picked = Decimal(str(_required(line, "picked_amount")))
    shipped = Decimal(str(_required(line, "shipped_amount")))
    if expected < 0 or picked != expected or shipped != expected:
        raise ValueError("v1.9 outbound feedback requires picked=shipped=expected")
    canonical = {
        "IdempotencyKey": idem,
        "ERPBillCode": _required(row.payload, "erp_bill_code"),
        "Revision": int(_required(row.payload, "revision")),
        "LineNo": line_no,
        "GoodsID": int(_required(line, "goods_id")),
        "GoodsCode": _required(line, "product_code"),
        "BatchNo": _required(line, "batch_no"),
        "ExpectedAmount": expected,
        "PickedAmount": picked,
        "ShippedAmount": shipped,
        "OperatorName": row.payload.get("operator_name"),
        "OwnerCode": owner_code,
        "SchemaVersion": "1",
        "CorrelationID": correlation_id,
        "SourceVersion": None,
    }
    digest = payload_digest("x_wmsinter_OutboundFeedback", canonical)
    columns = (
        "IdempotencyKey", "ERPBillCode", "Revision", "LineNo", "GoodsID",
        "GoodsCode", "BatchNo", "ExpectedAmount", "PickedAmount", "ShippedAmount",
        "OperatorName", "OwnerCode", "SchemaVersion", "PayloadDigest",
        "CorrelationID", "SourceVersion",
    )
    values = (
        _sql_text(idem), _sql_text(canonical["ERPBillCode"]), str(canonical["Revision"]),
        str(line_no), str(canonical["GoodsID"]), _sql_text(canonical["GoodsCode"]),
        _sql_text(canonical["BatchNo"]), _sql_text(_decimal4(expected)),
        _sql_text(_decimal4(picked)), _sql_text(_decimal4(shipped)),
        _sql_text(canonical["OperatorName"], unicode=True), _sql_text(owner_code),
        _sql_text("1"), _sql_text(digest), _sql_text(correlation_id), "NULL",
    )
    return idem, _single_record_insert_sql(
        "x_wmsinter_OutboundFeedback", owner_code, idem, digest, columns, values
    )


def _shipment_confirm_sql(row: OutboxRow, owner_code: str) -> str:
    payload = row.payload
    lines = list(_required(payload, "lines"))
    if not lines or len(lines) != int(_required(payload, "line_count")):
        raise ValueError("v1.9 outbound feedback line_count mismatch")
    correlation_id = str(_required(payload, "correlation_id"))
    ship_time = _required(payload, "ship_time")
    details = [
        _outbound_feedback_record(row, owner_code, correlation_id, line)[1]
        for line in sorted(lines, key=lambda item: int(item["line_no"]))
    ]
    barrier_payload = {
        "erp_bill_code": _required(payload, "erp_bill_code"),
        "revision": int(_required(payload, "revision")),
        "order_type": 2,
        "feedback_type": 6,
        "result_count": len(lines),
        "waybill_no": payload.get("waybill_no"),
        "express_company": payload.get("express_company"),
        "ship_time": ship_time,
        "feedback_time": ship_time,
        "operator_name": payload.get("operator_name"),
        "correlation_id": correlation_id,
    }
    barrier = insert_if_out_sql(
        OutboxRow(
            table=row.table,
            id=row.id,
            owner_id=row.owner_id,
            event_type="order_status",
            payload=barrier_payload,
            external_ref=row.external_ref,
            attempt_count=row.attempt_count,
            max_attempts=row.max_attempts,
            deadline_at=row.deadline_at,
            callback_path=row.callback_path,
        ),
        owner_code=owner_code,
    )
    return _transaction_sql(
        "\n".join((*details, barrier)),
        f"shipment_{row.id.replace('-', '')[:12]}",
    )


def _inventory_snapshot_sql(row: OutboxRow, owner_code: str) -> str:
    payload = row.payload
    snapshot_id = str(_required(payload, "snapshot_id"))
    correlation_id = str(payload.get("correlation_id") or row.id)
    receive_time = _event_time(row, "receive_time")
    lines = sorted(list(payload.get("lines") or []), key=lambda item: int(item["row_no"]))
    row_numbers = [int(_required(line, "row_no")) for line in lines]
    if row_numbers != list(range(1, len(lines) + 1)):
        raise ValueError("v1.9 inventory snapshot RowNo must be contiguous from 1")
    items: list[dict[str, Any]] = []
    item_sql: list[str] = []
    for line in lines:
        row_no = int(line["row_no"])
        amount = Decimal(str(_required(line, "wms_amount")))
        pickable = Decimal(str(_required(line, "wms_pickable")))
        allocated = Decimal(str(line.get("wms_allocated") or 0))
        frozen = Decimal(str(line.get("wms_frozen") or 0))
        if min(amount, pickable, allocated, frozen) < 0 or pickable > amount:
            raise ValueError("v1.9 inventory snapshot quantity constraint failed")
        item = {
            "SnapshotID": snapshot_id,
            "RowNo": row_no,
            "DepotCode": line.get("depot_code") or _required(payload, "depot_code"),
            "GoodsCode": _required(line, "product_code"),
            "BatchNo": _required(line, "batch_no"),
            "ValidDate": line.get("valid_date"),
            "GoodsStatus": _required(line, "goods_status"),
            "WMSAmount": amount,
            "WMSPickable": pickable,
            "WMSAllocated": allocated,
            "WMSFrozen": frozen,
            "OwnerCode": owner_code,
            "CorrelationID": correlation_id,
            "IdempotencyKey": f"{snapshot_id}:{row_no}",
        }
        items.append(item)
        item_sql.append(
            "INSERT INTO dbo.x_wmsinter_InventoryReceiveItems "
            "([SnapshotID],[RowNo],[DepotCode],[GoodsCode],[BatchNo],[ValidDate],[GoodsStatus],"
            "WMSAmount,WMSPickable,WMSAllocated,WMSFrozen,OwnerCode,CorrelationID,"
            "IdempotencyKey,inserttime) VALUES ("
            + ",".join(
                (
                    _sql_text(snapshot_id), str(row_no), _sql_text(item["DepotCode"]),
                    _sql_text(item["GoodsCode"]), _sql_text(item["BatchNo"]),
                    _sql_text(item["ValidDate"]), _sql_text(item["GoodsStatus"]),
                    _sql_text(_decimal4(amount)), _sql_text(_decimal4(pickable)),
                    _sql_text(_decimal4(allocated)), _sql_text(_decimal4(frozen)),
                    _sql_text(owner_code), _sql_text(correlation_id),
                    _sql_text(item["IdempotencyKey"]), "SYSUTCDATETIME()",
                )
            )
            + ");"
        )
    header = {
        "SnapshotID": snapshot_id,
        "ReceiveTime": receive_time,
        "TotalCount": len(items),
        "OwnerCode": owner_code,
        "SchemaVersion": "1",
        "IdempotencyKey": snapshot_id,
        "CorrelationID": correlation_id,
        "SourceVersion": None,
    }
    digest = payload_digest("x_wmsinter_InventoryReceiveHeader", header, items)
    suffix = f"snapshot_{digest[:12]}"
    existing_digest = f"@ExistingDigest_{suffix}"
    error_number = f"@ErrorNumber_{suffix}"
    error_message = f"@ErrorMessage_{suffix}"
    return f"""
SET NOCOUNT ON;
SET XACT_ABORT ON;
DECLARE {existing_digest} char(64);
SELECT {existing_digest} = PayloadDigest
  FROM dbo.x_wmsinter_InventoryReceiveHeader
 WHERE OwnerCode = {_sql_text(owner_code)} AND IdempotencyKey = {_sql_text(snapshot_id)};
IF {existing_digest} IS NOT NULL
BEGIN
  IF {existing_digest} <> '{digest}' RAISERROR('IDEMPOTENCY_CONFLICT', 16, 1);
END
ELSE
BEGIN
  BEGIN TRANSACTION;
  BEGIN TRY
    INSERT INTO dbo.x_wmsinter_InventoryReceiveHeader
      (SnapshotID,ReceiveTime,TotalCount,OwnerCode,SchemaVersion,IdempotencyKey,
       PayloadDigest,CorrelationID,SourceVersion,handelflag,retry_count,inserttime)
    VALUES ({_sql_text(snapshot_id)},{_sql_datetime(receive_time)},{len(items)},
      {_sql_text(owner_code)},'1',{_sql_text(snapshot_id)},'{digest}',
      {_sql_text(correlation_id)},NULL,0,0,SYSUTCDATETIME());
    {' '.join(item_sql)}
    COMMIT TRANSACTION;
  END TRY
  BEGIN CATCH
    DECLARE {error_number} int;
    DECLARE {error_message} nvarchar(4000);
    SELECT {error_number} = ERROR_NUMBER(), {error_message} = ERROR_MESSAGE();
    IF XACT_STATE() <> 0 ROLLBACK TRANSACTION;
    IF {error_number} IN (2601,2627)
    BEGIN
      SET {existing_digest} = NULL;
      SELECT {existing_digest} = PayloadDigest
        FROM dbo.x_wmsinter_InventoryReceiveHeader
       WHERE OwnerCode = {_sql_text(owner_code)}
         AND IdempotencyKey = {_sql_text(snapshot_id)};
      IF {existing_digest} IS NULL RAISERROR('BUSINESS_KEY_CONFLICT',16,1);
      IF {existing_digest} <> '{digest}' RAISERROR('IDEMPOTENCY_CONFLICT',16,1);
    END
    ELSE
      RAISERROR({error_message},16,1);
  END CATCH
END
"""


def insert_if_out_sql(row: OutboxRow, *, owner_code: str | None = None) -> str:
    """生成 v1.9 直写接口表 SQL；名称保留以兼容现有调用点。"""
    payload = row.payload
    owner = owner_code or str(payload.get("owner_code") or "").strip()
    if not owner:
        raise ValueError("v1.9 outbound payload missing owner_code")
    if row.event_type == "inbound_putaway_completed":
        return _inbound_feedback_sql(row, owner)
    if row.event_type == "shipment_confirm":
        return _shipment_confirm_sql(row, owner)
    if row.event_type == "inventory_snapshot":
        return _inventory_snapshot_sql(row, owner)
    if row.event_type != "order_status":
        return _wms_event_sql(row, owner)
    feedback_type = int(_required(payload, "feedback_type"))
    if feedback_type in (2, 6) and payload.get("result_count") is None:
        raise ValueError("v1.9 outbound payload missing result_count")
    if feedback_type == 6 and not payload.get("ship_time"):
        raise ValueError("v1.9 outbound payload missing ship_time")
    if feedback_type == 9 and not payload.get("result_code"):
        raise ValueError("v1.9 outbound payload missing result_code")
    if feedback_type == 100 and not payload.get("command_id"):
        raise ValueError("v1.9 outbound payload missing command_id")
    canonical = {
        "IdempotencyKey": row.id,
        "ERPBillCode": _required(payload, "erp_bill_code"),
        "Revision": int(_required(payload, "revision")),
        "OrderType": int(_required(payload, "order_type")),
        "FeedbackType": feedback_type,
        "CommandID": payload.get("command_id"),
        "ResultCount": payload.get("result_count"),
        "ResultCode": payload.get("result_code"),
        "ResultMessage": payload.get("result_message"),
        "WaybillNo": payload.get("waybill_no"),
        "ExpressCompany": payload.get("express_company"),
        "ShipTime": payload.get("ship_time"),
        "FeedbackTime": _required(payload, "feedback_time"),
        "OperatorName": payload.get("operator_name"),
        "OwnerCode": owner,
        "SchemaVersion": "1",
        "CorrelationID": _required(payload, "correlation_id"),
        "SourceVersion": None,
    }
    digest = payload_digest("x_wmsinter_OrderFeedback", canonical)
    values = (
        _sql_text(canonical["IdempotencyKey"]),
        _sql_text(canonical["ERPBillCode"]),
        str(canonical["Revision"]),
        str(canonical["OrderType"]),
        str(canonical["FeedbackType"]),
        _sql_text(canonical["CommandID"]),
        "NULL" if canonical["ResultCount"] is None else str(int(canonical["ResultCount"])),
        _sql_text(canonical["ResultCode"]),
        _sql_text(canonical["ResultMessage"], unicode=True),
        _sql_text(canonical["WaybillNo"]),
        _sql_text(canonical["ExpressCompany"], unicode=True),
        _sql_datetime(canonical["ShipTime"]),
        _sql_datetime(canonical["FeedbackTime"]),
        _sql_text(canonical["OperatorName"], unicode=True),
        _sql_text(canonical["OwnerCode"]),
        _sql_text(canonical["SchemaVersion"]),
        _sql_text(digest),
        _sql_text(canonical["CorrelationID"]),
    )
    columns = (
        "IdempotencyKey", "ERPBillCode", "Revision", "OrderType", "FeedbackType",
        "CommandID", "ResultCount", "ResultCode", "ResultMessage", "WaybillNo",
        "ExpressCompany", "ShipTime", "FeedbackTime", "OperatorName",
        "OwnerCode", "SchemaVersion", "PayloadDigest", "CorrelationID", "SourceVersion",
    )
    return _single_record_insert_sql(
        "x_wmsinter_OrderFeedback",
        owner,
        row.id,
        digest,
        columns,
        (*values, "NULL"),
    )


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
            msg_type = effective_message_type(src, row)
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
                    sqlcmd_exec(
                        insert_if_out_sql(
                            active,
                            owner_code=getattr(settings, "owner_code", None),
                        )
                    )

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
