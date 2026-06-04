#!/usr/bin/env python3
"""report_wave2_completion.py — Wave 2 完成度证据报告

类别：4. 流程治理（报告型，默认不阻塞）
Tier：手动 / Wave 出口检查
输入：ROADMAP.md Wave 2 完成标准 + 当前仓库文件
输出：人类可读 + --json
退出码：
  默认：0（只报告当前证据）
  --strict：静态完成项缺口返回 1；预发布 runtime evidence 缺口只报告不阻断
  --require-runtime-evidence：真实 dev/staging runtime evidence 缺口返回 1

本脚本只把 ROADMAP.md 已有完成标准转成可复跑证据检查，不新增业务语义。
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
DEFAULT_RUNTIME_EVIDENCE = REPO_ROOT / "docs/retros/wave-2-runtime-evidence.json"

PROVED_BY_STATIC_FILES = "PROVED_BY_STATIC_FILES"
PROVED_BY_RUNTIME_EVIDENCE = "PROVED_BY_RUNTIME_EVIDENCE"
MISSING = "MISSING"
PRE_RELEASE_GATE = "PRE_RELEASE_GATE"

BLOCKED_RUNTIME_REF_TOKENS = ("localhost", "127.0.0.1", "example.com", "prod", "production")


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
        return self.status in {PROVED_BY_STATIC_FILES, PROVED_BY_RUNTIME_EVIDENCE}

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


def validate_wave2_runtime_payload(data: object) -> tuple[bool, str]:
    if not isinstance(data, dict):
        return False, "runtime evidence 顶层必须是 object"

    environment = str(data.get("environment", "")).lower()
    if environment not in {"dev", "staging"}:
        return False, "environment 必须是真实 dev 或 staging，不能是 local/prod/example"

    target = " ".join(
        str(data.get(key, ""))
        for key in ("service_url", "smoke_log_ref", "reconcile_log_ref")
    ).lower()
    if any(token in target for token in BLOCKED_RUNTIME_REF_TOKENS):
        return False, "证据引用不能指向 local/example/prod"

    if data.get("source_switched_to") != "config_center":
        return False, "source_switched_to 必须为 config_center"

    reconcile = data.get("reconcile")
    if not isinstance(reconcile, dict):
        return False, "缺少 reconcile 对账结果"
    if reconcile.get("missing_in_config_center") or reconcile.get("mismatched"):
        return False, "对账结果仍有 missing_in_config_center 或 mismatched"

    if not data.get("smoke_log_ref") or not data.get("reconcile_log_ref"):
        return False, "必须记录 smoke_log_ref 与 reconcile_log_ref"

    return True, "docs/retros/wave-2-runtime-evidence.json 记录真实 dev/staging 配置中心灰度证据"


def valid_wave2_runtime_evidence() -> tuple[bool, str]:
    path = DEFAULT_RUNTIME_EVIDENCE
    if not path.exists():
        return False, "缺少 docs/retros/wave-2-runtime-evidence.json 真实 dev/staging 配置中心灰度证据"

    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        return False, f"docs/retros/wave-2-runtime-evidence.json JSON 无效：{error}"

    return validate_wave2_runtime_payload(data)


def collect_items() -> list[EvidenceItem]:
    items: list[EvidenceItem] = []

    m1_ok = (
        file_contains(
            "backend/crates/domain/src/lib.rs",
            "pub struct Product",
            "pub struct Supplier",
            "pub struct Customer",
            "pub struct Warehouse",
            "pub struct Location",
            "pub struct SpecialDrugCategory",
        )
        and file_contains("backend/crates/api/src/master_data.rs", "pub struct MasterDataStore", "product_crud_is_owner_scoped")
        and openapi_has(
            [
                "/api/v1/master-data/products",
                "/api/v1/master-data/suppliers",
                "/api/v1/master-data/customers",
                "/api/v1/master-data/warehouses",
                "/api/v1/master-data/locations",
                "/api/v1/master-data/special-drug-categories",
            ],
            ["Product", "Supplier", "Customer", "Warehouse", "Location", "SpecialDrugCategory"],
        )
    )
    items.append(EvidenceItem(
        "W2.A",
        "M1.a 基础档案 schema + 商品/供应商/客户/仓库/库位/特殊药品分类基础 CRUD",
        PROVED_BY_STATIC_FILES if m1_ok else MISSING,
        [
            "backend/crates/domain/src/lib.rs",
            "backend/crates/api/src/master_data.rs",
            "shared/openapi/openapi.json",
        ] if m1_ok else [],
        [] if m1_ok else ["缺少 M1 schema、CRUD 服务、测试或 OpenAPI 契约"],
    ))

    m2_ok = (
        file_contains("backend/crates/domain/src/lib.rs", "pub struct ReceivingOrder", "pub struct ReceivingOrderLine")
        and file_contains("backend/crates/api/src/inbound.rs", "pub struct ReceivingOrderStore", "receiving_order_crud_is_owner_scoped")
        and openapi_has(["/api/v1/inbound/receiving-orders"], ["ReceivingOrder", "CreateReceivingOrderRequest"])
    )
    items.append(EvidenceItem(
        "W2.B",
        "M2 入库收货单 schema 设计与基础 CRUD 骨架",
        PROVED_BY_STATIC_FILES if m2_ok else MISSING,
        ["backend/crates/domain/src/lib.rs", "backend/crates/api/src/inbound.rs", "shared/openapi/openapi.json"] if m2_ok else [],
        [] if m2_ok else ["缺少 M2 收货单 schema、基础 CRUD 或 OpenAPI 契约"],
    ))

    m6_ok = (
        file_contains("backend/crates/api/src/reports.rs", "pub struct ReportService", "report_query_returns_stable_skeleton_shape")
        and openapi_has(["/api/v1/reports/query"], ["ReportQueryRequest", "ReportQueryResponse", "ReportRow"])
    )
    items.append(EvidenceItem(
        "W2.C",
        "M6 报表查询接口骨架",
        PROVED_BY_STATIC_FILES if m6_ok else MISSING,
        ["backend/crates/api/src/reports.rs", "shared/openapi/openapi.json"] if m6_ok else [],
        [] if m6_ok else ["缺少 M6 报表服务骨架、测试或 OpenAPI 契约"],
    ))

    mpm_ok = (
        file_contains(
            "backend/crates/api/src/parameter_mapping.rs",
            "pub struct ParameterMappingService",
            "pub fn add_dictionary",
            "pub fn add_rule",
            "pub fn execute",
            "pub fn trace",
            "pending_mapping",
            "maps_irregular_erp_payload_and_traces_execution",
        )
        and openapi_has(
            ["/api/v1/parameter-mapping/execute", "/api/v1/parameter-mapping/traces/{execution_id}"],
            ["MappingDictionary", "MappingRule", "MappingQueueItem", "ExecuteMappingRequest", "MappingTraceResponse"],
        )
    )
    items.append(EvidenceItem(
        "W2.E",
        "M-PM 参数对照：字典 / 规则 / 待映射队列 / 执行 API / 反向追溯",
        PROVED_BY_STATIC_FILES if mpm_ok else MISSING,
        ["backend/crates/api/src/parameter_mapping.rs", "shared/openapi/openapi.json"] if mpm_ok else [],
        [] if mpm_ok else ["缺少 M-PM 映射能力、测试或 OpenAPI 契约"],
    ))

    config_ok = (
        file_contains(
            "backend/crates/api/src/config_center.rs",
            "pub struct ConfigCenterStore",
            "migrate_feature_flags_from_file",
            "import_feature_flags_batch",
            "reconcile_feature_flags",
            "switch_feature_flag_source",
            "export_feature_flags",
            "archive_file_feature_flags",
            "migrates_reconciles_and_switches_feature_flags",
        )
        and file_contains("backend/crates/api/src/feature_flags.rs", "pub fn flags(&self)")
        and openapi_has(
            [
                "/api/v1/config-center/feature-flags/migrate",
                "/api/v1/config-center/feature-flags/reconcile",
                "/api/v1/config-center/feature-flags/export",
                "/api/v1/config-center/feature-flags/import",
                "/api/v1/config-center/feature-flags/source",
                "/api/v1/config-center/feature-flags/archive-file-source",
            ],
            [
                "ConfigEntry",
                "FeatureFlagConfig",
                "FeatureFlagMigrationResult",
                "FeatureFlagReconcileReport",
                "FeatureFlagExportResponse",
                "FeatureFlagBatchImportRequest",
                "FeatureFlagArchiveResult",
            ],
        )
    )
    items.append(EvidenceItem(
        "W2.G-static",
        "Feature Flag 从 W1 文件版迁移到 M1-008 配置中心：迁移 / 导出 / 对账 / 切换读取源",
        PROVED_BY_STATIC_FILES if config_ok else MISSING,
        ["backend/crates/api/src/config_center.rs", "backend/crates/api/src/feature_flags.rs", "shared/openapi/openapi.json"] if config_ok else [],
        [] if config_ok else ["缺少配置中心版 Feature Flag 迁移、对账、切源或 OpenAPI 契约"],
    ))

    runtime_ok, runtime_message = valid_wave2_runtime_evidence()
    items.append(EvidenceItem(
        "W2.G-runtime",
        "配置中心版灰度链路在真实 dev/staging 验证可用，且 W1 文件版 flag 迁移对账通过",
        PROVED_BY_RUNTIME_EVIDENCE if runtime_ok else PRE_RELEASE_GATE,
        ["docs/retros/wave-2-runtime-evidence.json"] if runtime_ok else [],
        [] if runtime_ok else [runtime_message],
        strict_blocking=False,
    ))

    return items


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--strict", action="store_true", help="静态完成项缺口返回非零")
    parser.add_argument("--require-runtime-evidence", action="store_true", help="预发布阶段要求真实 runtime evidence")
    args = parser.parse_args(argv)

    items = collect_items()
    blocking = [item for item in items if item.blocks_strict]
    pre_release = [item for item in items if not item.strict_blocking and not item.complete]
    runtime_blocking = pre_release if args.require_runtime_evidence else []
    ok = not blocking and not runtime_blocking

    if args.json:
        print(json.dumps({
            "report": "wave2_completion",
            "tier": "manual",
            "category": "流程治理",
            "items": [asdict(item) for item in items],
            "blocking_gaps": [asdict(item) for item in blocking],
            "pre_release_gates": [asdict(item) for item in pre_release],
            "runtime_blocking_gaps": [asdict(item) for item in runtime_blocking],
            "ok": ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("report_wave2_completion (流程治理，静态证据覆盖报告)")
        for item in items:
            mark = "✓" if item.complete else ("!" if not item.strict_blocking else "✘")
            print(f"  {mark} {item.item_id}: {item.requirement}")
            print(f"    status: {item.status}")
            for evidence in item.evidence:
                print(f"    evidence: {evidence}")
            for gap in item.gaps:
                print(f"    gap: {gap}")
        if blocking:
            print(f"\n阻塞缺口: {len(blocking)}")
        if pre_release:
            print(f"\n预发布门禁缺口: {len(pre_release)}（不阻断 Wave 2 静态完成检查）")
        if runtime_blocking:
            print(f"runtime evidence 阻塞缺口: {len(runtime_blocking)}")

    if args.require_runtime_evidence and runtime_blocking:
        return 1
    return 1 if args.strict and blocking else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
