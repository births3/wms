"""H8 出站业务回执与回执超时处理。"""

from __future__ import annotations

import urllib.parse
from datetime import datetime, timezone
from typing import Any, Callable

from worker_route import WorkerHttpError, sanitize_worker_error


def requeue_outbox(
    database_url: str,
    idempotency_key: str,
    *,
    allowed_tables: set[str],
    query_fn: Callable[[str, str], str],
) -> None:
    """按原出站幂等键恢复技术已送达、但业务回执超时的 outbox。"""
    import uuid

    parts = idempotency_key.split(":")
    if len(parts) != 3 or parts[0] != "out" or parts[1] not in allowed_tables:
        raise ValueError("invalid outbound idempotency key")
    row_id = str(uuid.UUID(parts[2]))
    updated = query_fn(
        database_url,
        f"""
WITH updated AS (
  UPDATE {parts[1]}
     SET status = 'failed',
         last_error = 'business receipt timeout',
         next_attempt_at = now(),
         updated_at = now()
   WHERE id = '{row_id}'::uuid
     AND status = 'succeeded'
  RETURNING id
),
already_requeued AS (
  SELECT id
    FROM {parts[1]}
   WHERE id = '{row_id}'::uuid
     AND status = 'failed'
     AND last_error = 'business receipt timeout'
)
SELECT id::text FROM updated
UNION ALL
SELECT id::text FROM already_requeued
LIMIT 1;
""",
    )
    if not updated.strip():
        raise RuntimeError("original outbound outbox is not requeueable")


def process_receipt_timeouts(
    settings: Any,
    database_url: str,
    *,
    http_json_fn: Callable[..., tuple[int, Any, str]],
    requeue_fn: Callable[[str, str], None],
) -> int:
    """把到期的 awaiting_receipt 重新排队，耗尽则保留 dead。"""
    processed = 0
    now = datetime.now(timezone.utc)
    cursor: str | None = None
    while True:
        query_params = {
            "direction": "outbound",
            "status": "awaiting_receipt",
            "connector_id": settings.connector_id,
            "created_from": "1970-01-01T00:00:00Z",
            "limit": 200,
        }
        if cursor:
            query_params["cursor"] = cursor
        query = urllib.parse.urlencode(query_params)
        status, parsed, raw = http_json_fn(
            settings,
            "GET",
            f"/api/v1/integration/erp-messages?{query}",
            None,
            "h8-receipt-timeout-list",
        )
        if status != 200 or not isinstance(parsed, dict):
            raise WorkerHttpError(status, "outbound receipt timeout list", raw)
        for message in parsed.get("data", []):
            try:
                retry_at = datetime.fromisoformat(
                    str(message.get("next_retry_at") or "").replace("Z", "+00:00")
                )
                if retry_at.tzinfo is None:
                    retry_at = retry_at.replace(tzinfo=timezone.utc)
                if retry_at > now:
                    continue
                retry_count = int(message.get("retry_count") or 0)
                if retry_count < 4:
                    requeue_fn(database_url, message["idempotency_key"])
                body = {
                    "stage": "final_failure",
                    "result": "business receipt timeout",
                    "direction": "outbound",
                    "message_type": message["message_type"],
                    "schema_version": message["schema_version"],
                    "external_ref": message["external_ref"],
                    "idempotency_key": message["idempotency_key"],
                    "correlation_id": message["correlation_id"],
                    "channel": message["channel"],
                    "connector_id": message.get("connector_id"),
                    "connector_code": message.get("connector_code"),
                    "config_version": message.get("config_version"),
                    "message_id": message["id"],
                    "warehouse_id": message.get("warehouse_id"),
                }
                status, updated, raw = http_json_fn(
                    settings,
                    "POST",
                    "/api/v1/integration/erp-messages/lifecycle",
                    body,
                    f"h8-receipt-timeout-{message['id']}-{retry_count + 1}",
                )
                sync_status = (
                    updated.get("sync_status") if isinstance(updated, dict) else None
                )
                if status not in (200, 201) or sync_status not in (
                    "processing",
                    "dead",
                ):
                    raise WorkerHttpError(status, "outbound receipt timeout", raw)
                processed += 1
            except Exception as exc:  # noqa: BLE001 — 留待下一轮继续处理
                print(
                    f"[h8-out] receipt timeout pending {message.get('id')}: "
                    f"{sanitize_worker_error(str(exc), (settings.api_token, database_url))}",
                    flush=True,
                )
        cursor = parsed.get("page", {}).get("next_cursor")
        if not cursor:
            break
    return processed


def process_table_receipts(
    settings: Any,
    *,
    http_json_fn: Callable[..., tuple[int, Any, str]],
    list_acked_fn: Callable[[Any], list[dict[str, Any]]],
    mark_recorded_fn: Callable[[Any, str], None],
) -> int:
    """把接口表 ERP 业务确认回写为 H8 acked。"""
    processed = 0
    for row in list_acked_fn(settings):
        try:
            query = urllib.parse.urlencode(
                {
                    "direction": "outbound",
                    "connector_id": settings.connector_id,
                    "idempotency_key": row["idempotency_key"],
                    "created_from": "1970-01-01T00:00:00Z",
                    "limit": 2,
                }
            )
            status, parsed, raw = http_json_fn(
                settings,
                "GET",
                f"/api/v1/integration/erp-messages?{query}",
                None,
                f"h8-receipt-find-{row['id']}",
            )
            messages = parsed.get("data", []) if isinstance(parsed, dict) else []
            if status != 200 or len(messages) != 1:
                raise WorkerHttpError(status, "outbound receipt lookup", raw)
            message = messages[0]
            if message.get("connector_id") != settings.connector_id:
                raise RuntimeError("outbound receipt connector mismatch")
            body = {
                "stage": "receipt",
                "result": "ok",
                "direction": "outbound",
                "message_type": message["message_type"],
                "schema_version": message["schema_version"],
                "external_ref": message["external_ref"],
                "idempotency_key": message["idempotency_key"],
                "correlation_id": message["correlation_id"],
                "channel": message["channel"],
                "connector_id": message.get("connector_id"),
                "connector_code": message.get("connector_code"),
                "config_version": message.get("config_version"),
                "message_id": message["id"],
                "warehouse_id": message.get("warehouse_id"),
            }
            status, parsed, raw = http_json_fn(
                settings,
                "POST",
                "/api/v1/integration/erp-messages/lifecycle",
                body,
                f"h8-receipt-{row['id']}",
            )
            if status not in (200, 201) or not isinstance(parsed, dict):
                raise WorkerHttpError(status, "outbound receipt", raw)
            if parsed.get("sync_status") != "acked":
                raise RuntimeError("outbound receipt did not reach acked")
            mark_recorded_fn(settings, row["id"])
            processed += 1
        except Exception as exc:  # noqa: BLE001 — 保留 acked 行供下一轮重试
            summary = sanitize_worker_error(
                str(exc), (settings.api_token, settings.mssql_password)
            )
            print(f"[h8-out] receipt pending {row['id']}: {summary}", flush=True)
    return processed
