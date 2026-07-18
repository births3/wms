#!/usr/bin/env python3
"""H8 ERP 接口表同步 Worker（独立进程）。

连接 MSSQL 接口库，认领 pending 行，调用 WMS HTTP API，回写 success/failed。

环境变量：
  H8_MSSQL_HOST          默认 127.0.0.1
  H8_MSSQL_PORT          默认 14333
  H8_MSSQL_USER          默认 sa
  H8_MSSQL_PASSWORD      默认 Wms_Erp_If_Dev_2026!
  H8_MSSQL_DATABASE      默认 wms_erp_if
  H8_MSSQL_CONTAINER     默认 wms-mssql-erp-if（sqlcmd 回退用）
  WMS_API_BASE           默认 http://127.0.0.1:8080
  WMS_API_TOKEN          Bearer token（必填，除 --dry-run）
  H8_POLL_INTERVAL_SEC   默认 5
  H8_MAX_RETRY           默认 5
  H8_BATCH_SIZE          默认 10
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any, Callable

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

    @classmethod
    def from_env(cls) -> "Settings":
        use_sqlcmd = os.environ.get("H8_USE_SQLCMD", "1") != "0"
        return cls(
            mssql_host=os.environ.get("H8_MSSQL_HOST", "127.0.0.1"),
            mssql_port=os.environ.get("H8_MSSQL_PORT", "14333"),
            mssql_user=os.environ.get("H8_MSSQL_USER", "sa"),
            mssql_password=os.environ.get(
                "H8_MSSQL_PASSWORD", "Wms_Erp_If_Dev_2026!"
            ),
            mssql_database=os.environ.get("H8_MSSQL_DATABASE", "wms_erp_if"),
            mssql_container=os.environ.get("H8_MSSQL_CONTAINER", "wms-mssql-erp-if"),
            api_base=os.environ.get("WMS_API_BASE", "http://127.0.0.1:8080").rstrip(
                "/"
            ),
            api_token=os.environ.get("WMS_API_TOKEN") or None,
            poll_interval=float(os.environ.get("H8_POLL_INTERVAL_SEC", "5")),
            max_retry=int(os.environ.get("H8_MAX_RETRY", "5")),
            batch_size=int(os.environ.get("H8_BATCH_SIZE", "10")),
            use_sqlcmd=use_sqlcmd,
        )


# ---------------------------------------------------------------------------
# MSSQL 访问（默认 docker exec sqlcmd，避免本机装 ODBC）
# ---------------------------------------------------------------------------


def sqlcmd_query(settings: Settings, sql: str) -> str:
    """执行 SQL 并返回 stdout 文本（-s| -W -h-1）。"""
    cmd = [
        "docker",
        "exec",
        "-i",
        settings.mssql_container,
        "/opt/mssql-tools18/bin/sqlcmd",
        "-S",
        "localhost",
        "-U",
        settings.mssql_user,
        "-P",
        settings.mssql_password,
        "-C",
        "-b",  # 遇错误非零退出
        "-d",
        settings.mssql_database,
        "-s",
        "|",
        "-W",
        "-h",
        "-1",
        "-Q",
        sql,
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        raise RuntimeError(
            f"sqlcmd failed rc={proc.returncode}: {proc.stderr or proc.stdout}"
        )
    # 部分版本仍把 Msg 写到 stdout 且 rc=0
    if "Msg " in proc.stdout and "Level" in proc.stdout:
        raise RuntimeError(f"sqlcmd logical error: {proc.stdout[:800]}")
    return proc.stdout


def sql_escape(value: str) -> str:
    return value.replace("'", "''")


def claim_rows(settings: Settings, table: str) -> list[dict[str, str]]:
    """认领一批 pending 行，置为 processing，返回字段字典列表。

    SQL Server 不允许 ``UPDATE TOP ... ORDER BY``；用 CTE 排序后更新。
    """
    batch = int(settings.batch_size)
    claim_cte = f"""
