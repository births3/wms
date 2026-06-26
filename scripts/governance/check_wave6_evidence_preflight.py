#!/usr/bin/env python3
"""check_wave6_evidence_preflight.py — Wave 6 evidence preflight 静态检查。

类别：4. 流程治理
Tier：T1
输入：Wave 6 runbook / justfile / validator / record 脚本
输出：人类可读 + --json

本脚本只检查证据收口链路是否完整，不连接 dev/staging、硬件或外部系统，
也不会生成 runtime evidence。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

PREFLIGHT_DOC = "docs/runbooks/wave-6-evidence-preflight.md"
CLOSEOUT_DOC = "docs/runbooks/wave-6-closeout.md"
TODO_DOC = "TODO.md"
JUSTFILE = "justfile"
REQUIRED_BOUNDARY_TERMS = ("environment", "dev", "staging")
FORBIDDEN_BOUNDARY_TERMS = ("local", "prod", "production", "mock", "fake", "stub", "example")
TEMPLATE_PLACEHOLDER_TERMS = ("YYYY", "<...>", "TODO", "TBD", "待填", "待确认")
PLACEHOLDER_VALUE_TOKENS = ("yyyy", "todo", "tbd", "待填", "待确认")
REFERENCE_LINE_HINTS = (
    "_ref",
    "-ref",
    "ref\"",
    "ref'",
    "_url",
    "-url",
    "url\"",
    "url'",
    "_file",
    "-file",
    "_path",
    "-path",
    "_output",
    "-output",
    "s3://",
    "http://",
    "https://",
    "asset://",
    ".json",
    ".log",
    ".pdf",
    ".md",
)
HARDCODED_RECORD_REF_VALUE_HINTS = (
    "ci/",
    "docs/",
    "s3://",
    "asset://",
    "registry://",
    "grafana/",
    "gitlab/",
    "ticket://",
    "vault://",
    "postgres://",
    "http://",
    "https://",
    "/tmp/",
    "/srv/",
)
RECORD_REF_ARG_RE = re.compile(
    r"(?P<flag>--[a-z0-9-]*(?:-ref|-url|-file|-path|-output))\s+"
    r"(?:(?P<quote>['\"])(?P<quoted_value>[^'\"]+)(?P=quote)|(?P<bare_value>\S+))"
)
EXPORT_REF_RE = re.compile(
    r"\bexport\s+(?P<name>[A-Z0-9_]*(?:REF|URL|FILE|PATH|OUTPUT))\s*=\s*"
    r"(?:(?P<quote>['\"])(?P<quoted_value>[^'\"]+)(?P=quote)|(?P<bare_value>\S+))"
)
JUST_ENTRY_EXECUTION_FILE_OVERRIDES = {
    "wave-1-runtime-prereq-h2": "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
    "wave-1-h2-runtime-readiness": "scripts/governance/check_wave1_h2_runtime_readiness.py",
    "wave-1-h2-runtime-evidence": "scripts/governance/collect_wave1_h2_runtime_evidence.py",
    "wave-1-runtime-prereq-rollback-k8s": "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
    "wave-1-rollback-runtime-readiness-k8s": "deploy/scripts/wave1_auto_rollback_probe.sh",
    "wave-1-rollback-runtime-evidence-k8s": "deploy/scripts/wave1_auto_rollback_probe.sh",
    "wave-1-runtime-prereq-rollback-compose": "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
    "wave-1-rollback-runtime-readiness-compose": "deploy/scripts/wave1_auto_rollback_probe.sh",
    "wave-1-rollback-runtime-evidence-compose": "deploy/scripts/wave1_auto_rollback_probe.sh",
    "wave-2-runtime-evidence-validate": "scripts/governance/report_wave2_completion.py",
    "wave-2-runtime-evidence-readiness": "scripts/governance/collect_wave2_runtime_evidence.py",
    "wave-2-runtime-evidence-smoke": "scripts/governance/collect_wave2_runtime_evidence.py",
    "wave-2-runtime-evidence-collect": "scripts/governance/collect_wave2_runtime_evidence.py",
    "wave-3-pda-preaudit-kit": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-materials-checklist": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-field-work-request": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-field-execution-summary": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-field-precheck-summary": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-field-owner-gap-actions": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-field-handoff-bundle": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-service-precheck": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-trace-code-openapi-precheck": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-runtime-readiness": "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "wave-3-pda-evidence-package-template": "scripts/governance/record_wave3_pda_runtime_evidence.py",
    "wave-3-pda-intake-template": "scripts/governance/record_wave3_pda_runtime_evidence.py",
    "wave-3-pda-intake-check": "scripts/governance/record_wave3_pda_runtime_evidence.py",
    "wave-3-pda-intake-record": "scripts/governance/record_wave3_pda_runtime_evidence.py",
    "wave-4-external-dependencies-readiness": "scripts/governance/check_wave4_external_dependencies_readiness.py",
    "wave-5-hardware-materials": "scripts/governance/record_wave5_hardware_evidence.py",
    "wave-5-hardware-readiness": "scripts/governance/record_wave5_hardware_evidence.py",
    "wave-5-tms-materials": "scripts/governance/record_wave5_tms_evidence.py",
    "wave-5-tms-readiness": "scripts/governance/record_wave5_tms_evidence.py",
    "wave-6-deploy-materials": "scripts/governance/report_wave6_deploy_materials.py",
    "wave-6-deploy-readiness": "scripts/governance/check_wave6_deploy_readiness.py",
}


@dataclass(frozen=True)
class GateSpec:
    gate_id: str
    title: str
    runbook: str
    evidence_file: str
    just_entries: tuple[str, ...]
    required_terms: tuple[str, ...] = REQUIRED_BOUNDARY_TERMS
    forbidden_boundary_terms: tuple[str, ...] = FORBIDDEN_BOUNDARY_TERMS
    template_placeholder_terms: tuple[str, ...] = TEMPLATE_PLACEHOLDER_TERMS


@dataclass
class GateResult:
    gate_id: str
    title: str
    ok: bool
    errors: list[str] = field(default_factory=list)


GATES: tuple[GateSpec, ...] = (
    GateSpec(
        "W6.A",
        "Wave 1 H2 runtime evidence",
        "docs/runbooks/wave-1-runtime-evidence.md",
        "docs/retros/wave-1-h2-runtime-evidence.json",
        (
            "wave-1-runtime-prereq-h2",
            "wave-1-h2-runtime-readiness",
            "wave-1-h2-runtime-evidence",
            "wave-1-runtime-evidence-validate",
        ),
    ),
    GateSpec(
        "W6.B",
        "Wave 1 W1.D rollback evidence",
        "docs/runbooks/wave-1-runtime-evidence.md",
        "docs/retros/wave-1-runtime-evidence.json",
        (
            "wave-1-runtime-prereq-rollback-k8s",
            "wave-1-rollback-runtime-readiness-k8s",
            "wave-1-rollback-runtime-evidence-k8s",
            "wave-1-runtime-prereq-rollback-compose",
            "wave-1-rollback-runtime-readiness-compose",
            "wave-1-rollback-runtime-evidence-compose",
            "wave-1-runtime-evidence-validate",
        ),
    ),
    GateSpec(
        "W6.C",
        "Wave 2 config-center Feature Flag evidence",
        "docs/runbooks/wave-2-runtime-evidence.md",
        "docs/retros/wave-2-runtime-evidence.json",
        (
            "wave-2-runtime-evidence-readiness",
            "wave-2-runtime-evidence-smoke",
            "wave-2-runtime-evidence-record",
            "wave-2-runtime-evidence-validate",
        ),
    ),
    GateSpec(
        "W6.D",
        "Wave 3 real PDA / L7 evidence",
        "docs/runbooks/wave-3-pda-readiness.md",
        "docs/retros/wave-3-pda-runtime-evidence.json",
        (
            "wave-3-pda-preaudit-kit",
            "wave-3-pda-materials-checklist",
            "wave-3-pda-field-work-request",
            "wave-3-pda-field-execution-summary",
            "wave-3-pda-field-precheck-summary",
            "wave-3-pda-field-owner-gap-actions",
            "wave-3-pda-field-handoff-bundle",
            "wave-3-pda-evidence-package-template",
            "wave-3-pda-intake-template",
            "wave-3-pda-intake-check",
            "wave-3-pda-intake-record",
            "wave-3-pda-service-precheck",
            "wave-3-pda-trace-code-openapi-precheck",
            "wave-3-pda-runtime-readiness",
            "wave-3-pda-runtime-evidence-record",
            "wave-3-pda-runtime-evidence-validate",
        ),
        (
            *REQUIRED_BOUNDARY_TERMS,
            "真 PDA",
            "实体扫码键",
            "蓝牙打印",
            "Wave 5",
            "docs/retros/wave-5-hardware-evidence.json",
        ),
    ),
    GateSpec(
        "W6.E",
        "Wave 4 M-TC external evidence",
        "docs/runbooks/wave-4-external-dependencies.md",
        "docs/retros/wave-4-external-dependencies.json",
        (
            "wave-4-external-dependencies-readiness",
            "wave-4-external-dependencies-record",
            "wave-4-external-dependencies-validate",
        ),
    ),
    GateSpec(
        "W6.F",
        "Wave 5 M-PK hardware evidence",
        "docs/runbooks/wave-5-hardware-evidence.md",
        "docs/retros/wave-5-hardware-evidence.json",
        (
            "wave-5-hardware-materials",
            "wave-5-hardware-readiness",
            "wave-5-hardware-evidence-record",
            "wave-5-hardware-evidence-validate",
        ),
    ),
    GateSpec(
        "W6.G",
        "Wave 5 M10 TMS+ evidence",
        "docs/runbooks/wave-5-tms-evidence.md",
        "docs/retros/wave-5-tms-evidence.json",
        (
            "wave-5-tms-materials",
            "wave-5-tms-readiness",
            "wave-5-tms-evidence-record",
            "wave-5-tms-evidence-validate",
        ),
    ),
    GateSpec(
        "W6.H",
        "Wave 6 gray release evidence",
        "docs/runbooks/wave-6-deploy-evidence.md",
        "docs/retros/wave-6-deploy-evidence.json",
        (
            "wave-6-deploy-materials",
            "wave-6-deploy-readiness",
            "wave-6-deploy-audit",
            "wave-6-deploy-evidence-record",
            "wave-6-deploy-evidence-validate",
        ),
    ),
)

REQUIRED_TOP_LEVEL_FILES = (
    PREFLIGHT_DOC,
    CLOSEOUT_DOC,
    TODO_DOC,
    JUSTFILE,
    "docs/adr/0035-wave-6-pre-release-evidence-closeout.md",
)
WAVE6_EVIDENCE_BOUNDARY_DOCS = (
    TODO_DOC,
    "ROADMAP.md",
    "docs/adr/0035-wave-6-pre-release-evidence-closeout.md",
    "docs/retros/wave-1-retro.md",
)
WAVE6_SCOPE_GATE_DOCS = (
    TODO_DOC,
    "ROADMAP.md",
    "docs/architecture-dependencies.md",
    "docs/adr/0035-wave-6-pre-release-evidence-closeout.md",
)
WAVE6_CLOSEOUT_JUST_ENTRIES = (
    "wave-6-status",
    "wave-6-evidence-check",
    "wave-6-missing-evidence-commands",
    "wave-6-complete-check",
)
WAVE6_GATE_COMMAND_OVERRIDES = {
    "W6.D": {
        "readiness": [
            "just wave-3-pda-preaudit-kit --json",
            "just wave-3-pda-materials-checklist --json",
            "just wave-3-pda-field-work-request",
            "just wave-3-pda-field-execution-summary --json",
            "just wave-3-pda-field-precheck-summary --from-env --json",
            "just wave-3-pda-field-owner-gap-actions --json",
            "just wave-3-pda-field-handoff-bundle --json",
            "just wave-3-pda-evidence-package-template",
            "just wave-3-pda-intake-template --json",
            "just wave-3-pda-intake-check --json",
            "just wave-3-pda-service-precheck --from-env --json",
            "just wave-3-pda-trace-code-openapi-precheck --from-env --json",
            "just wave-3-pda-runtime-readiness --from-env --json",
        ],
        "record_check_only": [
            "just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
            "just wave-3-pda-intake-check --json",
        ],
        "record": [
            "just wave-3-pda-intake-record --json",
            "just wave-3-pda-runtime-evidence-record",
        ],
    },
    "W6.E": {
        "readiness": [
            "just wave-4-external-dependencies-record --export-template",
            "just wave-4-external-dependencies-readiness --from-env --json",
        ],
        "record_check_only": [
            "just wave-4-external-dependencies-record --from-env --check-only --json",
        ],
        "record": [
            "just wave-4-external-dependencies-record --from-env --json",
        ],
    },
    "W6.F": {
        "readiness": [
            "just wave-5-hardware-materials --export-template",
            "just wave-5-hardware-materials --from-env --json",
            "just wave-5-hardware-readiness --from-env --json",
        ],
        "record_check_only": [
            "just wave-5-hardware-evidence-record --from-env --check-only --json",
        ],
        "record": [
            "just wave-5-hardware-evidence-record --from-env --json",
        ],
    },
    "W6.G": {
        "readiness": [
            "just wave-5-tms-materials --export-template",
            "just wave-5-tms-materials --from-env --json",
            "just wave-5-tms-readiness --from-env --json",
        ],
        "record_check_only": [
            "just wave-5-tms-evidence-record --from-env --check-only --json",
        ],
        "record": [
            "just wave-5-tms-evidence-record --from-env --json",
        ],
    },
    "W6.H": {
        "readiness": [
            "just wave-6-deploy-materials --export-template",
            "just wave-6-deploy-materials --from-env --json",
            "just wave-6-deploy-readiness --from-env --json",
        ],
        "record_check_only": [
            "just wave-6-deploy-audit --from-env --check-only",
            "just wave-6-deploy-evidence-record --from-env --check-only --json",
        ],
        "record": [
            "just wave-6-deploy-audit --from-env",
            "just wave-6-deploy-evidence-record --from-env --json",
        ],
    },
}
WAVE6_H_CLOSEOUT_COMMAND_ORDER = (
    "just wave-6-deploy-materials --export-template",
    "just wave-6-deploy-materials --from-env --json",
    "just wave-6-deploy-audit --from-env --check-only",
    "just wave-6-deploy-audit --from-env",
    "just wave-6-deploy-readiness --from-env --json",
    "just wave-6-deploy-evidence-record --from-env --check-only --json",
    "just wave-6-deploy-evidence-record --from-env --json",
    "just wave-6-deploy-evidence-validate",
)
WAVE6_CLOSEOUT_JUST_ENTRY_COMMANDS = {
    "wave-6-status": (
        "python3",
        "scripts/governance/report_wave6_pre_release.py",
    ),
    "wave-6-evidence-check": (
        "python3",
        "scripts/governance/report_wave6_pre_release.py",
        "--strict",
        "--evidence-only",
    ),
    "wave-6-missing-evidence-commands": (
        "python3",
        "scripts/governance/report_wave6_pre_release.py",
        "--commands-only",
        "--strict",
        "--evidence-only",
    ),
    "wave-6-complete-check": (
        "python3",
        "scripts/governance/report_wave6_pre_release.py",
        "--strict",
    ),
}
WAVE6_CLOSEOUT_REPORT_JSON_FIELDS = (
    "schema_version",
    "available_modes",
    "writes_runtime_evidence",
    "closes_gate",
    "report_command",
    "evidence_only_command",
    "commands_only_command",
    "evidence_gate_ids",
    "evidence_gate_evidence_files",
    "evidence_gate_just_entries",
    "evidence_gate_execution_files",
    "required_top_level_files",
    "required_runbooks",
    "required_execution_files",
    "validation_commands",
    "closeout_just_entries",
    "evidence_gate_item_ids",
    "non_evidence_item_ids",
    "blocking_count",
    "ignored_count",
    "evidence_blocking_count",
    "evidence_ignored_count",
    "evidence_blocking_item_ids",
    "non_evidence_blocking_item_ids",
    "evidence_ignored_item_ids",
    "non_evidence_ignored_item_ids",
    "blocking_details",
    "ignored_details",
    "missing_evidence_count",
    "missing_evidence_item_ids",
    "missing_evidence_files",
    "missing_evidence_details",
    "readiness_commands",
    "record_check_only_commands",
    "record_commands",
    "validate_commands",
    "deployment_choice_required",
    "deployment_choice_label",
    "deployment_choice_options",
    "deployment_path_commands",
)
WAVE6_CLOSEOUT_REPORT_JSON_SECTION_MARKER = (
    "report_wave6_pre_release.py --strict --evidence-only --json"
)

REQUIRED_EXECUTION_FILES = (
    "deploy/scripts/wave1_auto_rollback_probe.sh",
    "deploy/scripts/wave1_rollback.sh",
    "scripts/governance/report_wave6_pre_release.py",
    "scripts/governance/validate_wave1_runtime_evidence.py",
    "scripts/governance/check_wave1_runtime_evidence_prereqs.py",
    "scripts/governance/check_wave1_h2_runtime_readiness.py",
    "scripts/governance/collect_wave1_h2_runtime_evidence.py",
    "scripts/governance/report_wave2_completion.py",
    "scripts/governance/collect_wave2_runtime_evidence.py",
    "scripts/governance/record_wave2_runtime_evidence.py",
    "scripts/governance/check_wave3_pda_runtime_readiness.py",
    "scripts/governance/validate_wave3_pda_runtime_evidence.py",
    "scripts/governance/record_wave3_pda_runtime_evidence.py",
    "scripts/governance/check_wave4_external_dependencies_readiness.py",
    "scripts/governance/validate_wave4_external_dependencies.py",
    "scripts/governance/record_wave4_external_dependencies.py",
    "scripts/governance/validate_wave5_hardware_evidence.py",
    "scripts/governance/record_wave5_hardware_evidence.py",
    "scripts/governance/validate_wave5_tms_evidence.py",
    "scripts/governance/record_wave5_tms_evidence.py",
    "scripts/governance/report_wave6_deploy_materials.py",
    "scripts/governance/check_wave6_deploy_readiness.py",
    "scripts/governance/validate_wave6_deploy_evidence.py",
    "scripts/governance/record_wave6_deploy_evidence.py",
)

OVERWRITE_GUARD_REQUIRED_MARKERS = (
    "--force",
    "already exists",
    "pass --force to overwrite",
)
SCHEMA_VERSION = 1
PREFLIGHT_MODE = "static-preflight"
PREFLIGHT_COMMAND = "python3 scripts/governance/check_wave6_evidence_preflight.py --json"


def repo_path(path: str) -> Path:
    return REPO_ROOT / path


def read_text(path: str) -> str:
    target = repo_path(path)
    return target.read_text(encoding="utf-8") if target.exists() else ""


def iter_fenced_code_lines(text: str) -> list[tuple[int, str]]:
    lines: list[tuple[int, str]] = []
    in_code = False
    for line_no, line in enumerate(text.splitlines(), start=1):
        if line.strip().startswith("```"):
            in_code = not in_code
            continue
        if in_code:
            lines.append((line_no, line))
    return lines


def iter_fenced_code_blocks(text: str) -> list[tuple[str, list[tuple[int, str]]]]:
    blocks: list[tuple[str, list[tuple[int, str]]]] = []
    language = ""
    current: list[tuple[int, str]] = []
    in_code = False
    for line_no, line in enumerate(text.splitlines(), start=1):
        stripped = line.strip()
        if stripped.startswith("```"):
            if in_code:
                blocks.append((language, current))
                language = ""
                current = []
                in_code = False
            else:
                language = stripped[3:].strip().lower()
                current = []
                in_code = True
            continue
        if in_code:
            current.append((line_no, line))
    return blocks


def iter_fenced_code_blocks_with_context(
    text: str,
) -> list[tuple[str, int, list[tuple[int, str]], list[str]]]:
    blocks: list[tuple[str, int, list[tuple[int, str]], list[str]]] = []
    all_lines = text.splitlines()
    language = ""
    current: list[tuple[int, str]] = []
    start_line = 0
    in_code = False
    for line_no, line in enumerate(all_lines, start=1):
        stripped = line.strip()
        if stripped.startswith("```"):
            if in_code:
                context_start = max(0, start_line - 7)
                context = all_lines[context_start : start_line - 1]
                blocks.append((language, start_line, current, context))
                language = ""
                current = []
                start_line = 0
                in_code = False
            else:
                language = stripped[3:].strip().lower()
                current = []
                start_line = line_no
                in_code = True
            continue
        if in_code:
            current.append((line_no, line))
    return blocks


def placeholder_terms_in_value(value: str) -> list[str]:
    lowered = value.lower()
    terms: list[str] = []
    for token in PLACEHOLDER_VALUE_TOKENS:
        if token in lowered:
            terms.append("YYYY" if token == "yyyy" else token)
    if "<" in lowered and ">" in lowered:
        terms.append("<...>")
    return terms


def is_reference_like_line(line: str) -> bool:
    lowered = line.lower()
    return any(hint in lowered for hint in REFERENCE_LINE_HINTS)


def check_runbook_code_placeholder_refs(runbook: str, text: str) -> list[str]:
    errors: list[str] = []
    for line_no, line in iter_fenced_code_lines(text):
        terms = placeholder_terms_in_value(line)
        if not terms or not is_reference_like_line(line):
            continue
        errors.append(
            f"{runbook}:{line_no} 示例证据引用保留模板占位 "
            f"{', '.join(sorted(set(terms)))}: {line.strip()}"
        )
    return errors


def iter_shell_commands(lines: list[tuple[int, str]]) -> list[tuple[int, str]]:
    commands: list[tuple[int, str]] = []
    start_line = 0
    current: list[str] = []
    for line_no, line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        if stripped.startswith("#"):
            continue
        if not current:
            start_line = line_no
        current.append(stripped.rstrip("\\").strip())
        if stripped.endswith("\\"):
            continue
        commands.append((start_line, " ".join(current)))
        current = []
        start_line = 0
    if current:
        commands.append((start_line, " ".join(current)))
    return commands


def is_record_example_command(command: str) -> bool:
    normalized = " ".join(command.split())
    if re.search(r"\bjust\s+\S*(?:record|evidence)(?!-validate)\b", normalized):
        return True
    return bool(re.search(r"\bpython3\s+\S*(?:record_|collect_)\S*", normalized))


def record_env_prefix(gate: GateSpec) -> str:
    record_entry = next(
        (
            entry for entry in gate.just_entries
            if entry.endswith("-record") or is_record_command_entry(entry)
        ),
        gate.just_entries[0] if gate.just_entries else gate.gate_id.lower(),
    )
    for suffix in (
        "-runtime-evidence-record",
        "-external-dependencies-record",
        "-hardware-evidence-record",
        "-tms-evidence-record",
        "-deploy-evidence-record",
        "-runtime-evidence",
        "-evidence-record",
        "-evidence",
        "-record",
    ):
        if record_entry.endswith(suffix):
            record_entry = record_entry[: -len(suffix)]
            break
    return re.sub(r"[^0-9A-Za-z]+", "_", record_entry).strip("_").upper()


def env_var_for_record_ref(gate: GateSpec, flag: str) -> str:
    suffix = re.sub(r"[^0-9A-Za-z]+", "_", flag.removeprefix("--")).strip("_").upper()
    prefix = record_env_prefix(gate)
    return f"{prefix}_{suffix}" if prefix else suffix


def check_runbook_hardcoded_record_refs(gate: GateSpec, text: str) -> list[str]:
    errors: list[str] = []
    for language, lines in iter_fenced_code_blocks(text):
        if language and language not in {"bash", "sh", "shell", "zsh"}:
            continue
        for line_no, command in iter_shell_commands(lines):
            if not is_record_example_command(command):
                continue
            for match in RECORD_REF_ARG_RE.finditer(command):
                value = (
                    match.group("quoted_value") or match.group("bare_value") or ""
                ).strip()
                if value.startswith("$"):
                    continue
                lowered = value.lower()
                if not any(hint in lowered for hint in HARDCODED_RECORD_REF_VALUE_HINTS):
                    continue
                flag = match.group("flag")
                errors.append(
                    f"{gate.runbook}:{line_no} record 命令不能硬编码证据引用 {flag}; "
                    f"请改用环境变量 ${env_var_for_record_ref(gate, flag)}"
                )
    return errors


def check_runbook_hardcoded_export_refs(gate: GateSpec, text: str) -> list[str]:
    errors: list[str] = []
    for language, lines in iter_fenced_code_blocks(text):
        if language and language not in {"bash", "sh", "shell", "zsh"}:
            continue
        for line_no, line in lines:
            for match in EXPORT_REF_RE.finditer(line):
                value = (
                    match.group("quoted_value") or match.group("bare_value") or ""
                ).strip()
                if "$" in value:
                    continue
                lowered = value.lower()
                if not any(hint in lowered for hint in HARDCODED_RECORD_REF_VALUE_HINTS):
                    continue
                name = match.group("name")
                errors.append(
                    f"{gate.runbook}:{line_no} export {name} 不能硬编码证据引用；"
                    "请改用现场环境变量或动态时间生成"
                )
    return errors


def check_runbook_force_record_examples(gate: GateSpec, text: str) -> list[str]:
    errors: list[str] = []
    for language, lines in iter_fenced_code_blocks(text):
        if language and language not in {"bash", "sh", "shell", "zsh"}:
            continue
        for line_no, line in lines:
            if line.lstrip().startswith("#"):
                continue
            normalized = " ".join(line.split())
            if "--force" not in normalized:
                continue
            if "record" not in normalized and "evidence" not in normalized:
                continue
            errors.append(
                f"{gate.runbook}:{line_no} record/evidence 示例不能使用 --force；"
                "真实 evidence 默认必须防覆盖"
            )
    return errors


def check_runbook_json_example_disclaimers(gate: GateSpec, text: str) -> list[str]:
    errors: list[str] = []
    for language, start_line, lines, context in iter_fenced_code_blocks_with_context(text):
        if language != "json":
            continue
        block_text = "\n".join(line for _line_no, line in lines).lower()
        if not any(hint in block_text for hint in HARDCODED_RECORD_REF_VALUE_HINTS):
            continue
        context_text = "\n".join(context)
        required_terms = ("字段结构示例", "不得复制为真实 evidence", "record 命令生成")
        missing = [term for term in required_terms if term not in context_text]
        if missing:
            errors.append(
                f"{gate.runbook}:{start_line} Evidence JSON 含真实形态引用时必须声明"
                f"字段结构示例且不得复制为真实 evidence，并说明真实 evidence 必须由 record 命令生成；"
                f"缺少: {', '.join(missing)}"
            )
    return errors


def check_closeout_placeholder_record_args(text: str) -> list[str]:
    errors: list[str] = []
    for language, lines in iter_fenced_code_blocks(text):
        if language and language not in {"bash", "sh", "shell", "zsh"}:
            continue
        for line_no, line in lines:
            if line.lstrip().startswith("#"):
                continue
            terms = placeholder_terms_in_value(line)
            if not terms:
                continue
            normalized = " ".join(line.split())
            if "just " not in normalized or "-record" not in normalized:
                continue
            errors.append(
                f"{CLOSEOUT_DOC}:{line_no} closeout record 命令不能保留模板占位 "
                f"{', '.join(sorted(set(terms)))}: {line.strip()}"
            )
    return errors


def check_closeout_force_record_examples(text: str) -> list[str]:
    errors: list[str] = []
    for language, lines in iter_fenced_code_blocks(text):
        if language and language not in {"bash", "sh", "shell", "zsh"}:
            continue
        for line_no, line in lines:
            if line.lstrip().startswith("#"):
                continue
            normalized = " ".join(line.split())
            if "--force" not in normalized:
                continue
            if "record" not in normalized and "evidence" not in normalized:
                continue
            errors.append(
                f"{CLOSEOUT_DOC}:{line_no} closeout record/evidence 示例不能使用 --force；"
                "真实 evidence 默认必须防覆盖"
            )
    return errors


def check_closeout_gate_matrix_sync(text: str) -> list[str]:
    if "## 当前 Gate" not in text:
        return [f"{CLOSEOUT_DOC} closeout 缺少 ## 当前 Gate 矩阵"]
    errors: list[str] = []
    matrix_text = text.split("## 当前 Gate", maxsplit=1)[1].split("## ", maxsplit=1)[0]
    matrix_lines = matrix_text.splitlines()
    for gate in GATES:
        gate_rows = [line for line in matrix_lines if gate.gate_id in line]
        if not gate_rows:
            errors.append(f"{CLOSEOUT_DOC} closeout gate 矩阵缺少 {gate.gate_id}")
            continue
        gate_row = "\n".join(gate_rows)
        matrix_entries = [
            entry for entry in gate.just_entries
            if is_record_command_entry(entry) or entry.endswith("-validate")
        ]
        for needle in (gate.evidence_file, *matrix_entries):
            if needle not in text:
                errors.append(f"{CLOSEOUT_DOC} closeout gate 矩阵缺少 {needle}")
                continue
            if needle not in gate_row:
                errors.append(
                    f"{CLOSEOUT_DOC} closeout gate 矩阵 {gate.gate_id} 行缺少 {needle}"
                )
    return errors


def check_closeout_recommended_order(text: str) -> list[str]:
    if "## 推荐执行顺序" not in text:
        return [f"{CLOSEOUT_DOC} closeout 缺少 ## 推荐执行顺序"]
    order_text = text.split("## 推荐执行顺序", maxsplit=1)[1]
    errors: list[str] = []
    command_order: list[str] = []
    for language, lines in iter_fenced_code_blocks(order_text):
        if language and language not in {"bash", "sh", "shell", "zsh"}:
            continue
        for _, command in iter_shell_commands(lines):
            if command.lstrip().startswith("#"):
                continue
            command_order.append(" ".join(command.split()))

    for gate in GATES:
        for validate_entry in [
            entry for entry in gate.just_entries if entry.endswith("-validate")
        ]:
            validate_command = f"just {validate_entry}"
            validate_indexes = [
                index for index, command in enumerate(command_order)
                if command.startswith(validate_command)
            ]
            if not validate_indexes:
                errors.append(
                    f"{CLOSEOUT_DOC} 推荐执行顺序缺少 {gate.gate_id} 验证命令 "
                    f"{validate_command}"
                )
                continue
            record_commands = [
                f"just {entry}"
                for entry in gate.just_entries
                if is_record_command_entry(entry)
            ]
            has_record_before_validate = any(
                any(
                    prior_command.startswith(command)
                    for prior_command in command_order[:validate_index]
                )
                for command in record_commands
                for validate_index in validate_indexes
            )
            if not has_record_before_validate:
                errors.append(
                    f"{CLOSEOUT_DOC} 推荐执行顺序必须先列 {gate.gate_id} record "
                    f"再列 validate；缺少早于 {validate_command} 的 record 命令: "
                    + ", ".join(record_commands)
                )
    return errors


def check_closeout_w6h_deploy_order(text: str) -> list[str]:
    if "W6.H" not in text or "## 推荐执行顺序" not in text:
        return []
    order_text = text.split("## 推荐执行顺序", maxsplit=1)[1]
    command_order: list[str] = []
    for language, lines in iter_fenced_code_blocks(order_text):
        if language and language not in {"bash", "sh", "shell", "zsh"}:
            continue
        for _, command in iter_shell_commands(lines):
            command_order.append(" ".join(command.split()))

    positions: list[int] = []
    missing: list[str] = []
    for expected in WAVE6_H_CLOSEOUT_COMMAND_ORDER:
        matches = [
            index for index, command in enumerate(command_order)
            if command == expected
        ]
        if not matches:
            missing.append(expected)
            continue
        positions.append(matches[0])

    errors: list[str] = []
    if missing:
        errors.append(
            f"{CLOSEOUT_DOC} 推荐执行顺序缺少 W6.H 命令: "
            + ", ".join(missing)
        )
    if len(positions) == len(WAVE6_H_CLOSEOUT_COMMAND_ORDER) and positions != sorted(positions):
        errors.append(
            f"{CLOSEOUT_DOC} 推荐执行顺序中 W6.H 命令必须按 "
            "materials --export-template → materials --json → deploy audit --check-only "
            "→ deploy audit → deploy readiness → evidence record --check-only "
            "→ evidence record → validate 顺序列出"
        )
    return errors


def check_closeout_retro_prerequisites(text: str) -> list[str]:
    if "docs/retros/wave-6-retro.md" not in text:
        return []

    required_terms = (
        "`missing_evidence_item_ids` / `missing_evidence_files`",
        "写 retro 前必须为空",
        "just wave-6-evidence-check",
    )
    missing = [term for term in required_terms if term not in text]
    if not missing:
        return []
    return [
        f"{CLOSEOUT_DOC} closeout 写 retro 前必须声明 "
        "`missing_evidence_item_ids` / `missing_evidence_files` 为空，"
        "并先运行 just wave-6-evidence-check；缺少: "
        + ", ".join(missing)
    ]


def check_closeout_retro_before_complete_check(text: str) -> list[str]:
    retro = "docs/retros/wave-6-retro.md"
    complete_check = "just wave-6-complete-check"
    if retro not in text or complete_check not in text:
        return []

    scope = text
    scope_name = CLOSEOUT_DOC
    if "## 完成口径" in text and "## 当前 Gate" in text:
        scope = text.split("## 完成口径", maxsplit=1)[1].split(
            "## 当前 Gate",
            maxsplit=1,
        )[0]
        scope_name = f"{CLOSEOUT_DOC} 完成口径"

    if retro not in scope or complete_check not in scope:
        return []
    if scope.index(retro) < scope.index(complete_check):
        return []
    return [
        f"{scope_name} closeout 必须先写 {retro}，再运行 {complete_check}；"
        f"{retro} 必须出现在 {complete_check} 之前"
    ]


def check_closeout_evidence_check_before_retro_in_completion_criteria(
    text: str,
) -> list[str]:
    if "## 完成口径" not in text or "## 当前 Gate" not in text:
        return []

    scope = text.split("## 完成口径", maxsplit=1)[1].split(
        "## 当前 Gate",
        maxsplit=1,
    )[0]
    evidence_check = "just wave-6-evidence-check"
    retro = "docs/retros/wave-6-retro.md"

    if retro not in scope:
        return []
    if evidence_check in scope and scope.index(evidence_check) < scope.index(retro):
        return []
    return [
        f"{CLOSEOUT_DOC} 完成口径必须先列 {evidence_check}，"
        f"再写 {retro}；{evidence_check} 必须出现在 {retro} 之前"
    ]


def check_closeout_report_json_fields(text: str) -> list[str]:
    if WAVE6_CLOSEOUT_REPORT_JSON_SECTION_MARKER not in text:
        return []

    missing = [
        field for field in WAVE6_CLOSEOUT_REPORT_JSON_FIELDS
        if f"`{field}`" not in text
    ]
    if not missing:
        return []
    return [
        f"{CLOSEOUT_DOC} closeout report JSON 字段清单缺少: "
        + ", ".join(missing)
    ]


def check_scope_boundary_docs() -> list[str]:
    errors: list[str] = []
    boundary_hints = ("localhost", "mock", "fake", "stub", "example")
    evidence_hints = ("evidence", "证据", "gate")
    for path in WAVE6_EVIDENCE_BOUNDARY_DOCS:
        target = repo_path(path)
        if not target.exists():
            continue
        for line_no, line in enumerate(target.read_text(encoding="utf-8").splitlines(), start=1):
            normalized = line.replace("`", "")
            lowered = normalized.lower()
            if "prod" not in lowered:
                continue
            if not any(hint in lowered for hint in boundary_hints):
                continue
            if not any(hint in normalized for hint in evidence_hints):
                continue
            if "production" not in lowered:
                errors.append(
                    f"{path}:{line_no} evidence 边界提到 prod 时必须同时写 production: {line.strip()}"
                )
    return errors


def check_scope_gate_docs_sync() -> list[str]:
    errors: list[str] = []
    for path in WAVE6_SCOPE_GATE_DOCS:
        target = repo_path(path)
        if not target.exists():
            continue
        text = target.read_text(encoding="utf-8")
        if not any(gate.gate_id in text for gate in GATES):
            continue
        missing = [gate.gate_id for gate in GATES if gate.gate_id not in text]
        if missing:
            errors.append(f"{path} 缺少 Wave 6 gate 登记: {', '.join(missing)}")
    return errors


def just_has_entry(just_text: str, entry: str) -> bool:
    return re.search(rf"(?m)^{re.escape(entry)}(?:\s|\*|:)", just_text) is not None


def just_recipe_body(just_text: str, entry: str) -> str:
    lines = just_text.splitlines()
    body: list[str] = []
    in_recipe = False
    for line in lines:
        if re.match(rf"^{re.escape(entry)}(?:\s|\*|:)", line):
            in_recipe = True
            continue
        if in_recipe and re.match(r"^[A-Za-z0-9_-]+(?:\s|\*|:)", line):
            break
        if in_recipe:
            body.append(line.strip())
    return " ".join(body)


def just_recipe_commands(just_text: str, entry: str) -> list[list[str]]:
    lines = just_text.splitlines()
    commands: list[list[str]] = []
    in_recipe = False
    for line in lines:
        if re.match(rf"^{re.escape(entry)}(?:\s|\*|:)", line):
            in_recipe = True
            continue
        if in_recipe and re.match(r"^[A-Za-z0-9_-]+(?:\s|\*|:)", line):
            break
        if not in_recipe:
            continue
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("@"):
            stripped = stripped[1:].strip()
        commands.append(stripped.split())
    return commands


def check_documented_closeout_just_entries(
    *,
    preflight_text: str,
    closeout_text: str,
    just_text: str,
) -> list[str]:
    errors: list[str] = []
    documented_text = f"{preflight_text}\n{closeout_text}"
    documented_commands = documented_just_commands(documented_text)
    for entry in WAVE6_CLOSEOUT_JUST_ENTRIES:
        if f"just {entry}" not in documented_commands:
            continue
        if not just_has_entry(just_text, entry):
            errors.append(f"justfile 缺少 Wave 6 收口入口: {entry}")
            continue
        expected_command = WAVE6_CLOSEOUT_JUST_ENTRY_COMMANDS.get(entry, ())
        recipe_commands = just_recipe_commands(just_text, entry)
        if len(recipe_commands) != 1:
            errors.append(
                f"justfile Wave 6 收口入口 {entry} 必须只有一条实际命令: "
                f"expected={' '.join(expected_command)}"
            )
            continue
        actual_command = recipe_commands[0]
        if tuple(actual_command) != expected_command:
            missing_tokens = [
                token for token in expected_command
                if token not in actual_command
            ]
            extra_tokens = [
                token for token in actual_command
                if token not in expected_command
            ]
            parts = []
            if missing_tokens:
                parts.append("缺少参数: " + ", ".join(missing_tokens))
            if extra_tokens:
                parts.append("包含多余参数: " + ", ".join(extra_tokens))
            if not parts:
                parts.append(
                    "命令顺序不一致: "
                    f"expected={' '.join(expected_command)}; "
                    f"actual={' '.join(actual_command)}"
                )
            errors.append(
                f"justfile Wave 6 收口入口 {entry} 命令不一致；"
                + "；".join(parts)
            )
    return errors


def check_required_closeout_just_entries_documented(
    *,
    preflight_text: str,
    closeout_text: str,
) -> list[str]:
    documented_text = f"{preflight_text}\n{closeout_text}"
    documented_commands = documented_just_commands(documented_text)
    missing = [
        entry
        for entry in WAVE6_CLOSEOUT_JUST_ENTRIES
        if f"just {entry}" not in documented_commands
    ]
    if not missing:
        return []
    return [
        "Wave 6 preflight/closeout 文档缺少 Wave 6 收口入口: "
        + ", ".join(missing)
    ]


def documented_just_commands(text: str) -> set[str]:
    commands: set[str] = set()
    in_shell_block = False
    for raw_line in text.splitlines():
        stripped = raw_line.strip()
        if stripped.startswith("```"):
            if in_shell_block:
                in_shell_block = False
            else:
                language = stripped[3:].strip().lower()
                in_shell_block = language in {"bash", "sh", "shell"}
            continue
        if not in_shell_block:
            continue
        if stripped.startswith("#"):
            continue
        match = re.match(r"^just\s+([A-Za-z0-9_-]+)\b", stripped)
        if match:
            commands.add(f"just {match.group(1)}")
    return commands


def is_evidence_writer_execution_file(path: str) -> bool:
    name = Path(path).name
    return (
        name.startswith("record_")
        or name.startswith("collect_")
        or name == "wave1_auto_rollback_probe.sh"
    )


def has_python_overwrite_guard(text: str) -> bool:
    if "write_text(" not in text:
        return True
    normalized = re.sub(r"\s+", " ", text)
    return (
        "exists() and not force" in normalized
        or "exists() and not args.force" in normalized
        or "not force and" in normalized and "exists()" in normalized
        or "not args.force and" in normalized and "exists()" in normalized
    )


def has_shell_overwrite_guard(text: str) -> bool:
    if "write_evidence_file" not in text and "EVIDENCE_FILE=" not in text:
        return True
    normalized = re.sub(r"\s+", " ", text)
    return (
        '-e "$evidence_file"' in normalized
        and '"$force" != "true"' in normalized
    )


def semantic_overwrite_guard_errors(path: str, text: str) -> list[str]:
    suffix = Path(path).suffix
    if suffix == ".py" and not has_python_overwrite_guard(text):
        return [
            f"{path} 是 Python evidence 写入器，write_text 前必须有 "
            "exists() and not force 保护"
        ]
    if suffix == ".sh" and not has_shell_overwrite_guard(text):
        return [
            f"{path} 是 shell evidence 写入器，写入前必须检查 "
            '-e "$evidence_file" 且 "$force" != "true"'
        ]
    return []


def check_execution_file_overwrite_guards(paths: tuple[str, ...] = REQUIRED_EXECUTION_FILES) -> list[str]:
    errors: list[str] = []
    for path in paths:
        if not is_evidence_writer_execution_file(path):
            continue
        target = repo_path(path)
        if not target.exists():
            continue
        text = target.read_text(encoding="utf-8")
        missing = [marker for marker in OVERWRITE_GUARD_REQUIRED_MARKERS if marker not in text]
        if missing:
            errors.append(
                f"{path} 是 evidence 写入器，必须默认防覆盖并要求显式 --force；"
                f"缺少: {', '.join(missing)}"
            )
            continue
        errors.extend(semantic_overwrite_guard_errors(path, text))
    return errors


def overwrite_guard_execution_files() -> list[str]:
    return [
        path for path in REQUIRED_EXECUTION_FILES
        if is_evidence_writer_execution_file(path)
    ]


def required_runbooks() -> list[str]:
    return list(dict.fromkeys(gate.runbook for gate in GATES))


def gate_commands_by_phase() -> dict[str, dict[str, list[str]]]:
    commands: dict[str, dict[str, list[str]]] = {}
    for gate in GATES:
        phases = {
            "readiness": [
                f"just {entry}" for entry in gate.just_entries
                if "readiness" in entry or "prereq" in entry or "materials" in entry
            ],
            "record_check_only": [],
            "record": [
                f"just {entry}" for entry in gate.just_entries
                if is_record_command_entry(entry)
            ],
            "validate": [
                f"just {entry}" for entry in gate.just_entries
                if entry.endswith("-validate")
            ],
        }
        for phase, phase_commands in WAVE6_GATE_COMMAND_OVERRIDES.get(gate.gate_id, {}).items():
            phases[phase] = list(phase_commands)
        commands[gate.gate_id] = phases
    return commands


def is_record_command_entry(entry: str) -> bool:
    return (
        entry.endswith("-record")
        or entry == "wave-6-deploy-audit"
        or entry.endswith("-evidence")
        or (
            "-evidence-" in entry
            and not entry.endswith("-validate")
            and "readiness" not in entry
            and "prereq" not in entry
        )
    )


def validation_commands() -> list[str]:
    return list(dict.fromkeys([
        f"just {entry}"
        for gate in GATES
        for entry in gate.just_entries
        if entry.endswith("-validate")
    ]))


def top_error_details(errors: list[str]) -> list[dict[str, object]]:
    return [
        {"scope": "top", "gate_id": None, "message": error}
        for error in errors
    ]


def gate_error_details(gate_results: list[GateResult]) -> list[dict[str, object]]:
    return [
        {"scope": "gate", "gate_id": result.gate_id, "message": error}
        for result in gate_results
        for error in result.errors
    ]


def gate_ids() -> list[str]:
    return [gate.gate_id for gate in GATES]


def gate_evidence_files() -> list[str]:
    return [gate.evidence_file for gate in GATES]


def gate_runbooks() -> list[str]:
    return [gate.runbook for gate in GATES]


def gate_just_entries() -> dict[str, list[str]]:
    return {
        gate.gate_id: list(gate.just_entries)
        for gate in GATES
    }


def gate_execution_files(gate: GateSpec) -> list[str]:
    """Return execution files implied by record / validate just entries."""
    files: list[str] = []
    for entry in gate.just_entries:
        execution_file = execution_file_for_just_entry(entry)
        if execution_file is not None:
            files.append(execution_file)
    return list(dict.fromkeys(files))


def execution_file_for_just_entry(entry: str) -> str | None:
    if entry in JUST_ENTRY_EXECUTION_FILE_OVERRIDES:
        return JUST_ENTRY_EXECUTION_FILE_OVERRIDES[entry]
    if not re.match(r"^wave-\d", entry):
        return None
    if entry.endswith("-record"):
        stem = re.sub(r"^wave-(\d)", r"wave\1", entry.removesuffix("-record"))
        return f"scripts/governance/record_{stem.replace('-', '_')}.py"
    if entry.endswith("-validate"):
        stem = re.sub(r"^wave-(\d)", r"wave\1", entry.removesuffix("-validate"))
        return f"scripts/governance/validate_{stem.replace('-', '_')}.py"
    return None


def gate_execution_file_map() -> dict[str, list[str]]:
    return {
        gate.gate_id: gate_execution_files(gate)
        for gate in GATES
    }


def gate_spec_payload(gate: GateSpec) -> dict[str, object]:
    payload = asdict(gate)
    payload["execution_files"] = gate_execution_files(gate)
    return payload


def check_gate(gate: GateSpec, *, preflight_text: str, just_text: str) -> GateResult:
    errors: list[str] = []

    if not repo_path(gate.runbook).exists():
        errors.append(f"缺少 runbook: {gate.runbook}")
        return GateResult(gate.gate_id, gate.title, False, errors)

    runbook_text = read_text(gate.runbook)
    required_in_runbook = [
        gate.evidence_file,
        *gate.just_entries,
        *gate.required_terms,
        *gate.forbidden_boundary_terms,
        *gate.template_placeholder_terms,
    ]
    for needle in required_in_runbook:
        if needle not in runbook_text:
            errors.append(f"{gate.runbook} 缺少 {needle}")

    errors.extend(check_runbook_code_placeholder_refs(gate.runbook, runbook_text))
    errors.extend(check_runbook_hardcoded_record_refs(gate, runbook_text))
    errors.extend(check_runbook_hardcoded_export_refs(gate, runbook_text))
    errors.extend(check_runbook_force_record_examples(gate, runbook_text))
    errors.extend(check_runbook_json_example_disclaimers(gate, runbook_text))

    for entry in gate.just_entries:
        if not just_has_entry(just_text, entry):
            errors.append(f"justfile 缺少入口: {entry}")
            continue
        execution_file = execution_file_for_just_entry(entry)
        if execution_file is None:
            continue
        recipe_commands = just_recipe_commands(just_text, entry)
        if not any(execution_file in command for command in recipe_commands):
            errors.append(
                f"justfile 入口 {entry} 必须调用 {execution_file}"
            )

    required_execution_files = set(REQUIRED_EXECUTION_FILES)
    for path in gate_execution_files(gate):
        if path not in required_execution_files:
            errors.append(f"REQUIRED_EXECUTION_FILES 缺少 {gate.gate_id} 执行文件: {path}")
        elif not repo_path(path).exists():
            errors.append(f"缺少 {gate.gate_id} 执行文件: {path}")

    required_in_preflight = [gate.gate_id, gate.evidence_file, *gate.just_entries]
    for needle in required_in_preflight:
        if needle not in preflight_text:
            errors.append(f"{PREFLIGHT_DOC} 缺少 {needle}")

    return GateResult(gate.gate_id, gate.title, not errors, errors)


def collect_results() -> tuple[list[str], list[GateResult]]:
    errors: list[str] = []

    for path in REQUIRED_TOP_LEVEL_FILES:
        if not repo_path(path).exists():
            errors.append(f"缺少文件: {path}")

    for path in REQUIRED_EXECUTION_FILES:
        if not repo_path(path).exists():
            errors.append(f"缺少执行文件: {path}")
    errors.extend(check_execution_file_overwrite_guards())

    preflight_text = read_text(PREFLIGHT_DOC)
    closeout_text = read_text(CLOSEOUT_DOC)
    todo_text = read_text(TODO_DOC)
    just_text = read_text(JUSTFILE)

    for needle in (
        "just wave-6-evidence-preflight",
        "不会写入 runtime evidence",
        "不能关闭 gate",
        *REQUIRED_BOUNDARY_TERMS,
        *FORBIDDEN_BOUNDARY_TERMS,
        *TEMPLATE_PLACEHOLDER_TERMS,
    ):
        if needle not in preflight_text:
            errors.append(f"{PREFLIGHT_DOC} 缺少 {needle}")

    for needle in ("just wave-6-evidence-preflight", PREFLIGHT_DOC):
        if needle not in closeout_text:
            errors.append(f"{CLOSEOUT_DOC} 缺少 {needle}")
    errors.extend(check_closeout_placeholder_record_args(closeout_text))
    errors.extend(check_closeout_force_record_examples(closeout_text))
    errors.extend(check_closeout_gate_matrix_sync(closeout_text))
    errors.extend(check_closeout_recommended_order(closeout_text))
    errors.extend(check_closeout_w6h_deploy_order(closeout_text))
    errors.extend(check_closeout_retro_prerequisites(closeout_text))
    errors.extend(check_closeout_retro_before_complete_check(closeout_text))
    errors.extend(
        check_closeout_evidence_check_before_retro_in_completion_criteria(
            closeout_text,
        )
    )
    errors.extend(check_closeout_report_json_fields(closeout_text))
    errors.extend(check_scope_boundary_docs())
    errors.extend(check_scope_gate_docs_sync())
    errors.extend(
        check_required_closeout_just_entries_documented(
            preflight_text=preflight_text,
            closeout_text=closeout_text,
        )
    )
    errors.extend(
        check_documented_closeout_just_entries(
            preflight_text=preflight_text,
            closeout_text=closeout_text,
            just_text=just_text,
        )
    )

    if "W6 evidence preflight" not in todo_text:
        errors.append(f"{TODO_DOC} 缺少 W6 evidence preflight 任务登记")

    if not just_has_entry(just_text, "wave-6-evidence-preflight"):
        errors.append("justfile 缺少入口: wave-6-evidence-preflight")

    gate_results = [
        check_gate(gate, preflight_text=preflight_text, just_text=just_text)
        for gate in GATES
    ]
    return errors, gate_results


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    top_errors, gate_results = collect_results()
    gate_errors = [error for result in gate_results for error in result.errors]
    errors = [*top_errors, *gate_errors]
    top_details = top_error_details(top_errors)
    gate_details = gate_error_details(gate_results)
    error_details = [*top_details, *gate_details]
    failed_gates = [result for result in gate_results if not result.ok]
    ok = not errors

    if args.json:
        print(json.dumps({
            "check": "check_wave6_evidence_preflight",
            "tier": "T1",
            "category": "流程治理",
            "ok": ok,
            "errors": errors,
            "error_count": len(errors),
            "top_errors": top_errors,
            "top_error_count": len(top_errors),
            "top_error_details": top_details,
            "gate_errors": gate_errors,
            "gate_error_count": len(gate_errors),
            "gate_error_details": gate_details,
            "error_details": error_details,
            "script": "check_wave6_evidence_preflight",
            "schema_version": SCHEMA_VERSION,
            "mode": PREFLIGHT_MODE,
            "writes_runtime_evidence": False,
            "closes_gate": False,
            "preflight_command": PREFLIGHT_COMMAND,
            "gate_count": len(GATES),
            "ok_gate_count": len(gate_results) - len(failed_gates),
            "failed_gate_count": len(failed_gates),
            "failed_gate_ids": [result.gate_id for result in failed_gates],
            "failed_gates": [asdict(result) for result in failed_gates],
            "evidence_gate_ids": gate_ids(),
            "evidence_gate_evidence_files": gate_evidence_files(),
            "evidence_gate_runbooks": gate_runbooks(),
            "evidence_gate_just_entries": gate_just_entries(),
            "evidence_gate_execution_files": gate_execution_file_map(),
            "required_top_level_files": list(REQUIRED_TOP_LEVEL_FILES),
            "required_runbooks": required_runbooks(),
            "required_execution_files": list(REQUIRED_EXECUTION_FILES),
            "overwrite_guard_execution_files": overwrite_guard_execution_files(),
            "overwrite_guard_required_markers": list(OVERWRITE_GUARD_REQUIRED_MARKERS),
            "gate_commands_by_phase": gate_commands_by_phase(),
            "validation_commands": validation_commands(),
            "closeout_just_entries": list(WAVE6_CLOSEOUT_JUST_ENTRIES),
            "gate_specs": [gate_spec_payload(gate) for gate in GATES],
            "gates": [asdict(result) for result in gate_results],
        }, ensure_ascii=False, indent=2))
    else:
        print("check_wave6_evidence_preflight (T1, 流程治理)")
        for result in gate_results:
            mark = "✓" if result.ok else "✘"
            print(f"  {mark} {result.gate_id}: {result.title}")
            for error in result.errors:
                print(f"    - {error}")
        if top_errors:
            print("  ✘ top-level:")
            for error in top_errors:
                print(f"    - {error}")
        if ok:
            print(
                "  ✓ Wave 6 evidence preflight 静态预检通过：链路完整；"
                "不会写入 runtime evidence，不能关闭 evidence gate"
            )
        else:
            print(
                "  ✘ Wave 6 evidence preflight 静态预检未通过："
                "不会写入 runtime evidence，不能关闭 evidence gate"
            )

    return 0 if ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
