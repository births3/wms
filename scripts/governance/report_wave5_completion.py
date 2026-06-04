#!/usr/bin/env python3
"""report_wave5_completion.py — Wave 5 完成度证据报告。

类别：4. 流程治理（报告型，默认不阻塞）
Tier：手动 / Wave 出口检查
输入：ROADMAP.md Wave 5 完成标准 + architecture-dependencies.md + 当前仓库文件
输出：人类可读 + --json
退出码：
  默认：0（只报告当前证据）
  --strict：任一阻塞缺口返回 1
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

PROVED_BY_STATIC_FILES = "PROVED_BY_STATIC_FILES"
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
        return self.status == PROVED_BY_STATIC_FILES

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
    openapi_schemas = (
        components.get("schemas") if isinstance(components.get("schemas"), dict) else {}
    )
    return all(path in openapi_paths for path in paths) and all(
        schema in openapi_schemas for schema in schemas
    )


def wave5_todo_recorded() -> bool:
    text = read_text("TODO.md")
    if not text:
        return False
    phase_recorded = "当前 Wave：Wave 5" in text or "已归档：Wave 5" in text
    return phase_recorded and all(
        marker in text
        for marker in (
            "W5.A",
            "W5.B",
            "W5.C",
            "W5.D",
        )
    )


def wave5_scope_aligned() -> bool:
    roadmap_ok = file_contains("ROADMAP.md", "W5.A", "W5.B", "W5.C", "W5.D")
    architecture_ok = file_contains(
        "docs/architecture-dependencies.md",
        "W5.A",
        "W5.B",
        "W5.C",
        "W5.D",
    )
    return roadmap_ok and architecture_ok


def collect_items() -> list[EvidenceItem]:
    items: list[EvidenceItem] = []

    startup_ok = wave5_todo_recorded() and file_contains(
        "justfile",
        "wave-5-status",
        "wave-5-complete-check",
    )
    items.append(EvidenceItem(
        "W5-startup",
        "Wave 5 TODO 记录与完成度检查入口已登记",
        PROVED_BY_STATIC_FILES if startup_ok else MISSING_OR_NEEDS_CONFIRMATION,
        ["TODO.md", "justfile", "scripts/governance/report_wave5_completion.py"]
        if startup_ok
        else [],
        [] if startup_ok else ["需要在 TODO 登记当前或归档的 Wave 5，并登记 just wave-5-status / wave-5-complete-check"],
    ))

    scope_ok = wave5_scope_aligned()
    items.append(EvidenceItem(
        "W5-scope-alignment",
        "Wave 5 范围在 ROADMAP 与 architecture-dependencies 中一致",
        PROVED_BY_STATIC_FILES if scope_ok else NEEDS_BUSINESS_DECISION,
        ["ROADMAP.md", "docs/architecture-dependencies.md"] if scope_ok else [],
        [] if scope_ok else ["需要确认 W5.A-D 范围与依赖图是否一致"],
    ))

    external_gate_ok = file_contains(
        "TODO.md",
        "W5.A hardware evidence gate",
        "W5.D TMS evidence gate",
        "W4.D external evidence gate",
    )
    items.append(EvidenceItem(
        "W5-external-gates-tracked",
        "Wave 5 外部硬件 / TMS / 码上放心 evidence gate 已单独跟踪",
        PROVED_BY_STATIC_FILES if external_gate_ok else MISSING_OR_NEEDS_CONFIRMATION,
        ["TODO.md"] if external_gate_ok else [],
        [] if external_gate_ok else ["需要把硬件、TMS、码上放心真实 evidence 从开发完成门禁中拆出并单独跟踪"],
    ))

    packing_ok = (
        file_contains("docs/domain/user-stories-mpk-packing-station.md", "波次：Wave 5")
        and file_contains("backend/crates/api/src/packing_station.rs", "PackingStationService")
        and file_contains("backend/crates/api/src/wave5_repository.rs", "create_packing_station")
        and file_contains("backend/crates/api/src/wave5_handlers.rs", "create_pack_job_handler")
        and file_contains("backend/migrations/202606050001_wave5_value_added_tables.sql", "packing_jobs")
        and openapi_has(
            [
                "/api/v1/packing/stations",
                "/api/v1/packing/jobs",
                "/api/v1/packing/jobs/{id}/weigh",
                "/api/v1/packing/jobs/{id}/waybill",
            ],
            [
                "PackingStation",
                "PackJob",
                "CreatePackingStationRequest",
                "CreatePackJobRequest",
                "WeighPackJobRequest",
                "PrintWaybillRequest",
            ],
        )
    )
    items.append(EvidenceItem(
        "W5.A-packing-station",
        "M-PK 包装站工位、装箱、称重、面单打印生产接口落地",
        PROVED_BY_STATIC_FILES if packing_ok else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "docs/domain/user-stories-mpk-packing-station.md",
            "backend/crates/api/src/packing_station.rs",
            "backend/crates/api/src/wave5_repository.rs",
            "backend/crates/api/src/wave5_handlers.rs",
            "backend/migrations/202606050001_wave5_value_added_tables.sql",
            "shared/openapi/openapi.json",
        ] if packing_ok else [],
        [] if packing_ok else ["缺少 M-PK 工位 / 装箱 / 称重 / 面单接口、持久化或 OpenAPI 证据"],
    ))

    retail_ok = (
        file_contains("backend/crates/api/src/retail_chain.rs", "RetailChainService")
        and file_contains("backend/crates/api/src/wave5_repository.rs", "create_replenishment_suggestion")
        and file_contains("backend/crates/api/src/wave5_handlers.rs", "create_crossdock_plan_handler")
        and file_contains("backend/migrations/202606050001_wave5_value_added_tables.sql", "retail_replenishment_suggestions")
        and openapi_has(
            [
                "/api/v1/retail/replenishment-suggestions",
                "/api/v1/retail/crossdock-plans",
            ],
            [
                "RetailReplenishmentSuggestion",
                "CrossdockPlan",
                "CreateRetailReplenishmentSuggestionRequest",
                "CreateCrossdockPlanRequest",
            ],
        )
    )
    items.append(EvidenceItem(
        "W5.B-retail-chain",
        "M8 连锁门店水位补货与越库作业生产接口落地",
        PROVED_BY_STATIC_FILES if retail_ok else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "backend/crates/api/src/retail_chain.rs",
            "backend/crates/api/src/wave5_repository.rs",
            "backend/crates/api/src/wave5_handlers.rs",
            "backend/migrations/202606050001_wave5_value_added_tables.sql",
            "shared/openapi/openapi.json",
        ] if retail_ok else [],
        [] if retail_ok else ["缺少 M8 补货建议 / 越库计划接口、持久化或 OpenAPI 证据"],
    ))

    billing_ok = (
        file_contains("backend/crates/api/src/billing.rs", "calculate_period_charges")
        and file_contains("backend/crates/api/src/wave5_repository.rs", "generate_billing_statement")
        and file_contains("backend/crates/api/src/wave5_handlers.rs", "generate_billing_statement_handler")
        and file_contains("backend/migrations/202606050001_wave5_value_added_tables.sql", "billing_statements")
        and openapi_has(
            [
                "/api/v1/billing/charges/calculate",
                "/api/v1/billing/statements",
                "/api/v1/billing/statements/{id}/confirm",
            ],
            [
                "BillingChargeCalculation",
                "BillingStatement",
                "CalculateBillingChargesRequest",
                "GenerateBillingStatementRequest",
                "ConfirmBillingStatementRequest",
            ],
        )
    )
    items.append(EvidenceItem(
        "W5.C-billing-rules",
        "M9 3PL 自动计费、计费明细与月结账单生产接口落地",
        PROVED_BY_STATIC_FILES if billing_ok else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "backend/crates/api/src/billing.rs",
            "backend/crates/api/src/wave5_repository.rs",
            "backend/crates/api/src/wave5_handlers.rs",
            "backend/migrations/202606050001_wave5_value_added_tables.sql",
            "shared/openapi/openapi.json",
        ] if billing_ok else [],
        [] if billing_ok else ["缺少 M9 自动计费 / 月结账单接口、持久化或 OpenAPI 证据"],
    ))

    tms_ok = (
        file_contains("backend/crates/api/src/tms_plus.rs", "TmsPlusService")
        and file_contains("backend/crates/api/src/wave5_repository.rs", "receive_tms_dispatch")
        and file_contains("backend/crates/api/src/wave5_handlers.rs", "confirm_container_recovery_handler")
        and file_contains("backend/migrations/202606050001_wave5_value_added_tables.sql", "tms_dispatches")
        and openapi_has(
            [
                "/api/v1/tms/dispatches",
                "/api/v1/tms/transit-temperature-readings",
                "/api/v1/tms/container-recoveries",
            ],
            [
                "TmsDispatch",
                "TransitTemperatureReading",
                "ContainerRecovery",
                "ReceiveTmsDispatchRequest",
                "IngestTransitTemperatureRequest",
                "ConfirmContainerRecoveryRequest",
            ],
        )
    )
    items.append(EvidenceItem(
        "W5.D-tms-plus",
        "M10 TMS+ 调度接收、在途温控关联和容器回收生产接口落地",
        PROVED_BY_STATIC_FILES if tms_ok else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "backend/crates/api/src/tms_plus.rs",
            "backend/crates/api/src/wave5_repository.rs",
            "backend/crates/api/src/wave5_handlers.rs",
            "backend/migrations/202606050001_wave5_value_added_tables.sql",
            "shared/openapi/openapi.json",
        ] if tms_ok else [],
        [] if tms_ok else ["缺少 M10 调度 / 在途温控 / 容器回收接口、持久化或 OpenAPI 证据"],
    ))

    tenant_ok = (
        file_contains("backend/crates/api/tests/wave5_postgres.rs", "wave5_owner_isolation")
        and file_contains("backend/crates/api/src/wave5_repository.rs", "ctx.owner_id")
        and file_contains("backend/crates/api/src/wave5_repository.rs", "owner_id")
    )
    items.append(EvidenceItem(
        "W5-tenant-isolation",
        "M-PK / M8 / M9 / M10 写操作保持 owner_id 多货主隔离",
        PROVED_BY_STATIC_FILES if tenant_ok else MISSING_OR_NEEDS_CONFIRMATION,
        ["backend/crates/api/src/wave5_repository.rs", "backend/crates/api/tests/wave5_postgres.rs"] if tenant_ok else [],
        [] if tenant_ok else ["缺少 Wave 5 owner_id 隔离持久化逻辑和真实 PostgreSQL 测试证据"],
    ))

    scenario_ok = file_contains(
        "backend/crates/api/tests/wave5_postgres.rs",
        "chain_store_replenishment_to_packing_tms_and_billing",
    )
    items.append(EvidenceItem(
        "W5-chain-scenario",
        "至少一个连锁客户端到端场景可复跑",
        PROVED_BY_STATIC_FILES if scenario_ok else MISSING_OR_NEEDS_CONFIRMATION,
        ["backend/crates/api/tests/wave5_postgres.rs"] if scenario_ok else [],
        [] if scenario_ok else ["缺少门店补货 → 出库 → 装箱 → TMS/快递 → 计费的可复跑场景测试"],
    ))

    return items


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--strict", action="store_true", help="Wave 5 出口检查，阻塞缺口返回非零")
    args = parser.parse_args(argv)

    items = collect_items()
    blocking = [item for item in items if item.blocks_strict]
    ok = not blocking

    if args.json:
        print(json.dumps({
            "report": "wave5_completion",
            "tier": "manual",
            "category": "流程治理",
            "items": [asdict(item) for item in items],
            "blocking_gaps": [asdict(item) for item in blocking],
            "ok": ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("report_wave5_completion (流程治理，静态证据覆盖报告)")
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
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
