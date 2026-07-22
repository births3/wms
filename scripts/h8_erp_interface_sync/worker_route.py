"""H8 Worker 入站契约版本、错误分类与连接路由。"""

from __future__ import annotations

import urllib.parse
from dataclasses import dataclass
from typing import Any, Callable

H8_SUPPORTED_SCHEMA_VERSIONS = {"1"}

HttpJsonFn = Callable[
    [Any, str, str, dict[str, Any] | None, str],
    tuple[int, dict[str, Any] | None, str],
]


@dataclass(frozen=True)
class RouteBinding:
    connector_id: str
    connector_code: str
    config_version: int
    channel: str
    message_type: str


class WorkerHttpError(RuntimeError):
    def __init__(self, status: int, operation: str, detail: str) -> None:
        super().__init__(f"{operation} HTTP {status}: {detail[:500]}")
        self.status = status


def get_worker_claim_decision(
    settings: Any,
    connector_id: str,
    direction: str,
    *,
    http_json_fn: HttpJsonFn,
) -> bool:
    path = "/api/v1/integration/erp-messages/worker-runtime/claim-decision?" + (
        urllib.parse.urlencode({"connector_id": connector_id, "direction": direction})
    )
    status, parsed, raw = http_json_fn(
        settings, "GET", path, None, f"worker-claim-{connector_id}-{direction}"
    )
    if (
        status != 200
        or not isinstance(parsed, dict)
        or not isinstance(parsed.get("allowed"), bool)
    ):
        raise WorkerHttpError(status, "worker claim decision", raw)
    return parsed["allowed"]


def post_worker_heartbeat(
    settings: Any,
    directions: list[str],
    current_claims: int,
    *,
    http_json_fn: HttpJsonFn,
) -> None:
    body = {
        "worker_id": settings.worker_id,
        "worker_version": settings.worker_version,
        "connector_id": settings.connector_id,
        "directions": directions,
        "current_claims": current_claims,
        "heartbeat_ttl_seconds": settings.heartbeat_ttl_seconds,
    }
    status, parsed, raw = http_json_fn(
        settings,
        "POST",
        "/api/v1/integration/erp-messages/worker-runtime/heartbeat",
        body,
        f"worker-heartbeat-{settings.worker_id}",
    )
    if status != 200 or not isinstance(parsed, dict):
        raise WorkerHttpError(status, "worker heartbeat", raw)


def is_retryable_worker_error(error: Exception) -> bool:
    return isinstance(error, WorkerHttpError) and (
        error.status == 0 or error.status in (408, 425, 429) or error.status >= 500
    )


def validate_row_schema_version(row: dict[str, str]) -> str:
    version = (row.get("schema_version") or "").strip()
    if version not in H8_SUPPORTED_SCHEMA_VERSIONS:
        raise WorkerHttpError(422, "schema", f"unsupported schema_version {version}")
    return version


def resolve_inbound_route(
    settings: Any,
    message_type: str,
    row: dict[str, str],
    *,
    http_json_fn: HttpJsonFn,
) -> RouteBinding:
    query = {
        "direction": "inbound",
        "message_type": message_type,
    }
    warehouse_id = (row.get("warehouse_id") or "").strip()
    if warehouse_id:
        query["warehouse_id"] = warehouse_id
    path = "/api/v1/config/erp-connectors/route-resolve?" + urllib.parse.urlencode(
        query
    )
    status, parsed, raw = http_json_fn(
        settings,
        "GET",
        path,
        None,
        f"route-{row['idempotency_key']}",
    )
    if status != 200 or not isinstance(parsed, dict):
        raise WorkerHttpError(status, "route resolve", raw)
    connector = parsed.get("connector")
    if not isinstance(connector, dict):
        raise WorkerHttpError(502, "route resolve", "connector missing")
    mode = str(connector.get("channel_mode") or "")
    if mode not in ("interface_table", "rest_primary_table_fallback"):
        raise WorkerHttpError(
            409, "route resolve", "connector has no interface table channel"
        )
    connector_id = str(connector.get("id") or "")
    connector_code = str(connector.get("connector_code") or "")
    config_version = int(connector.get("config_version") or 0)
    if not connector_id or not connector_code or config_version < 1:
        raise WorkerHttpError(502, "route resolve", "connector binding incomplete")
    return RouteBinding(
        connector_id=connector_id,
        connector_code=connector_code,
        config_version=config_version,
        channel="interface_table",
        message_type=message_type,
    )
