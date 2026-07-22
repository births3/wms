#!/usr/bin/env python3
"""H8 ERP 接口表同步 Worker（独立进程）。

双向：
  入站：MSSQL if_in_* pending → WMS HTTP API → success/failed
  出站：WMS PG *erp_feedback_outbox → 通道 B(if_out_message) 和/或 通道 A(HTTP 回调)

环境变量：
  H8_MSSQL_HOST          默认 127.0.0.1
  H8_MSSQL_PORT          默认 14333
  H8_MSSQL_USER          默认 sa
  H8_MSSQL_PASSWORD      默认 Wms_Erp_If_Dev_2026!
  H8_MSSQL_DATABASE      默认 wms_erp_if
  H8_MSSQL_CONTAINER     默认 wms-mssql-erp-if（sqlcmd 回退用）
  WMS_API_BASE           默认 http://127.0.0.1:8080
  WMS_API_TOKEN          Bearer token（入站必填，除 --dry-run）
  H8_CONNECTOR_ID        本 Worker 唯一绑定的 H8 连接 UUID（必填）
  H8_WORKER_ID           默认主机名-PID
  H8_WORKER_VERSION      默认 1
  H8_HEARTBEAT_TTL_SEC   默认 max(15, 3 × 轮询秒数)
  WMS_DB_URL / DATABASE_URL  出站读 WMS outbox（PostgreSQL）
  H8_POLL_INTERVAL_SEC   默认 5
  H8_MAX_RETRY           默认 5
  H8_BATCH_SIZE          默认 10
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request
import socket
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

# 同目录 outbound_publish
_sys_path = str(Path(__file__).resolve().parent)
if _sys_path not in sys.path:
    sys.path.insert(0, _sys_path)
from exchange_lifecycle import record_preflight_failure, run_inbound_pipeline  # noqa: E402
from outbound_publish import (  # noqa: E402
    process_outbound_once,
    resolve_callback_base,
    resolve_outbound_transport,
    resolve_wms_db_url,
)
from worker_route import (  # noqa: E402
    RouteBinding,
    WorkerHttpError,
    get_worker_claim_decision as get_worker_claim_decision_with_http,
    is_retryable_worker_error,
    post_worker_heartbeat as post_worker_heartbeat_with_http,
    resolve_inbound_route as resolve_inbound_route_with_http,
    validate_row_schema_version,
)
from worker_mssql import claim_rows, mark_row, sqlcmd_query  # noqa: E402

# ---------------------------------------------------------------------------
# 配置
# ---------------------------------------------------------------------------


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
    mssql_container: str
    api_base: str
    api_token: str | None
    poll_interval: float
    max_retry: int
    batch_size: int
    use_sqlcmd: bool
    connector_id: str
    worker_id: str
    worker_version: str
    heartbeat_ttl_seconds: int

    @classmethod
    def from_env(cls) -> "Settings":
        use_sqlcmd = os.environ.get("H8_USE_SQLCMD", "1") != "0"
        poll_interval = float(os.environ.get("H8_POLL_INTERVAL_SEC", "5"))
        connector_id = env("H8_CONNECTOR_ID")
        try:
            uuid.UUID(connector_id)
        except ValueError as exc:
            raise SystemExit("H8_CONNECTOR_ID must be UUID") from exc
        return cls(
            mssql_host=os.environ.get("H8_MSSQL_HOST", "127.0.0.1"),
            mssql_port=os.environ.get("H8_MSSQL_PORT", "14333"),
            mssql_user=os.environ.get("H8_MSSQL_USER", "sa"),
            mssql_password=os.environ.get("H8_MSSQL_PASSWORD", "Wms_Erp_If_Dev_2026!"),
            mssql_database=os.environ.get("H8_MSSQL_DATABASE", "wms_erp_if"),
            mssql_container=os.environ.get("H8_MSSQL_CONTAINER", "wms-mssql-erp-if"),
            api_base=os.environ.get("WMS_API_BASE", "http://127.0.0.1:8080").rstrip(
                "/"
            ),
            api_token=os.environ.get("WMS_API_TOKEN") or None,
            poll_interval=poll_interval,
            max_retry=int(os.environ.get("H8_MAX_RETRY", "5")),
            batch_size=int(os.environ.get("H8_BATCH_SIZE", "10")),
            use_sqlcmd=use_sqlcmd,
            connector_id=connector_id,
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


# ---------------------------------------------------------------------------
# WMS HTTP
# ---------------------------------------------------------------------------


def http_json(
    settings: Settings,
    method: str,
    path: str,
    body: dict[str, Any] | None,
    idempotency_key: str,
) -> tuple[int, dict[str, Any] | None, str]:
    url = f"{settings.api_base}{path}"
    data = None if body is None else json.dumps(body).encode("utf-8")
    headers = {
        "Accept": "application/json",
        "Idempotency-Key": idempotency_key,
    }
    if body is not None:
        headers["Content-Type"] = "application/json"
    if settings.api_token:
        headers["Authorization"] = f"Bearer {settings.api_token}"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            raw = resp.read().decode("utf-8")
            parsed = json.loads(raw) if raw else None
            return resp.status, parsed, raw
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        try:
            parsed = json.loads(raw) if raw else None
        except json.JSONDecodeError:
            parsed = None
        return exc.code, parsed, raw
    except urllib.error.URLError as exc:
        return 0, None, str(exc.reason)


def resolve_inbound_route(
    settings: Settings,
    message_type: str,
    row: dict[str, str],
    *,
    http_json_fn: Callable[..., tuple[int, dict[str, Any] | None, str]] = http_json,
) -> RouteBinding:
    return resolve_inbound_route_with_http(
        settings,
        message_type,
        row,
        http_json_fn=http_json_fn,
    )


def get_worker_claim_decision(settings: Settings, direction: str) -> bool:
    return get_worker_claim_decision_with_http(
        settings,
        settings.connector_id,
        direction,
        http_json_fn=http_json,
    )


def try_record_worker_heartbeat(
    settings: Settings, directions: list[str], current_claims: int
) -> None:
    try:
        post_worker_heartbeat_with_http(
            settings,
            directions,
            current_claims,
            http_json_fn=http_json,
        )
    except Exception as exc:  # noqa: BLE001 — 监控失败不得中断在途业务
        print(f"[h8] heartbeat warn: {exc}", flush=True)


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def handle_asn(settings: Settings, row: dict[str, str]) -> str:
    receipt_no = row.get("receipt_no") or ""
    if not receipt_no.strip():
        # 由服务端 M-CG 生成时部分实现仍要求字段；给外部可读单号
        receipt_no = f"ERP-{row['external_doc_no']}"
    # expected_arrival 需 RFC3339
    arrival = row["expected_arrival_at"]
    if arrival and not arrival.endswith("Z") and "+" not in arrival:
        arrival = arrival + "Z"
    body = {
        "receipt_no": receipt_no,
        "document_type": row["document_type"] or "purchase_inbound",
        "supplier_id": row["supplier_id"],
        "warehouse_id": row["warehouse_id"],
        "external_ref": row.get("external_ref") or row["external_doc_no"],
        "expected_arrival_at": arrival,
        "lines": [
            {
                "line_no": 1,
                "product_id": None,
                "product_code": row["product_code"],
                "expected_qty": int(row["expected_qty"]),
                "batch_no": None,
                "production_date": None,
                "expiry_date": None,
            }
        ],
    }
    status, parsed, raw = http_json(
        settings,
        "POST",
        "/api/v1/inbound/receiving-orders",
        body,
        row["idempotency_key"],
    )
    if status in (200, 201) and isinstance(parsed, dict) and parsed.get("id"):
        return str(parsed["id"])
    raise WorkerHttpError(status, "ASN API", raw)


def handle_outbound(settings: Settings, row: dict[str, str]) -> str:
    wms_no = row.get("wms_order_no") or ""
    if not wms_no.strip():
        wms_no = f"WMS-{row['external_doc_no']}"
    ship_at = row.get("required_ship_at") or None
    if ship_at == "":
        ship_at = None
    elif ship_at and not ship_at.endswith("Z") and "+" not in ship_at:
        ship_at = ship_at + "Z"
    batch_no = (row.get("batch_no") or "").strip() or "ERP-UNSPEC"
    line: dict[str, Any] = {
        "line_no": 1,
        "product_code": row["product_code"],
        "batch_no": batch_no,
        "planned_qty": int(row["planned_qty"]),
    }
    body: dict[str, Any] = {
        "document_type": row["document_type"] or "sales_outbound",
        "wms_order_no": wms_no,
        "erp_order_no": row.get("erp_order_no") or row["external_doc_no"],
        "customer_id": row["customer_id"],
        "warehouse_id": row["warehouse_id"],
        "required_ship_at": ship_at,
        "lines": [line],
    }
    status, parsed, raw = http_json(
        settings,
        "POST",
        "/api/v1/outbound/orders",
        body,
        row["idempotency_key"],
    )
    if status in (200, 201) and isinstance(parsed, dict) and parsed.get("id"):
        return str(parsed["id"])
    raise WorkerHttpError(status, "Outbound API", raw)


def handle_product(settings: Settings, row: dict[str, str]) -> str:
    # 与 master-data CreateProduct 契约一致：storage_condition 走 attrs 枚举
    storage = (row.get("storage_condition") or "normal").strip().lower()
    if storage not in ("frozen", "cold", "cool", "normal"):
        storage = "normal"
    attrs: dict[str, Any] = {
        "storage_condition": storage,
        "source": "erp_interface",
    }
    body = {
        "product_code": row["product_code"],
        "product_name": row["product_name"],
        "approval_no": row.get("approval_no") or None,
        "spec": row.get("spec") or None,
        "dosage_form": row.get("dosage_form") or None,
        "manufacturer": row.get("manufacturer") or None,
        "special_drug_category_code": "none",
        "attrs": attrs,
    }
    # 空串转 null
    for key in ("approval_no", "spec", "dosage_form", "manufacturer"):
        if body[key] == "":
            body[key] = None
    status, parsed, raw = http_json(
        settings,
        "POST",
        "/api/v1/master-data/products",
        body,
        row["idempotency_key"],
    )
    if status in (200, 201) and isinstance(parsed, dict) and parsed.get("id"):
        return str(parsed["id"])
    raise WorkerHttpError(status, "Product API", raw)


def handle_return(settings: Settings, row: dict[str, str]) -> str:
    """销退入库：走收货单 API，document_type 默认 sales_return（必填原批号）。"""
    receipt_no = row.get("receipt_no") or ""
    if not receipt_no.strip():
        receipt_no = f"ERP-RET-{row['external_doc_no']}"
    arrival = row["expected_arrival_at"]
    if arrival and not arrival.endswith("Z") and "+" not in arrival:
        arrival = arrival + "Z"
    batch_no = (row.get("batch_no") or "").strip()
    if not batch_no:
        raise RuntimeError("sales_return requires batch_no (original batch)")
    supplier_id = (row.get("supplier_id") or "").strip() or row["customer_id"]
    body = {
        "receipt_no": receipt_no,
        "document_type": row.get("document_type") or "sales_return",
        "supplier_id": supplier_id,
        "warehouse_id": row["warehouse_id"],
        "external_ref": row.get("external_ref") or row["external_doc_no"],
        "expected_arrival_at": arrival,
        "lines": [
            {
                "line_no": 1,
                "product_id": None,
                "product_code": row["product_code"],
                "expected_qty": int(row["expected_qty"]),
                "batch_no": batch_no,
                "production_date": None,
                "expiry_date": None,
            }
        ],
    }
    status, parsed, raw = http_json(
        settings,
        "POST",
        "/api/v1/inbound/receiving-orders",
        body,
        row["idempotency_key"],
    )
    if status in (200, 201) and isinstance(parsed, dict) and parsed.get("id"):
        return str(parsed["id"])
    raise WorkerHttpError(status, "Return ASN API", raw)


def handle_product_change(settings: Settings, row: dict[str, str]) -> str:
    """档案补录/主数据变更回写：按 product_id 或 list 匹配 product_code 后 PATCH。"""
    product_id = (row.get("product_id") or "").strip()
    if not product_id:
        status, parsed, raw = http_json(
            settings,
            "GET",
            "/api/v1/master-data/products",
            None,
            row["idempotency_key"] + "-list",
        )
        if status != 200 or not isinstance(parsed, dict):
            raise WorkerHttpError(status, "list products", raw)
        items = parsed.get("data") or parsed.get("items") or []
        if not isinstance(items, list):
            raise RuntimeError("products list shape unexpected")
        code = row["product_code"]
        match = next(
            (p for p in items if isinstance(p, dict) and p.get("product_code") == code),
            None,
        )
        if not match or not match.get("id"):
            raise RuntimeError(f"product_code {code} not found")
        product_id = str(match["id"])

    field = (row.get("field_name") or "").strip()
    new_value = row.get("new_value") or ""
    body: dict[str, Any] = {}
    # 常见字段映射到 UpdateProductRequest
    if field in (
        "product_name",
        "approval_no",
        "spec",
        "dosage_form",
        "manufacturer",
        "status",
        "special_drug_category_code",
    ):
        body[field] = new_value
    else:
        body["attrs"] = {field: new_value}

    status, parsed, raw = http_json(
        settings,
        "PATCH",
        f"/api/v1/master-data/products/{product_id}",
        body,
        row["idempotency_key"],
    )
    if status in (200, 201) and isinstance(parsed, dict) and parsed.get("id"):
        return str(parsed["id"])
    raise WorkerHttpError(status, "Product change API", raw)


HANDLERS: dict[str, tuple[str, Callable[[Settings, dict[str, str]], str]]] = {
    "asn": ("if_in_asn", handle_asn),
    "outbound_order": ("if_in_outbound_order", handle_outbound),
    "product_master": ("if_in_product_master", handle_product),
    "return_order": ("if_in_return_order", handle_return),
    "product_change": ("if_in_product_change", handle_product_change),
}


def process_once(
    settings: Settings,
    types: list[str],
    dry_run: bool,
    heartbeat_directions: list[str] | None = None,
) -> int:
    processed = 0
    heartbeat_directions = heartbeat_directions or ["inbound"]
    for type_name in types:
        table, handler = HANDLERS[type_name]
        if not dry_run:
            try_record_worker_heartbeat(settings, heartbeat_directions, 0)
            if not get_worker_claim_decision(settings, "inbound"):
                print(
                    f"[h8] paused connector={settings.connector_id} direction=inbound",
                    flush=True,
                )
                continue
        rows = claim_rows(settings, table)
        if not dry_run:
            try_record_worker_heartbeat(settings, heartbeat_directions, len(rows))
        for row in rows:
            processed += 1
            row_id = row["id"]
            retry = int(row.get("retry_count") or "0")
            print(
                f"[h8] claim {type_name} id={row_id} doc={row.get('external_doc_no')}",
                flush=True,
            )
            if dry_run:
                # 认领已置 processing：释放回 pending；仍走 lifecycle receive/convert 审计
                try:
                    run_inbound_pipeline(
                        settings,
                        type_name,
                        row,
                        handler,
                        dry_run=True,
                    )
                except Exception as life_exc:  # noqa: BLE001
                    print(f"[h8] lifecycle dry-run warn: {life_exc}", flush=True)
                mark_row(
                    settings,
                    table,
                    row_id,
                    "pending",
                    error=None,
                    retry_count=retry,
                )
                print(f"[h8] dry-run release {type_name} id={row_id}", flush=True)
                continue
            try:
                pipeline_started = False
                validate_row_schema_version(row)
                binding = resolve_inbound_route(settings, type_name, row)
                if binding.connector_id != settings.connector_id:
                    raise WorkerHttpError(
                        409,
                        "route binding",
                        f"resolved connector {binding.connector_id} differs from worker binding",
                    )
                # US-H8-002 AC11：真实路径 emit receive→convert→business_api→receipt
                pipeline_started = True
                wms_id, _life = run_inbound_pipeline(
                    settings,
                    type_name,
                    row,
                    handler,
                    route_binding=binding,
                    dry_run=False,
                )
                mark_row(settings, table, row_id, "success", wms_id=wms_id)
                print(f"[h8] success {type_name} -> {wms_id}", flush=True)
            except Exception as exc:  # noqa: BLE001 — worker 边界
                error = exc
                if not pipeline_started:
                    try:
                        record_preflight_failure(
                            settings,
                            type_name,
                            row,
                            settings.connector_id,
                        )
                    except Exception as audit_exc:  # noqa: BLE001 — 释放认领后重试审计
                        error = audit_exc
                retry += 1
                retryable = is_retryable_worker_error(error)
                next_status = (
                    "pending" if retryable and retry < settings.max_retry else "dead"
                )
                mark_row(
                    settings,
                    table,
                    row_id,
                    next_status,
                    error=str(error),
                    retry_count=retry,
                )
                print(f"[h8] error {type_name}: {error}", flush=True)
        if not dry_run:
            try_record_worker_heartbeat(settings, heartbeat_directions, 0)
    return processed


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="H8 ERP interface table sync worker")
    parser.add_argument(
        "--once",
        action="store_true",
        help="只跑一轮",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="认领但不调用 WMS / 不投递出站",
    )
    parser.add_argument(
        "--direction",
        choices=("in", "out", "both"),
        default="both",
        help="in=仅入站, out=仅出站(WMS outbox→if_out), both=双向",
    )
    parser.add_argument(
        "--types",
        default="asn,outbound_order,product_master,return_order,product_change",
        help="入站类型：asn,outbound_order,product_master,return_order,product_change",
    )
    parser.add_argument(
        "--transport",
        choices=("table", "http", "both", "failover"),
        default=None,
        help=(
            "出站：table=B, http=A, failover=REST失败转表, both=本地双写；"
            "默认 H8_OUTBOUND_TRANSPORT / H8_CHANNEL_MODE / DB channel_mode"
        ),
    )
    args = parser.parse_args(argv)
    settings = Settings.from_env()
    need_in = args.direction in ("in", "both")
    need_out = args.direction in ("out", "both")
    wms_db = resolve_wms_db_url()
    transport = args.transport or resolve_outbound_transport(database_url=wms_db)
    callback_base = resolve_callback_base()
    # US-H8-001：生产 channel_mode 禁止同时双写；both 仅本地联调
    if (
        transport == "both"
        and os.environ.get("H8_ALLOW_LOCAL_DUAL_TRANSPORT", "0") != "1"
    ):
        print(
            "transport=both is local dual-channel probe only; "
            "set H8_ALLOW_LOCAL_DUAL_TRANSPORT=1 to override "
            "(production uses rest_primary_table_fallback failover, not dual write)",
            file=sys.stderr,
        )
        return 2
    if need_in and not args.dry_run and not settings.api_token:
        print("WMS_API_TOKEN is required for inbound unless --dry-run", file=sys.stderr)
        return 2
    if need_out and not wms_db and not args.dry_run:
        print(
            "WMS_DB_URL (or DATABASE_URL) is required for outbound publish",
            file=sys.stderr,
        )
        return 2
    if (
        need_out
        and transport in ("http", "both", "failover")
        and not callback_base
        and not args.dry_run
    ):
        print(
            "ERP_CALLBACK_BASE required for http/both/failover transport",
            file=sys.stderr,
        )
        return 2
    if need_out and transport in ("table", "both", "failover") and not args.dry_run:
        # failover 需要 table 后备；table 接口由 sqlcmd 提供
        pass
    types = [t.strip() for t in args.types.split(",") if t.strip()]
    for t in types:
        if t not in HANDLERS:
            print(f"unknown type {t}", file=sys.stderr)
            return 2

    print(
        f"[h8] worker start api={settings.api_base} direction={args.direction} "
        f"transport={transport} types={types} once={args.once}",
        flush=True,
    )
    heartbeat_directions = [
        direction
        for direction, enabled in (("inbound", need_in), ("outbound", need_out))
        if enabled
    ]
    while True:
        n = 0
        if not args.dry_run:
            try_record_worker_heartbeat(settings, heartbeat_directions, 0)
        if need_in:
            n += process_once(
                settings,
                types,
                dry_run=args.dry_run,
                heartbeat_directions=heartbeat_directions,
            )
        outbound_allowed = (
            not need_out
            or args.dry_run
            or get_worker_claim_decision(settings, "outbound")
        )
        if need_out and wms_db and outbound_allowed:
            n += process_outbound_once(
                database_url=wms_db,
                sqlcmd_exec=(
                    (lambda sql: sqlcmd_query(settings, sql))
                    if transport in ("table", "both", "failover")
                    else None
                ),
                batch_size=settings.batch_size,
                dry_run=args.dry_run,
                transport=transport,
                callback_base=callback_base,
                connector_id=settings.connector_id,
            )
        elif need_out and not outbound_allowed:
            print(
                f"[h8] paused connector={settings.connector_id} direction=outbound",
                flush=True,
            )
        if not args.dry_run:
            try_record_worker_heartbeat(settings, heartbeat_directions, 0)
        if args.once:
            print(f"[h8] done processed={n}", flush=True)
            return 0
        time.sleep(settings.poll_interval)


if __name__ == "__main__":
    raise SystemExit(main())
