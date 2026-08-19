"""ERP-WMS v1.9 接口表与 PayloadDigest 规范。"""

from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
from typing import Any


CONTROL_FIELDS = {
    "PayloadDigest",
    "handelflag",
    "handelmsg",
    "error_code",
    "retry_count",
    "next_retry_at",
    "worker_id",
    "lease_until",
    "inserttime",
    "processtime",
}

# 字段顺序即 v1.9 表定义顺序；数据库生成的主键、关联代理键和控制列不列入摘要。
FIELD_SPECS: dict[str, tuple[tuple[str, str], ...]] = {
    "x_wmsinter_GoodsInfo": (
        ("GoodsID", "int"), ("GoodsCode", "str"), ("GoodsName", "str"),
        ("SubName", "str"), ("ClassCode", "str"), ("BarCode", "str"),
        ("Spec", "str"), ("Unit", "str"), ("Brand", "str"),
        ("ProduceArea", "str"), ("License", "str"), ("IsDanger", "int"),
        ("ValidityType", "str"), ("ValidityNum", "int"),
        ("RetailPrice", "decimal6"), ("TaxRate", "int"),
        ("ProduceCorp", "str"), ("StoreMemo", "str"), ("Deposite", "str"),
        ("MedicalType", "str"), ("PackagingJson", "json"), ("opType", "str"),
        ("OwnerCode", "str"), ("SchemaVersion", "str"),
        ("IdempotencyKey", "str"), ("CorrelationID", "str"),
        ("SourceVersion", "int"),
    ),
    "x_wmsinter_CustomerInfo": (
        ("ClientID", "int"), ("ClientCode", "str"), ("ClientName", "str"),
        ("SubCorpName", "str"), ("CorpType", "str"), ("Area", "str"),
        ("Address", "str"), ("Lawman", "str"), ("PostCode", "str"),
        ("LinkMan", "str"), ("LinkPhone", "str"), ("DepotAddr", "str"),
        ("DepotMan", "str"), ("DepotCall", "str"), ("SendWay", "int"),
        ("StopSend", "int"), ("opType", "str"), ("OwnerCode", "str"),
        ("SchemaVersion", "str"), ("IdempotencyKey", "str"),
        ("CorrelationID", "str"), ("SourceVersion", "int"),
    ),
    "x_wmsinter_SupplierInfo": (
        ("SupplierID", "int"), ("SupplierCode", "str"),
        ("SupplierName", "str"), ("Lawman", "str"), ("Address", "str"),
        ("LinkMan", "str"), ("LinkPhone", "str"), ("opType", "str"),
        ("OwnerCode", "str"), ("SchemaVersion", "str"),
        ("IdempotencyKey", "str"), ("CorrelationID", "str"),
        ("SourceVersion", "int"),
    ),
    "x_wmsinter_InboundOrder": (
        ("ERPBillID", "int"), ("ERPBillCode", "str"), ("Revision", "int"),
        ("OrderType", "int"), ("PartnerType", "str"), ("PartnerID", "int"),
        ("PartnerCode", "str"), ("PartnerName", "str"), ("DepotID", "int"),
        ("DepotCode", "str"), ("DeptID", "int"), ("BusiDate", "date"),
        ("SumMoney", "decimal4"), ("NoteCode", "str"), ("LineCount", "int"),
        ("OwnerCode", "str"), ("SchemaVersion", "str"),
        ("IdempotencyKey", "str"), ("CorrelationID", "str"),
        ("SourceVersion", "int"),
    ),
    "x_wmsinter_InboundOrderItems": (
        ("ERPBillID", "int"), ("ERPBillCode", "str"),
        ("Revision", "int"), ("LineNo", "int"), ("GoodsID", "int"),
        ("GoodsCode", "str"), ("GoodsName", "str"), ("Amount", "decimal4"),
        ("Price", "decimal8"), ("Sums", "decimal4"), ("BatchNo", "str"),
        ("ProduceDate", "date"), ("ValidDate", "date"), ("Unit", "str"),
        ("OwnerCode", "str"), ("CorrelationID", "str"),
        ("IdempotencyKey", "str"),
    ),
    "x_wmsinter_OutboundOrder": (
        ("ERPBillID", "int"), ("ERPBillCode", "str"), ("Revision", "int"),
        ("OrderType", "int"), ("ClientID", "int"), ("ClientCode", "str"),
        ("ClientName", "str"), ("DepotID", "int"), ("DepotCode", "str"),
        ("DeptID", "int"), ("BusiDate", "date"),
        ("RequiredShipAt", "datetime"), ("SumMoney", "decimal4"),
        ("SumTax", "decimal4"), ("SendMode", "int"),
        ("ERPAddressID", "int"), ("AddressCode", "str"), ("LinkMan", "str"),
        ("LinkCall", "str"), ("Address", "str"), ("PostCode", "str"),
        ("IsTight", "int"), ("SellType", "int"), ("LineCount", "int"),
        ("OwnerCode", "str"), ("SchemaVersion", "str"),
        ("IdempotencyKey", "str"), ("CorrelationID", "str"),
        ("SourceVersion", "int"),
    ),
    "x_wmsinter_OutboundOrderItems": (
        ("ERPBillID", "int"), ("ERPBillCode", "str"),
        ("Revision", "int"), ("LineNo", "int"), ("GoodsID", "int"),
        ("GoodsCode", "str"), ("GoodsName", "str"), ("Amount", "decimal4"),
        ("Price", "decimal8"), ("Sums", "decimal4"), ("BatchNo", "str"),
        ("Unit", "str"), ("OwnerCode", "str"), ("CorrelationID", "str"),
        ("IdempotencyKey", "str"),
    ),
    "x_wmsinter_OrderCommand": (
        ("CommandID", "str"), ("CommandType", "int"), ("ERPBillCode", "str"),
        ("Revision", "int"), ("OrderType", "int"), ("Memo", "str"),
        ("OwnerCode", "str"), ("SchemaVersion", "str"),
        ("IdempotencyKey", "str"), ("CorrelationID", "str"),
        ("SourceVersion", "int"),
    ),
    "x_wmsinter_OrderFeedback": (
        ("IdempotencyKey", "str"), ("ERPBillCode", "str"),
        ("Revision", "int"), ("OrderType", "int"), ("FeedbackType", "int"),
        ("CommandID", "str"), ("ResultCount", "int"), ("ResultCode", "str"),
        ("ResultMessage", "str"), ("WaybillNo", "str"),
        ("ExpressCompany", "str"), ("ShipTime", "datetime"),
        ("FeedbackTime", "datetime"), ("OperatorName", "str"),
        ("OwnerCode", "str"), ("SchemaVersion", "str"),
        ("CorrelationID", "str"), ("SourceVersion", "int"),
    ),
    "x_wmsinter_InboundFeedback": (
        ("IdempotencyKey", "str"), ("ERPBillCode", "str"),
        ("Revision", "int"), ("LineNo", "int"), ("GoodsID", "int"),
        ("GoodsCode", "str"), ("ExpectedAmount", "decimal4"),
        ("ActualAmount", "decimal4"), ("RejectAmount", "decimal4"),
        ("ShortageAmount", "decimal4"), ("RejectReason", "str"),
        ("ShortageReason", "str"), ("BatchNo", "str"),
        ("ProduceDate", "date"), ("ValidDate", "date"), ("StallCode", "str"),
        ("OperatorName", "str"), ("ScanTime", "datetime"),
        ("OwnerCode", "str"), ("SchemaVersion", "str"),
        ("CorrelationID", "str"), ("SourceVersion", "int"),
    ),
    "x_wmsinter_OutboundFeedback": (
        ("IdempotencyKey", "str"), ("ERPBillCode", "str"),
        ("Revision", "int"), ("LineNo", "int"), ("GoodsID", "int"),
        ("GoodsCode", "str"), ("BatchNo", "str"),
        ("ExpectedAmount", "decimal4"), ("PickedAmount", "decimal4"),
        ("ShippedAmount", "decimal4"), ("OperatorName", "str"),
        ("OwnerCode", "str"), ("SchemaVersion", "str"),
        ("CorrelationID", "str"), ("SourceVersion", "int"),
    ),
    "x_wmsinter_WmsEvent": (
        ("IdempotencyKey", "str"), ("EventType", "str"),
        ("SchemaVersion", "str"), ("PayloadJson", "json"),
        ("EventTime", "datetime"), ("OwnerCode", "str"),
        ("CorrelationID", "str"), ("SourceVersion", "int"),
    ),
    "x_wmsinter_InventoryPushHeader": (
        ("SnapshotID", "str"), ("DepotID", "int"), ("DepotCode", "str"),
        ("PushType", "int"), ("PushTime", "datetime"), ("TotalCount", "int"),
        ("OwnerCode", "str"), ("SchemaVersion", "str"),
        ("IdempotencyKey", "str"), ("CorrelationID", "str"),
        ("SourceVersion", "int"),
    ),
    "x_wmsinter_InventoryPushItems": (
        ("SnapshotID", "str"), ("RowNo", "int"), ("GoodsID", "int"),
        ("GoodsCode", "str"), ("BatchID", "int"), ("BatchNo", "str"),
        ("ValidDate", "date"), ("StallCode", "str"),
        ("GoodsStatus", "str"), ("RealAmount", "decimal4"),
        ("CanSell", "decimal4"), ("OwnerCode", "str"),
        ("CorrelationID", "str"), ("IdempotencyKey", "str"),
    ),
    "x_wmsinter_InventoryReceiveHeader": (
        ("SnapshotID", "str"), ("ReceiveTime", "datetime"),
        ("TotalCount", "int"), ("OwnerCode", "str"),
        ("SchemaVersion", "str"), ("IdempotencyKey", "str"),
        ("CorrelationID", "str"), ("SourceVersion", "int"),
    ),
    "x_wmsinter_InventoryReceiveItems": (
        ("SnapshotID", "str"), ("RowNo", "int"), ("DepotCode", "str"),
        ("GoodsCode", "str"), ("BatchNo", "str"), ("ValidDate", "date"),
        ("GoodsStatus", "str"), ("WMSAmount", "decimal4"),
        ("WMSPickable", "decimal4"),
        ("WMSAllocated", "decimal4"), ("WMSFrozen", "decimal4"),
        ("OwnerCode", "str"), ("CorrelationID", "str"),
        ("IdempotencyKey", "str"),
    ),
}

