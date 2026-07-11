#!/usr/bin/env python3
"""report_wave6_pre_release.py — Wave 6 预发布证据收口报告。

类别：4. 流程治理（报告型，默认不阻塞）
Tier：手动 / Wave 出口检查
输入：ADR-0035 + TODO.md + 已有 runtime evidence validator
输出：人类可读 + --json
退出码：
  默认：0（只报告当前证据）
  --strict：任一 strict blocking item 未关闭返回 1
  evidence-only 仅忽略 W6-retro；startup/tooling/Wave 5 closeout 等前置项仍阻塞
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

from check_wave6_evidence_preflight import GATES as PREFLIGHT_GATES
from check_wave6_evidence_preflight import REQUIRED_EXECUTION_FILES
from check_wave6_evidence_preflight import REQUIRED_TOP_LEVEL_FILES
from check_wave6_evidence_preflight import WAVE6_CLOSEOUT_JUST_ENTRIES
from check_wave6_evidence_preflight import gate_execution_file_map
from check_wave6_evidence_preflight import gate_just_entries
from check_wave6_evidence_preflight import gate_commands_by_phase
from check_wave6_evidence_preflight import required_runbooks as preflight_required_runbooks
from check_wave6_evidence_preflight import validation_commands as preflight_validation_commands

from _wave6_pre_release_config import *  # noqa: F403


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


def is_wave6_evidence_gate_item(item: EvidenceItem) -> bool:
    return any(item.item_id.startswith(f"{gate_id}-") for gate_id in WAVE6_GATE_IDS)


def item_ids(items: list[EvidenceItem]) -> list[str]:
    return [item.item_id for item in items]


def normalized_gaps(item: EvidenceItem) -> list[str]:
    return [normalize_gap_text(gap) for gap in item.gaps]


def item_dicts(items: list[EvidenceItem]) -> list[dict[str, object]]:
    dicts = [asdict(item) for item in items]
    for item_dict in dicts:
        item_dict["gaps"] = [
            normalize_gap_text(str(gap))
            for gap in item_dict["gaps"]
        ]
    return dicts


def evidence_file_by_gate_id() -> dict[str, str]:
    return {gate.gate_id: gate.evidence_file for gate in PREFLIGHT_GATES}


def evidence_file_for_item(item: EvidenceItem) -> str | None:
    gate_id = item.item_id.split("-", 1)[0]
    return evidence_file_by_gate_id().get(gate_id)


def external_prerequisites_for_gate(gate_id: str) -> list[str]:
    return list(WAVE6_EXTERNAL_PREREQUISITES.get(gate_id, ()))


def minimum_evidence_refs_for_gate(gate_id: str) -> list[str]:
    return list(WAVE6_MINIMUM_EVIDENCE_REFS.get(gate_id, ()))


def record_check_only_commands_for_gate(gate_id: str) -> list[str]:
    return list(WAVE6_RECORD_CHECK_ONLY_COMMANDS.get(gate_id, ()))


def phase_commands_for_gate(gate_id: str, phase: str) -> list[str]:
    commands = gate_commands_by_phase().get(gate_id, {}).get(phase, [])
    replacements = WAVE6_PHASE_COMMAND_REPLACEMENTS.get(gate_id, {}).get(phase, {})
    return [replacements.get(command, command) for command in commands]


def readiness_commands_for_gate(gate_id: str) -> list[str]:
    return list(dict.fromkeys([
        *WAVE6_EXPORT_TEMPLATE_COMMANDS.get(gate_id, ()),
        *phase_commands_for_gate(gate_id, "readiness"),
    ]))


def record_commands_for_gate(gate_id: str) -> list[str]:
    return phase_commands_for_gate(gate_id, "record")


def evidence_files_for_items(items: list[EvidenceItem]) -> list[str]:
    files: list[str] = []
    for item in items:
        evidence_file = evidence_file_for_item(item)
        if evidence_file is not None:
            files.append(evidence_file)
    return files


def missing_evidence_details_for_items(items: list[EvidenceItem]) -> list[dict[str, object]]:
    details: list[dict[str, object]] = []
    commands_by_phase = gate_commands_by_phase()
    for item in items:
        evidence_file = evidence_file_for_item(item)
        if evidence_file is None:
            continue
        gate_id = item.item_id.split("-", 1)[0]
        detail: dict[str, object] = {
            "gate_id": gate_id,
            "item_id": item.item_id,
            "evidence_file": evidence_file,
            "status": item.status,
            "requirement": item.requirement,
            "gaps": normalized_gaps(item),
            "external_prerequisites": external_prerequisites_for_gate(gate_id),
            "minimum_evidence_refs": minimum_evidence_refs_for_gate(gate_id),
            "readiness_commands": readiness_commands_for_gate(gate_id),
            "record_check_only_commands": record_check_only_commands_for_gate(gate_id),
            "record_commands": record_commands_for_gate(gate_id),
            "validate_commands": commands_by_phase.get(gate_id, {}).get("validate", []),
        }
        detail.update(deployment_choice_metadata_for_detail(detail))
        details.append(detail)
    return details


def report_details_for_items(items: list[EvidenceItem]) -> list[dict[str, object]]:
    details: list[dict[str, object]] = []
    for item in items:
        is_evidence = is_wave6_evidence_gate_item(item)
        details.append({
            "kind": "evidence" if is_evidence else "non_evidence",
            "gate_id": item.item_id.split("-", 1)[0] if is_evidence else None,
            "item_id": item.item_id,
            "status": item.status,
            "requirement": item.requirement,
            "gaps": normalized_gaps(item),
        })
    return details


def command_details_for_item(item: EvidenceItem) -> dict[str, list[str]]:
    if not is_wave6_evidence_gate_item(item):
        return {"readiness": [], "record_check_only": [], "record": [], "validate": []}
    gate_id = item.item_id.split("-", 1)[0]
    commands = gate_commands_by_phase().get(gate_id, {})
    return {
        "readiness": readiness_commands_for_gate(gate_id),
        "record_check_only": record_check_only_commands_for_gate(gate_id),
        "record": record_commands_for_gate(gate_id),
        "validate": commands.get("validate", []),
    }


def command_detail_for_item(item: EvidenceItem) -> dict[str, object] | None:
    if not is_wave6_evidence_gate_item(item):
        return None
    evidence_file = evidence_file_for_item(item)
    if evidence_file is None:
        return None
    commands = command_details_for_item(item)
    gate_id = item.item_id.split("-", 1)[0]
    return {
        "gate_id": gate_id,
        "item_id": item.item_id,
        "evidence_file": evidence_file,
        "status": item.status,
        "requirement": item.requirement,
        "gaps": normalized_gaps(item),
        "external_prerequisites": external_prerequisites_for_gate(gate_id),
        "minimum_evidence_refs": minimum_evidence_refs_for_gate(gate_id),
        "readiness_commands": commands["readiness"],
        "record_check_only_commands": commands["record_check_only"],
        "record_commands": commands["record"],
        "validate_commands": commands["validate"],
    }


def choice_line_for_gate_id(gate_id: str) -> str | None:
    if gate_id == "W6.B":
        return W6B_ROLLBACK_DEPLOYMENT_CHOICE_LINE
    return None


def commands_only_choice_line_for_detail(detail: dict[str, object]) -> str | None:
    return choice_line_for_gate_id(str(detail["gate_id"]))


def text_report_choice_line_for_item(item: EvidenceItem) -> str | None:
    if not is_wave6_evidence_gate_item(item):
        return None
    return choice_line_for_gate_id(item.item_id.split("-", 1)[0])


def w6b_rollback_path_commands(
    detail: dict[str, object],
    suffix: str,
) -> tuple[list[str], list[str]]:
    readiness_commands = [
        str(command)
        for command in detail["readiness_commands"]
        if str(command).endswith(suffix)
    ]
    record_commands = [
        str(command)
        for command in detail["record_commands"]
        if str(command).endswith(suffix)
    ]
    return readiness_commands, record_commands


def deployment_choice_metadata_for_detail(detail: dict[str, object]) -> dict[str, object]:
    if str(detail["gate_id"]) != "W6.B":
        return {}

    path_commands: list[dict[str, object]] = []
    for path_label, command_suffix in (
        ("k8s", "-k8s"),
        ("docker-compose", "-compose"),
    ):
        readiness_commands, record_commands = w6b_rollback_path_commands(
            detail,
            command_suffix,
        )
        if not readiness_commands and not record_commands:
            continue
        path_commands.append({
            "path": path_label,
            "readiness_commands": readiness_commands,
            "record_commands": record_commands,
        })

    return {
        "deployment_choice_required": True,
        "deployment_choice_label": W6B_ROLLBACK_DEPLOYMENT_CHOICE_LABEL,
        "deployment_choice_options": list(W6B_ROLLBACK_DEPLOYMENT_OPTIONS),
        "deployment_path_commands": path_commands,
    }


def commands_only_phase_lines_for_detail(detail: dict[str, object]) -> list[str]:
    gate_id = str(detail["gate_id"])
    if gate_id == "W6.H":
        readiness_commands = [str(command) for command in detail["readiness_commands"]]
        record_check_only_commands = [
            str(command) for command in detail.get("record_check_only_commands", [])
        ]
        record_commands = [str(command) for command in detail["record_commands"]]
        return [
            *(f"readiness: {command}" for command in readiness_commands[:2]),
            *(f"record-check-only: {command}" for command in record_check_only_commands[:1]),
            *(f"record: {command}" for command in record_commands[:1]),
            *(f"readiness: {command}" for command in readiness_commands[2:]),
            *(f"record-check-only: {command}" for command in record_check_only_commands[1:]),
            *(f"record: {command}" for command in record_commands[1:]),
            *(f"validate: {command}" for command in detail["validate_commands"]),
        ]

    if gate_id != "W6.B":
        return [
            *(f"readiness: {command}" for command in detail["readiness_commands"]),
            *(
                f"record-check-only: {command}"
                for command in detail.get("record_check_only_commands", [])
            ),
            *(f"record: {command}" for command in detail["record_commands"]),
            *(f"validate: {command}" for command in detail["validate_commands"]),
        ]

    lines: list[str] = []
    for path_label, command_suffix in (
        ("k8s", "-k8s"),
        ("docker-compose", "-compose"),
    ):
        readiness_commands, record_commands = w6b_rollback_path_commands(
            detail,
            command_suffix,
        )
        if not readiness_commands and not record_commands:
            continue
        lines.append(f"path: {path_label}")
        lines.extend(f"readiness: {command}" for command in readiness_commands)
        lines.extend(f"record: {command}" for command in record_commands)
    lines.extend(f"validate: {command}" for command in detail["validate_commands"])
    return lines


def print_missing_evidence_commands(
    details: list[dict[str, object]],
    *,
    has_non_evidence_blockers: bool = False,
) -> None:
    """Print a compact command checklist for missing Wave 6 evidence gates."""
    if not details:
        print(COMMANDS_ONLY_NONE_LINE)
        if has_non_evidence_blockers:
            print(COMMANDS_ONLY_NONE_COMPLETE_MODE_LINE)
        return

    print(COMMANDS_ONLY_BOUNDARY_LINE)
    print(COMMANDS_ONLY_STRICT_EXIT_LINE)
    for detail in details:
        gate_id = str(detail["gate_id"])
        item_id = str(detail["item_id"])
        print(f"# {gate_id} {item_id}")
        print(f"evidence: {detail['evidence_file']}")
        choice_line = commands_only_choice_line_for_detail(detail)
        if choice_line is not None:
            print(choice_line)
        for prerequisite in detail["external_prerequisites"]:
            print(f"external-prereq: {prerequisite}")
        for evidence_ref in detail["minimum_evidence_refs"]:
            print(f"minimum-evidence-ref: {evidence_ref}")
        for line in commands_only_phase_lines_for_detail(detail):
            print(line)


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
    return result.returncode == 0, normalize_gap_text(output or f"exit={result.returncode}")


def normalize_gap_text(text: str) -> str:
    """Keep report gaps portable by rendering repo paths as relative paths."""
    repo_root = str(REPO_ROOT)
    return text.replace(f"{repo_root}/", "").replace(repo_root, ".")


def summarize_json_gap_output(output: str) -> list[str]:
    try:
        payload = json.loads(output)
    except json.JSONDecodeError:
        return [normalize_gap_text(output)]

    if not isinstance(payload, dict):
        return [normalize_gap_text(output)]

    gaps: list[str] = []
    seen: set[str] = set()

    def append_gap(value: str) -> None:
        if value in seen:
            return
        seen.add(value)
        gaps.append(normalize_gap_text(value))

    for bucket in ("runtime_blocking_gaps", "blocking_gaps", "pre_release_gates"):
        items = payload.get(bucket)
        if not isinstance(items, list):
            continue
        for item in items:
            if not isinstance(item, dict):
                continue
            item_id = str(item.get("item_id") or "?")
            item_gaps = item.get("gaps")
            if isinstance(item_gaps, list) and item_gaps:
                for gap in item_gaps:
                    append_gap(f"{item_id}: {gap}")
            elif item.get("requirement"):
                append_gap(f"{item_id}: {item['requirement']}")

    return gaps or [normalize_gap_text(output)]


def wave6_tooling_gaps() -> list[str]:
    gaps: list[str] = []
    preflight_gate_ids = [gate.gate_id for gate in PREFLIGHT_GATES]
    if WAVE6_GATE_IDS != preflight_gate_ids:
        gaps.append(
            "Wave 6 report gate 清单与 preflight GATES 不一致: "
            f"report={', '.join(WAVE6_GATE_IDS)}; preflight={', '.join(preflight_gate_ids)}"
        )

    preflight_evidence_files = [gate.evidence_file for gate in PREFLIGHT_GATES]
    if WAVE6_EVIDENCE_FILES != preflight_evidence_files:
        gaps.append(
            "Wave 6 report evidence 文件清单与 preflight GATES 不一致: "
            f"report={', '.join(WAVE6_EVIDENCE_FILES)}; "
            f"preflight={', '.join(preflight_evidence_files)}"
        )

    preflight_runbooks = list(dict.fromkeys(gate.runbook for gate in PREFLIGHT_GATES))
    missing_tooling_docs = [
        runbook for runbook in preflight_runbooks if runbook not in WAVE6_TOOLING_DOCS
    ]
    if missing_tooling_docs:
        gaps.append(
            "Wave 6 tooling 文档清单未覆盖 preflight GATES runbook: "
            f"{', '.join(missing_tooling_docs)}"
        )

    preflight_just_entries = [
        entry for gate in PREFLIGHT_GATES for entry in gate.just_entries
    ]
    expected_just_entries = [
        *preflight_just_entries,
        "wave-6-evidence-preflight",
        *WAVE6_CLOSEOUT_JUST_ENTRIES,
    ]
    missing_report_just_entries = [
        entry for entry in expected_just_entries if entry not in WAVE6_JUST_ENTRIES
    ]
    if missing_report_just_entries:
        gaps.append(
            "Wave 6 report just 入口清单未覆盖 preflight GATES / closeout 入口: "
            f"{', '.join(missing_report_just_entries)}"
        )

    preflight_validation_commands: list[str] = []
    for entry in preflight_just_entries:
        if not entry.endswith("-validate"):
            continue
        command = f"just {entry}"
        if command not in preflight_validation_commands:
            preflight_validation_commands.append(command)
    missing_validation_commands = [
        command
        for command in preflight_validation_commands
        if command not in WAVE6_VALIDATION_COMMANDS
    ]
    if missing_validation_commands:
        gaps.append(
            "Wave 6 retro 验证命令清单未覆盖 preflight validator 入口: "
            f"{', '.join(missing_validation_commands)}"
        )

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
        "just wave-6-evidence-preflight",
        "just wave-6-complete-check",
        "docs/retros/wave-6-retro.md",
        "Wave 6 完成需要以下全部条件成立",
    )
    if not file_contains("docs/runbooks/wave-6-closeout.md", *closeout_needles):
        gaps.append("Wave 6 closeout runbook 缺少最终关闭命令或 retro 要求")

    preflight_ok, preflight_output = run_validator(*WAVE6_PREFLIGHT_COMMAND)
    if not preflight_ok:
        gaps.append(f"Wave 6 evidence preflight 未通过: {preflight_output}")

    return gaps


def wave6_startup_gaps() -> list[str]:
    gaps: list[str] = []
    for path, required_needles in WAVE6_STARTUP_DOC_REQUIREMENTS.items():
        text = read_text(path)
        if not text:
            gaps.append(f"缺少 Wave 6 范围文档: {path}")
            continue
        missing = [
            needle
            for needle in (*required_needles, *WAVE6_GATE_IDS)
            if needle not in text
        ]
        if missing:
            gaps.append(f"{path} 缺少 Wave 6 启动登记内容: {', '.join(missing)}")
    return gaps


def wave6_retro_gaps() -> list[str]:
    if not file_exists(WAVE6_RETRO_FILE):
        return [f"缺少 {WAVE6_RETRO_FILE} Wave 6 收口回顾"]

    required_needles = [
        *WAVE6_GATE_IDS,
        *WAVE6_EVIDENCE_FILES,
        *WAVE6_VALIDATION_COMMANDS,
        "验证结果",
        "剩余风险",
        WAVE6_FORBIDDEN_EVIDENCE_BOUNDARY_STATEMENT,
    ]
    missing = [
        needle
        for needle in required_needles
        if not file_contains(WAVE6_RETRO_FILE, needle)
    ]
    if missing:
        return [f"{WAVE6_RETRO_FILE} 缺少必需收口内容: {', '.join(missing)}"]
    return []


def collect_items() -> list[EvidenceItem]:
    items: list[EvidenceItem] = []

    startup_gaps = wave6_startup_gaps()
    items.append(EvidenceItem(
        "W6-startup",
        "Wave 6 范围、TODO、依赖图与 ADR 已启动",
        PROVED_BY_STATIC_FILES if not startup_gaps else MISSING_OR_NEEDS_EXTERNAL_STATE,
        [
            "TODO.md",
            "ROADMAP.md",
            "docs/architecture-dependencies.md",
            "docs/adr/0035-wave-6-pre-release-evidence-closeout.md",
        ] if not startup_gaps else [],
        startup_gaps,
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

    w1_h2_ok, w1_h2_output = run_validator(
        "python3",
        "scripts/governance/validate_wave1_runtime_evidence.py",
        "--kind",
        "h2",
    )
    items.append(EvidenceItem(
        "W6.A-wave1-h2-runtime",
        "Wave 1 H2 压测封档真实 runtime evidence",
        PROVED_BY_RUNTIME_EVIDENCE if w1_h2_ok else MISSING_OR_NEEDS_EXTERNAL_STATE,
        [
            "docs/retros/wave-1-h2-runtime-evidence.json",
            "just wave-1-runtime-evidence-validate",
        ] if w1_h2_ok else [],
        [] if w1_h2_ok else [w1_h2_output],
    ))

    w1d_ok, w1d_output = run_validator(
        "python3",
        "scripts/governance/validate_wave1_runtime_evidence.py",
        "--kind",
        "w1d",
    )
    items.append(EvidenceItem(
        "W6.B-wave1-rollback-runtime",
        "Wave 1 W1.D 自动回滚真实 runtime evidence",
        PROVED_BY_RUNTIME_EVIDENCE if w1d_ok else MISSING_OR_NEEDS_EXTERNAL_STATE,
        [
            "docs/retros/wave-1-runtime-evidence.json",
            "just wave-1-runtime-evidence-validate",
        ] if w1d_ok else [],
        [] if w1d_ok else [w1d_output],
    ))

    w2_ok, w2_output = run_validator(
        "python3",
        "scripts/governance/report_wave2_completion.py",
        "--strict",
        "--require-runtime-evidence",
        "--json",
    )
    items.append(EvidenceItem(
        "W6.C-wave2-runtime",
        "Wave 2 配置中心 Feature Flag 真实 dev/staging runtime evidence",
        PROVED_BY_RUNTIME_EVIDENCE if w2_ok else MISSING_OR_NEEDS_EXTERNAL_STATE,
        [
            "docs/retros/wave-2-runtime-evidence.json",
            "just wave-2-runtime-evidence-validate",
        ] if w2_ok else [],
        [] if w2_ok else summarize_json_gap_output(w2_output),
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

    retro_gaps = wave6_retro_gaps()
    items.append(EvidenceItem(
        WAVE6_RETRO_ITEM_ID,
        "Wave 6 收口回顾记录 8 个 gate 的真实 evidence、验证结果和剩余风险",
        PROVED_BY_STATIC_FILES if not retro_gaps else MISSING_OR_NEEDS_EXTERNAL_STATE,
        [WAVE6_RETRO_FILE] if not retro_gaps else [],
        retro_gaps,
        strict_blocking=True,
    ))

    return items


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--strict", action="store_true", help="Wave 6 出口检查，阻塞缺口返回非零")
    parser.add_argument(
        "--evidence-only",
        action="store_true",
        help=(
            "写 retro 前检查模式：忽略 W6-retro；"
            "startup/tooling/Wave 5 closeout 与 8 个真实 evidence gate 仍阻塞"
        ),
    )
    parser.add_argument(
        "--commands-only",
        action="store_true",
        help="只输出缺失 evidence gate 的 readiness / record / validate 命令清单",
    )
    args = parser.parse_args(argv)
    if args.commands_only and args.json:
        print("--commands-only cannot be combined with --json", file=sys.stderr)
        return 2

    items = collect_items()
    ignored_for_mode_item_ids = {WAVE6_RETRO_ITEM_ID} if args.evidence_only else set()
    blocking = [
        item
        for item in items
        if item.blocks_strict and item.item_id not in ignored_for_mode_item_ids
    ]
    ignored = [
        item
        for item in items
        if item.blocks_strict and item.item_id in ignored_for_mode_item_ids
    ]
    evidence_blocking = [item for item in blocking if is_wave6_evidence_gate_item(item)]
    evidence_ignored = [item for item in ignored if is_wave6_evidence_gate_item(item)]
    evidence_gate_items = [
        item for item in items if is_wave6_evidence_gate_item(item)
    ]
    non_evidence_items = [
        item for item in items if not is_wave6_evidence_gate_item(item)
    ]
    non_evidence_blocking = [
        item for item in blocking if not is_wave6_evidence_gate_item(item)
    ]
    non_evidence_ignored = [
        item for item in ignored if not is_wave6_evidence_gate_item(item)
    ]
    missing_evidence_files = evidence_files_for_items(evidence_blocking)
    missing_evidence_details = missing_evidence_details_for_items(evidence_blocking)
    ok = not blocking

    if args.commands_only:
        print_missing_evidence_commands(
            missing_evidence_details,
            has_non_evidence_blockers=bool(non_evidence_blocking),
        )
    elif args.json:
        print(json.dumps({
            "script": "report_wave6_pre_release",
            "report": "wave6_pre_release",
            "tier": "manual",
            "category": "流程治理",
            "schema_version": SCHEMA_VERSION,
            "mode": "evidence-only" if args.evidence_only else "complete",
            "available_modes": list(REPORT_MODES),
            "writes_runtime_evidence": False,
            "closes_gate": False,
            "report_command": REPORT_COMMAND,
            "evidence_only_command": (
                "python3 scripts/governance/report_wave6_pre_release.py "
                "--strict --evidence-only --json"
            ),
            "commands_only_command": COMMANDS_ONLY_COMMAND,
            "items": item_dicts(items),
            "evidence_gate_count": len(WAVE6_GATE_IDS),
            "evidence_gate_ids": WAVE6_GATE_IDS,
            "evidence_gate_evidence_files": WAVE6_EVIDENCE_FILES,
            "evidence_gate_just_entries": gate_just_entries(),
            "evidence_gate_execution_files": gate_execution_file_map(),
            "required_top_level_files": list(REQUIRED_TOP_LEVEL_FILES),
            "required_runbooks": preflight_required_runbooks(),
            "required_execution_files": list(REQUIRED_EXECUTION_FILES),
            "validation_commands": WAVE6_VALIDATION_COMMANDS,
            "closeout_just_entries": list(WAVE6_CLOSEOUT_JUST_ENTRIES),
            "evidence_gate_item_ids": item_ids(evidence_gate_items),
            "evidence_gate_items": item_dicts(evidence_gate_items),
            "non_evidence_item_ids": item_ids(non_evidence_items),
            "non_evidence_items": item_dicts(non_evidence_items),
            "evidence_blocking_count": len(evidence_blocking),
            "evidence_blocking_item_ids": item_ids(evidence_blocking),
            "evidence_blocking_gaps": item_dicts(evidence_blocking),
            "missing_evidence_count": len(missing_evidence_files),
            "missing_evidence_item_ids": item_ids(evidence_blocking),
            "missing_evidence_files": missing_evidence_files,
            "missing_evidence_details": missing_evidence_details,
            "evidence_ignored_count": len(evidence_ignored),
            "evidence_ignored_item_ids": item_ids(evidence_ignored),
            "evidence_ignored_gaps": item_dicts(evidence_ignored),
            "blocking_count": len(blocking),
            "blocking_item_ids": item_ids(blocking),
            "blocking_details": report_details_for_items(blocking),
            "blocking_gaps": item_dicts(blocking),
            "non_evidence_blocking_item_ids": item_ids(non_evidence_blocking),
            "non_evidence_blocking_gaps": item_dicts(non_evidence_blocking),
            "ignored_count": len(ignored),
            "ignored_item_ids": item_ids(ignored),
            "ignored_details": report_details_for_items(ignored),
            "ignored_gaps": item_dicts(ignored),
            "non_evidence_ignored_item_ids": item_ids(non_evidence_ignored),
            "non_evidence_ignored_gaps": item_dicts(non_evidence_ignored),
            "ok": ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("report_wave6_pre_release (流程治理，预发布证据收口报告)")
        for item in items:
            ignored_in_mode = item.item_id in ignored_for_mode_item_ids and item.blocks_strict
            mark = "!" if ignored_in_mode else ("✓" if item.complete else "✘")
            print(f"  {mark} {item.item_id}: {item.requirement}")
            status = f"{item.status} (evidence-only 不阻塞)" if ignored_in_mode else item.status
            print(f"    status: {status}")
            for evidence in item.evidence:
                print(f"    evidence: {evidence}")
            if ignored_in_mode:
                print("    ignored: 写 retro 前只检查真实 evidence gate；最终 complete-check 仍要求本项")
            else:
                for gap in item.gaps:
                    print(f"    gap: {gap}")
                if item.blocks_strict and is_wave6_evidence_gate_item(item):
                    choice_line = text_report_choice_line_for_item(item)
                    if choice_line is not None:
                        print(f"    {choice_line}")
                    command_detail = command_detail_for_item(item)
                    if command_detail is not None:
                        for prerequisite in command_detail["external_prerequisites"]:
                            print(f"    external-prereq: {prerequisite}")
                        for evidence_ref in command_detail["minimum_evidence_refs"]:
                            print(f"    minimum-evidence-ref: {evidence_ref}")
                        for line in commands_only_phase_lines_for_detail(command_detail):
                            print(f"    {line}")
        if blocking:
            print(f"\n阻塞缺口: {len(blocking)}")

    return 1 if args.strict and blocking else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