SET NOCOUNT ON;
DECLARE @claimed TABLE (id UNIQUEIDENTIFIER);
;WITH cte AS (
  SELECT TOP ({batch}) id
    FROM dbo.{{table}} WITH (ROWLOCK, READPAST)
   WHERE sync_status = N'pending'
   ORDER BY updated_at ASC
)
UPDATE t
   SET sync_status = N'processing', updated_at = SYSUTCDATETIME()
OUTPUT inserted.id INTO @claimed
  FROM dbo.{{table}} t
  INNER JOIN cte ON cte.id = t.id;
"""
    # 不同表列不同：用动态列集合查询
    if table == "if_in_asn":
        select_sql = (
            claim_cte.format(table="if_in_asn")
            + """
SELECT
  CONVERT(NVARCHAR(36), id), external_doc_no,
  CONVERT(NVARCHAR(36), owner_id), CONVERT(NVARCHAR(36), warehouse_id),
  CONVERT(NVARCHAR(36), supplier_id), product_code,
  CONVERT(NVARCHAR(32), expected_qty),
  CONVERT(NVARCHAR(33), expected_arrival_at, 126),
  document_type, ISNULL(external_ref,N''), ISNULL(receipt_no,N''),
  idempotency_key, CONVERT(NVARCHAR(16), retry_count)
FROM dbo.if_in_asn WHERE id IN (SELECT id FROM @claimed);
"""
        )
        cols = [
            "id",
            "external_doc_no",
            "owner_id",
            "warehouse_id",
            "supplier_id",
            "product_code",
            "expected_qty",
            "expected_arrival_at",
            "document_type",
            "external_ref",
            "receipt_no",
            "idempotency_key",
            "retry_count",
        ]
    elif table == "if_in_outbound_order":
        select_sql = (
            claim_cte.format(table="if_in_outbound_order")
            + """
SELECT
  CONVERT(NVARCHAR(36), id), external_doc_no,
  CONVERT(NVARCHAR(36), owner_id), CONVERT(NVARCHAR(36), warehouse_id),
  CONVERT(NVARCHAR(36), customer_id), document_type,
  ISNULL(erp_order_no,N''), ISNULL(wms_order_no,N''), product_code,
  ISNULL(batch_no,N''), CONVERT(NVARCHAR(32), planned_qty),
  ISNULL(CONVERT(NVARCHAR(33), required_ship_at, 126),N''),
  idempotency_key, CONVERT(NVARCHAR(16), retry_count)
FROM dbo.if_in_outbound_order WHERE id IN (SELECT id FROM @claimed);
"""
        )
        cols = [
            "id",
            "external_doc_no",
            "owner_id",
            "warehouse_id",
            "customer_id",
            "document_type",
            "erp_order_no",
            "wms_order_no",
            "product_code",
            "batch_no",
            "planned_qty",
            "required_ship_at",
            "idempotency_key",
            "retry_count",
        ]
    elif table == "if_in_product_master":
        select_sql = (
            claim_cte.format(table="if_in_product_master")
            + """
SELECT
  CONVERT(NVARCHAR(36), id), external_doc_no,
  CONVERT(NVARCHAR(36), owner_id), product_code, product_name,
  ISNULL(approval_no,N''), ISNULL(spec,N''), ISNULL(dosage_form,N''),
  ISNULL(manufacturer,N''), ISNULL(storage_condition,N''),
  idempotency_key, CONVERT(NVARCHAR(16), retry_count)
FROM dbo.if_in_product_master WHERE id IN (SELECT id FROM @claimed);
"""
        )
        cols = [
            "id",
            "external_doc_no",
            "owner_id",
            "product_code",
            "product_name",
            "approval_no",
            "spec",
            "dosage_form",
            "manufacturer",
            "storage_condition",
            "idempotency_key",
            "retry_count",
        ]
    else:
        raise ValueError(table)

    out = sqlcmd_query(settings, select_sql)
    rows: list[dict[str, str]] = []
    for line in out.splitlines():
        line = line.strip()
        if not line or line.startswith("(") or line.startswith("rows"):
            continue
        parts = [p.strip() for p in line.split("|")]
        if len(parts) < len(cols):
            continue
        rows.append({cols[i]: parts[i] for i in range(len(cols))})
    return rows


def mark_row(
    settings: Settings,
    table: str,
    row_id: str,
    status: str,
    error: str | None = None,
    wms_id: str | None = None,
    retry_count: int | None = None,
) -> None:
    err_sql = "NULL" if not error else f"N'{sql_escape(error[:900])}'"
    wms_sql = "NULL" if not wms_id else f"N'{sql_escape(wms_id)}'"
    retry_sql = "" if retry_count is None else f", retry_count = {int(retry_count)}"
    sql = f"""
