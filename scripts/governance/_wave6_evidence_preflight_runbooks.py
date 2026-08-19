"""Runbook checks for Wave 6 evidence preflight."""
import re
from pathlib import Path

from _wave6_evidence_preflight_specs import *  # noqa: F403


def is_record_command_entry(entry: str) -> bool:
    from check_wave6_evidence_preflight import is_record_command_entry as check

    return check(entry)


def _gates():
    from check_wave6_evidence_preflight import GATES

    return GATES

def repo_path(path: str) -> Path:
    from check_wave6_evidence_preflight import REPO_ROOT as root

    return root / path

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
    for gate in _gates():
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

    for gate in _gates():
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
