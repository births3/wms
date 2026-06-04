#!/usr/bin/env python3
"""report_wave6_pre_release.py — Wave 6 预发布证据收口报告。

类别：4. 流程治理（报告型，默认不阻塞）
Tier：手动 / Wave 出口检查
输入：ADR-0035 + TODO.md + 已有 runtime evidence validator
输出：人类可读 + --json
退出码：
  默认：0（只报告当前证据）
  --strict：任一 evidence gate 未关闭返回 1
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

PROVED_BY_STATIC_FILES = "PROVED_BY_STATIC_FILES"
PROVED_BY_RUNTIME_EVIDENCE = "PROVED_BY_RUNTIME_EVIDENCE"
MISSING_OR_NEEDS_EXTERNAL_STATE = "MISSING_OR_NEEDS_EXTERNAL_STATE"
NEEDS_VALIDATOR = "NEEDS_VALIDATOR"

WAVE6_TOOLING_FILES = [
    "docs/runbooks/wave-1-runtime-evidence.md",
    "docs/runbooks/wave-2-runtime-evidence.md",
    "docs/runbooks/wave-3-pda-readiness.md",
    "docs/runbooks/wave-4-external-dependencies.md",
    "docs/runbooks/wave-5-hardware-evidence.md",
    "docs/runbooks/wave-5-tms-evidence.md",
    "docs/runbooks/wave-6-deploy-evidence.md",
    "docs/runbooks/wave-6-closeout.md",
    "scripts/governance/validate_wave1_runtime_evidence.py",
    "scripts/governance/record_wave2_runtime_evidence.py",
    "scripts/governance/validate_wave3_pda_runtime_evidence.py",
    "scripts/governance/record_wave3_pda_runtime_evidence.py",
    "scripts/governance/record_wave4_external_dependencies.py",
    "scripts/governance/validate_wave4_external_dependencies.py",
    "scripts/governance/record_wave5_hardware_evidence.py",
    "scripts/governance/validate_wave5_hardware_evidence.py",
    "scripts/governance/record_wave5_tms_evidence.py",
    "scripts/governance/validate_wave5_tms_evidence.py",
    "scripts/governance/record_wave6_deploy_evidence.py",
    "scripts/governance/validate_wave6_deploy_evidence.py",
]

WAVE6_JUST_ENTRIES = [
    "wave-1-runtime-evidence-validate",
    "wave-1-h2-runtime-evidence",
    "wave-1-rollback-runtime-evidence-k8s",
    "wave-1-rollback-runtime-evidence-compose",
    "wave-2-runtime-evidence-record",
    "wave-2-runtime-evidence-validate",
    "wave-3-pda-runtime-evidence-record",
    "wave-3-pda-runtime-evidence-validate",
    "wave-4-external-dependencies-record",
    "wave-4-external-dependencies-validate",
    "wave-5-hardware-evidence-record",
    "wave-5-hardware-evidence-validate",
    "wave-5-tms-evidence-record",
    "wave-5-tms-evidence-validate",
    "wave-6-deploy-evidence-record",
    "wave-6-deploy-evidence-validate",
    "wave-6-status",
    "wave-6-complete-check",
]


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


def run_validator(*args: str) -> tuple[bool, str]:
    result = subprocess.run(
        [*args],
        cwd=REPO_ROOT,
        check=False,
        text=True,
        capture_output=True,
    )
    output = "\n".join(
        part.strip() for part in [result.stdout, result.stderr] if part.strip()
    )
    return result.returncode == 0, output or f"exit={result.returncode}"


def wave6_tooling_gaps() -> list[str]:
    gaps: list[str] = []
    missing_files = [path for path in WAVE6_TOOLING_FILES if not file_exists(path)]
    if missing_files:
        gaps.append(f"缺少 Wave 6 tooling 文件: {', '.join(missing_files)}")

    missing_just_entries = [
        entry
        for entry in WAVE6_JUST_ENTRIES
        if not file_contains("justfile", entry)
    ]
    if missing_just_entries:
        gaps.append(f"justfile 缺少 Wave 6 evidence 入口: {', '.join(missing_just_entries)}")

    closeout_needles = (
        "just wave-6-complete-check",
        "docs/retros/wave-6-retro.md",
        "Wave 6 完成需要以下全部条件成立",
    )
    if not file_contains("docs/runbooks/wave-6-closeout.md", *closeout_needles):
        gaps.append("Wave 6 closeout runbook 缺少最终关闭命令或 retro 要求")

    return gaps


def collect_items() -> list[EvidenceItem]:
    items: list[EvidenceItem] = []

    startup_ok = (
        file_contains("TODO.md", "当前 Wave：Wave 6", "W6.A", "W6.H")
        and file_contains("ROADMAP.md", "Wave 6：预发布证据与外部依赖收口")
        and file_contains("docs/architecture-dependencies.md", "Wave 6：预发布证据与外部依赖收口")
        and file_contains("docs/adr/0035-wave-6-pre-release-evidence-closeout.md", "ADR-0035")
    )
    items.append(EvidenceItem(
        "W6-startup",
        "Wave 6 范围、TODO、依赖图与 ADR 已启动",
        PROVED_BY_STATIC_FILES if startup_ok else MISSING_OR_NEEDS_EXTERNAL_STATE,
        [
            "TODO.md",
            "ROADMAP.md",
            "docs/architecture-dependencies.md",
            "docs/adr/0035-wave-6-pre-release-evidence-closeout.md",
        ] if startup_ok else [],
        [] if startup_ok else ["需要同步 TODO / ROADMAP / dependency graph / ADR-0035"],
        strict_blocking=True,
    ))

    tooling_gaps = wave6_tooling_gaps()
    items.append(EvidenceItem(
        "W6-tooling",
        "Wave 6 evidence record / validate / closeout 工具链齐备",
        PROVED_BY_STATIC_FILES if not tooling_gaps else MISSING_OR_NEEDS_EXTERNAL_STATE,
        WAVE6_TOOLING_FILES + ["justfile"] if not tooling_gaps else [],
        tooling_gaps,
        strict_blocking=True,
    ))

    wave5_closeout_ok = (
        file_contains("TODO.md", "已归档：Wave 5", "Wave 5 开发完成")
        and file_contains("docs/retros/wave-5-retro.md", "Wave 5 开发完成")
        and file_contains("scripts/governance/report_wave5_completion.py", "W5-chain-scenario")
    )
    items.append(EvidenceItem(
        "W6-wave5-closeout",
        "Wave 5 closeout 已归档，Wave 6 不在未关闭 Wave 5 上继续叠加",
        PROVED_BY_STATIC_FILES if wave5_closeout_ok else MISSING_OR_NEEDS_EXTERNAL_STATE,
        ["TODO.md", "docs/retros/wave-5-retro.md", "scripts/governance/report_wave5_completion.py"] if wave5_closeout_ok else [],
        [] if wave5_closeout_ok else ["需要补 Wave 5 retro / TODO 归档 / completion report"],
        strict_blocking=True,
    ))

    w1_ok, w1_output = run_validator(
        "python3",
        "scripts/governance/validate_wave1_runtime_evidence.py",
        "--kind",
        "all",
    )
    items.append(EvidenceItem(
        "W6.AB-wave1-runtime",
        "Wave 1 H2 压测封档 + W1.D 自动回滚真实 runtime evidence",
        PROVED_BY_RUNTIME_EVIDENCE if w1_ok else MISSING_OR_NEEDS_EXTERNAL_STATE,
        [
            "docs/retros/wave-1-h2-runtime-evidence.json",
            "docs/retros/wave-1-runtime-evidence.json",
            "just wave-1-runtime-evidence-validate",
        ] if w1_ok else [],
        [] if w1_ok else [w1_output],
    ))

    w2_ok, w2_output = run_validator(
        "python3",
        "scripts/governance/report_wave2_completion.py",
        "--strict",
        "--require-runtime-evidence",
    )
    items.append(EvidenceItem(
        "W6.C-wave2-runtime",
        "Wave 2 配置中心 Feature Flag 真实 dev/staging runtime evidence",
        PROVED_BY_RUNTIME_EVIDENCE if w2_ok else MISSING_OR_NEEDS_EXTERNAL_STATE,
        [
            "docs/retros/wave-2-runtime-evidence.json",
            "just wave-2-runtime-evidence-validate",
        ] if w2_ok else [],
        [] if w2_ok else [w2_output],
    ))

    w4_ok, w4_output = run_validator(
        "python3",
        "scripts/governance/validate_wave4_external_dependencies.py",
    )
    items.append(EvidenceItem(
        "W6.E-wave4-traceability-external",
        "Wave 4 M-TC “码上放心”真实 dev/staging 外部 evidence",
        PROVED_BY_RUNTIME_EVIDENCE if w4_ok else MISSING_OR_NEEDS_EXTERNAL_STATE,
        [
            "docs/retros/wave-4-external-dependencies.json",
            "just wave-4-external-dependencies-validate",
        ] if w4_ok else [],
        [] if w4_ok else [w4_output],
    ))

    pda_validator_ok = file_exists("scripts/governance/validate_wave3_pda_runtime_evidence.py")
    if pda_validator_ok:
        pda_evidence_ok, pda_output = run_validator(
            "python3",
            "scripts/governance/validate_wave3_pda_runtime_evidence.py",
        )
        pda_status = (
            PROVED_BY_RUNTIME_EVIDENCE
            if pda_evidence_ok
            else MISSING_OR_NEEDS_EXTERNAL_STATE
        )
        pda_gaps = [] if pda_evidence_ok else [pda_output]
    else:
        pda_evidence_ok = False
        pda_status = NEEDS_VALIDATOR
        pda_gaps = ["缺少 Wave 3 真 PDA/L7 evidence validator 或真实 evidence 文件"]
    items.append(EvidenceItem(
        "W6.D-wave3-pda-l7",
        "Wave 3 真 PDA + L7 性能 / 易用性 runtime evidence",
        pda_status,
        ["docs/retros/wave-3-pda-runtime-evidence.json"] if pda_validator_ok and pda_evidence_ok else [],
        pda_gaps,
    ))

    hardware_validator_ok = file_exists("scripts/governance/validate_wave5_hardware_evidence.py")
    if hardware_validator_ok:
        hardware_evidence_ok, hardware_output = run_validator(
            "python3",
            "scripts/governance/validate_wave5_hardware_evidence.py",
        )
        hardware_status = (
            PROVED_BY_RUNTIME_EVIDENCE
            if hardware_evidence_ok
            else MISSING_OR_NEEDS_EXTERNAL_STATE
        )
        hardware_gaps = [] if hardware_evidence_ok else [hardware_output]
    else:
        hardware_evidence_ok = False
        hardware_status = NEEDS_VALIDATOR
        hardware_gaps = ["缺少 Wave 5 hardware evidence runbook / validator / 真实 evidence"]
    items.append(EvidenceItem(
        "W6.F-wave5-hardware",
        "Wave 5 M-PK 电子秤 / 蓝牙打印机 / 面单打印真实硬件 evidence",
        hardware_status,
        ["docs/retros/wave-5-hardware-evidence.json"] if hardware_validator_ok and hardware_evidence_ok else [],
        hardware_gaps,
    ))

    tms_validator_ok = file_exists("scripts/governance/validate_wave5_tms_evidence.py")
    if tms_validator_ok:
        tms_evidence_ok, tms_output = run_validator(
            "python3",
            "scripts/governance/validate_wave5_tms_evidence.py",
        )
        tms_status = (
            PROVED_BY_RUNTIME_EVIDENCE
            if tms_evidence_ok
            else MISSING_OR_NEEDS_EXTERNAL_STATE
        )
        tms_gaps = [] if tms_evidence_ok else [tms_output]
    else:
        tms_evidence_ok = False
        tms_status = NEEDS_VALIDATOR
        tms_gaps = ["缺少 Wave 5 TMS evidence runbook / validator / 真实 evidence"]
    items.append(EvidenceItem(
        "W6.G-wave5-tms",
        "Wave 5 M10 TMS+ 真实 dev/staging 推送、回调、失败重试和 audit_event 查询 evidence",
        tms_status,
        ["docs/retros/wave-5-tms-evidence.json"] if tms_validator_ok and tms_evidence_ok else [],
        tms_gaps,
    ))

    deploy_validator_ok = file_exists("scripts/governance/validate_wave6_deploy_evidence.py")
    if deploy_validator_ok:
        deploy_evidence_ok, deploy_output = run_validator(
            "python3",
            "scripts/governance/validate_wave6_deploy_evidence.py",
        )
        deploy_status = (
            PROVED_BY_RUNTIME_EVIDENCE
            if deploy_evidence_ok
            else MISSING_OR_NEEDS_EXTERNAL_STATE
        )
        deploy_gaps = [] if deploy_evidence_ok else [deploy_output]
    else:
        deploy_evidence_ok = False
        deploy_status = NEEDS_VALIDATOR
        deploy_gaps = ["缺少 Wave 6 灰度发布 evidence validator 或真实 evidence 文件"]
    items.append(EvidenceItem(
        "W6.H-gray-release",
        "首次试运行投产按 ADR-0016 灰度发布链路执行",
        deploy_status,
        ["docs/retros/wave-6-deploy-evidence.json"] if deploy_validator_ok and deploy_evidence_ok else [],
        deploy_gaps,
    ))

    return items


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--strict", action="store_true", help="Wave 6 出口检查，阻塞缺口返回非零")
    args = parser.parse_args(argv)

    items = collect_items()
    blocking = [item for item in items if item.blocks_strict]
    ok = not blocking

    if args.json:
        print(json.dumps({
            "report": "wave6_pre_release",
            "tier": "manual",
            "category": "流程治理",
            "items": [asdict(item) for item in items],
            "blocking_gaps": [asdict(item) for item in blocking],
            "ok": ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("report_wave6_pre_release (流程治理，预发布证据收口报告)")
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