SET NOCOUNT ON;
UPDATE dbo.{table}
   SET sync_status = N'{sql_escape(status)}',
       last_error = {err_sql},
       wms_resource_id = COALESCE({wms_sql}, wms_resource_id),
       updated_at = SYSUTCDATETIME()
       {retry_sql}
 WHERE id = '{sql_escape(row_id)}';
"""
    sqlcmd_query(settings, sql)


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
        "Content-Type": "application/json",
        "Idempotency-Key": idempotency_key,
    }
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
    raise RuntimeError(f"ASN API {status}: {raw[:500]}")


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
    raise RuntimeError(f"Outbound API {status}: {raw[:500]}")


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
    raise RuntimeError(f"Product API {status}: {raw[:500]}")


HANDLERS: dict[str, tuple[str, Callable[[Settings, dict[str, str]], str]]] = {
    "asn": ("if_in_asn", handle_asn),
    "outbound_order": ("if_in_outbound_order", handle_outbound),
    "product_master": ("if_in_product_master", handle_product),
}


def process_once(settings: Settings, types: list[str], dry_run: bool) -> int:
    processed = 0
    for type_name in types:
        table, handler = HANDLERS[type_name]
        rows = claim_rows(settings, table)
        for row in rows:
            processed += 1
            row_id = row["id"]
            retry = int(row.get("retry_count") or "0")
            print(
                f"[h8] claim {type_name} id={row_id} doc={row.get('external_doc_no')}",
                flush=True,
            )
            if dry_run:
                mark_row(settings, table, row_id, "pending", error="dry-run-release")
                # dry-run 不真正调 API：退回 pending 不方便，标记 failed 说明
                mark_row(
                    settings,
                    table,
                    row_id,
                    "failed",
                    error="dry-run: claimed only",
                    retry_count=retry,
                )
                continue
            try:
                wms_id = handler(settings, row)
                mark_row(settings, table, row_id, "success", wms_id=wms_id)
                print(f"[h8] success {type_name} -> {wms_id}", flush=True)
            except Exception as exc:  # noqa: BLE001 — worker 边界
                retry += 1
                # 未达上限：回 pending 便于下一轮；达上限：dead
                next_status = "dead" if retry >= settings.max_retry else "pending"
                mark_row(
                    settings,
                    table,
                    row_id,
                    next_status,
                    error=str(exc),
                    retry_count=retry,
                )
                print(f"[h8] error {type_name}: {exc}", flush=True)
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
        help="认领但不调用 WMS（标记 failed/dry-run）",
    )
    parser.add_argument(
        "--types",
        default="asn,outbound_order,product_master",
        help="逗号分隔：asn,outbound_order,product_master",
    )
    args = parser.parse_args(argv)
    settings = Settings.from_env()
    if not args.dry_run and not settings.api_token:
        print("WMS_API_TOKEN is required unless --dry-run", file=sys.stderr)
        return 2
    types = [t.strip() for t in args.types.split(",") if t.strip()]
    for t in types:
        if t not in HANDLERS:
            print(f"unknown type {t}", file=sys.stderr)
            return 2

    print(
        f"[h8] worker start api={settings.api_base} types={types} once={args.once}",
        flush=True,
    )
    while True:
        n = process_once(settings, types, dry_run=args.dry_run)
        if args.once:
            print(f"[h8] done processed={n}", flush=True)
            return 0
        time.sleep(settings.poll_interval)


if __name__ == "__main__":
    raise SystemExit(main())