CHILD_TABLES = {
    "x_wmsinter_InboundOrder": ("x_wmsinter_InboundOrderItems", "LineNo"),
    "x_wmsinter_OutboundOrder": ("x_wmsinter_OutboundOrderItems", "LineNo"),
    "x_wmsinter_InventoryPushHeader": ("x_wmsinter_InventoryPushItems", "RowNo"),
    "x_wmsinter_InventoryReceiveHeader": ("x_wmsinter_InventoryReceiveItems", "RowNo"),
}


class ContractError(ValueError):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def _canonical_value(value: Any, kind: str) -> Any:
    if value is None:
        return None
    if kind == "str" or kind == "date":
        return str(value)
    if kind == "int":
        return int(value)
    if kind.startswith("decimal"):
        scale = int(kind.removeprefix("decimal"))
        try:
            number = Decimal(str(value))
        except InvalidOperation as exc:
            raise ValueError(f"invalid decimal: {value}") from exc
        return f"{number:.{scale}f}"
    if kind == "datetime":
        if isinstance(value, datetime):
            parsed = value
        else:
            raw = str(value).strip().replace("Z", "+00:00")
            parsed = datetime.fromisoformat(raw)
        if parsed.tzinfo is None:
            parsed = parsed.replace(tzinfo=timezone.utc)
        return parsed.astimezone(timezone.utc).isoformat(timespec="milliseconds").replace(
            "+00:00", "Z"
        )
    if kind == "json":
        parsed = json.loads(value) if isinstance(value, str) else value
        return json.dumps(parsed, ensure_ascii=False, separators=(",", ":"))
    raise ValueError(f"unsupported field kind: {kind}")


