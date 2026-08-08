#!/usr/bin/env python3
# @governance: skip-page-size Worker 的认领、路由、处理与终态回写共用单一租约状态机，需整体审计。
"""H8 ERP 接口表同步 Worker（独立进程）。

双向：
  入站：MSSQL if_in_* pending → WMS HTTP API → success/failed
  出站：WMS PG *erp_feedback_outbox → 通道 B(if_out_message) 和/或 通道 A(HTTP 回调)

环境变量：
  WMS_API_BASE           默认 http://127.0.0.1:8080
  WMS_API_TOKEN          Bearer token（启动时读取不可变连接快照，必填）
  WMS_API_KEY            连接绑定的 WMS API Key（入站业务调用必填）
  WMS_H8_SECRET_ALIASES  secret alias 到真实凭据的本机映射
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
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable

# 同目录 outbound_publish
_sys_path = str(Path(__file__).resolve().parent)
if _sys_path not in sys.path:
    sys.path.insert(0, _sys_path)
from inbound_canonical import (  # noqa: E402
    CanonicalMappingError,
    H8CanonicalInboundCommand,
    build_inbound_canonical,
)
from outbound_publish import (  # noqa: E402
    process_outbound_once,
    process_outbound_receipts,
    process_outbound_receipt_timeouts,
    resolve_wms_db_url,
)
from reconciliation_pull import (  # noqa: E402
    pull_due_reconciliation_snapshots,
    pull_reconciliation_snapshot,
)
from worker_route import (  # noqa: E402
    RouteBinding,
    WorkerHttpError,
    claim_manual_replay as claim_manual_replay_with_http,
    get_worker_claim_decision as get_worker_claim_decision_with_http,
    is_retryable_worker_error,
    list_manual_replays as list_manual_replays_with_http,
    mark_inbound_message_dead as mark_inbound_message_dead_with_http,
    post_worker_heartbeat as post_worker_heartbeat_with_http,
    resolve_existing_inbound_binding,
    resolve_inbound_route as resolve_inbound_route_with_http,
    sanitize_worker_error,
    validate_row_schema_version,
)
from worker_mssql import (  # noqa: E402
    TABLE_CONTRACTS,
    claim_rows,
    mark_row,
    requeue_replay_row,
    sqlcmd_query,
)
from worker_settings import Settings, load_runtime_settings  # noqa: E402
from v19_contract import ContractError, validate_published_unit  # noqa: E402


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
    if settings.api_key and path.startswith(
        "/api/v1/integration/erp-messages/inbound/"
    ):
        headers["x-wms-api-key"] = settings.api_key
    elif settings.api_token:
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


def build_inbound_canonical_with_mpm(
    settings: Settings,
    message_type: str,
    row: dict[str, str],
    binding: Any | None,
    *,
    http_json_fn: Callable[..., tuple[int, dict[str, Any] | None, str]] = http_json,
) -> H8CanonicalInboundCommand:
    """通过持久 M-PM 规整受控外部值，再建立 canonical 命令。"""
    normalized_row = dict(row)
    mapped_fields = []
    if message_type in ("asn", "outbound_order", "return_order") and row.get(
        "document_type"
    ):
        mapped_fields.append(("document_type", "document_type"))
    for field, dict_code in mapped_fields:
        source_value = (row.get(field) or "").strip()
        if not source_value:
            raise CanonicalMappingError(f"unmapped {field}")
        body = {
            "dict_code": dict_code,
            "source_value": source_value,
            "source_system": getattr(binding, "connector_code", None) or "ERP",
            "source_record_id": row.get("external_ref")
            or row.get("external_doc_no")
            or row.get("id"),
        }
        idempotency_key = f"{row['idempotency_key']}:mpm:{dict_code}"
        status, parsed, raw = http_json_fn(
            settings,
            "POST",
            "/api/v1/parameter-mapping/map",
            body,
            idempotency_key,
        )
        if status != 200 or not isinstance(parsed, dict):
            raise WorkerHttpError(status, "M-PM API", raw)
        target = parsed.get("target_value")
        if parsed.get("status") != "matched" or not isinstance(target, str) or not target:
            if dict_code == "dosage_form":
                continue
            raise CanonicalMappingError(f"unmapped {dict_code} {source_value}")
        normalized_row[field] = target
    return build_inbound_canonical(message_type, normalized_row, binding)


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


def list_manual_replays(settings: Settings, message_type: str) -> list[dict[str, Any]]:
    return list_manual_replays_with_http(
        settings,
        message_type,
        http_json_fn=http_json,
    )


def claim_manual_replay(settings: Settings, message_id: str) -> None:
    claim_manual_replay_with_http(settings, message_id, http_json_fn=http_json)


def prepare_manual_replays(settings: Settings, message_type: str, table: str) -> None:
    for message in list_manual_replays(settings, message_type):
        if not requeue_replay_row(settings, table, str(message["idempotency_key"])):
            print(
                f"[h8] manual replay row missing message={message['id']}",
                flush=True,
            )
            continue
        claim_manual_replay(settings, str(message["id"]))


def mark_terminal_inbound_message(
    settings: Settings,
    message_type: str,
    row: dict[str, str],
    error_summary: str,
) -> None:
    mark_inbound_message_dead_with_http(
        settings,
        message_type,
        row,
        error_summary,
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
        summary = sanitize_worker_error(
            str(exc), (settings.api_token, settings.api_key, settings.mssql_password)
        )
        print(
            f"[h8] heartbeat warn: {summary}",
            flush=True,
        )


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def _utc_text(value: Any) -> str:
    text = value.isoformat(timespec="milliseconds") if hasattr(value, "isoformat") else str(value)
    return text.replace("+00:00", "Z") if text.endswith("+00:00") else text.rstrip("Z") + "Z"


def _runtime_row(table: str, row: dict[str, Any]) -> dict[str, Any]:
    """给 H8 路由补运行别名；原始 v1.9 字段保留给摘要校验。"""
    external_fields = {
        "x_wmsinter_GoodsInfo": "GoodsCode",
        "x_wmsinter_CustomerInfo": "ClientCode",
        "x_wmsinter_SupplierInfo": "SupplierCode",
        "x_wmsinter_InboundOrder": "ERPBillCode",
        "x_wmsinter_OutboundOrder": "ERPBillCode",
        "x_wmsinter_OrderCommand": "CommandID",
        "x_wmsinter_InventoryPushHeader": "SnapshotID",
    }
    runtime = dict(row)
    pk = TABLE_CONTRACTS[table]["pk"]
    external = row.get(external_fields[table]) or row[pk]
    runtime.update(
        {
            "id": str(row[pk]),
            "owner_id": str(row["OwnerCode"]),
            "external_doc_no": str(external),
            "external_ref": str(external),
            "idempotency_key": str(row["IdempotencyKey"]),
            "schema_version": str(row["SchemaVersion"]),
            "correlation_id": str(row["CorrelationID"]),
            "created_at": _utc_text(row["inserttime"]),
            "warehouse_code": str(row.get("DepotCode") or ""),
        }
    )
    return runtime


def build_v19_inbound_canonical(
    message_type: str,
    row: dict[str, Any],
    binding: Any | None,
) -> H8CanonicalInboundCommand:
    envelope = {
        "schema_version": row["SchemaVersion"],
        "external_ref": row["external_ref"],
        "correlation_id": row["CorrelationID"],
        "occurred_at": row["created_at"],
        "payload_digest": row["PayloadDigest"],
        "source_version": int(row["SourceVersion"])
        if row.get("SourceVersion") is not None
        else None,
    }
    if message_type == "product_master":
        packaging = json.loads(row["PackagingJson"]) if row.get("PackagingJson") else []
        for index, level in enumerate(packaging, 1):
            level.setdefault("sort_order", index)
        body = envelope | {
            "entity_id": row["GoodsID"],
            "op_type": row["opType"],
            "product_code": row.get("GoodsCode"),
            "product_name": row.get("GoodsName"),
            "approval_no": row.get("License"),
            "spec": row.get("Spec"),
            "manufacturer": row.get("ProduceCorp"),
            "special_drug_category": row.get("SpecialCategory"),
            "storage_condition": row.get("Deposite"),
            "packaging_levels": packaging,
        }
    elif message_type == "customer_master":
        body = envelope | {
            "entity_id": row["ClientID"],
            "op_type": row["opType"],
            "customer_code": row.get("ClientCode"),
            "customer_name": row.get("ClientName"),
            "customer_type": row.get("CorpType"),
            "address": row.get("Address"),
            "contact_name": row.get("LinkMan"),
            "contact_phone": row.get("LinkPhone"),
            "delivery_address": row.get("DepotAddr"),
            "delivery_contact": row.get("DepotMan"),
            "delivery_phone": row.get("DepotCall"),
            "delivery_mode": row.get("SendWay"),
            "stop_send": bool(row.get("StopSend")),
        }
    elif message_type == "supplier_master":
        body = envelope | {
            "entity_id": row["SupplierID"],
            "op_type": row["opType"],
            "supplier_code": row.get("SupplierCode"),
            "supplier_name": row.get("SupplierName"),
            "address": row.get("Address"),
            "contact_name": row.get("LinkMan"),
            "contact_phone": row.get("LinkPhone"),
        }
    elif message_type == "asn":
        body = envelope | {
            "erp_bill_id": row["ERPBillID"],
            "erp_bill_code": row["ERPBillCode"],
            "revision": row["Revision"],
            "order_type": row["OrderType"],
            "partner_type": row.get("PartnerType"),
            "partner_code": row.get("PartnerCode"),
            "depot_code": row["DepotCode"],
            "business_date": str(row["BusiDate"]),
            "note_code": row.get("NoteCode"),
            "lines": [
                {
                    "line_no": item["LineNo"],
                    "product_code": item["GoodsCode"],
                    "expected_qty": f"{item['Amount']:.4f}",
                    "batch_no": item.get("BatchNo"),
                    "production_date": str(item["ProduceDate"]) if item.get("ProduceDate") else None,
                    "expiry_date": str(item["ValidDate"]) if item.get("ValidDate") else None,
                }
                for item in row.get("_items", [])
            ],
        }
    elif message_type == "outbound_order":
        body = envelope | {
            "erp_bill_id": row["ERPBillID"],
            "erp_bill_code": row["ERPBillCode"],
            "revision": row["Revision"],
            "order_type": row["OrderType"],
            "customer_code": row.get("ClientCode"),
            "depot_code": row["DepotCode"],
            "required_ship_at": _utc_text(row["RequiredShipAt"]),
            "send_mode": row.get("SendMode"),
            "erp_address_id": row["ERPAddressID"],
            "address_code": row["AddressCode"],
            "contact_name": row.get("LinkMan"),
            "contact_phone": row.get("LinkCall"),
            "address": row["Address"],
            "lines": [
                {
                    "line_no": item["LineNo"],
                    "product_code": item["GoodsCode"],
                    "batch_no": item["BatchNo"],
                    "planned_qty": f"{item['Amount']:.4f}",
                }
                for item in row.get("_items", [])
            ],
        }
    elif message_type == "order_cancel":
        body = envelope | {
            "command_id": row["CommandID"],
            "command_type": row["CommandType"],
            "erp_bill_code": row["ERPBillCode"],
            "revision": row["Revision"],
            "order_type": row["OrderType"],
            "memo": row.get("Memo"),
        }
    elif message_type == "inventory_seed_snapshot":
        body = envelope | {
            "snapshot_id": row["SnapshotID"],
            "depot_code": row["DepotCode"],
            "push_type": row["PushType"],
            "push_time": _utc_text(row["PushTime"]),
            "items": [
                {
                    "row_no": item["RowNo"],
                    "product_code": item["GoodsCode"],
                    "batch_no": item["BatchNo"],
                    "expiry_date": str(item["ValidDate"]) if item.get("ValidDate") else None,
                    "location_code": item.get("StallCode"),
                    "goods_status": item.get("GoodsStatus"),
                    "quantity": f"{item['RealAmount']:.4f}",
                }
                for item in row.get("_items", [])
            ],
        }
    else:
        raise CanonicalMappingError(f"unsupported message_type {message_type}")
    return H8CanonicalInboundCommand(
        owner_id=row["owner_id"],
        warehouse_id=None,
        message_type=message_type,
        external_ref=row["external_ref"],
        idempotency_key=row["idempotency_key"],
        correlation_id=row["correlation_id"],
        connector_id=binding.connector_id if binding else None,
        config_version=binding.config_version if binding else None,
        channel=binding.channel if binding else "interface_table",
        fields={"request_body": body},
        occurred_at=row["created_at"],
    )


def _handle_v19(
    settings: Settings,
    command: H8CanonicalInboundCommand,
) -> str:
    body = command.fields["request_body"]
    status, parsed, raw = http_json(
        settings,
        "POST",
        f"/api/v1/integration/erp-messages/inbound/{command.message_type}",
        body,
        command.idempotency_key,
    )
    if status in (200, 201) and isinstance(parsed, dict):
        resource = parsed.get("wms_resource_id") or parsed.get("id")
        if resource:
            command.fields["ignored_old_version"] = bool(
                parsed.get("ignored_old_version")
            )
            return str(resource)
    code = str(parsed.get("code")) if isinstance(parsed, dict) and parsed.get("code") else None
    raise WorkerHttpError(status, f"H8 {command.message_type} API", raw, code)


def _order_not_ready_timed_out(value: Any) -> bool:
    inserted = value if isinstance(value, datetime) else datetime.fromisoformat(str(value).replace("Z", "+00:00"))
    if inserted.tzinfo is None:
        inserted = inserted.replace(tzinfo=timezone.utc)
    return (datetime.now(timezone.utc) - inserted).total_seconds() >= 30 * 60


def handle_asn(settings: Settings, command: H8CanonicalInboundCommand) -> str:
    fields = command.fields
    body = {
        "receipt_no": fields["receipt_no"],
        "document_type": fields["document_type"],
        "supplier_id": fields["supplier_id"],
        "warehouse_id": command.warehouse_id,
        "external_ref": command.external_ref,
        "expected_arrival_at": fields["expected_arrival_at"],
        "lines": [
            {
                "line_no": 1,
                "product_id": None,
                "product_code": fields["product_code"],
                "expected_qty": fields["expected_qty"],
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
        command.idempotency_key,
    )
    if status in (200, 201) and isinstance(parsed, dict) and parsed.get("id"):
        return str(parsed["id"])
    raise WorkerHttpError(status, "ASN API", raw)


def handle_outbound(settings: Settings, command: H8CanonicalInboundCommand) -> str:
    fields = command.fields
    line: dict[str, Any] = {
        "line_no": 1,
        "product_code": fields["product_code"],
        "batch_no": fields["batch_no"],
        "planned_qty": fields["planned_qty"],
    }
    body: dict[str, Any] = {
        "document_type": fields["document_type"],
        "wms_order_no": fields["wms_order_no"],
        "erp_order_no": fields["erp_order_no"],
        "customer_id": fields["customer_id"],
        "warehouse_id": command.warehouse_id,
        "required_ship_at": fields["required_ship_at"],
        "lines": [line],
    }
    status, parsed, raw = http_json(
        settings,
        "POST",
        "/api/v1/outbound/orders",
        body,
        command.idempotency_key,
    )
    if status in (200, 201) and isinstance(parsed, dict) and parsed.get("id"):
        return str(parsed["id"])
    raise WorkerHttpError(status, "Outbound API", raw)


def handle_product(settings: Settings, command: H8CanonicalInboundCommand) -> str:
    fields = command.fields
    body = {
        "schema_version": fields["schema_version"],
        "external_ref": command.external_ref,
        "correlation_id": command.correlation_id,
        "occurred_at": command.occurred_at,
        "product_code": fields["product_code"],
        "product_name": fields["product_name"],
        "approval_no": fields["approval_no"],
        "spec": fields["spec"],
        "dosage_form": fields["dosage_form"],
        "manufacturer": fields["manufacturer"],
        "special_drug_category": fields["special_drug_category"],
        "udi_code": fields["udi_code"],
        "electronic_regulatory_code": fields["electronic_regulatory_code"],
        "length_mm": fields["length_mm"],
        "width_mm": fields["width_mm"],
        "height_mm": fields["height_mm"],
        "volume_cm3": fields["volume_cm3"],
        "weight_g": fields["weight_g"],
        "packaging_levels": fields["packaging_levels"],
        "storage_condition": fields["storage_condition"],
    }
    status, parsed, raw = http_json(
        settings,
        "POST",
        "/api/v1/integration/erp-messages/inbound/product_master",
        body,
        command.idempotency_key,
    )
    if status in (200, 201) and isinstance(parsed, dict) and parsed.get(
        "wms_resource_id"
    ):
        return str(parsed["wms_resource_id"])
    raise WorkerHttpError(status, "H8 product master API", raw)


def handle_return(settings: Settings, command: H8CanonicalInboundCommand) -> str:
    """销退入库：走收货单 API，document_type 默认 sales_return（必填原批号）。"""
    fields = command.fields
    body = {
        "receipt_no": fields["receipt_no"],
        "document_type": fields["document_type"],
        "supplier_id": fields["supplier_id"],
        "warehouse_id": command.warehouse_id,
        "external_ref": command.external_ref,
        "expected_arrival_at": fields["expected_arrival_at"],
        "lines": [
            {
                "line_no": 1,
                "product_id": None,
                "product_code": fields["product_code"],
                "expected_qty": fields["expected_qty"],
                "batch_no": fields["batch_no"],
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
        command.idempotency_key,
    )
    if status in (200, 201) and isinstance(parsed, dict) and parsed.get("id"):
        return str(parsed["id"])
    raise WorkerHttpError(status, "Return ASN API", raw)


def handle_product_change(
    settings: Settings, command: H8CanonicalInboundCommand
) -> str:
    """接口表商品变更复用共享 H8 REST 防腐层，统一 M-PM、审计与闭环。"""
    fields = command.fields
    body = {
        "schema_version": "1",
        "external_ref": command.external_ref,
        "correlation_id": command.correlation_id,
        "occurred_at": command.occurred_at,
        "product_id": fields["product_id"],
        "product_code": fields["product_code"],
        "field_name": fields["field_name"],
        "liaison_id": fields["liaison_id"],
        "asn_id": fields["asn_id"],
    }
    if fields["field_name"] == "physical_dimensions":
        body["physical_dimensions"] = fields["physical_dimensions"]
    else:
        body["new_value"] = fields["new_value"]
    status, parsed, raw = http_json(
        settings,
        "POST",
        "/api/v1/integration/erp-messages/inbound/product_change",
        body,
        command.idempotency_key,
    )
    if status in (200, 201) and isinstance(parsed, dict) and parsed.get(
        "wms_resource_id"
    ):
        return str(parsed["wms_resource_id"])
    raise WorkerHttpError(status, "H8 product change API", raw)


HANDLERS: dict[
    str, tuple[str, Callable[[Settings, H8CanonicalInboundCommand], str]]
] = {
    "product_master": ("x_wmsinter_GoodsInfo", _handle_v19),
    "customer_master": ("x_wmsinter_CustomerInfo", _handle_v19),
    "supplier_master": ("x_wmsinter_SupplierInfo", _handle_v19),
    "asn": ("x_wmsinter_InboundOrder", _handle_v19),
    "outbound_order": ("x_wmsinter_OutboundOrder", _handle_v19),
    "order_cancel": ("x_wmsinter_OrderCommand", _handle_v19),
    "inventory_seed_snapshot": ("x_wmsinter_InventoryPushHeader", _handle_v19),
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
            try:
                prepare_manual_replays(settings, type_name, table)
            except Exception as exc:  # noqa: BLE001 — 未接管重放前不得认领接口行
                summary = sanitize_worker_error(
                    str(exc),
                    (settings.api_token, settings.api_key, settings.mssql_password),
                )
                print(f"[h8] manual replay warn: {summary}", flush=True)
                continue
        rows = claim_rows(settings, table)
        if not dry_run:
            try_record_worker_heartbeat(settings, heartbeat_directions, len(rows))
        for row in rows:
            processed += 1
            row_id = row[TABLE_CONTRACTS[table]["pk"]]
            row = _runtime_row(table, row)
            retry = int(row.get("retry_count") or "0")
            print(
                f"[h8] claim {type_name} id={row_id} doc={row.get('external_doc_no')}",
                flush=True,
            )
            if dry_run:
                try:
                    validate_published_unit(table, row)
                    validate_row_schema_version(row)
                    build_v19_inbound_canonical(type_name, row, None)
                except Exception as dry_run_exc:  # noqa: BLE001
                    print(f"[h8] dry-run validation warn: {dry_run_exc}", flush=True)
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
                validate_published_unit(table, row)
                validate_row_schema_version(row)
                binding = (
                    resolve_existing_inbound_binding(
                        settings, type_name, row, http_json_fn=http_json
                    )
                    if retry > 0 or row.get("error_code") == "ORDER_NOT_READY"
                    else None
                )
                if binding is None:
                    binding = resolve_inbound_route(settings, type_name, row)
                if binding.connector_id != settings.connector_id:
                    raise WorkerHttpError(
                        409,
                        "route binding",
                        f"resolved connector {binding.connector_id} differs from worker binding",
                    )
                command = build_v19_inbound_canonical(type_name, row, binding)
                wms_id = handler(settings, command)
                ignored = bool(command.fields.get("ignored_old_version"))
                if ignored:
                    mark_row(
                        settings,
                        table,
                        row_id,
                        "accepted",
                        error="旧版本已忽略",
                    )
                else:
                    mark_row(settings, table, row_id, "success")
                print(f"[h8] success {type_name} -> {wms_id}", flush=True)
            except Exception as exc:  # noqa: BLE001 — worker 边界
                error = exc
                error_code = getattr(error, "code", None)
                order_not_ready = type_name == "order_cancel" and error_code == "ORDER_NOT_READY"
                if not order_not_ready:
                    retry += 1
                retryable = is_retryable_worker_error(error)
                error_summary = sanitize_worker_error(
                    str(error),
                    (settings.api_token, settings.api_key, settings.mssql_password),
                )
                next_status = "dead" if order_not_ready and _order_not_ready_timed_out(row["inserttime"]) else (
                    "retry" if retryable and retry < settings.max_retry else "dead"
                )
                mark_row(
                    settings,
                    table,
                    row_id,
                    next_status,
                    error=error_summary,
                    retry_count=retry,
                    error_code=error_code,
                )
                print(f"[h8] error {type_name}: {error_summary}", flush=True)
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
        default=(
            "product_master,customer_master,supplier_master,asn,outbound_order,"
            "order_cancel,inventory_seed_snapshot"
        ),
        help="v1.9 ERP→WMS 主记录类型",
    )
    parser.add_argument("--reconcile-owner", help="一次性主动拉取该货主的 ERP 库存快照")
    parser.add_argument("--reconcile-window", help="对账时间窗口幂等键")
    parser.add_argument(
        "--reconcile-due",
        action="store_true",
        help="由 H-SCH 单次触发当前 Worker 货主的到期库存快照拉取",
    )
    args = parser.parse_args(argv)
    bootstrap = Settings.from_env()
    try:
        settings = load_runtime_settings(bootstrap, http_json_fn=http_json)
    except WorkerHttpError as error:
        print(f"[h8] worker bootstrap failed: {error}", file=sys.stderr)
        return 2
    if args.reconcile_owner and args.reconcile_due:
        print("--reconcile-owner and --reconcile-due are mutually exclusive", file=sys.stderr)
        return 2
    if args.reconcile_due:
        if not args.once or not settings.api_token:
            print(
                "--reconcile-due requires --once and WMS_API_TOKEN",
                file=sys.stderr,
            )
            return 2
        run_ids = pull_due_reconciliation_snapshots(
            settings,
            http_json_fn=http_json,
        )
        print(f"[h8] reconciliation due runs={len(run_ids)}", flush=True)
        return 0
    if args.reconcile_owner:
        if not args.once or not args.reconcile_window or not settings.api_token:
            print(
                "--reconcile-owner requires --once, --reconcile-window and WMS_API_TOKEN",
                file=sys.stderr,
            )
            return 2
        run_id = pull_reconciliation_snapshot(
            settings,
            args.reconcile_owner,
            args.reconcile_window,
            http_json_fn=http_json,
        )
        print(f"[h8] reconciliation run={run_id}", flush=True)
        return 0
    need_in = args.direction in ("in", "both")
    need_out = args.direction in ("out", "both")
    wms_db = resolve_wms_db_url()
    if (need_in or need_out) and not args.dry_run and not settings.api_token:
        print("WMS_API_TOKEN is required unless --dry-run", file=sys.stderr)
        return 2
    if need_out and not wms_db and not args.dry_run:
        print(
            "WMS_DB_URL (or DATABASE_URL) is required for outbound publish",
            file=sys.stderr,
        )
        return 2
    types = [t.strip() for t in args.types.split(",") if t.strip()]
    for t in types:
        if t not in HANDLERS:
            print(f"unknown type {t}", file=sys.stderr)
            return 2

    print(
        f"[h8] worker start api={settings.api_base} direction={args.direction} "
        f"transport=configured-route types={types} once={args.once}",
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
        if need_out and wms_db and not args.dry_run:
            # Pause blocks new claims/retries, but an ERP acknowledgement already in
            # flight must still be consumed so accepted work can reach a terminal state.
            n += process_outbound_receipts(
                settings,
                http_json_fn=http_json,
            )
        if need_out and wms_db and outbound_allowed:
            if not args.dry_run:
                n += process_outbound_receipt_timeouts(
                    settings,
                    wms_db,
                    http_json_fn=http_json,
                )
            n += process_outbound_once(
                database_url=wms_db,
                sqlcmd_exec=lambda sql: sqlcmd_query(settings, sql),
                batch_size=settings.batch_size,
                dry_run=args.dry_run,
                transport="table",
                callback_base=None,
                connector_id=settings.connector_id,
                settings=settings,
                http_json_fn=http_json,
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
