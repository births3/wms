"""H8 Worker 启动配置与不可变连接快照。"""

from __future__ import annotations

import json
import os
import socket
import uuid
from dataclasses import dataclass, replace
from typing import Any, Callable

from worker_route import WorkerHttpError


def env(name: str, default: str | None = None) -> str:
    value = os.environ.get(name, default)
    if value is None or value == "":
        raise SystemExit(f"missing env {name}")
    return value


@dataclass
class Settings:
    mssql_host: str
    mssql_port: str
    mssql_user: str
    mssql_password: str
    mssql_database: str
    api_base: str
    api_token: str | None
    poll_interval: float
    max_retry: int
    batch_size: int
    connector_id: str
    connector_config_version: int
    worker_id: str
    worker_version: str
    heartbeat_ttl_seconds: int

    @classmethod
    def from_env(cls) -> "Settings":
        poll_interval = float(os.environ.get("H8_POLL_INTERVAL_SEC", "5"))
        connector_id = env("H8_CONNECTOR_ID")
        try:
            uuid.UUID(connector_id)
        except ValueError as exc:
            raise SystemExit("H8_CONNECTOR_ID must be UUID") from exc
        return cls(
            mssql_host="",
            mssql_port="",
            mssql_user="",
            mssql_password="",
            mssql_database="",
            api_base=os.environ.get("WMS_API_BASE", "http://127.0.0.1:8080").rstrip(
                "/"
            ),
            api_token=os.environ.get("WMS_API_TOKEN") or None,
            poll_interval=poll_interval,
            max_retry=int(os.environ.get("H8_MAX_RETRY", "5")),
            batch_size=int(os.environ.get("H8_BATCH_SIZE", "10")),
            connector_id=connector_id,
            connector_config_version=0,
            worker_id=os.environ.get(
                "H8_WORKER_ID", f"{socket.gethostname()}-{os.getpid()}"
            ),
            worker_version=os.environ.get("H8_WORKER_VERSION", "1"),
            heartbeat_ttl_seconds=int(
                os.environ.get(
                    "H8_HEARTBEAT_TTL_SEC", str(max(15, int(poll_interval * 3)))
                )
            ),
        )


def load_runtime_settings(
    settings: Settings,
    *,
    http_json_fn: Callable[
        [Settings, str, str, dict[str, Any] | None, str],
        tuple[int, dict[str, Any] | None, str],
    ],
) -> Settings:
    """启动时加载一次不可变连接快照；运行中不得被环境变量覆盖。"""
    if not settings.api_token:
        raise WorkerHttpError(401, "connector bootstrap", "WMS_API_TOKEN required")
    connector_path = f"/api/v1/config/erp-connectors/{settings.connector_id}"
    status, connector, raw = http_json_fn(
        settings,
        "GET",
        connector_path,
        None,
        f"worker-bootstrap-{settings.connector_id}",
    )
    if status != 200 or not isinstance(connector, dict):
        raise WorkerHttpError(status, "connector bootstrap", raw)
    if (
        str(connector.get("id") or "") != settings.connector_id
        or connector.get("status") != "active"
    ):
        raise WorkerHttpError(409, "connector bootstrap", "connector is not active")
    try:
        config_version = int(connector.get("config_version") or 0)
    except (TypeError, ValueError) as exc:
        raise WorkerHttpError(
            502, "connector bootstrap", "invalid config version"
        ) from exc
    if config_version < 1:
        raise WorkerHttpError(502, "connector bootstrap", "invalid config version")

    snapshot_path = (
        f"/api/v1/config/erp-connectors/{settings.connector_id}"
        f"/versions/{config_version}"
    )
    status, snapshot, raw = http_json_fn(
        settings,
        "GET",
        snapshot_path,
        None,
        f"worker-snapshot-{settings.connector_id}-{config_version}",
    )
    if status != 200 or not isinstance(snapshot, dict):
        raise WorkerHttpError(status, "connector snapshot", raw)
    expected = {
        "id": settings.connector_id,
        "config_version": config_version,
    }
    if any(snapshot.get(key) != value for key, value in expected.items()):
        raise WorkerHttpError(409, "connector snapshot", "snapshot identity changed")
    if snapshot.get("channel_mode") not in (
        "interface_table",
        "rest_primary_table_fallback",
    ):
        raise WorkerHttpError(
            409, "connector snapshot", "interface table channel required"
        )

    required = (
        "interface_db_host",
        "interface_db_name",
        "interface_db_username",
        "interface_db_password_alias",
    )
    values = {
        name: str(snapshot.get(name) or "").strip()
        for name in required
    }
    if any(not value for value in values.values()):
        raise WorkerHttpError(409, "connector snapshot", "MSSQL transport incomplete")
    try:
        port = int(snapshot.get("interface_db_port") or 0)
    except (TypeError, ValueError) as exc:
        raise WorkerHttpError(409, "connector snapshot", "MSSQL port invalid") from exc
    if not 1 <= port <= 65535:
        raise WorkerHttpError(409, "connector snapshot", "MSSQL port invalid")
    password = _resolve_runtime_secret(values["interface_db_password_alias"])
    return replace(
        settings,
        mssql_host=values["interface_db_host"],
        mssql_port=str(port),
        mssql_user=values["interface_db_username"],
        mssql_password=password,
        mssql_database=values["interface_db_name"],
        connector_config_version=config_version,
    )


def _resolve_runtime_secret(alias: str) -> str:
    raw = os.environ.get("WMS_H8_SECRET_ALIASES") or os.environ.get("WMS_SECRETS_MAP")
    try:
        secrets = json.loads(raw) if raw else {}
    except json.JSONDecodeError as exc:
        raise WorkerHttpError(500, "connector secret", "secrets map invalid") from exc
    if not isinstance(secrets, dict):
        raise WorkerHttpError(500, "connector secret", "secrets map invalid")
    value = secrets.get(alias)
    if not isinstance(value, str) or not value.strip():
        raise WorkerHttpError(503, "connector secret", "MSSQL secret unavailable")
    return value.strip()
