"""H8 Worker 的 MSSQL 接口表认领与状态回写。"""

from __future__ import annotations

import json
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
    """执行 SQL 并返回 stdout 文本；NVARCHAR(MAX) 不截断。"""
    cmd = [
        "sqlcmd",
        "-S",
        f"tcp:{settings.mssql_host},{settings.mssql_port}",
        "-U",
        settings.mssql_user,
        "-P",
        settings.mssql_password,
        "-C",
        "-b",
        "-d",
        settings.mssql_database,
        "-y",
        "0",
        "-w",
        "65535",
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


def parse_json_rows(output: str) -> list[dict[str, str]]:
    """解析 SQL Server `FOR JSON PATH` 输出，兼容 sqlcmd 的视觉换行。"""
    start = output.find("[")
    end = output.rfind("]")
    if start < 0 or end < start:
        if not output.strip():
            return []
        raise RuntimeError("sqlcmd JSON output missing")
    compact = output[start : end + 1].replace("\r", "").replace("\n", "")
    if not compact:
        return []
    try:
        value = json.loads(compact)
    except json.JSONDecodeError as exc:
        raise RuntimeError("sqlcmd JSON output invalid") from exc
    if not isinstance(value, list) or any(not isinstance(row, dict) for row in value):
        raise RuntimeError("sqlcmd JSON rows required")
    return [
        {
            str(key): "" if item is None else str(item)
            for key, item in row.items()
        }
        for row in value
    ]


def list_acked_outbound(settings: Any) -> list[dict[str, str]]:
    """读取 ERP 已确认、但尚未回写 H8 的出站接口行。"""
    output = sqlcmd_query(
        settings,
        f"""
SET NOCOUNT ON;
SELECT TOP ({int(settings.batch_size)})
  CONVERT(NVARCHAR(36), id) AS id,
  idempotency_key AS idempotency_key,
  ISNULL(erp_ack_ref,N'') AS erp_ack_ref
  FROM dbo.if_out_message WITH (ROWLOCK, READPAST)
 WHERE sync_status = N'acked'
 ORDER BY updated_at ASC
 FOR JSON PATH;
""",
    )
    return parse_json_rows(output)


def mark_outbound_receipt_recorded(settings: Any, row_id: str) -> None:
    sqlcmd_query(
        settings,
        f"""
SET NOCOUNT ON;
UPDATE dbo.if_out_message
   SET sync_status = N'success', updated_at = SYSUTCDATETIME()
 WHERE id = '{sql_escape(row_id)}' AND sync_status = N'acked';
""",
    )


def claim_rows(settings: Any, table: str) -> list[dict[str, str]]:
    """认领一批 pending 行，置为 processing，返回字段字典列表。"""
    claim_cte = f"""
SET NOCOUNT ON;
DECLARE @claimed TABLE (id UNIQUEIDENTIFIER);
;WITH cte AS (
  SELECT TOP ({int(settings.batch_size)}) id
    FROM dbo.{{table}} WITH (ROWLOCK, READPAST)
   WHERE sync_status = N'pending'
     AND (
       retry_count = 0
       OR last_error IS NULL
       OR DATEADD(MILLISECOND,
            (CASE
              WHEN retry_count = 1 THEN 1000
              WHEN retry_count = 2 THEN 2000
              WHEN retry_count = 3 THEN 4000
              WHEN retry_count = 4 THEN 8000
              ELSE 16000
            END * (8000 + (
              UNICODE(LEFT(idempotency_key, 1)) * 31
              + UNICODE(RIGHT(idempotency_key, 1)) * 17
              + LEN(idempotency_key)
            ) % 4001)) / 10000,
            updated_at) <= SYSUTCDATETIME()
     )
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
  CONVERT(NVARCHAR(36), id) AS id,
  external_doc_no AS external_doc_no,
  CONVERT(NVARCHAR(36), owner_id) AS owner_id,
  CONVERT(NVARCHAR(36), warehouse_id) AS warehouse_id,
  CONVERT(NVARCHAR(36), supplier_id) AS supplier_id,
  product_code AS product_code,
  CONVERT(NVARCHAR(32), expected_qty) AS expected_qty,
  CONVERT(NVARCHAR(33), expected_arrival_at, 126) AS expected_arrival_at,
  document_type AS document_type,
  ISNULL(external_ref,N'') AS external_ref,
  ISNULL(receipt_no,N'') AS receipt_no,
  schema_version AS schema_version,
  idempotency_key AS idempotency_key,
  CONVERT(NVARCHAR(16), retry_count) AS retry_count,
  CONVERT(NVARCHAR(33), created_at, 126) AS created_at
FROM dbo.if_in_asn WHERE id IN (SELECT id FROM @claimed)
FOR JSON PATH;
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
            "created_at",
        ]
    elif table == "if_in_outbound_order":
        select_sql = (
            claim_cte.format(table=table)
            + """
SELECT
  CONVERT(NVARCHAR(36), id) AS id,
  external_doc_no AS external_doc_no,
  CONVERT(NVARCHAR(36), owner_id) AS owner_id,
  CONVERT(NVARCHAR(36), warehouse_id) AS warehouse_id,
  CONVERT(NVARCHAR(36), customer_id) AS customer_id,
  document_type AS document_type,
  ISNULL(erp_order_no,N'') AS erp_order_no,
  ISNULL(wms_order_no,N'') AS wms_order_no,
  product_code AS product_code,
  ISNULL(batch_no,N'') AS batch_no,
  CONVERT(NVARCHAR(32), planned_qty) AS planned_qty,
  ISNULL(CONVERT(NVARCHAR(33), required_ship_at, 126),N'') AS required_ship_at,
  schema_version AS schema_version,
  idempotency_key AS idempotency_key,
  CONVERT(NVARCHAR(16), retry_count) AS retry_count,
  CONVERT(NVARCHAR(33), created_at, 126) AS created_at
FROM dbo.if_in_outbound_order WHERE id IN (SELECT id FROM @claimed)
FOR JSON PATH;
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
            "created_at",
        ]
    elif table == "if_in_product_master":
        select_sql = (
            claim_cte.format(table=table)
            + """
SELECT
  CONVERT(NVARCHAR(36), id) AS id,
  external_doc_no AS external_doc_no,
  CONVERT(NVARCHAR(36), owner_id) AS owner_id,
  product_code AS product_code,
  product_name AS product_name,
  ISNULL(approval_no,N'') AS approval_no,
  ISNULL(spec,N'') AS spec,
  ISNULL(dosage_form,N'') AS dosage_form,
  ISNULL(manufacturer,N'') AS manufacturer,
  special_drug_category AS special_drug_category,
  storage_condition AS storage_condition,
  ISNULL(udi_code,N'') AS udi_code,
  ISNULL(electronic_regulatory_code,N'') AS electronic_regulatory_code,
  ISNULL(CONVERT(NVARCHAR(64), length_mm),N'') AS length_mm,
  ISNULL(CONVERT(NVARCHAR(64), width_mm),N'') AS width_mm,
  ISNULL(CONVERT(NVARCHAR(64), height_mm),N'') AS height_mm,
  ISNULL(CONVERT(NVARCHAR(64), volume_cm3),N'') AS volume_cm3,
  ISNULL(CONVERT(NVARCHAR(64), weight_g),N'') AS weight_g,
  packaging_json AS packaging_json,
  schema_version AS schema_version,
  idempotency_key AS idempotency_key,
  CONVERT(NVARCHAR(16), retry_count) AS retry_count,
  CONVERT(NVARCHAR(33), created_at, 126) AS created_at
FROM dbo.if_in_product_master WHERE id IN (SELECT id FROM @claimed)
FOR JSON PATH;
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
            "special_drug_category",
            "storage_condition",
            "udi_code",
            "electronic_regulatory_code",
            "length_mm",
            "width_mm",
            "height_mm",
            "volume_cm3",
            "weight_g",
            "packaging_json",
            "schema_version",
            "idempotency_key",
            "retry_count",
            "created_at",
        ]
    elif table == "if_in_return_order":
        select_sql = (
            claim_cte.format(table=table)
            + """
SELECT
  CONVERT(NVARCHAR(36), id) AS id,
  external_doc_no AS external_doc_no,
  CONVERT(NVARCHAR(36), owner_id) AS owner_id,
  CONVERT(NVARCHAR(36), warehouse_id) AS warehouse_id,
  CONVERT(NVARCHAR(36), customer_id) AS customer_id,
  CONVERT(NVARCHAR(36), ISNULL(supplier_id, customer_id)) AS supplier_id,
  product_code AS product_code,
  CONVERT(NVARCHAR(32), expected_qty) AS expected_qty,
  CONVERT(NVARCHAR(33), expected_arrival_at, 126) AS expected_arrival_at,
  document_type AS document_type,
  ISNULL(external_ref,N'') AS external_ref,
  ISNULL(receipt_no,N'') AS receipt_no,
  ISNULL(batch_no,N'') AS batch_no,
  schema_version AS schema_version,
  idempotency_key AS idempotency_key,
  CONVERT(NVARCHAR(16), retry_count) AS retry_count,
  CONVERT(NVARCHAR(33), created_at, 126) AS created_at
FROM dbo.if_in_return_order WHERE id IN (SELECT id FROM @claimed)
FOR JSON PATH;
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
            "created_at",
        ]
    elif table == "if_in_product_change":
        select_sql = (
            claim_cte.format(table=table)
            + """
SELECT
  CONVERT(NVARCHAR(36), id) AS id,
  external_doc_no AS external_doc_no,
  CONVERT(NVARCHAR(36), owner_id) AS owner_id,
  product_code AS product_code,
  ISNULL(CONVERT(NVARCHAR(36), product_id),N'') AS product_id,
  field_name AS field_name,
  new_value AS new_value,
  ISNULL(CONVERT(NVARCHAR(36), liaison_id),N'') AS liaison_id,
  ISNULL(CONVERT(NVARCHAR(36), asn_id),N'') AS asn_id,
  schema_version AS schema_version,
  idempotency_key AS idempotency_key,
  CONVERT(NVARCHAR(16), retry_count) AS retry_count,
  CONVERT(NVARCHAR(33), created_at, 126) AS created_at
FROM dbo.if_in_product_change WHERE id IN (SELECT id FROM @claimed)
FOR JSON PATH;
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
            "created_at",
        ]
    else:
        raise ValueError(table)

    rows = parse_json_rows(sqlcmd_query(settings, select_sql))
    return [
        {column: row.get(column, "") for column in cols}
        for row in rows
    ]


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
