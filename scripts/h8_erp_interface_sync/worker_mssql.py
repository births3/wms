"""H8 Worker 的 MSSQL 接口表认领与状态回写。"""

from __future__ import annotations

import subprocess
from typing import Any


INBOUND_TABLES = {
    "if_in_asn",
    "if_in_outbound_order",
    "if_in_product_master",
    "if_in_return_order",
    "if_in_product_change",
}


def sqlcmd_query(settings: Any, sql: str) -> str:
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
        "-b",
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
    if "Msg " in proc.stdout and "Level" in proc.stdout:
        raise RuntimeError(f"sqlcmd logical error: {proc.stdout[:800]}")
    return proc.stdout


def sql_escape(value: str) -> str:
    return value.replace("'", "''")


def claim_rows(settings: Any, table: str) -> list[dict[str, str]]:
    """认领一批 pending 行，置为 processing，返回字段字典列表。"""
    claim_cte = f"""
SET NOCOUNT ON;
DECLARE @claimed TABLE (id UNIQUEIDENTIFIER);
;WITH cte AS (
  SELECT TOP ({int(settings.batch_size)}) id
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
    if table == "if_in_asn":
        select_sql = (
            claim_cte.format(table=table)
            + """
SELECT
  CONVERT(NVARCHAR(36), id), external_doc_no,
  CONVERT(NVARCHAR(36), owner_id), CONVERT(NVARCHAR(36), warehouse_id),
  CONVERT(NVARCHAR(36), supplier_id), product_code,
  CONVERT(NVARCHAR(32), expected_qty),
  CONVERT(NVARCHAR(33), expected_arrival_at, 126),
  document_type, ISNULL(external_ref,N''), ISNULL(receipt_no,N''),
  schema_version,
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
            "schema_version",
            "idempotency_key",
            "retry_count",
        ]
    elif table == "if_in_outbound_order":
        select_sql = (
            claim_cte.format(table=table)
            + """
SELECT
  CONVERT(NVARCHAR(36), id), external_doc_no,
  CONVERT(NVARCHAR(36), owner_id), CONVERT(NVARCHAR(36), warehouse_id),
  CONVERT(NVARCHAR(36), customer_id), document_type,
  ISNULL(erp_order_no,N''), ISNULL(wms_order_no,N''), product_code,
  ISNULL(batch_no,N''), CONVERT(NVARCHAR(32), planned_qty),
  ISNULL(CONVERT(NVARCHAR(33), required_ship_at, 126),N''),
  schema_version,
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
            "schema_version",
            "idempotency_key",
            "retry_count",
        ]
    elif table == "if_in_product_master":
        select_sql = (
            claim_cte.format(table=table)
            + """
SELECT
  CONVERT(NVARCHAR(36), id), external_doc_no,
  CONVERT(NVARCHAR(36), owner_id), product_code, product_name,
  ISNULL(approval_no,N''), ISNULL(spec,N''), ISNULL(dosage_form,N''),
  ISNULL(manufacturer,N''), ISNULL(storage_condition,N''),
  schema_version,
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
            "schema_version",
            "idempotency_key",
            "retry_count",
        ]
    elif table == "if_in_return_order":
        select_sql = (
            claim_cte.format(table=table)
            + """
SELECT
  CONVERT(NVARCHAR(36), id), external_doc_no,
  CONVERT(NVARCHAR(36), owner_id), CONVERT(NVARCHAR(36), warehouse_id),
  CONVERT(NVARCHAR(36), customer_id),
  CONVERT(NVARCHAR(36), ISNULL(supplier_id, customer_id)),
  product_code,
  CONVERT(NVARCHAR(32), expected_qty),
  CONVERT(NVARCHAR(33), expected_arrival_at, 126),
  document_type, ISNULL(external_ref,N''), ISNULL(receipt_no,N''),
  ISNULL(batch_no,N''),
  schema_version,
  idempotency_key, CONVERT(NVARCHAR(16), retry_count)
FROM dbo.if_in_return_order WHERE id IN (SELECT id FROM @claimed);
"""
        )
        cols = [
            "id",
            "external_doc_no",
            "owner_id",
            "warehouse_id",
            "customer_id",
            "supplier_id",
            "product_code",
            "expected_qty",
            "expected_arrival_at",
            "document_type",
            "external_ref",
            "receipt_no",
            "batch_no",
            "schema_version",
            "idempotency_key",
            "retry_count",
        ]
    elif table == "if_in_product_change":
        select_sql = (
            claim_cte.format(table=table)
            + """
SELECT
  CONVERT(NVARCHAR(36), id), external_doc_no,
  CONVERT(NVARCHAR(36), owner_id), product_code,
  ISNULL(CONVERT(NVARCHAR(36), product_id),N''),
  field_name, new_value,
  ISNULL(CONVERT(NVARCHAR(36), liaison_id),N''),
  ISNULL(CONVERT(NVARCHAR(36), asn_id),N''),
  schema_version,
  idempotency_key, CONVERT(NVARCHAR(16), retry_count)
FROM dbo.if_in_product_change WHERE id IN (SELECT id FROM @claimed);
"""
        )
        cols = [
            "id",
            "external_doc_no",
            "owner_id",
            "product_code",
            "product_id",
            "field_name",
            "new_value",
            "liaison_id",
            "asn_id",
            "schema_version",
            "idempotency_key",
            "retry_count",
        ]
    else:
        raise ValueError(table)

    rows: list[dict[str, str]] = []
    for line in sqlcmd_query(settings, select_sql).splitlines():
        line = line.strip()
        if not line or line.startswith("(") or line.startswith("rows"):
            continue
        parts = [part.strip() for part in line.split("|")]
        if len(parts) >= len(cols):
            rows.append(dict(zip(cols, parts)))
    return rows


def requeue_replay_row(settings: Any, table: str, idempotency_key: str) -> bool:
    """按原幂等键将终态接口行恢复为 pending，供现有认领路径处理。"""
    if table not in INBOUND_TABLES:
        raise ValueError(table)
    key = sql_escape(idempotency_key)
    output = sqlcmd_query(
        settings,
        f"""
SET NOCOUNT ON;
UPDATE dbo.{table} WITH (ROWLOCK)
   SET sync_status = N'pending',
       retry_count = CASE WHEN retry_count < 1 THEN 1 ELSE retry_count END,
       last_error = NULL,
       updated_at = SYSUTCDATETIME()
 WHERE idempotency_key = N'{key}'
   AND sync_status IN (N'failed', N'dead', N'success');
SELECT CASE WHEN EXISTS (
  SELECT 1 FROM dbo.{table}
   WHERE idempotency_key = N'{key}'
     AND sync_status = N'pending'
) THEN N'ready' ELSE N'missing' END;
""",
    )
    return any(line.strip() == "ready" for line in output.splitlines())


def mark_row(
    settings: Any,
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
    sqlcmd_query(
        settings,
        f"""
SET NOCOUNT ON;
UPDATE dbo.{table}
   SET sync_status = N'{sql_escape(status)}',
       last_error = {err_sql},
       wms_resource_id = COALESCE({wms_sql}, wms_resource_id),
       updated_at = SYSUTCDATETIME()
       {retry_sql}
 WHERE id = '{sql_escape(row_id)}';
""",
    )
