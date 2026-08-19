"""ERP-WMS v1.9 MSSQL 接口表认领与状态回写。"""

from __future__ import annotations

import importlib
from contextlib import closing
from typing import Any, Iterable


TABLE_CONTRACTS: dict[str, dict[str, Any]] = {
    "x_wmsinter_GoodsInfo": {"pk": "seqid", "pk_sql": "int"},
    "x_wmsinter_CustomerInfo": {"pk": "seqid", "pk_sql": "int"},
    "x_wmsinter_SupplierInfo": {"pk": "seqid", "pk_sql": "int"},
    "x_wmsinter_InboundOrder": {
        "pk": "OrderID",
        "pk_sql": "int",
        "child": "x_wmsinter_InboundOrderItems",
        "child_fk": "OrderID",
        "child_order": "LineNo",
    },
    "x_wmsinter_OutboundOrder": {
        "pk": "OrderID",
        "pk_sql": "int",
        "child": "x_wmsinter_OutboundOrderItems",
        "child_fk": "OrderID",
        "child_order": "LineNo",
    },
    "x_wmsinter_OrderCommand": {"pk": "CommandID", "pk_sql": "varchar(32)"},
    "x_wmsinter_InventoryPushHeader": {
        "pk": "PushID",
        "pk_sql": "int",
        "child": "x_wmsinter_InventoryPushItems",
        "child_fk": "SnapshotID",
        "child_parent": "SnapshotID",
        "child_order": "RowNo",
    },
}

INBOUND_TABLES = set(TABLE_CONTRACTS)

STATUS_VALUES = {
    "pending": 0,
    "accepted": 1,
    "processing": 2,
    "retry": 3,
    "dead": 4,
    "success": 5,
}


def _connect(settings: Any) -> Any:
    """延迟导入，治理测试无需安装运行时 TDS 驱动。"""
    try:
        pymssql = importlib.import_module("pymssql")
    except ModuleNotFoundError as exc:
        raise RuntimeError("pymssql is required for ERP-WMS interface-table mode") from exc
    return pymssql.connect(
        server=settings.mssql_host,
        port=int(settings.mssql_port),
        user=settings.mssql_user,
        password=settings.mssql_password,
        database=settings.mssql_database,
        charset="UTF-8",
        login_timeout=10,
        timeout=60,
        as_dict=True,
    )


def _plain_rows(rows: Iterable[dict[str, Any]]) -> list[dict[str, Any]]:
    def decode(value: Any) -> Any:
        if not isinstance(value, str) or value.isascii():
            return value
        try:
            return value.encode("latin1").decode("gb18030")
        except (UnicodeEncodeError, UnicodeDecodeError):
            return value

    return [{str(key): decode(value) for key, value in row.items()} for row in rows]


def mssql_query(
    settings: Any,
    sql: str,
    params: tuple[Any, ...] = (),
) -> list[dict[str, Any]]:
    """执行参数化查询；只为 SELECT/OUTPUT 返回字典行。"""
    with closing(_connect(settings)) as connection:
        with closing(connection.cursor(as_dict=True)) as cursor:
            cursor.execute(sql, params)
            rows = _plain_rows(cursor.fetchall()) if cursor.description else []
        connection.commit()
    return rows


def mssql_execute(
    settings: Any,
    sql: str,
    params: tuple[Any, ...] = (),
) -> int:
    """执行参数化 DML；返回受影响行数供调用方校验。"""
    with closing(_connect(settings)) as connection:
        with closing(connection.cursor()) as cursor:
            cursor.execute(sql, params)
            affected = cursor.rowcount
        connection.commit()
    return affected


def sqlcmd_query(settings: Any, sql: str) -> str:
    """兼容现有出站调用点；v1.9 迁移后不再启动外部 sqlcmd。"""
    mssql_execute(settings, sql)
    return ""


def sql_escape(value: str) -> str:
    """仅供旧证据脚本；运行路径必须使用参数化查询。"""
    return value.replace("'", "''")


def parse_json_rows(_output: str) -> list[dict[str, str]]:
    raise RuntimeError("FOR JSON PATH is unsupported by the SQL Server 2008 R2 baseline")


