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

from _wave6_evidence_preflight_specs import *  # noqa: F403


from _wave6_evidence_preflight_runbooks import *  # noqa: F403



















































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
