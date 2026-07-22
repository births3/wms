"""H8 Worker 入站契约版本、错误分类与连接路由。"""

from __future__ import annotations

import re
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
        super().__init__(f"{operation} HTTP {status}: {sanitize_worker_error(detail)}")
        self.status = status


def sanitize_worker_error(raw: str, secrets: tuple[str | None, ...] = ()) -> str:
    """Worker 日志/失败摘要边界：隐藏已知凭据和常见认证字段。"""
    safe = raw
    for secret in secrets:
        if secret:
            safe = safe.replace(secret, "***")
    safe = re.sub(r"(?i)(bearer\s+)[^\s,;\"'}]+", r"\1***", safe)
    safe = re.sub(
        r"(?i)([\"']?(?:password|token|api[_-]?key)[\"']?\s*[:=]\s*[\"']?)[^\"'\s,;}]+",
        r"\1***",
        safe,
    )
    return safe[:500]


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


def list_manual_replays(
    settings: Any,
    message_type: str,
    *,
    http_json_fn: HttpJsonFn,
) -> list[dict[str, Any]]:
    """读取当前连接、当前类型等待 Worker 接管的人工重放消息。"""
    path = "/api/v1/integration/erp-messages?" + urllib.parse.urlencode(
        {
            "direction": "inbound",
            "message_type": message_type,
            "status": "processing",
            "connector_id": settings.connector_id,
            "channel": "interface_table",
            "replay_requested": "true",
            "created_from": "1970-01-01T00:00:00Z",
            "limit": 200,
        }
    )
    status, parsed, raw = http_json_fn(
        settings,
        "GET",
        path,
        None,
        f"manual-replays-{settings.connector_id}-{message_type}",
    )
    if status != 200 or not isinstance(parsed, dict):
        raise WorkerHttpError(status, "manual replay list", raw)
    data = parsed.get("data")
    if not isinstance(data, list):
        raise WorkerHttpError(502, "manual replay list", "data missing")
    for message in data:
        if not isinstance(message, dict):
            raise WorkerHttpError(502, "manual replay list", "invalid message")
        expected = {
            "connector_id": settings.connector_id,
            "direction": "inbound",
            "message_type": message_type,
            "channel": "interface_table",
        }
        if any(str(message.get(key) or "") != value for key, value in expected.items()):
            raise WorkerHttpError(409, "manual replay list", "message scope changed")
        if not str(message.get("claimed_by") or "").startswith("replay:"):
            raise WorkerHttpError(409, "manual replay list", "replay marker missing")
        if not message.get("id") or not str(message.get("idempotency_key") or "").strip():
            raise WorkerHttpError(502, "manual replay list", "message identity missing")
    return data


def claim_manual_replay(
    settings: Any,
    message_id: str,
    *,
    http_json_fn: HttpJsonFn,
) -> None:
    body = {"worker_id": settings.worker_id, "lease_seconds": 300}
    status, parsed, raw = http_json_fn(
        settings,
        "POST",
        f"/api/v1/integration/erp-messages/{message_id}/claim",
        body,
        f"manual-replay-claim-{message_id}-{settings.worker_id}",
    )
    if (
        status != 200
        or not isinstance(parsed, dict)
        or str(parsed.get("id") or "") != message_id
        or str(parsed.get("claimed_by") or "") != settings.worker_id
    ):
        raise WorkerHttpError(status, "manual replay claim", raw)


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


def resolve_existing_inbound_binding(
    settings: Any,
    message_type: str,
    row: dict[str, str],
    *,
    http_json_fn: HttpJsonFn,
) -> RouteBinding | None:
    """重试时按完整幂等身份读取首次处理绑定；预检记录未绑定则返回 None。"""
    external_ref = str(
        row.get("external_ref") or row.get("external_doc_no") or row.get("id") or ""
    )
    idempotency_key = str(
        row.get("idempotency_key") or f"{message_type}-{row.get('id', 'x')}"
    )
    path = "/api/v1/integration/erp-messages?" + urllib.parse.urlencode(
        {
            "direction": "inbound",
            "message_type": message_type,
            "external_ref": external_ref,
            "idempotency_key": idempotency_key,
            "created_from": "1970-01-01T00:00:00Z",
            "limit": 2,
        }
    )
    status, parsed, raw = http_json_fn(
        settings,
        "GET",
        path,
        None,
        f"binding-{idempotency_key}",
    )
    if status != 200 or not isinstance(parsed, dict):
        raise WorkerHttpError(status, "message binding lookup", raw)
    data = parsed.get("data")
    if not isinstance(data, list):
        raise WorkerHttpError(502, "message binding lookup", "data missing")
    if not data:
        return None
    if len(data) != 1 or not isinstance(data[0], dict):
        raise WorkerHttpError(409, "message binding lookup", "binding is ambiguous")
    message = data[0]
    expected = {
        "direction": "inbound",
        "message_type": message_type,
        "schema_version": str(row.get("schema_version") or ""),
        "channel": "interface_table",
    }
    if any(str(message.get(key) or "") != value for key, value in expected.items()):
        raise WorkerHttpError(409, "message binding lookup", "binding identity changed")
    connector_id = str(message.get("connector_id") or "")
    connector_code = str(message.get("connector_code") or "")
    try:
        config_version = int(message.get("config_version") or 0)
    except (TypeError, ValueError) as exc:
        raise WorkerHttpError(502, "message binding lookup", "invalid config version") from exc
    if not connector_id or not connector_code or config_version < 1:
        return None
    binding = RouteBinding(
        connector_id=connector_id,
        connector_code=connector_code,
        config_version=config_version,
        channel="interface_table",
        message_type=message_type,
    )
    path = f"/api/v1/config/erp-connectors/{connector_id}/versions/{config_version}"
    status, snapshot, raw = http_json_fn(
        settings,
        "GET",
        path,
        None,
        f"connector-version-{connector_id}-{config_version}",
    )
    if status != 200 or not isinstance(snapshot, dict):
        raise WorkerHttpError(status, "connector version lookup", raw)
    expected_snapshot = {
        "id": connector_id,
        "connector_code": connector_code,
        "config_version": config_version,
    }
    if any(snapshot.get(key) != value for key, value in expected_snapshot.items()):
        raise WorkerHttpError(409, "connector version lookup", "binding changed")
    if snapshot.get("channel_mode") not in (
        "interface_table",
        "rest_primary_table_fallback",
    ):
        raise WorkerHttpError(409, "connector version lookup", "channel unavailable")
    directions = snapshot.get("directions")
    message_types = snapshot.get("message_types")
    warehouse_ids = snapshot.get("warehouse_ids")
    if not isinstance(directions, list) or "inbound" not in directions:
        raise WorkerHttpError(409, "connector version lookup", "direction unavailable")
    if not isinstance(message_types, list) or message_type not in message_types:
        raise WorkerHttpError(409, "connector version lookup", "message type unavailable")
    if not isinstance(warehouse_ids, list):
        raise WorkerHttpError(409, "connector version lookup", "warehouse scope missing")
    warehouse_id = str(row.get("warehouse_id") or "").strip()
    if warehouse_id and warehouse_ids and warehouse_id not in warehouse_ids:
        raise WorkerHttpError(409, "connector version lookup", "warehouse unavailable")
    return binding