def claim_rows(settings: Any, table: str) -> list[dict[str, Any]]:
    """按 v1.9 以租约原子认领主记录，并在同一事务读取只读子记录。"""
    try:
        contract = TABLE_CONTRACTS[table]
    except KeyError as exc:
        raise ValueError(table) from exc
    pk = contract["pk"]
    pk_sql = contract["pk_sql"]
    lease_minutes = int(getattr(settings, "lease_minutes", 5))
    sql = f"""
SET NOCOUNT ON;
DECLARE @claimed TABLE (id {pk_sql});
;WITH claimable AS (
    SELECT TOP (%s) {pk}
      FROM dbo.{table} WITH (UPDLOCK, READPAST, ROWLOCK)
     WHERE OwnerCode = %s
       AND (handelflag = 0
         OR (handelflag = 3 AND next_retry_at <= SYSUTCDATETIME())
         OR (handelflag = 2 AND lease_until < SYSUTCDATETIME()))
     ORDER BY inserttime, {pk}
)
UPDATE source
   SET handelflag = 2,
       worker_id = %s,
       lease_until = DATEADD(MINUTE, %s, SYSUTCDATETIME())
OUTPUT INSERTED.{pk} INTO @claimed
  FROM dbo.{table} source
  JOIN claimable ON claimable.{pk} = source.{pk};
SELECT source.*
  FROM dbo.{table} source
  JOIN @claimed claimed ON source.{pk} = claimed.id
 ORDER BY source.inserttime, source.{pk};
"""

    params = (
        int(settings.batch_size),
        settings.owner_code,
        settings.worker_id,
        lease_minutes,
    )
    if not contract.get("child"):
        return mssql_query(settings, sql, params)

    with closing(_connect(settings)) as connection:
        with closing(connection.cursor(as_dict=True)) as cursor:
            cursor.execute(sql, params)
            rows = _plain_rows(cursor.fetchall())
            child_table = contract.get("child")
            if child_table:
                child_fk = contract["child_fk"]
                parent_field = contract.get("child_parent", pk)
                child_order = contract["child_order"]
                for row in rows:
                    cursor.execute(
                        f"SELECT * FROM dbo.{child_table} "
                        f"WHERE OwnerCode = %s AND {child_fk} = %s "
                        f"ORDER BY {child_order}",
                        (row["OwnerCode"], row[parent_field]),
                    )
                    row["_items"] = _plain_rows(cursor.fetchall())
        connection.commit()
    return rows


def requeue_replay_row(settings: Any, table: str, idempotency_key: str) -> bool:
    """人工重放仅恢复失败态；已接收/已提交记录不可回退。"""
    if table not in INBOUND_TABLES:
        raise ValueError(table)
    rows = mssql_query(
        settings,
        f"""
SET NOCOUNT ON;
UPDATE dbo.{table} WITH (ROWLOCK)
   SET handelflag = 0,
       handelmsg = NULL,
       error_code = NULL,
       next_retry_at = NULL,
       worker_id = NULL,
       lease_until = NULL,
       processtime = NULL
OUTPUT INSERTED.IdempotencyKey AS IdempotencyKey
 WHERE OwnerCode = %s AND IdempotencyKey = %s AND handelflag IN (3, 4);
""",
        (settings.owner_code, idempotency_key),
    )
    return bool(rows)


def mark_row(
    settings: Any,
    table: str,
    row_id: Any,
    status: str,
    error: str | None = None,
    retry_count: int | None = None,
    error_code: str | None = None,
) -> None:
    try:
        pk = TABLE_CONTRACTS[table]["pk"]
        handelflag = STATUS_VALUES[status]
    except KeyError as exc:
        raise ValueError(f"unsupported table/status: {table}/{status}") from exc
    if handelflag == 4:
        error_code = error_code or "INVALID_DATA"
        error = error or "不可重试错误"
    next_retry_seconds = min(60, 2 ** max(0, int(retry_count or 1) - 1))
    affected = mssql_execute(
        settings,
        f"""
UPDATE dbo.{table} WITH (ROWLOCK)
   SET handelflag = %s,
       handelmsg = %s,
       error_code = %s,
       retry_count = COALESCE(%s, retry_count),
       next_retry_at = CASE WHEN %s = 3
            THEN DATEADD(SECOND, %s, SYSUTCDATETIME()) ELSE NULL END,
       lease_until = NULL,
       processtime = CASE WHEN %s IN (1, 4, 5)
            THEN SYSUTCDATETIME() ELSE NULL END
 WHERE OwnerCode = %s AND {pk} = %s;
""",
        (
            handelflag,
            error,
            error_code,
            retry_count,
            handelflag,
            next_retry_seconds,
            handelflag,
            settings.owner_code,
            row_id,
        ),
    )
    if affected == 0:
        raise RuntimeError(
            f"no matching {table} row {row_id} for owner {settings.owner_code}"
        )


OUTBOUND_RECEIPT_TABLES: tuple[tuple[str, str], ...] = (
    ("x_wmsinter_OrderFeedback", "FeedbackID"),
    ("x_wmsinter_InboundFeedback", "FeedbackID"),
    ("x_wmsinter_OutboundFeedback", "FeedbackID"),
    ("x_wmsinter_WmsEvent", "EventID"),
    ("x_wmsinter_InventoryReceiveHeader", "ReceiveID"),
)


def list_acked_outbound(
    settings: Any,
    idempotency_keys: Iterable[str] = (),
) -> list[dict[str, Any]]:
    """只查询 H8 正在等待回执的 v1.9 出站主记录，避免全表反复扫描。"""
    keys = tuple(dict.fromkeys(str(key) for key in idempotency_keys if key))
    if not keys:
        return []
    placeholders = ", ".join("%s" for _ in keys)
    rows: list[dict[str, Any]] = []
    for table, pk in OUTBOUND_RECEIPT_TABLES:
        for row in mssql_query(
            settings,
            f"""
SELECT {pk} AS row_id, IdempotencyKey
  FROM dbo.{table}
 WHERE OwnerCode = %s
   AND handelflag = 5
   AND IdempotencyKey IN ({placeholders});
""",
            (getattr(settings, "owner_code", "ZBPF7"), *keys),
        ):
            rows.append(
                {
                    "id": f"{table}:{row['row_id']}",
                    "idempotency_key": row["IdempotencyKey"],
                }
            )
    return rows


def mark_outbound_receipt_recorded(_settings: Any, _row_id: str) -> None:
    return None
