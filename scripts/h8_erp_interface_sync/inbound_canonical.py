"""H8 入站接口 DTO → WMS canonical 命令。"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass
from typing import Any


class CanonicalMappingError(ValueError):
    status = 422


@dataclass(frozen=True)
class H8CanonicalInboundCommand:
    owner_id: str
    warehouse_id: str | None
    message_type: str
    external_ref: str
    idempotency_key: str
    correlation_id: str
    connector_id: str | None
    config_version: int | None
    channel: str
    fields: dict[str, Any]
    occurred_at: str


def _rfc3339(value: str | None) -> str | None:
    if not value:
        return None
    return value if value.endswith("Z") or "+" in value else value + "Z"


def _quantity(row: dict[str, str], key: str) -> int:
    try:
        value = int(row[key])
    except (KeyError, ValueError) as exc:
        raise CanonicalMappingError(f"invalid {key}") from exc
    if value <= 0:
        raise CanonicalMappingError(f"invalid {key}")
    return value


def _optional_positive_number(row: dict[str, str], key: str) -> float | None:
    raw = (row.get(key) or "").strip()
    if not raw:
        return None
    try:
        value = float(raw)
    except ValueError as exc:
        raise CanonicalMappingError(f"invalid {key}") from exc
    if not math.isfinite(value) or value <= 0:
        raise CanonicalMappingError(f"invalid {key}")
    return value


def _packaging_levels(row: dict[str, str]) -> list[dict[str, Any]]:
    try:
        value = json.loads(row.get("packaging_json") or "")
    except json.JSONDecodeError as exc:
        raise CanonicalMappingError("invalid packaging_json") from exc
    if not isinstance(value, list) or not value or any(
        not isinstance(level, dict) for level in value
    ):
        raise CanonicalMappingError("invalid packaging_json")
    return value


def _physical_dimensions(row: dict[str, str]) -> dict[str, float]:
    try:
        value = json.loads(row.get("new_value") or "")
    except json.JSONDecodeError as exc:
        raise CanonicalMappingError("invalid physical_dimensions") from exc
    keys = {"length_mm", "width_mm", "height_mm"}
    if not isinstance(value, dict) or set(value) != keys:
        raise CanonicalMappingError("invalid physical_dimensions")
    dimensions: dict[str, float] = {}
    for key in keys:
        raw = value[key]
        if isinstance(raw, bool) or not isinstance(raw, (int, float)):
            raise CanonicalMappingError("invalid physical_dimensions")
        number = float(raw)
        if not math.isfinite(number) or number <= 0:
            raise CanonicalMappingError("invalid physical_dimensions")
        dimensions[key] = number
    return dimensions


def _fields(message_type: str, row: dict[str, str], external_ref: str) -> dict[str, Any]:
    source_doc_no = row.get("external_doc_no") or external_ref
    if message_type == "asn":
        return {
            "receipt_no": (row.get("receipt_no") or "").strip()
            or f"ERP-{source_doc_no}",
            "document_type": row.get("document_type") or "purchase_inbound",
            "supplier_id": row["supplier_id"],
            "product_code": row["product_code"],
            "expected_qty": _quantity(row, "expected_qty"),
            "expected_arrival_at": _rfc3339(row.get("expected_arrival_at")),
        }
    if message_type == "outbound_order":
        return {
            "wms_order_no": (row.get("wms_order_no") or "").strip()
            or f"WMS-{source_doc_no}",
            "document_type": row.get("document_type") or "sales_outbound",
            "erp_order_no": row.get("erp_order_no") or source_doc_no,
            "customer_id": row["customer_id"],
            "product_code": row["product_code"],
            "batch_no": (row.get("batch_no") or "").strip() or "ERP-UNSPEC",
            "planned_qty": _quantity(row, "planned_qty"),
            "required_ship_at": _rfc3339(row.get("required_ship_at")),
        }
    if message_type == "product_master":
        spec = (row.get("spec") or "").strip()
        if not spec:
            raise CanonicalMappingError("invalid spec")
        return {
            "schema_version": row.get("schema_version") or "1",
            "product_code": row["product_code"],
            "product_name": row["product_name"],
            "approval_no": row.get("approval_no") or None,
            "spec": spec,
            "dosage_form": row.get("dosage_form") or None,
            "manufacturer": row.get("manufacturer") or None,
            "special_drug_category": row.get("special_drug_category") or "",
            "udi_code": row.get("udi_code") or None,
            "electronic_regulatory_code": row.get("electronic_regulatory_code")
            or None,
            "length_mm": _optional_positive_number(row, "length_mm"),
            "width_mm": _optional_positive_number(row, "width_mm"),
            "height_mm": _optional_positive_number(row, "height_mm"),
            "volume_cm3": _optional_positive_number(row, "volume_cm3"),
            "weight_g": _optional_positive_number(row, "weight_g"),
            "packaging_levels": _packaging_levels(row),
            "storage_condition": (row.get("storage_condition") or "").strip(),
        }
    if message_type == "return_order":
        batch_no = (row.get("batch_no") or "").strip()
        if not batch_no:
            raise CanonicalMappingError("sales_return requires batch_no")
        return {
            "receipt_no": (row.get("receipt_no") or "").strip()
            or f"ERP-RET-{source_doc_no}",
            "document_type": row.get("document_type") or "sales_return",
            "supplier_id": (row.get("supplier_id") or "").strip()
            or row["customer_id"],
            "product_code": row["product_code"],
            "expected_qty": _quantity(row, "expected_qty"),
            "expected_arrival_at": _rfc3339(row.get("expected_arrival_at")),
            "batch_no": batch_no,
        }
    if message_type == "product_change":
        fields = {
            "product_id": (row.get("product_id") or "").strip() or None,
            "product_code": row["product_code"],
            "field_name": row["field_name"],
            "liaison_id": (row.get("liaison_id") or "").strip() or None,
            "asn_id": (row.get("asn_id") or "").strip() or None,
        }
        if row["field_name"] == "physical_dimensions":
            fields["physical_dimensions"] = _physical_dimensions(row)
        else:
            fields["new_value"] = row.get("new_value") or ""
        return fields
    raise CanonicalMappingError(f"unsupported message_type {message_type}")


def build_inbound_canonical(
    message_type: str,
    row: dict[str, str],
    binding: Any | None,
) -> H8CanonicalInboundCommand:
    external_ref = str(
        row.get("external_ref") or row.get("external_doc_no") or row.get("id") or ""
    )
    return H8CanonicalInboundCommand(
        owner_id=row["owner_id"],
        warehouse_id=(row.get("warehouse_id") or "").strip() or None,
        message_type=message_type,
        external_ref=external_ref,
        idempotency_key=row["idempotency_key"],
        correlation_id=row.get("id") or row["idempotency_key"],
        connector_id=binding.connector_id if binding else None,
        config_version=binding.config_version if binding else None,
        channel=binding.channel if binding else "interface_table",
        fields=_fields(message_type, row, external_ref),
        occurred_at=_rfc3339(row.get("created_at")) or "",
    )
