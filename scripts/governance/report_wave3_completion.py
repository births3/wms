#!/usr/bin/env python3
"""report_wave3_completion.py — Wave 3 完成度证据报告

类别：4. 流程治理（报告型，默认不阻塞）
Tier：手动 / Wave 出口检查
输入：ROADMAP.md Wave 3 完成标准 + 当前仓库文件
输出：人类可读 + --json
退出码：
  默认：0（只报告当前证据）
  --strict：任一阻塞缺口返回 1

本脚本只把 ROADMAP.md / TODO.md 已有完成标准转成可复跑证据检查，不新增业务语义。
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from validate_wave3_pda_runtime_evidence import validate_one

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent

PROVED_BY_STATIC_FILES = "PROVED_BY_STATIC_FILES"
MISSING_OR_NEEDS_CONFIRMATION = "MISSING_OR_NEEDS_CONFIRMATION"
PRE_RELEASE_GATE = "PRE_RELEASE_GATE"


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


@dataclass
class LayerEvidence:
    layer_id: str
    requirement: str
    complete: bool
    evidence: list[str] = field(default_factory=list)
    gaps: list[str] = field(default_factory=list)
    strict_blocking: bool = True


def read_text(path: str) -> str:
    target = REPO_ROOT / path
    if not target.exists() or not target.is_file():
        return ""
    family = [target]
    if target.suffix:
        family.extend(sorted(target.parent.glob(f"{target.stem}_part*{target.suffix}")))
        split_dir = target.parent / target.stem
        if split_dir.is_dir():
            family.extend(sorted(split_dir.rglob(f"*{target.suffix}")))
    return "\n".join(item.read_text(encoding="utf-8") for item in dict.fromkeys(family))


def file_exists(path: str) -> bool:
    return (REPO_ROOT / path).exists()


def file_contains(path: str, *needles: str) -> bool:
    text = read_text(path)
    return bool(text) and all(needle in text for needle in needles)


def files_contain(paths: list[str], *needles: str) -> bool:
    text = "\n".join(read_text(path) for path in paths)
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


def pda_app_started() -> bool:
    root = REPO_ROOT / "apps" / "pda-mobile"
    if not root.exists():
        return False
    real_files = [
        path
        for path in root.rglob("*")
        if path.is_file() and path.name != ".gitkeep"
    ]
    return bool(real_files) and (root / "package.json").exists()


def adr_0027_accepted() -> bool:
    text = read_text("docs/adr/0027-pda-offline-model.md")
    return "- 状态：Accepted" in text or "- 状态: Accepted" in text


def pda_runtime_evidence_status() -> tuple[bool, str]:
    return validate_one(
        REPO_ROOT / "docs/retros/wave-3-pda-runtime-evidence.json",
        allow_example_refs=False,
    )


def pda_readiness_recorded() -> bool:
    return (
        file_contains(
            "docs/runbooks/wave-3-pda-readiness.md",
            "SPIKE-005B",
            "不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖",
            "设备清单",
            "蓝牙打印",
            "Wave 5",
            "docs/retros/wave-5-hardware-evidence.json",
        )
        and file_contains(
            "docs/spikes/spike-005-rn-scanner.md",
            "7.7 Wave 3 readiness 决策",
            "先落 readiness/runbook",
            "真 PDA",
            "手机摄像头不能作为 SPIKE-005 evidence",
        )
        and file_contains(
            "docs/spikes/spike-005b-webview-capacitor-pda.md",
            "7.1 用户确认",
            "WebView/Capacitor native shell",
            "不直接替换 ADR-0001",
        )
        and file_contains(
            "docs/adr/0027-pda-offline-model.md",
            "PDA 离线模型与技术栈定版框架",
            "react-native",
            "webview-capacitor",
            "本 ADR 进入 Accepted 的前置条件",
            "apps/pda-mobile",
        )
        and file_contains(
            "docs/domain/clarifications.md",
            "PDA 端推进方式",
            "SPIKE-005 / SPIKE-005B readiness",
            "PDA Web 打包方案边界",
            "ADR-0027 定版",
            "不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖",
        )
    )


def latest_gsp_qualification_decision() -> str | None:
    clarification = read_text("docs/domain/clarifications.md")
    rows = [
        line
        for line in clarification.splitlines()
        if line.strip().startswith("|") and "GSP 资质有效期校验来源" in line
    ]
    return rows[-1] if rows else None


def gsp_qualification_source_frozen() -> bool:
    latest_decision = latest_gsp_qualification_decision()
    if latest_decision is None:
        return False
    unresolved_tokens = ["暂不冻结", "接口占位", "来源未冻结"]
    if any(token in latest_decision for token in unresolved_tokens):
        return False
    frozen_sources = [
        "M1 本地资质档案",
        "M1 本地资质表",
        "M-VR 校验规则",
        "ERP/H8 校验端口",
        "ERP 校验端口",
        "H8 校验端口",
    ]
    return any(source in latest_decision for source in frozen_sources)


def gsp_qualification_chain_recorded() -> bool:
    return (
        file_contains(
            "docs/domain/user-stories-m1-master-data-product.md",
            "US-M1-002",
            "资质证照",
            "资质过期的供应商，系统拒绝其收货单创建",
        )
        and file_contains(
            "docs/domain/user-stories-m2-inbound-asn.md",
            "创建时校验：供应商资质有效期",
            "由 M-VR 校验规则模块执行",
        )
        and file_contains(
            "docs/domain/user-stories-mvr-validation-rules.md",
            "IF 供应商资质过期 THEN 拒绝",
            "供应商资质有效期",
        )
        and file_contains(
            "docs/compliance/gsp-ch6-procurement-acceptance.md",
            "M1-002 供应商资质录入",
            "M-VR-001 校验规则配置",
            "M2-001 ASN 创建时自动校验",
        )
    )


def collect_key_path_layers() -> list[LayerEvidence]:
    layers: list[LayerEvidence] = []

    l1_ok = (
        file_contains(
            "backend/crates/api/src/inbound.rs",
            "receiving_workflow_enforces_quantity_closure_and_dual_signature",
            "receiving_inspection_rejects_expired_batch",
        )
        and file_contains(
            "backend/crates/api/src/inventory.rs",
            "inbound_putaway_increases_owner_scoped_available_inventory",
            "inventory_status_transition_requires_allowed_approval_source",
        )
    )
    layers.append(LayerEvidence(
        "L1",
        "领域规则单元测试：M2 收货/验收/签字/上架 + M3 库存状态规则",
        l1_ok,
        ["backend/crates/api/src/inbound.rs", "backend/crates/api/src/inventory.rs"] if l1_ok else [],
        [] if l1_ok else ["缺少 M2/M3 领域规则单元测试函数"],
    ))

    l2_ok = openapi_has(
        [
            "/api/v1/inbound/receiving-orders/{id}/receive",
            "/api/v1/inbound/receiving-orders/{id}/inspect",
            "/api/v1/inbound/receiving-orders/{id}/sign",
            "/api/v1/inbound/receiving-orders/{id}/putaway",
            "/api/v1/inventory/batches",
            "/api/v1/inventory/batches/status",
        ],
        [
            "ReceiveReceivingOrderRequest",
            "ReceivingOrderReceipt",
            "PutawayRequest",
            "InventoryBatch",
            "ChangeInventoryStatusRequest",
        ],
    ) and file_contains("packages/api-client/src/schema.ts", '"Idempotency-Key": string')
    layers.append(LayerEvidence(
        "L2",
        "API 契约：M2/M3 path、schema 与 Idempotency-Key 客户端类型",
        l2_ok,
        ["shared/openapi/openapi.json", "packages/api-client/src/schema.ts"] if l2_ok else [],
        [] if l2_ok else ["缺少 M2/M3 OpenAPI path/schema 或 api-client header 类型"],
    ))

    handler_sources = [
        "backend/crates/api/src/wave3_handlers.rs",
        "backend/crates/api/src/wave3_handlers_part1.rs",
        "backend/crates/api/src/wave3_handlers_part2.rs",
        "backend/crates/api/src/wave3_handlers_part3.rs",
        "backend/crates/api/src/wave3_handlers_part4.rs",
    ]
    l3_ok = files_contain(
        handler_sources,
        "postgres_receive_handler_writes_business_idempotency_and_audit",
        "postgres_putaway_handler_commits_inventory_and_audit",
        "postgres_inventory_query_and_status_change_are_scoped_idempotent_and_audited",
    )
    layers.append(LayerEvidence(
        "L3",
        "业务流程：真实 PostgreSQL handler 覆盖 M2 receive/putaway 与 M3 查询/状态变更",
        l3_ok,
        ["backend/crates/api/src/wave3_handlers.rs"] if l3_ok else [],
        [] if l3_ok else ["缺少 M2/M3 PostgreSQL handler 业务流程测试"],
    ))

    l4_ok = (
        file_contains("backend/crates/api/src/inbound.rs", "receiving_inspection_rejects_expired_batch")
        and files_contain(
            handler_sources,
            "MissingApprovalSource",
            "MissingIdempotencyKey",
        )
    )
    layers.append(LayerEvidence(
        "L4",
        "错误路径：过期批次、缺审批源、缺幂等键均有测试",
        l4_ok,
        ["backend/crates/api/src/inbound.rs", "backend/crates/api/src/wave3_handlers.rs"] if l4_ok else [],
        [] if l4_ok else ["缺少 M2/M3 关键错误路径测试"],
    ))

    l5_ok = (
        file_contains(
            "backend/crates/api/tests/wave3_postgres.rs",
            "putaway_commits_receiving_inventory_and_movement_in_one_transaction",
            "receiving_putaways",
            "inventory_batches",
            "inventory_movements",
        )
        and files_contain(
            handler_sources,
            "audit_event",
            "inventory_status_changes",
        )
    )
    layers.append(LayerEvidence(
        "L5",
        "数据一致性：M2 上架、M3 库存、流水、审计同事务断言",
        l5_ok,
        ["backend/crates/api/tests/wave3_postgres.rs", "backend/crates/api/src/wave3_handlers.rs"] if l5_ok else [],
        [] if l5_ok else ["缺少跨表一致性或审计同写断言"],
    ))

    l6_ok = file_contains(
        "backend/crates/api/tests/wave3_postgres.rs",
        "concurrent_same_idempotency_key_replays_first_receipt",
        "tokio::join!",
    )
    layers.append(LayerEvidence(
        "L6",
        "并发：同 owner + 同 Idempotency-Key 并发重放一致",
        l6_ok,
        ["backend/crates/api/tests/wave3_postgres.rs"] if l6_ok else [],
        [] if l6_ok else ["缺少 M2 幂等并发竞争测试"],
    ))

    layers.append(LayerEvidence(
        "L7",
        "性能 / 易用性 SLA：关键路径 P95 或 PDA 单步交互基准（预发布门禁）",
        False,
        [],
        ["按用户决策降级为预发布 gate；不能用 local/mock/fake/example 代替真实 dev/staging/PDA 证据"],
        strict_blocking=False,
    ))

    l8_ok = files_contain(
        handler_sources,
        "inbound_receive_handler_requires_permission_and_appends_audit",
        "PermissionDenied",
        "postgres_inventory_query_and_status_change_are_scoped_idempotent_and_audited",
        "other_owner_id",
    )
    layers.append(LayerEvidence(
        "L8",
        "权限 / 租户隔离：M2 写权限与 M3 owner 隔离",
        l8_ok,
        ["backend/crates/api/src/wave3_handlers.rs"] if l8_ok else [],
        [] if l8_ok else ["缺少 M2 权限或 M3 owner 隔离测试"],
    ))

    layers.append(LayerEvidence(
        "L9",
        "版本兼容：首个正式版本前按 ADR-0038 不启用",
        True,
        ["docs/adr/0038-pre-v1-compatibility-policy.md"],
        [],
        strict_blocking=False,
    ))

    l10_ok = files_contain(
        handler_sources,
        "audit_event",
        "action = 'receive'",
        "action = 'putaway'",
        "action = 'change_status'",
        "verify_hash_chain",
    )
    layers.append(LayerEvidence(
        "L10",
        "可观测性：关键业务动作写 H2 audit_event / fallback 审计链可校验",
        l10_ok,
        ["backend/crates/api/src/wave3_handlers.rs"] if l10_ok else [],
        [] if l10_ok else ["缺少关键业务动作 audit_event 或审计链断言"],
    ))

    l11_ok = (
        files_contain(
            [
                "backend/crates/api/tests/wave3_postgres.rs",
                "backend/crates/api/tests/wave3_evidence_postgres.rs",
            ],
            "receiving_receipt_is_single_closure_and_idempotent",
            "expired_idempotency_key_is_not_replayed",
            "idempotency_request",
        )
        and files_contain(
            handler_sources,
            "same idempotency key should replay",
            "Idempotency-Key",
        )
    )
    layers.append(LayerEvidence(
        "L11",
        "幂等性：M2/M3 写操作 idempotency_request replay 与 TTL 过期处理",
        l11_ok,
        ["backend/crates/api/tests/wave3_postgres.rs", "backend/crates/api/src/wave3_handlers.rs"] if l11_ok else [],
        [] if l11_ok else ["缺少 M2/M3 幂等 replay 或 TTL 测试"],
    ))

    return layers


def collect_items() -> list[EvidenceItem]:
    items: list[EvidenceItem] = []

    repository_sources = [
        "backend/crates/api/src/wave3_repository.rs",
        "backend/crates/api/src/wave3_repository_part1.rs",
        "backend/crates/api/src/wave3_repository_part2.rs",
        "backend/crates/api/src/wave3_repository/receiving_read.rs",
        "backend/crates/api/src/wave3_repository/receiving_update.rs",
    ]
    handler_sources = [
        "backend/crates/api/src/wave3_handlers.rs",
        "backend/crates/api/src/wave3_handlers_part1.rs",
        "backend/crates/api/src/wave3_handlers_part2.rs",
        "backend/crates/api/src/wave3_handlers_part3.rs",
        "backend/crates/api/src/wave3_handlers_part4.rs",
    ]

    m2_ok = (
        file_contains("backend/crates/api/src/inbound.rs", "ReceivingOrderStore", "ReceivingOrderError")
        and files_contain(
            repository_sources,
            "receive_receiving_order_with_audit",
            "inspect_receiving_order_with_audit",
            "sign_receiving_order_with_audit",
            "putaway_receiving_order_and_inventory_with_audit",
        )
        and files_contain(
            handler_sources,
            "postgres_receive_handler_writes_business_idempotency_and_audit",
            "postgres_inspect_and_sign_handlers_write_idempotency_and_audit",
            "postgres_putaway_handler_commits_inventory_and_audit",
        )
    )
    items.append(EvidenceItem(
        "W3.A-backend",
        "M2 收货 / 验收 / 双人签字 / 上架规则与 PostgreSQL handler 持久化",
        PROVED_BY_STATIC_FILES if m2_ok else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "backend/crates/api/src/inbound.rs",
            "backend/crates/api/src/wave3_repository.rs",
            "backend/crates/api/src/wave3_handlers.rs",
        ] if m2_ok else [],
        [] if m2_ok else ["缺少 M2 领域规则、repository 或 handler PostgreSQL 测试证据"],
    ))

    m3_ok = (
        file_contains("backend/crates/api/src/inventory.rs", "InventoryStore", "allowed_transition")
        and files_contain(
            repository_sources,
            "list_inventory_batches",
            "change_inventory_status_with_audit",
            "inventory_status_changes",
        )
        and files_contain(
            handler_sources,
            "postgres_inventory_query_and_status_change_are_scoped_idempotent_and_audited",
        )
    )
    items.append(EvidenceItem(
        "W3.B-backend",
        "M3 库存批次、上架入库、状态变更与 PostgreSQL handler 持久化",
        PROVED_BY_STATIC_FILES if m3_ok else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "backend/crates/api/src/inventory.rs",
            "backend/crates/api/src/wave3_repository.rs",
            "backend/crates/api/src/wave3_handlers.rs",
        ] if m3_ok else [],
        [] if m3_ok else ["缺少 M3 领域规则、repository 或 handler PostgreSQL 测试证据"],
    ))

    m5_ok = (
        file_contains("backend/crates/api/src/cold_chain.rs", "ColdChainService", "IngestTemperatureReadingRequest")
        and files_contain(
            handler_sources,
            "ExternalApiKeyConfig",
            "EXTERNAL_API_KEY_HEADER",
            "external_auth_headers",
            "postgres_cold_chain_reading_uses_external_api_key_idempotency_and_audit",
            "postgres_cold_chain_excursion_is_idempotent_and_audited",
        )
        and file_contains("shared/openapi/openapi.json", "X-WMS-API-Key")
    )
    items.append(EvidenceItem(
        "W3.C-backend",
        "M5 外部冷链 readings/excursions schema、外部 API Key、幂等与审计落库",
        PROVED_BY_STATIC_FILES if m5_ok else MISSING_OR_NEEDS_CONFIRMATION,
        ["backend/crates/api/src/cold_chain.rs", "backend/crates/api/src/wave3_handlers.rs"] if m5_ok else [],
        [] if m5_ok else ["缺少 M5 外部鉴权、幂等或 PostgreSQL 审计测试证据"],
    ))

    m9_ok = (
        file_contains(
            "backend/crates/api/src/billing.rs",
            "BillingStore",
            "InvalidEffectiveWindow",
            "BillingRuleConflict",
            "billing_effective_windows_reject_invalid_or_overlapping_ranges",
        )
        and file_contains(
            "backend/crates/api/tests/wave3_postgres.rs",
            "billing_rule_effective_window_rejects_overlap",
        )
    )
    items.append(EvidenceItem(
        "W3.D-current-scope",
        "M9 当前 Wave 范围：账户 / 合同 / 规则模型与生效期冲突校验",
        PROVED_BY_STATIC_FILES if m9_ok else MISSING_OR_NEEDS_CONFIRMATION,
        ["backend/crates/api/src/billing.rs", "backend/crates/api/tests/wave3_postgres.rs"] if m9_ok else [],
        [] if m9_ok else ["缺少 M9 当前范围模型或生效期冲突测试证据"],
        strict_blocking=False,
    ))

    pda_ready = pda_readiness_recorded()
    pda_started = pda_app_started()
    pda_adr_accepted = adr_0027_accepted()
    pda_evidence_ok, pda_evidence_message = (
        pda_runtime_evidence_status()
        if pda_started and pda_adr_accepted
        else (False, "")
    )
    pda_ok = pda_started and pda_adr_accepted and pda_evidence_ok
    items.append(EvidenceItem(
        "W3.A-PDA-readiness",
        "PDA 生产端启动前置：SPIKE-005 / SPIKE-005B readiness、设备清单与执行 runbook",
        PROVED_BY_STATIC_FILES if pda_ready else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "docs/runbooks/wave-3-pda-readiness.md",
            "docs/adr/0027-pda-offline-model.md",
            "docs/domain/clarifications.md",
            "docs/spikes/spike-005-rn-scanner.md",
            "docs/spikes/spike-005b-webview-capacitor-pda.md",
        ] if pda_ready else [],
        [] if pda_ready else ["缺少 PDA readiness runbook、ADR-0027、clarifications 或 SPIKE-005 / 005B readiness 决策记录"],
    ))
    items.append(EvidenceItem(
        "W3.A-PDA-production",
        "PDA 生产 app：承接 M2/M3 扫码与离线队列",
        PROVED_BY_STATIC_FILES if pda_ok else PRE_RELEASE_GATE,
        [
            "apps/pda-mobile",
            "docs/adr/0027-pda-offline-model.md",
            "docs/retros/wave-3-pda-runtime-evidence.json",
        ] if pda_ok else [],
        [] if pda_ok else [
            f"ADR-0027 Accepted 后还必须通过 PDA runtime evidence validator：{pda_evidence_message}"
            if pda_started and pda_adr_accepted
            else
            "生产 app 必须等 ADR-0027 Accepted；当前仍需 SPIKE-005 / 005B 真机验证和 dev/staging evidence"
            if pda_started
            else "按用户决策先不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖；生产 app 等 SPIKE-005 / 005B 真机验证、ADR-0027 Accepted 后启动"
        ],
        strict_blocking=False,
    ))

    layers = collect_key_path_layers()
    missing_blocking_layers = [layer for layer in layers if layer.strict_blocking and not layer.complete]
    pre_release_layers = [layer for layer in layers if not layer.strict_blocking and not layer.complete]
    key_path_ok = not missing_blocking_layers
    items.append(EvidenceItem(
        "W3-keypath-11-layers",
        "M2 / M3 关键路径 11 层测试覆盖（L7 为预发布 gate）",
        PROVED_BY_STATIC_FILES if key_path_ok else MISSING_OR_NEEDS_CONFIRMATION,
        sorted({evidence for layer in layers for evidence in layer.evidence}) if key_path_ok else [],
        [f"{layer.layer_id}: {'; '.join(layer.gaps)}" for layer in missing_blocking_layers],
    ))
    items.append(EvidenceItem(
        "W3-L7-pre-release",
        "M2 / M3 L7 性能与易用性真实环境证据",
        PRE_RELEASE_GATE if pre_release_layers else PROVED_BY_STATIC_FILES,
        [],
        [f"{layer.layer_id}: {'; '.join(layer.gaps)}" for layer in pre_release_layers],
        strict_blocking=False,
    ))

    gsp_source_ok = gsp_qualification_source_frozen()
    gsp_chain_ok = gsp_qualification_chain_recorded()
    gsp_ok = gsp_source_ok and gsp_chain_ok
    items.append(EvidenceItem(
        "W3-GSP-qualification-source",
        "GSP 资质有效期校验来源冻结并生效",
        PROVED_BY_STATIC_FILES if gsp_ok else MISSING_OR_NEEDS_CONFIRMATION,
        [
            "docs/domain/clarifications.md",
            "docs/domain/user-stories-m1-master-data-product.md",
            "docs/domain/user-stories-m2-inbound-asn.md",
            "docs/domain/user-stories-mvr-validation-rules.md",
            "docs/compliance/gsp-ch6-procurement-acceptance.md",
        ] if gsp_ok else [],
        [] if gsp_ok else [
            "需要最新澄清记录冻结来源，并保证 M1-002 / M2-001 / M-VR / GSP ch6 链路均声明供应商资质有效期校验",
        ],
    ))

    return items


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--strict", action="store_true", help="Wave 3 出口检查，阻塞缺口返回非零")
    args = parser.parse_args(argv)

    layers = collect_key_path_layers()
    items = collect_items()
    blocking = [item for item in items if item.blocks_strict]
    ok = not blocking

    if args.json:
        print(json.dumps({
            "report": "wave3_completion",
            "tier": "manual",
            "category": "流程治理",
            "items": [asdict(item) for item in items],
            "key_path_layers": [asdict(layer) for layer in layers],
            "blocking_gaps": [asdict(item) for item in blocking],
            "ok": ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("report_wave3_completion (流程治理，静态证据覆盖报告)")
        for item in items:
            mark = "✓" if item.complete else ("!" if not item.strict_blocking else "✘")
            print(f"  {mark} {item.item_id}: {item.requirement}")
            print(f"    status: {item.status}")
            for evidence in item.evidence:
                print(f"    evidence: {evidence}")
            for gap in item.gaps:
                print(f"    gap: {gap}")

        print("\nM2/M3 关键路径 11 层明细:")
        for layer in layers:
            mark = "✓" if layer.complete else ("!" if not layer.strict_blocking else "✘")
            print(f"  {mark} {layer.layer_id}: {layer.requirement}")
            for evidence in layer.evidence:
                print(f"    evidence: {evidence}")
            for gap in layer.gaps:
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
