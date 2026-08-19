#!/usr/bin/env python3
"""report_wave4_completion.py — Wave 4 完成度证据报告

类别：4. 流程治理（报告型，默认不阻塞）
Tier：手动 / Wave 出口检查
输入：ROADMAP.md Wave 4 完成标准 + architecture-dependencies.md + 当前仓库文件
输出：人类可读 + --json
退出码：
  默认：0（只报告当前证据）
  --strict：任一阻塞缺口返回 1

本脚本只把既有 Wave 4 范围和完成标准转成可复跑证据检查；外部 evidence
延期必须有 clarifications 决策记录，不能伪造为真实平台证据。
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from validate_wave4_external_dependencies import (
    DEFAULT_EVIDENCE as WAVE4_EXTERNAL_EVIDENCE,
    validate_one as validate_wave4_external_evidence,
)

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent

PROVED_BY_STATIC_FILES = "PROVED_BY_STATIC_FILES"
ACCEPTED_DEFERRED = "ACCEPTED_DEFERRED"
MISSING_OR_NEEDS_CONFIRMATION = "MISSING_OR_NEEDS_CONFIRMATION"
NEEDS_BUSINESS_DECISION = "NEEDS_BUSINESS_DECISION"


@dataclass
class EvidenceItem:
    item_id: str
    requirement: str
    status: str
    evidence: list[str] = field(default_factory=list)
    gaps: list[str] = field(default_factory=list)
    strict_blocking: bool = True

    @property
    def complete(self) -> bool:
        return self.status in {PROVED_BY_STATIC_FILES, ACCEPTED_DEFERRED}

    @property
    def blocks_strict(self) -> bool:
        return self.strict_blocking and not self.complete


def read_text(path: str) -> str:
    target = REPO_ROOT / path
    return target.read_text(encoding="utf-8") if target.exists() else ""


def file_exists(path: str) -> bool:
    return (REPO_ROOT / path).exists()


def file_contains(path: str, *needles: str) -> bool:
    text = read_text(path)
    return bool(text) and all(needle in text for needle in needles)


def load_openapi() -> dict[str, Any]:
    path = REPO_ROOT / "shared/openapi/openapi.json"
    if not path.exists():
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {}
    return data if isinstance(data, dict) else {}


def openapi_has(paths: list[str], schemas: list[str]) -> bool:
    data = load_openapi()
    openapi_paths = data.get("paths") if isinstance(data.get("paths"), dict) else {}
    components = data.get("components") if isinstance(data.get("components"), dict) else {}
    openapi_schemas = components.get("schemas") if isinstance(components.get("schemas"), dict) else {}
    return all(path in openapi_paths for path in paths) and all(schema in openapi_schemas for schema in schemas)


def latest_clarification_row(question: str) -> str | None:
    rows = [
        line
        for line in read_text("docs/domain/clarifications.md").splitlines()
        if line.strip().startswith("|") and question in line
    ]
    return rows[-1] if rows else None


def latest_markdown_row(path: str, token: str) -> str | None:
    rows = [
        line
        for line in read_text(path).splitlines()
        if line.strip().startswith("|") and token in line
    ]
    return rows[-1] if rows else None


def markdown_row_last_cell(row: str | None) -> str | None:
    if row is None:
        return None
    cells = [cell.strip() for cell in row.strip().strip("|").split("|")]
    return cells[-1] if cells else None


def external_dependency_status(label: str) -> str | None:
    return markdown_row_last_cell(latest_markdown_row("ROADMAP.md", label))


def wave4_todo_started() -> bool:
    text = read_text("TODO.md")
    if not text:
        return False
    phase_recorded = "当前 Wave：Wave 4" in text or "已归档：Wave 4" in text
    return phase_recorded and all(
        marker in text
        for marker in (
            "W4.A",
            "W4.B",
            "W4.C",
            "W4.D",
            "W4.E",
        )
    )


def wave4_scope_aligned() -> bool:
    roadmap_has_core = file_contains("ROADMAP.md", "W4.A", "W4.B", "W4.C", "W4.D", "W4.E")
    architecture_has_core = file_contains(
        "docs/architecture-dependencies.md",
        "W4.A",
        "W4.B",
        "W4.C",
        "W4.D",
        "W4.E",
    )
    roadmap_has_w4f = "W4.F" in read_text("ROADMAP.md")
    architecture_has_w4f = "W4.F" in read_text("docs/architecture-dependencies.md")
    return roadmap_has_core and architecture_has_core and roadmap_has_w4f == architecture_has_w4f


def short_pick_decision_closed() -> bool:
    latest = latest_clarification_row("短拣后是否允许少量发货")
    if latest is None:
        return False
    if "待定" in latest or "必须确认" in latest:
        return False
    accepted_tokens = ["不允许短拣发货", "部分发货", "补拣补齐", "不允许部分发货"]
    return any(token in latest for token in accepted_tokens)


def traceability_external_contract_ready() -> bool:
    story = read_text("docs/domain/user-stories-mtc-traceability-code.md")
    account_status = external_dependency_status("\"码上放心\"账号开通")
    contract_status = external_dependency_status("\"码上放心\"正式接口文档 / 鉴权方式 / 错误码 / 频率限制确认")
    external_dependency_open = (
        account_status is None
        or contract_status is None
        or any(token in account_status for token in ["未启动", "未确认"])
        or any(token in contract_status for token in ["未启动", "未确认"])
    )
    if external_dependency_open or "待接口确认" in story:
        return False
    ok, _message = validate_wave4_external_evidence(
        WAVE4_EXTERNAL_EVIDENCE,
        allow_example_refs=False,
    )
    return ok


def traceability_external_evidence_deferred() -> bool:
    latest = latest_clarification_row("W4.D 码上放心外部 evidence 延期")
    if latest is None:
        return False
    required_tokens = [
        "不阻塞 Wave 4",
        "后续",
        "真实 dev/staging",
        "不伪造 evidence",
    ]
    return all(token in latest for token in required_tokens)


def collect_items() -> list[EvidenceItem]:
    items: list[EvidenceItem] = []

    startup_ok = wave4_todo_started() and file_contains(
        "justfile",
        "wave-4-status",
        "wave-4-complete-check",
    )
    items.append(EvidenceItem(
        "W4-startup",
        "Wave 4 TODO 记录与完成度检查入口已登记",
        PROVED_BY_STATIC_FILES if startup_ok else MISSING_OR_NEEDS_CONFIRMATION,
        ["TODO.md", "justfile", "scripts/governance/report_wave4_completion.py"] if startup_ok else [],
        [] if startup_ok else ["需要在 TODO 登记当前或归档的 Wave 4，并登记 just wave-4-status / wave-4-complete-check"],
    ))

    scope_ok = wave4_scope_aligned()
    items.append(EvidenceItem(
        "W4-scope-alignment",
        "Wave 4 范围在 ROADMAP 与 architecture-dependencies 中一致",
        PROVED_BY_STATIC_FILES if scope_ok else NEEDS_BUSINESS_DECISION,
        ["ROADMAP.md", "docs/architecture-dependencies.md"] if scope_ok else [],
        [] if scope_ok else ["ROADMAP 当前列 W4.A-E；architecture-dependencies 另列 W4.F M-PK 包装站基础能力，需要确认是否纳入 Wave 4"],
    ))

    short_pick_ok = short_pick_decision_closed()
    items.append(EvidenceItem(
        "W4.A-short-pick-decision",
        "M4 TDD 前关闭短拣 #43 业务决策",
        PROVED_BY_STATIC_FILES if short_pick_ok else NEEDS_BUSINESS_DECISION,
        ["docs/domain/clarifications.md"] if short_pick_ok else [],
        [] if short_pick_ok else ["clarifications #43 仍为待定；M4 文档明确进入 TDD 前必须关闭"],
    ))

    outbound_contract_ok = openapi_has(
        [
            "/api/v1/outbound/orders",
            "/api/v1/outbound/waves",
            "/api/v1/outbound/pick-tasks/{id}/complete",
            "/api/v1/outbound/orders/{id}/review",
            "/api/v1/outbound/orders/{id}/ship",
        ],
        [
            "CreateOutboundOrderRequest",
            "OutboundOrder",
            "CreateOutboundWaveRequest",
            "OutboundWave",
            "CompletePickTaskRequest",
            "ReviewOutboundOrderRequest",
            "ShipOutboundOrderRequest",
        ],
    )
    items.append(EvidenceItem(
        "W4.A-outbound-contract",
        "M4 出库订单 / 波次 / 拣选 / 复核 / 发货 OpenAPI 契约",
        PROVED_BY_STATIC_FILES if outbound_contract_ok else MISSING_OR_NEEDS_CONFIRMATION,
        ["shared/openapi/openapi.json", "packages/api-client/src/schema.ts"] if outbound_contract_ok else [],
        [] if outbound_contract_ok else ["缺少 M4 outbound OpenAPI path/schema 与 api-client 类型"],
    ))

    outbound_backend_ok = (
        file_contains("backend/crates/api/src/outbound.rs", "OutboundOrderStore", "OutboundOrderError")
        and file_contains("backend/crates/api/src/wave4_repository.rs", "create_outbound_order")
        and file_contains("backend/crates/api/src/wave4_handlers.rs", "postgres_outbound")
    )
    items.append(EvidenceItem(
        "W4.A-outbound-backend",
        "M4 出库核心后端：订单、波次、拣选、复核、发货",
        PROVED_BY_STATIC_FILES if outbound_backend_ok else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "backend/crates/api/src/outbound.rs",
            "backend/crates/api/src/wave4_repository.rs",
            "backend/crates/api/src/wave4_handlers.rs",
        ] if outbound_backend_ok else [],
        [] if outbound_backend_ok else ["缺少 M4 outbound domain/repository/handler 证据"],
    ))

    cold_chain_link_ok = (
        file_contains("backend/crates/api/src/cold_chain.rs", "ingest_excursion", "pending_disposition")
        and file_contains(
            "backend/crates/api/src/wave4_handlers.rs",
            "pending-disposition",
            "dispose_temperature_excursion",
            "APPROVAL_SOURCE_TEMPERATURE_EXCURSION",
        )
        and file_contains(
            "backend/crates/api/src/wave4_repository.rs",
            "list_pending_temperature_excursions",
            "dispose_temperature_excursion_and_quarantine_batches",
            "APPROVAL_SOURCE_TEMPERATURE_EXCURSION",
        )
        and file_contains(
            "backend/crates/api/tests/wave4_postgres.rs",
            "temperature_excursion_disposition_quarantines_selected_batches_and_audits",
        )
        and openapi_has(
            [
                "/api/v1/cold-chain/excursions/pending-disposition",
                "/api/v1/cold-chain/excursions/{external_event_id}/dispose",
            ],
            [
                "DisposeTemperatureExcursionRequest",
                "TemperatureExcursionDispositionResponse",
                "TemperatureExcursionEventListResponse",
            ],
        )
    )
    items.append(EvidenceItem(
        "W4.B-cold-chain-isolation",
        "M5 温度超标事件处置联动 M3 批次隔离",
        PROVED_BY_STATIC_FILES if cold_chain_link_ok else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "backend/crates/api/src/cold_chain.rs",
            "backend/crates/api/src/wave4_repository.rs",
            "backend/crates/api/src/wave4_handlers.rs",
            "backend/crates/api/tests/wave4_postgres.rs",
            "shared/openapi/openapi.json",
        ] if cold_chain_link_ok else [],
        [] if cold_chain_link_ok else ["缺少温度超标待处置列表、主管勾选处置、approval_source=M5-TEMP_EXCURSION 联动隔离证据"],
    ))

    reports_ok = (
        file_contains("backend/crates/api/src/reports.rs", "gsp_ledger")
        and openapi_has(
            [
                "/api/v1/reports/gsp/inbound-ledger",
                "/api/v1/reports/gsp/outbound-ledger",
                "/api/v1/reports/gsp/inventory-ledger",
            ],
            ["GspLedgerReport", "GspLedgerRow"],
        )
    )
    items.append(EvidenceItem(
        "W4.C-gsp-reports",
        "M6 GSP 法定台账报表可生成",
        PROVED_BY_STATIC_FILES if reports_ok else MISSING_OR_NEEDS_CONFIRMATION,
        ["backend/crates/api/src/reports.rs", "shared/openapi/openapi.json"] if reports_ok else [],
        [] if reports_ok else ["缺少 GSP 入库/出库/库存法定台账报表接口与实现证据"],
    ))

    traceability_internal_ok = (
        file_exists("docs/domain/user-stories-mtc-traceability-code.md")
        and file_contains("backend/crates/api/src/traceability_code.rs", "traceability_report")
        and file_contains(
            "backend/migrations/202606040001_wave4_outbound_tables.sql",
            "traceability_outbound_reports",
            "traceability_outbound_report_events",
        )
        and file_contains(
            "backend/crates/api/src/wave4_repository.rs",
            "create_traceability_outbound_report",
            "apply_traceability_platform_response",
            "traceability_outbound_report_events",
        )
        and file_contains(
            "backend/crates/api/src/wave4_handlers.rs",
            "create_traceability_outbound_report_handler",
            "m-tc.write",
        )
        and openapi_has(
            ["/api/v1/traceability/outbound-reports"],
            ["TraceabilityOutboundReport"],
        )
    )
    traceability_external_ok = traceability_external_contract_ready()
    traceability_external_deferred = traceability_external_evidence_deferred()
    traceability_gaps = []
    if not traceability_internal_ok:
        traceability_gaps.append("缺少 M-TC 追溯码出库核销上报接口、实现或契约证据")
    if not traceability_external_ok and traceability_external_deferred:
        traceability_gaps.append(
            "“码上放心”真实 dev/staging 外部 evidence 已按 clarifications #50 延期；不伪造 evidence，后续仍需用 just wave-4-external-dependencies-record 关闭"
        )
    elif not traceability_external_ok:
        traceability_gaps.append(
            "缺少“码上放心”正式接口文档 / 鉴权方式 / 错误码 / 频率限制确认与外部平台适配器证据；见 docs/runbooks/wave-4-external-dependencies.md"
        )
    items.append(EvidenceItem(
        "W4.D-traceability-code-reporting",
        "M-TC 码上放心追溯码核销事件实时上报",
        PROVED_BY_STATIC_FILES
        if traceability_internal_ok and traceability_external_ok
        else ACCEPTED_DEFERRED
        if traceability_internal_ok and traceability_external_deferred
        else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "backend/crates/api/src/traceability_code.rs",
            "backend/crates/api/src/wave4_repository.rs",
            "backend/crates/api/src/wave4_handlers.rs",
            "backend/migrations/202606040001_wave4_outbound_tables.sql",
            "shared/openapi/openapi.json",
            *(
                [
                    "docs/domain/clarifications.md",
                    "TODO.md",
                    "ROADMAP.md",
                ]
                if traceability_external_deferred
                else []
            ),
        ] if traceability_internal_ok else [],
        traceability_gaps,
    ))

    driver_store_ok = (
        file_contains("docs/domain/user-stories-h-driver.md", "司机")
        and file_contains("docs/domain/user-stories-h-store.md", "门店")
        and file_contains("backend/crates/api/src/wave4_handlers.rs", "driver")
        and file_contains("backend/crates/api/src/wave4_handlers.rs", "store")
    )
    items.append(EvidenceItem(
        "W4.E-driver-store",
        "司机端 / 门店端主动故事落地",
        PROVED_BY_STATIC_FILES if driver_store_ok else MISSING_OR_NEEDS_CONFIRMATION,
        ["docs/domain/user-stories-h-driver.md", "docs/domain/user-stories-h-store.md", "backend/crates/api/src/wave4_handlers.rs"] if driver_store_ok else [],
        [] if driver_store_ok else ["缺少 H-Driver / H-Store 主动故事的生产接口或 handler 证据"],
    ))

    audit_invariant_ok = (
        file_contains("backend/migrations/202606020001_audit_event.sql", "audit_event_immutable", "BEFORE UPDATE OR DELETE OR TRUNCATE")
        and file_contains("backend/crates/api/src/wave4_handlers.rs", "AuditWriteRequest", "dispose_temperature_excursion")
        and file_contains("backend/crates/api/src/wave4_repository.rs", "append_event_in_tx")
        and file_contains("backend/crates/api/tests/wave4_postgres.rs", "append_only", "seal_audit_chain")
    )
    items.append(EvidenceItem(
        "W4-audit-invariant",
        "Wave 4 关键写操作保持 H2 append-only 审计不变量",
        PROVED_BY_STATIC_FILES if audit_invariant_ok else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "backend/migrations/202606020001_audit_event.sql",
            "backend/crates/api/src/wave4_handlers.rs",
            "backend/crates/api/src/wave4_repository.rs",
            "backend/crates/api/tests/wave4_postgres.rs",
        ] if audit_invariant_ok else [],
        [] if audit_invariant_ok else ["缺少 Wave 4 关键写操作 audit_event 与 append-only/hash-chain 测试证据"],
    ))

    return items


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--strict", action="store_true", help="Wave 4 出口检查，阻塞缺口返回非零")
    args = parser.parse_args(argv)

    items = collect_items()
    blocking = [item for item in items if item.blocks_strict]
    ok = not blocking

    if args.json:
        print(json.dumps({
            "report": "wave4_completion",
            "tier": "manual",
            "category": "流程治理",
            "items": [asdict(item) for item in items],
            "blocking_gaps": [asdict(item) for item in blocking],
            "ok": ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("report_wave4_completion (流程治理，静态证据覆盖报告)")
        for item in items:
            mark = "✓" if item.complete else "✘"
            print(f"  {mark} {item.item_id}: {item.requirement}")
            print(f"    status: {item.status}")
            for evidence in item.evidence:
                print(f"    evidence: {evidence}")
            for gap in item.gaps:
                print(f"    gap: {gap}")
        if blocking:
            print(f"\n阻塞缺口: {len(blocking)}")

    return 1 if args.strict and blocking else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