def canonical_record(table: str, row: dict[str, Any]) -> dict[str, Any]:
    try:
        specs = FIELD_SPECS[table]
    except KeyError as exc:
        raise ValueError(f"unsupported v1.9 table: {table}") from exc
    return {name: _canonical_value(row.get(name), kind) for name, kind in specs}


def canonical_payload_json(
    table: str,
    row: dict[str, Any],
    children: list[dict[str, Any]] | None = None,
    *,
    sort_children: bool = True,
) -> str:
    head = canonical_record(table, row)
    if table not in CHILD_TABLES:
        payload: Any = head
    else:
        child_table, order_field = CHILD_TABLES[table]
        child_rows = list(children or [])
        if sort_children:
            child_rows.sort(key=lambda item: int(item[order_field]))
        payload = [head, *(canonical_record(child_table, item) for item in child_rows)]
    return json.dumps(payload, ensure_ascii=False, separators=(",", ":"))


def payload_digest(
    table: str,
    row: dict[str, Any],
    children: list[dict[str, Any]] | None = None,
) -> str:
    canonical = canonical_payload_json(table, row, children)
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def validate_published_unit(table: str, row: dict[str, Any]) -> None:
    children = list(row.get("_items") or [])
    if table in CHILD_TABLES:
        count_field = "TotalCount" if "TotalCount" in row else "LineCount"
        if len(children) != int(row[count_field]):
            raise ContractError(
                "LINE_COUNT_MISMATCH",
                f"{count_field}={row[count_field]}, actual={len(children)}",
            )
        for child in children:
            if child.get("OwnerCode") != row.get("OwnerCode") or child.get(
                "CorrelationID"
            ) != row.get("CorrelationID"):
                raise ContractError("INVALID_DATA", "child envelope mismatch")
            if "Revision" in row and child.get("Revision") != row.get("Revision"):
                raise ContractError("INVALID_DATA", "child revision mismatch")
            if "SnapshotID" in row and child.get("SnapshotID") != row.get("SnapshotID"):
                raise ContractError("INVALID_DATA", "child snapshot mismatch")
    actual_digest = payload_digest(table, row, children)
    if actual_digest != str(row.get("PayloadDigest") or "").lower():
        raise ContractError("INVALID_DATA", "PayloadDigest mismatch")
